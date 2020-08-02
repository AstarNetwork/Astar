#![warn(unused_extern_crates)]

//! Service implementation. Specialized wrapper over substrate service.

use plasm_primitives::Block;
use plasm_runtime::RuntimeApi;
use sc_client_api::{ExecutorProvider, RemoteBackend};
use sc_consensus_babe;
use sc_finality_grandpa::{
    self as grandpa, FinalityProofProvider as GrandpaFinalityProofProvider, StorageAndProofProvider,
};
use sc_network::NetworkService;
use sc_service::{
    config::Configuration, error::Error as ServiceError, RpcHandlers, ServiceComponents,
    TaskManager,
};
use sp_core::traits::BareCryptoStorePtr;
use sp_inherents::InherentDataProviders;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

sc_executor::native_executor_instance!(
    pub Executor,
    plasm_runtime::api::dispatch,
    plasm_runtime::native_version,
    plasm_runtime::legacy::storage::HostFunctions,
);

type FullClient = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullGrandpaBlockImport =
    grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;
type LightClient = sc_service::TLightClient<Block, RuntimeApi, Executor>;

pub fn new_full_params(
    config: Configuration,
) -> Result<
    (
        sc_service::ServiceParams<
            Block,
            FullClient,
            sc_consensus_babe::BabeImportQueue<Block, FullClient>,
            sc_transaction_pool::FullPool<Block, FullClient>,
            plasm_rpc::IoHandler,
            FullBackend,
        >,
        (
            sc_consensus_babe::BabeBlockImport<Block, FullClient, FullGrandpaBlockImport>,
            grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
            sc_consensus_babe::BabeLink<Block>,
        ),
        grandpa::SharedVoterState,
        FullSelectChain,
        InherentDataProviders,
    ),
    ServiceError,
> {
    let (client, backend, keystore, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
    let client = Arc::new(client);

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let pool_api =
        sc_transaction_pool::FullChainApi::new(client.clone(), config.prometheus_registry());
    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        std::sync::Arc::new(pool_api),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
    );

    let (grandpa_block_import, grandpa_link) = grandpa::block_import(
        client.clone(),
        &(client.clone() as Arc<_>),
        select_chain.clone(),
    )?;
    let justification_import = grandpa_block_import.clone();

    let (block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::Config::get_or_compute(&*client)?,
        grandpa_block_import,
        client.clone(),
    )?;

    let inherent_data_providers = sp_inherents::InherentDataProviders::new();

    let import_queue = sc_consensus_babe::import_queue(
        babe_link.clone(),
        block_import.clone(),
        Some(Box::new(justification_import)),
        None,
        client.clone(),
        select_chain.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
    )?;

    let import_setup = (block_import, grandpa_link, babe_link);

    let (rpc_extensions_builder, rpc_setup) = {
        let (_, grandpa_link, babe_link) = &import_setup;

        let shared_authority_set = grandpa_link.shared_authority_set().clone();
        let shared_voter_state = grandpa::SharedVoterState::empty();

        let rpc_setup = shared_voter_state.clone();

        let babe_config = babe_link.config().clone();
        let shared_epoch_changes = babe_link.epoch_changes().clone();

        let client = client.clone();
        let pool = transaction_pool.clone();
        let select_chain = select_chain.clone();
        let keystore = keystore.clone();

        let rpc_extensions_builder = Box::new(move |deny_unsafe| {
            let deps = plasm_rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
                select_chain: select_chain.clone(),
                deny_unsafe,
                babe: plasm_rpc::BabeDeps {
                    babe_config: babe_config.clone(),
                    shared_epoch_changes: shared_epoch_changes.clone(),
                    keystore: keystore.clone(),
                },
                grandpa: plasm_rpc::GrandpaDeps {
                    shared_voter_state: shared_voter_state.clone(),
                    shared_authority_set: shared_authority_set.clone(),
                },
            };

            plasm_rpc::create_full(deps)
        });

        (rpc_extensions_builder, rpc_setup)
    };

    let provider = client.clone() as Arc<dyn grandpa::StorageAndProofProvider<_, _>>;
    let finality_proof_provider = Arc::new(grandpa::FinalityProofProvider::new(
        backend.clone(),
        provider,
    ));

    let params = sc_service::ServiceParams {
        config,
        backend,
        client,
        import_queue,
        keystore,
        task_manager,
        rpc_extensions_builder,
        transaction_pool,
        block_announce_validator_builder: None,
        finality_proof_request_builder: None,
        finality_proof_provider: Some(finality_proof_provider),
        on_demand: None,
        remote_blockchain: None,
    };

    Ok((
        params,
        import_setup,
        rpc_setup,
        select_chain,
        inherent_data_providers,
    ))
}

