use cumulus_client_consensus_relay_chain::{
    build_relay_chain_consensus, BuildRelayChainConsensusParams,
};
use cumulus_client_network::build_block_announce_validator;
use cumulus_client_service::{
    prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use fc_rpc_core::types::{FilterPool, PendingTransactions};
use plasm_primitives::Block;
use plasm_runtime::RuntimeApi;
use polkadot_primitives::v0::CollatorPair;
use sc_client_api::client::BlockchainEvents;
use sc_service::{Configuration, PartialComponents, Role, TFullBackend, TFullClient, TaskManager};
use sp_core::Pair;
use sp_runtime::traits::BlakeTwo256;
use sp_trie::PrefixedMemoryDB;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

// Native executor instance.
sc_executor::native_executor_instance!(
    pub Executor,
    plasm_runtime::api::dispatch,
    plasm_runtime::native_version,
);

/*
pub fn open_frontier_backend(config: &Configuration) -> Result<Arc<fc_db::Backend<Block>>, String> {
    let config_dir = config.base_path.as_ref()
        .map(|base_path| base_path.config_dir(config.chain_spec.id()))
        .unwrap_or_else(|| {
            BasePath::from_project("", "", &crate::cli::Cli::executable_name())
                .config_dir(config.chain_spec.id())
        });
    let database_dir = config_dir.join("frontier").join("db");

    Ok(Arc::new(fc_db::Backend::<Block>::new(&fc_db::DatabaseSettings {
        source: fc_db::DatabaseSettingsSrc::RocksDb {
            path: database_dir,
            cache_size: 0,
        }
    })?))
}
*/

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial(
    config: &Configuration,
) -> Result<
    PartialComponents<
        TFullClient<Block, RuntimeApi, Executor>,
        TFullBackend<Block>,
        (),
        sp_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
        sc_transaction_pool::FullPool<Block, TFullClient<Block, RuntimeApi, Executor>>,
        (), //Arc<fc_db::Backend<Block>>,
    >,
    sc_service::Error,
> {
    let inherent_data_providers = sp_inherents::InherentDataProviders::new();

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
    let client = Arc::new(client);

    let registry = config.prometheus_registry();

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
    );

    //let frontier_backend = open_frontier_backend(config)?;

    let frontier_block_import = fc_consensus::FrontierBlockImport::new(
        client.clone(),
        client.clone(),
        //frontier_backend.clone(),
        true,
    );

    let import_queue = cumulus_client_consensus_relay_chain::import_queue(
        client.clone(),
        frontier_block_import,
        inherent_data_providers.clone(),
        &task_manager.spawn_essential_handle(),
        registry.clone(),
    )?;

    let params = PartialComponents {
        backend,
        client,
        import_queue,
        keystore_container,
        task_manager,
        transaction_pool,
        inherent_data_providers,
        select_chain: (),
        other: (),
    };

    Ok(params)
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
pub async fn start_node(
    parachain_config: Configuration,
    collator_key: CollatorPair,
    polkadot_config: Configuration,
    id: polkadot_primitives::v0::Id,
    validator: bool,
) -> sc_service::error::Result<(TaskManager, Arc<TFullClient<Block, RuntimeApi, Executor>>)> {
    if matches!(parachain_config.role, Role::Light) {
        return Err("Light client not supported!".into());
    }

    let parachain_config = prepare_node_config(parachain_config);

    let polkadot_full_node =
        cumulus_client_service::build_polkadot_full_node(polkadot_config, collator_key.public())
            .map_err(|e| match e {
                polkadot_service::Error::Sub(x) => x,
                s => format!("{}", s).into(),
            })?;

    let params = new_partial(&parachain_config)?;
    params
        .inherent_data_providers
        .register_provider(sp_timestamp::InherentDataProvider)
        .unwrap();

    let client = params.client.clone();
    let backend = params.backend.clone();
    let block_announce_validator = build_block_announce_validator(
        polkadot_full_node.client.clone(),
        id,
        Box::new(polkadot_full_node.network.clone()),
        polkadot_full_node.backend.clone(),
    );

    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let transaction_pool = params.transaction_pool.clone();
    let mut task_manager = params.task_manager;
    let import_queue = params.import_queue;
    let (network, network_status_sinks, system_rpc_tx, start_network) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &parachain_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: None,
            block_announce_validator_builder: Some(Box::new(|_| block_announce_validator)),
        })?;

    let pending_transactions: PendingTransactions = Some(Arc::new(Mutex::new(HashMap::new())));
    let filter_pool: Option<FilterPool> = Some(Arc::new(Mutex::new(BTreeMap::new())));
    //let frontier_backend = params.other;

    let rpc_extensions_builder = {
        let client = client.clone();
        let pool = transaction_pool.clone();
        let network = network.clone();
        let pending = pending_transactions.clone();
        let filter_pool = filter_pool.clone();
        let is_authority = parachain_config.role.is_authority();
        let builder = move |deny_unsafe, subscription| {
            let deps = plasm_rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
                network: network.clone(),
                pending_transactions: pending.clone(),
                filter_pool: filter_pool.clone(),
                //backend: frontier_backend.clone(),
                deny_unsafe,
                is_authority,
            };

            plasm_rpc::create_full(deps, subscription)
        };
        Box::new(builder)
    };

    let telemetry_span = sc_telemetry::TelemetrySpan::new();
    let _telemetry_span_entered = telemetry_span.enter();
    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        on_demand: None,
        remote_blockchain: None,
        rpc_extensions_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: params.keystore_container.sync_keystore(),
        backend: backend.clone(),
        network: network.clone(),
        network_status_sinks,
        system_rpc_tx,
        telemetry_span: Some(telemetry_span.clone()),
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

    let announce_block = {
        let network = network.clone();
        Arc::new(move |hash, data| network.announce_block(hash, Some(data)))
    };

    if validator {
        let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool,
            prometheus_registry.as_ref(),
        );
        let spawner = task_manager.spawn_handle();

        let parachain_consensus = build_relay_chain_consensus(BuildRelayChainConsensusParams {
            para_id: id,
            proposer_factory,
            inherent_data_providers: params.inherent_data_providers,
            block_import: client.clone(),
            relay_chain_client: polkadot_full_node.client.clone(),
            relay_chain_backend: polkadot_full_node.backend.clone(),
        });

        let params = StartCollatorParams {
            para_id: id,
            block_status: client.clone(),
            announce_block,
            client: client.clone(),
            task_manager: &mut task_manager,
            collator_key,
            relay_chain_full_node: polkadot_full_node,
            spawner,
            backend,
            parachain_consensus,
        };

        start_collator(params).await?;
    } else {
        let params = StartFullNodeParams {
            client: client.clone(),
            announce_block,
            task_manager: &mut task_manager,
            para_id: id,
            polkadot_full_node,
        };

        start_full_node(params)?;
    }

    start_network.start_network();

    Ok((task_manager, client))
}
