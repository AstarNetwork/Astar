//! Service implementation. Specialized wrapper over substrate service.

use fc_consensus::FrontierBlockImport;
use fc_rpc_core::types::{FilterPool, PendingTransactions};
use plasm_primitives::Block;
use plasm_runtime::RuntimeApi;
use sc_client_api::{BlockchainEvents, ExecutorProvider, RemoteBackend};
use sc_finality_grandpa::{self as grandpa, FinalityProofProvider as GrandpaFinalityProofProvider};
use sc_network::NetworkService;
use sc_service::{config::Configuration, error::Error as ServiceError, RpcHandlers, TaskManager};
use sp_inherents::InherentDataProviders;
use sp_runtime::traits::Block as BlockT;
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

sc_executor::native_executor_instance!(
    pub Executor,
    plasm_runtime::api::dispatch,
    plasm_runtime::native_version,
);

type FullClient = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullGrandpaBlockImport =
    grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;
type LightClient = sc_service::TLightClient<Block, RuntimeApi, Executor>;

pub fn new_partial(
    config: &Configuration,
) -> Result<
    sc_service::PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        sp_consensus::DefaultImportQueue<Block, FullClient>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        (
            sc_consensus_babe::BabeBlockImport<
                Block,
                FullClient,
                FrontierBlockImport<Block, FullGrandpaBlockImport, FullClient>,
            >,
            grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
            sc_consensus_babe::BabeLink<Block>,
        ),
    >,
    ServiceError,
> {
    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
    let client = Arc::new(client);

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
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

    let frontier_block_import =
        FrontierBlockImport::new(grandpa_block_import.clone(), client.clone(), true);

    let (block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::Config::get_or_compute(&*client)?,
        frontier_block_import,
        client.clone(),
    )?;

    let inherent_data_providers = sp_inherents::InherentDataProviders::new();

    let import_queue = sc_consensus_babe::import_queue(
        babe_link.clone(),
        block_import.clone(),
        Some(Box::new(justification_import)),
        client.clone(),
        select_chain.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
        sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
    )?;

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        keystore_container,
        select_chain,
        import_queue,
        transaction_pool,
        inherent_data_providers,
        other: (block_import, grandpa_link, babe_link),
    })
}

/// Creates a full service from the configuration.
pub fn new_full_base(
    config: Configuration,
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
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        inherent_data_providers,
        other: import_setup,
    } = new_partial(&config)?;

    let (block_import, grandpa_link, babe_link) = import_setup;

    let pending_transactions: PendingTransactions = Some(Arc::new(Mutex::new(HashMap::new())));

    let filter_pool: Option<FilterPool> = Some(Arc::new(Mutex::new(BTreeMap::new())));

    let justification_stream = grandpa_link.justification_stream();
    let shared_authority_set = grandpa_link.shared_authority_set().clone();
    let shared_voter_state = grandpa::SharedVoterState::empty();
    let finality_proof_provider = GrandpaFinalityProofProvider::new_for_service(
        backend.clone(),
        Some(shared_authority_set.clone()),
    );

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: None,
            block_announce_validator_builder: None,
        })?;

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(
            &config,
            backend.clone(),
            task_manager.spawn_handle(),
            client.clone(),
            network.clone(),
        );
    }

    let role = config.role.clone();
    let is_authority = role.is_authority();
    let force_authoring = config.force_authoring;
    let backoff_authoring_blocks =
        Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
    let name = config.network.node_name.clone();
    let enable_grandpa = !config.disable_grandpa;
    let prometheus_registry = config.prometheus_registry().cloned();

    let rpc_extensions_builder = {
        let babe_config = babe_link.config().clone();
        let shared_epoch_changes = babe_link.epoch_changes().clone();
        let shared_voter_state = shared_voter_state.clone();

        let client = client.clone();
        let pool = transaction_pool.clone();
        let select_chain = select_chain.clone();
        let keystore = keystore_container.sync_keystore();
        let network = network.clone();
        let pending = pending_transactions.clone();
        let filter_pool = filter_pool.clone();

        move |deny_unsafe, subscription_executor: sc_rpc::SubscriptionTaskExecutor| {
            let deps = plasm_rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
                select_chain: select_chain.clone(),
                deny_unsafe,
                is_authority,
                network: network.clone(),
                pending_transactions: pending.clone(),
                filter_pool: filter_pool.clone(),
                babe: plasm_rpc::BabeDeps {
                    babe_config: babe_config.clone(),
                    shared_epoch_changes: shared_epoch_changes.clone(),
                    keystore: keystore.clone(),
                },
                grandpa: plasm_rpc::GrandpaDeps {
                    shared_voter_state: shared_voter_state.clone(),
                    shared_authority_set: shared_authority_set.clone(),
                    justification_stream: justification_stream.clone(),
                    subscription_executor: subscription_executor.clone(),
                    finality_provider: finality_proof_provider.clone(),
                },
            };

            plasm_rpc::create_full(deps, subscription_executor)
        }
    };

    let (_rpc_handlers, telemetry_connection_notifier) =
        sc_service::spawn_tasks(sc_service::SpawnTasksParams {
            config,
            backend: backend.clone(),
            client: client.clone(),
            keystore: keystore_container.sync_keystore(),
            network: network.clone(),
            rpc_extensions_builder: Box::new(rpc_extensions_builder),
            transaction_pool: transaction_pool.clone(),
            task_manager: &mut task_manager,
            on_demand: None,
            remote_blockchain: None,
            network_status_sinks,
            system_rpc_tx,
        })?;

    // Spawn Frontier EthFilterApi maintenance task.
    if filter_pool.is_some() {
        use futures::StreamExt;
        // Each filter is allowed to stay in the pool for 100 blocks.
        const FILTER_RETAIN_THRESHOLD: u64 = 100;
        task_manager.spawn_essential_handle().spawn(
            "frontier-filter-pool",
            client
                .import_notification_stream()
                .for_each(move |notification| {
                    if let Ok(locked) = &mut filter_pool.clone().unwrap().lock() {
                        let imported_number: u64 = notification.header.number as u64;
                        for (k, v) in locked.clone().iter() {
                            let lifespan_limit = v.at_block + FILTER_RETAIN_THRESHOLD;
                            if lifespan_limit <= imported_number {
                                locked.remove(&k);
                            }
                        }
                    }
                    futures::future::ready(())
                }),
        );
    }

    // Spawn Frontier pending transactions maintenance task (as essential, otherwise we leak).
    if pending_transactions.is_some() {
        use fp_consensus::{ConsensusLog, FRONTIER_ENGINE_ID};
        use futures::StreamExt;
        use sp_runtime::generic::OpaqueDigestItemId;

        const TRANSACTION_RETAIN_THRESHOLD: u64 = 5;
        task_manager.spawn_essential_handle().spawn(
            "frontier-pending-transactions",
            client
                .import_notification_stream()
                .for_each(move |notification| {
                    if let Ok(locked) = &mut pending_transactions.clone().unwrap().lock() {
                        // As pending transactions have a finite lifespan anyway
                        // we can ignore MultiplePostRuntimeLogs error checks.
                        let mut frontier_log: Option<_> = None;
                        for log in notification.header.digest.logs {
                            let log = log.try_to::<ConsensusLog>(OpaqueDigestItemId::Consensus(
                                &FRONTIER_ENGINE_ID,
                            ));
                            if let Some(log) = log {
                                frontier_log = Some(log);
                            }
                        }

                        let imported_number: u64 = notification.header.number as u64;

                        if let Some(ConsensusLog::EndBlock {
                            block_hash: _,
                            transaction_hashes,
                        }) = frontier_log
                        {
                            // Retain all pending transactions that were not
                            // processed in the current block.
                            locked.retain(|&k, _| !transaction_hashes.contains(&k));
                        }
                        locked.retain(|_, v| {
                            // Drop all the transactions that exceeded the given lifespan.
                            let lifespan_limit = v.at_block + TRANSACTION_RETAIN_THRESHOLD;
                            lifespan_limit > imported_number
                        });
                    }
                    futures::future::ready(())
                }),
        );
    }

    if let sc_service::config::Role::Authority { .. } = &role {
        let proposer = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
        );

        let can_author_with =
            sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

        let babe_config = sc_consensus_babe::BabeParams {
            keystore: keystore_container.sync_keystore(),
            client: client.clone(),
            select_chain,
            env: proposer,
            block_import,
            sync_oracle: network.clone(),
            inherent_data_providers: inherent_data_providers.clone(),
            force_authoring,
            backoff_authoring_blocks,
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
        Some(keystore_container.sync_keystore())
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
            telemetry_on_connect: telemetry_connection_notifier.map(|x| x.on_connect_stream()),
            voting_rule: grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry,
            shared_voter_state,
        };

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        task_manager
            .spawn_essential_handle()
            .spawn_blocking("grandpa-voter", grandpa::run_grandpa_voter(grandpa_config)?);
    }

    network_starter.start_network();
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
    new_full_base(config).map(|(task_manager, _, _, _, _)| task_manager)
}

