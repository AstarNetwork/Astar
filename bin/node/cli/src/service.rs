#![warn(unused_extern_crates)]

//! Service implementation. Specialized wrapper over substrate service.

use std::sync::Arc;

use sp_runtime::traits::Block as BlockT;
use sp_inherents::InherentDataProviders;
use sc_consensus_babe;
use sc_client::LongestChain;
use sc_client_api::ExecutorProvider;
use sc_finality_grandpa::{
    StorageAndProofProvider,
    FinalityProofProvider as GrandpaFinalityProofProvider,
};
use sc_service::{
    AbstractService, ServiceBuilder, config::Configuration, error::{Error as ServiceError},
};
use sc_service::{Service, NetworkStatus};
use sc_client::{Client, LocalCallExecutor};
use sc_client_db::Backend;
use sc_network::NetworkService;
use sc_offchain::OffchainWorkers;
use plasm_executor::NativeExecutor;
use plasm_primitives::Block;
use plasm_runtime::RuntimeApi;

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
macro_rules! new_full_start {
    ($config:expr) => {{
        type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;
        let mut import_setup = None;
        let inherent_data_providers = sp_inherents::InherentDataProviders::new();

        let builder = sc_service::ServiceBuilder::new_full::<
            plasm_primitives::Block, plasm_runtime::RuntimeApi, plasm_executor::Executor
        >($config)?
            .with_select_chain(|_config, backend| {
                Ok(sc_client::LongestChain::new(backend.clone()))
            })?
            .with_transaction_pool(|config, client, _fetcher| {
                let pool_api = sc_transaction_pool::FullChainApi::new(client.clone());
                Ok(sc_transaction_pool::BasicPool::new(config, std::sync::Arc::new(pool_api)))
            })?
            .with_import_queue(|_config, client, mut select_chain, _transaction_pool| {
                let select_chain = select_chain.take()
                    .ok_or_else(|| sc_service::Error::SelectChainRequired)?;
                let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
                    client.clone(),
                    &(client.clone() as std::sync::Arc<_>),
                    select_chain,
                )?;
                let justification_import = grandpa_block_import.clone();

                let (block_import, babe_link) = sc_consensus_babe::block_import(
                    sc_consensus_babe::Config::get_or_compute(&*client)?,
                    grandpa_block_import,
                    client.clone(),
                )?;

                let import_queue = sc_consensus_babe::import_queue(
                    babe_link.clone(),
                    block_import.clone(),
                    Some(Box::new(justification_import)),
                    None,
                    client,
                    inherent_data_providers.clone(),
                )?;

                import_setup = Some((block_import, grandpa_link, babe_link));
                Ok(import_queue)
            })?
            .with_rpc_extensions(|builder| -> Result<RpcExtension, _> {
                let babe_link = import_setup.as_ref().map(|s| &s.2)
                    .expect("BabeLink is present for full services or set up failed; qed.");
                let deps = plasm_rpc::FullDeps {
                    client: builder.client().clone(),
                    pool: builder.pool(),
                    select_chain: builder.select_chain().cloned()
                        .expect("SelectChain is present for full services or set up failed; qed."),
                    babe: plasm_rpc::BabeDeps {
                        keystore: builder.keystore(),
                        babe_config: sc_consensus_babe::BabeLink::config(babe_link).clone(),
                        shared_epoch_changes: sc_consensus_babe::BabeLink::epoch_changes(babe_link).clone()
                    }
                };
                Ok(plasm_rpc::create_full(deps))
            })?;

        (builder, import_setup, inherent_data_providers)
    }}
}

/// Creates a full service from the configuration.
///
/// We need to use a macro because the test suit doesn't work with an opaque service. It expects
/// concrete types instead.
macro_rules! new_full {
    ($config:expr, $with_startup_data: expr) => {{
        let (
            is_authority,
            force_authoring,
            name,
            disable_grandpa,
        ) = (
            $config.roles.is_authority(),
            $config.force_authoring,
            $config.name.clone(),
            $config.disable_grandpa,
        );

        // sentry nodes announce themselves as authorities to the network
        // and should run the same protocols authorities do, but it should
        // never actively participate in any consensus process.
        let participates_in_consensus = is_authority && !$config.sentry_mode;

        let (builder, mut import_setup, inherent_data_providers) = new_full_start!($config);

        let service = builder
            .with_finality_proof_provider(|client, backend| {
                let provider = client as Arc<dyn StorageAndProofProvider<_, _>>;
                Ok(Arc::new(GrandpaFinalityProofProvider::new(backend, provider)) as _)
            })?
            .build()?;

        let (block_import, grandpa_link, babe_link) = import_setup.take()
                .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

        ($with_startup_data)(&block_import, &babe_link);

        if participates_in_consensus {
            let proposer = sc_basic_authorship::ProposerFactory::new(
                service.client(),
                service.transaction_pool(),
            );

            let client = service.client();
            let select_chain = service.select_chain()
                .ok_or(sc_service::Error::SelectChainRequired)?;

            let can_author_with =
                sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

            let babe_config = sc_consensus_babe::BabeParams {
                keystore: service.keystore(),
                client,
                select_chain,
                env: proposer,
                block_import,
                sync_oracle: service.network(),
                inherent_data_providers: inherent_data_providers.clone(),
                force_authoring,
                babe_link,
                can_author_with,
            };

            let babe = sc_consensus_babe::start_babe(babe_config)?;
            service.spawn_essential_task("babe-proposer", babe);
        }

        // if the node isn't actively participating in consensus then it doesn't
        // need a keystore, regardless of which protocol we use below.
        let keystore = if participates_in_consensus {
            Some(service.keystore())
        } else {
            None
        };

        let config = sc_finality_grandpa::Config {
            // FIXME #1578 make this available through chainspec
            gossip_duration: std::time::Duration::from_millis(333),
            justification_period: 512,
            name: Some(name),
            observer_enabled: false,
            keystore,
            is_authority,
        };

        let enable_grandpa = !disable_grandpa;
        if enable_grandpa {
            // start the full GRANDPA voter
            // NOTE: non-authorities could run the GRANDPA observer protocol, but at
            // this point the full voter should provide better guarantees of block
            // and vote data availability than the observer. The observer has not
            // been tested extensively yet and having most nodes in a network run it
            // could lead to finality stalls.
            let grandpa_config = sc_finality_grandpa::GrandpaParams {
                config,
                link: grandpa_link,
                network: service.network(),
                inherent_data_providers: inherent_data_providers.clone(),
                telemetry_on_connect: Some(service.telemetry_on_connect_stream()),
                voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
                prometheus_registry: service.prometheus_registry(),
            };

            // the GRANDPA voter task is considered infallible, i.e.
            // if it fails we take down the service with it.
            service.spawn_essential_task(
                "grandpa-voter",
                sc_finality_grandpa::run_grandpa_voter(grandpa_config)?
            );
        } else {
            sc_finality_grandpa::setup_disabled_grandpa(
                service.client(),
                &inherent_data_providers,
                service.network(),
            )?;
        }

        Ok((service, inherent_data_providers))
    }};
    ($config:expr) => {{
        new_full!($config, |_, _| {})
    }}
}