/// Creates a full service from the configuration.
pub fn new_full_base(
    config: Configuration,
    with_startup_data: impl FnOnce(
        &sc_consensus_babe::BabeBlockImport<Block, FullClient, FullGrandpaBlockImport>,
        &sc_consensus_babe::BabeLink<Block>,
    ),
) -> Result<
    (
        TaskManager,
        InherentDataProviders,
        Arc<FullClient>,
        Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
        Arc<sc_transaction_pool::FullPool<Block, FullClient>>,
    ),
    ServiceError,
> {
    let (params, import_setup, rpc_setup, select_chain, inherent_data_providers) =
        new_full_params(config)?;

    let (
        role,
        force_authoring,
        name,
        enable_grandpa,
        prometheus_registry,
        client,
        transaction_pool,
        keystore,
    ) = {
        let sc_service::ServiceParams {
            config,
            client,
            transaction_pool,
            keystore,
            ..
        } = &params;

        (
            config.role.clone(),
            config.force_authoring,
            config.network.node_name.clone(),
            !config.disable_grandpa,
            config.prometheus_registry().cloned(),
            client.clone(),
            transaction_pool.clone(),
            keystore.clone(),
        )
    };

    let ServiceComponents {
        task_manager,
        network,
        telemetry_on_connect_sinks,
        ..
    } = sc_service::build(params)?;

    let (block_import, grandpa_link, babe_link) = import_setup;
    let shared_voter_state = rpc_setup;

    (with_startup_data)(&block_import, &babe_link);

    if let sc_service::config::Role::Authority { .. } = &role {
        let proposer = sc_basic_authorship::ProposerFactory::new(
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
        );

        let can_author_with =
            sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

        let babe_config = sc_consensus_babe::BabeParams {
            keystore: keystore.clone(),
            client: client.clone(),
            select_chain,
            env: proposer,
            block_import,
            sync_oracle: network.clone(),
            inherent_data_providers: inherent_data_providers.clone(),
            force_authoring,
            babe_link,
            can_author_with,
        };

        let babe = sc_consensus_babe::start_babe(babe_config)?;
        task_manager
            .spawn_essential_handle()
            .spawn_blocking("babe-proposer", babe);
    }

    // if the node isn't actively participating in consensus then it doesn't
    // need a keystore, regardless of which protocol we use below.
    let keystore = if role.is_authority() {
        Some(keystore as BareCryptoStorePtr)
    } else {
        None
    };

    let config = grandpa::Config {
        // FIXME #1578 make this available through chainspec
        gossip_duration: std::time::Duration::from_millis(333),
        justification_period: 512,
        name: Some(name),
        observer_enabled: false,
        keystore,
        is_authority: role.is_network_authority(),
    };

    if enable_grandpa {
        // start the full GRANDPA voter
        // NOTE: non-authorities could run the GRANDPA observer protocol, but at
        // this point the full voter should provide better guarantees of block
        // and vote data availability than the observer. The observer has not
        // been tested extensively yet and having most nodes in a network run it
        // could lead to finality stalls.
        let grandpa_config = grandpa::GrandpaParams {
            config,
            link: grandpa_link,
            network: network.clone(),
            inherent_data_providers: inherent_data_providers.clone(),
            telemetry_on_connect: Some(telemetry_on_connect_sinks.on_connect_stream()),
            voting_rule: grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry,
            shared_voter_state,
        };

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        task_manager
            .spawn_essential_handle()
            .spawn_blocking("grandpa-voter", grandpa::run_grandpa_voter(grandpa_config)?);
    } else {
        grandpa::setup_disabled_grandpa(client.clone(), &inherent_data_providers, network.clone())?;
    }

    Ok((
        task_manager,
        inherent_data_providers,
        client,
        network,
        transaction_pool,
    ))
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
    new_full_base(config, |_, _| ()).map(|(task_manager, _, _, _, _)| task_manager)
}