pub fn new_light_base(
    config: Configuration,
) -> Result<
    (
        TaskManager,
        RpcHandlers,
        Arc<LightClient>,
        Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
        Arc<
            sc_transaction_pool::LightPool<Block, LightClient, sc_network::config::OnDemand<Block>>,
        >,
    ),
    ServiceError,
> {
    let (client, backend, keystore_container, mut task_manager, on_demand) =
        sc_service::new_light_parts::<Block, RuntimeApi, Executor>(&config)?;

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
        config.transaction_pool.clone(),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
        on_demand.clone(),
    ));

    let (grandpa_block_import, _) = grandpa::block_import(
        client.clone(),
        &(client.clone() as Arc<_>),
        select_chain.clone(),
    )?;

    let finality_proof_import = grandpa_block_import.clone();

    let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::Config::get_or_compute(&*client)?,
        grandpa_block_import,
        client.clone(),
    )?;

    let inherent_data_providers = sp_inherents::InherentDataProviders::new();

    let import_queue = sc_consensus_babe::import_queue(
        babe_link,
        babe_block_import,
        Some(Box::new(finality_proof_import)),
        client.clone(),
        select_chain.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
        sp_consensus::NeverCanAuthor,
    )?;

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: Some(on_demand.clone()),
            block_announce_validator_builder: None,
        })?;
    network_starter.start_network();

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(
            &config,
            backend.clone(),
            task_manager.spawn_handle(),
            client.clone(),
            network.clone(),
        );
    }

    let light_deps = plasm_rpc::LightDeps {
        remote_blockchain: backend.remote_blockchain(),
        fetcher: on_demand.clone(),
        client: client.clone(),
        pool: transaction_pool.clone(),
    };

    let rpc_extensions = plasm_rpc::create_light(light_deps);

    let (rpc_handlers, _telemetry_connection_notifier) =
        sc_service::spawn_tasks(sc_service::SpawnTasksParams {
            on_demand: Some(on_demand),
            remote_blockchain: Some(backend.remote_blockchain()),
            rpc_extensions_builder: Box::new(sc_service::NoopRpcExtensionBuilder(rpc_extensions)),
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            config,
            keystore: keystore_container.sync_keystore(),
            backend,
            network_status_sinks,
            system_rpc_tx,
            network: network.clone(),
            task_manager: &mut task_manager,
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