type ConcreteBlock = plasm_primitives::Block;
type ConcreteClient =
    Client<
        Backend<ConcreteBlock>,
        LocalCallExecutor<Backend<ConcreteBlock>,
        NativeExecutor<plasm_executor::Executor>>,
        ConcreteBlock,
        plasm_runtime::RuntimeApi
    >;
type ConcreteBackend = Backend<ConcreteBlock>;
type ConcreteTransactionPool = sc_transaction_pool::BasicPool<
    sc_transaction_pool::FullChainApi<ConcreteClient, ConcreteBlock>,
    ConcreteBlock
>;

/// Builds a new service for a full client.
pub fn new_full(config: Configuration)
-> Result<
    Service<
        ConcreteBlock,
        ConcreteClient,
        LongestChain<ConcreteBackend, ConcreteBlock>,
        NetworkStatus<ConcreteBlock>,
        NetworkService<ConcreteBlock, <ConcreteBlock as BlockT>::Hash>,
        ConcreteTransactionPool,
        OffchainWorkers<
            ConcreteClient,
            <ConcreteBackend as sc_client_api::backend::Backend<Block>>::OffchainStorage,
            ConcreteBlock,
        >
    >,
    ServiceError,
>
{
    new_full!(config).map(|(service, _)| service)
}

/// Builds a new service for a light client.
pub fn new_light(config: Configuration)
-> Result<impl AbstractService, ServiceError> {
    type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;
    let inherent_data_providers = InherentDataProviders::new();

    let service = ServiceBuilder::new_light::<Block, RuntimeApi, plasm_executor::Executor>(config)?
        .with_select_chain(|_config, backend| {
            Ok(LongestChain::new(backend.clone()))
        })?
        .with_transaction_pool(|config, client, fetcher| {
            let fetcher = fetcher
                .ok_or_else(|| "Trying to start light transaction pool without active fetcher")?;
            let pool_api = sc_transaction_pool::LightChainApi::new(client.clone(), fetcher.clone());
            let pool = sc_transaction_pool::BasicPool::with_revalidation_type(
                config, Arc::new(pool_api), sc_transaction_pool::RevalidationType::Light,
            );
            Ok(pool)
        })?
        .with_import_queue_and_fprb(|_config, client, backend, fetcher, _select_chain, _tx_pool| {
            let fetch_checker = fetcher
                .map(|fetcher| fetcher.checker().clone())
                .ok_or_else(|| "Trying to start light import queue without active fetch checker")?;
            let grandpa_block_import = sc_finality_grandpa::light_block_import(
                client.clone(),
                backend,
                &(client.clone() as Arc<_>),
                Arc::new(fetch_checker),
            )?;

            let finality_proof_import = grandpa_block_import.clone();
            let finality_proof_request_builder =
                finality_proof_import.create_finality_proof_request_builder();

            let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
                sc_consensus_babe::Config::get_or_compute(&*client)?,
                grandpa_block_import,
                client.clone(),
            )?;

            let import_queue = sc_consensus_babe::import_queue(
                babe_link,
                babe_block_import,
                None,
                Some(Box::new(finality_proof_import)),
                client.clone(),
                inherent_data_providers.clone(),
            )?;

            Ok((import_queue, finality_proof_request_builder))
        })?
        .with_finality_proof_provider(|client, backend| {
            let provider = client as Arc<dyn StorageAndProofProvider<_, _>>;
            Ok(Arc::new(GrandpaFinalityProofProvider::new(backend, provider)) as _)
        })?
        .with_rpc_extensions(|builder,| ->
            Result<RpcExtension, _>
        {
            let fetcher = builder.fetcher()
                .ok_or_else(|| "Trying to start node RPC without active fetcher")?;
            let remote_blockchain = builder.remote_backend()
                .ok_or_else(|| "Trying to start node RPC without active remote blockchain")?;

            let light_deps = plasm_rpc::LightDeps {
                remote_blockchain,
                fetcher,
                client: builder.client().clone(),
                pool: builder.pool(),
            };
            Ok(plasm_rpc::create_light(light_deps))
        })?
        .build()?;

    Ok(service)
}