pub fn new_light_base(
    config: Configuration,
) -> Result<
    (
        TaskManager,
        Arc<RpcHandlers>,
        Arc<LightClient>,
        Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
        Arc<
            sc_transaction_pool::LightPool<Block, LightClient, sc_network::config::OnDemand<Block>>,
        >,
    ),
    ServiceError,
> {
    let (client, backend, keystore, task_manager, on_demand) =
        sc_service::new_light_parts::<Block, RuntimeApi, Executor>(&config)?;

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool_api = Arc::new(sc_transaction_pool::LightChainApi::new(
        client.clone(),
        on_demand.clone(),
    ));
    let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
        config.transaction_pool.clone(),
        transaction_pool_api,
        config.prometheus_registry(),
        task_manager.spawn_handle(),
    ));

    let grandpa_block_import = grandpa::light_block_import(
        client.clone(),
        backend.clone(),
        &(client.clone() as Arc<_>),
        Arc::new(on_demand.checker().clone()),
    )?;

    let finality_proof_import = grandpa_block_import.clone();
    let finality_proof_request_builder =
        finality_proof_import.create_finality_proof_request_builder();

    let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::Config::get_or_compute(&*client)?,
        grandpa_block_import,
        client.clone(),
    )?;

    let inherent_data_providers = sp_inherents::InherentDataProviders::new();

    let import_queue = sc_consensus_babe::import_queue(
        babe_link,
        babe_block_import,
        None,
        Some(Box::new(finality_proof_import)),
        client.clone(),
        select_chain.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
    )?;

    // GenesisAuthoritySetProvider is implemented for StorageAndProofProvider
    let provider = client.clone() as Arc<dyn StorageAndProofProvider<_, _>>;
    let finality_proof_provider =
        Arc::new(GrandpaFinalityProofProvider::new(backend.clone(), provider));

    let light_deps = plasm_rpc::LightDeps {
        remote_blockchain: backend.remote_blockchain(),
        fetcher: on_demand.clone(),
        client: client.clone(),
        pool: transaction_pool.clone(),
    };

    let rpc_extensions = plasm_rpc::create_light(light_deps);

    let ServiceComponents {
        task_manager,
        rpc_handlers,
        network,
        ..
    } = sc_service::build(sc_service::ServiceParams {
        block_announce_validator_builder: None,
        finality_proof_request_builder: Some(finality_proof_request_builder),
        finality_proof_provider: Some(finality_proof_provider),
        on_demand: Some(on_demand),
        remote_blockchain: Some(backend.remote_blockchain()),
        rpc_extensions_builder: Box::new(sc_service::NoopRpcExtensionBuilder(rpc_extensions)),
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        config,
        import_queue,
        keystore,
        backend,
        task_manager,
    })?;

    Ok((
        task_manager,
        rpc_handlers,
        client,
        network,
        transaction_pool,
    ))
}

/// Builds a new service for a light client.
pub fn new_light(config: Configuration) -> Result<TaskManager, ServiceError> {
    new_light_base(config).map(|(task_manager, _, _, _, _)| task_manager)
}
