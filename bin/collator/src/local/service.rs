// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

//! Local Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use crate::{
    evm_tracing_types::{EthApi as EthApiCmd, FrontierConfig},
    rpc::tracing,
};
use cumulus_client_parachain_inherent::MockValidationDataInherentDataProvider;
use cumulus_primitives_core::{CollectCollationInfo, ParaId};
use fc_consensus::FrontierBlockImport;
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use fc_storage::StorageOverrideHandler;
use futures::{FutureExt, StreamExt};
use parity_scale_codec::Encode;
use polkadot_primitives::UpgradeGoAhead;
use sc_client_api::{Backend, BlockchainEvents};
use sc_consensus_manual_seal::consensus::aura::AuraConsensusDataProvider;
use sc_executor::{HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::NetworkBackend;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, UniqueSaturatedInto};
use std::{collections::BTreeMap, ops::Sub, sync::Arc, time::Duration};

use astar_primitives::parachain::SHIBUYA_ID;
use astar_primitives::*;

const RELAY_CHAIN_SLOT_DURATION_MILLIS: u64 = 2_000;
const DEV_BLOCK_TIME_MS: u64 = 2_000;

/// Parachain host functions
#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions = (
    frame_benchmarking::benchmarking::HostFunctions,
    cumulus_client_service::ParachainHostFunctions,
    moonbeam_primitives_ext::moonbeam_ext::HostFunctions,
);

/// Parachain host functions
#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = (
    cumulus_client_service::ParachainHostFunctions,
    moonbeam_primitives_ext::moonbeam_ext::HostFunctions,
);

type ParachainExecutor = WasmExecutor<HostFunctions>;

type FullClient = sc_service::TFullClient<
    Block,
    crate::parachain::fake_runtime_api::RuntimeApi,
    ParachainExecutor,
>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

/// Build a partial chain component config
pub fn new_partial(
    config: &Configuration,
    evm_tracing_config: &FrontierConfig,
) -> Result<
    sc_service::PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        sc_consensus::DefaultImportQueue<Block>,
        sc_transaction_pool::TransactionPoolHandle<Block, FullClient>,
        (
            FrontierBlockImport<Block, Arc<FullClient>, FullClient>,
            Option<Telemetry>,
            Arc<fc_db::Backend<Block, FullClient>>,
        ),
    >,
    ServiceError,
> {
    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    let heap_pages = config
        .executor
        .default_heap_pages
        .map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static {
            extra_pages: h as _,
        });

    let executor = ParachainExecutor::builder()
        .with_execution_method(config.executor.wasm_method)
        .with_onchain_heap_alloc_strategy(heap_pages)
        .with_offchain_heap_alloc_strategy(heap_pages)
        .with_max_runtime_instances(config.executor.max_runtime_instances)
        .with_runtime_cache_size(config.executor.runtime_cache_size)
        .build();

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts_record_import::<
            Block,
            crate::parachain::fake_runtime_api::RuntimeApi,
            _,
        >(
            config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
            true,
        )?;
    let client = Arc::new(client);
    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager
            .spawn_handle()
            .spawn("telemetry", None, worker.run());
        telemetry
    });
    let select_chain = sc_consensus::LongestChain::new(backend.clone());
    let transaction_pool = sc_transaction_pool::Builder::new(
        task_manager.spawn_essential_handle(),
        client.clone(),
        config.role.is_authority().into(),
    )
    .with_options(config.transaction_pool.clone())
    .with_prometheus(config.prometheus_registry())
    .build();

    let frontier_backend = Arc::new(crate::rpc::open_frontier_backend(
        client.clone(),
        config,
        evm_tracing_config,
    )?);
    let frontier_block_import = FrontierBlockImport::new(client.clone(), client.clone());

    let import_queue = sc_consensus_manual_seal::import_queue(
        Box::new(client.clone()),
        &task_manager.spawn_essential_handle(),
        config.prometheus_registry(),
    );

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool: transaction_pool.into(),
        other: (frontier_block_import, telemetry, frontier_backend),
    })
}

/// Configuration for dev node parachain inherents
struct DevParachainInherents {
    para_id: ParaId,
    initial_relay_slot: u64,
}

impl DevParachainInherents {
    fn new(para_id: ParaId) -> Self {
        // Start 2 hours in the past to avoid timestamp conflicts
        let initial_relay_slot = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Current time is always after UNIX_EPOCH")
            .sub(Duration::from_secs(2 * 60 * 60))
            .as_millis() as u64
            / RELAY_CHAIN_SLOT_DURATION_MILLIS;

        Self {
            para_id,
            initial_relay_slot,
        }
    }

    /// Create inherent data providers for manual seal consensus
    fn create_providers(
        &self,
        client: Arc<FullClient>,
    ) -> impl Fn(
        Hash,
        (),
    ) -> futures::future::Ready<
        Result<
            (
                sp_timestamp::InherentDataProvider,
                MockValidationDataInherentDataProvider<()>,
            ),
            Box<dyn std::error::Error + Send + Sync>,
        >,
    > + Send
           + Sync
           + Clone {
        let para_id = self.para_id;
        let initial_relay_slot = self.initial_relay_slot;

        move |block: Hash, ()| {
            let current_para_head = client
                .header(block)
                .expect("Header lookup should succeed")
                .expect("Header should be present");

            let should_send_go_ahead = client
                .runtime_api()
                .collect_collation_info(block, &current_para_head)
                .map(|info| info.new_validation_code.is_some())
                .unwrap_or_default();

            let current_para_block_head =
                Some(polkadot_primitives::HeadData(current_para_head.encode()));
            let current_block_number =
                UniqueSaturatedInto::<u32>::unique_saturated_into(*current_para_head.number()) + 1;

            // Dev mode: always advance at least one relay slot per para block
            let relay_blocks_per_para_block: u32 = 1;

            let target_relay_slot = initial_relay_slot
                + u64::from(current_block_number) * u64::from(relay_blocks_per_para_block);

            let relay_offset = (target_relay_slot as u32)
                .saturating_sub(relay_blocks_per_para_block * current_block_number);

            let mocked_parachain = MockValidationDataInherentDataProvider::<()> {
                current_para_block: current_block_number,
                para_id,
                current_para_block_head,
                relay_blocks_per_para_block,
                relay_offset,
                para_blocks_per_relay_epoch: 10,
                upgrade_go_ahead: should_send_go_ahead.then(|| {
                    log::info!("Detected pending validation code, sending go-ahead signal.");
                    UpgradeGoAhead::GoAhead
                }),
                ..Default::default()
            };

            let timestamp = target_relay_slot * RELAY_CHAIN_SLOT_DURATION_MILLIS;
            let timestamp_provider = sp_timestamp::InherentDataProvider::new(timestamp.into());

            futures::future::ready(Ok((timestamp_provider, mocked_parachain)))
        }
    }
}

/// Builds a new service.
pub fn start_node<N>(
    mut config: Configuration,
    evm_tracing_config: FrontierConfig,
) -> Result<TaskManager, ServiceError>
where
    N: NetworkBackend<Block, <Block as BlockT>::Hash>,
{
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (block_import, mut telemetry, frontier_backend),
    } = new_partial(&config, &evm_tracing_config)?;

    // Dev node: no peers
    config.network.default_peers_set.in_peers = 0;
    config.network.default_peers_set.out_peers = 0;

    let net_config = sc_network::config::FullNetworkConfiguration::<_, _, N>::new(
        &config.network,
        config.prometheus_registry().cloned(),
    );

    let metrics = N::register_notification_metrics(
        config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
    );

    let (network, system_rpc_tx, tx_handler_controller, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_config: None,
            block_relay: None,
            metrics,
        })?;

    if config.offchain_worker.enabled {
        task_manager.spawn_handle().spawn(
            "offchain-workers-runner",
            "offchain-work",
            sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
                runtime_api_provider: client.clone(),
                keystore: Some(keystore_container.keystore()),
                offchain_db: backend.offchain_storage(),
                transaction_pool: Some(OffchainTransactionPoolFactory::new(
                    transaction_pool.clone(),
                )),
                network_provider: Arc::new(network.clone()),
                is_validator: config.role.is_authority(),
                enable_http_requests: true,
                custom_extensions: move |_| vec![],
            })?
            .run(client.clone(), task_manager.spawn_handle())
            .boxed(),
        );
    }

    let filter_pool: FilterPool = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let fee_history_cache: FeeHistoryCache = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let storage_override = Arc::new(StorageOverrideHandler::new(client.clone()));

    // Sinks for pubsub notifications.
    // Everytime a new subscription is created, a new mpsc channel is added to the sink pool.
    // The MappingSyncWorker sends through the channel on block import and the subscription emits a notification to the subscriber on receiving a message through this channel.
    // This way we avoid race conditions when using native substrate block import notification stream.
    let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
        fc_mapping_sync::EthereumBlockNotification<Block>,
    > = Default::default();
    let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

    let ethapi_cmd = evm_tracing_config.ethapi.clone();

    let tracing_requesters =
        if ethapi_cmd.contains(&EthApiCmd::Debug) || ethapi_cmd.contains(&EthApiCmd::Trace) {
            tracing::spawn_tracing_tasks(
                &evm_tracing_config,
                config.prometheus_registry().cloned(),
                tracing::SpawnTasksParams {
                    task_manager: &task_manager,
                    client: client.clone(),
                    substrate_backend: backend.clone(),
                    frontier_backend: frontier_backend.clone(),
                    storage_override: storage_override.clone(),
                },
            )
        } else {
            tracing::RpcRequesters {
                debug: None,
                trace: None,
            }
        };

    // Frontier offchain DB task. Essential.
    // Maps emulated ethereum data to substrate native data.
    match frontier_backend.as_ref() {
        fc_db::Backend::KeyValue(ref b) => {
            task_manager.spawn_essential_handle().spawn(
                "frontier-mapping-sync-worker",
                Some("frontier"),
                fc_mapping_sync::kv::MappingSyncWorker::new(
                    client.import_notification_stream(),
                    Duration::new(6, 0),
                    client.clone(),
                    backend.clone(),
                    storage_override.clone(),
                    b.clone(),
                    3,
                    0,
                    fc_mapping_sync::SyncStrategy::Parachain,
                    sync_service.clone(),
                    pubsub_notification_sinks.clone(),
                )
                .for_each(|()| futures::future::ready(())),
            );
        }
        fc_db::Backend::Sql(ref b) => {
            task_manager.spawn_essential_handle().spawn_blocking(
                "frontier-mapping-sync-worker",
                Some("frontier"),
                fc_mapping_sync::sql::SyncWorker::run(
                    client.clone(),
                    backend.clone(),
                    b.clone(),
                    client.import_notification_stream(),
                    fc_mapping_sync::sql::SyncWorkerConfig {
                        read_notification_timeout: Duration::from_secs(10),
                        check_indexed_blocks_interval: Duration::from_secs(60),
                    },
                    fc_mapping_sync::SyncStrategy::Parachain,
                    sync_service.clone(),
                    pubsub_notification_sinks.clone(),
                ),
            );
        }
    }

    // Frontier `EthFilterApi` maintenance. Manages the pool of user-created Filters.
    // Each filter is allowed to stay in the pool for 100 blocks.
    const FILTER_RETAIN_THRESHOLD: u64 = 100;
    task_manager.spawn_essential_handle().spawn(
        "frontier-filter-pool",
        Some("frontier"),
        fc_rpc::EthTask::filter_pool_task(
            client.clone(),
            filter_pool.clone(),
            FILTER_RETAIN_THRESHOLD,
        ),
    );

    const FEE_HISTORY_LIMIT: u64 = 2048;
    task_manager.spawn_essential_handle().spawn(
        "frontier-fee-history",
        Some("frontier"),
        fc_rpc::EthTask::fee_history_task(
            client.clone(),
            storage_override.clone(),
            fee_history_cache.clone(),
            FEE_HISTORY_LIMIT,
        ),
    );

    let prometheus_registry = config.prometheus_registry().cloned();
    let is_authority = config.role.is_authority();

    let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
        task_manager.spawn_handle(),
        storage_override.clone(),
        50,
        50,
        prometheus_registry.clone(),
    ));

    // Channel for the rpc handler to communicate with the authorship task.
    let (command_sink, commands_stream) = futures::channel::mpsc::channel(1024);

    let aura_digest_provider =
        AuraConsensusDataProvider::<Block, FullClient, ()>::new(client.clone());

    // Auto-seal blocks
    spawn_block_authoring_task(
        task_manager.spawn_essential_handle(),
        command_sink.clone(),
        DEV_BLOCK_TIME_MS,
    );

    let rpc_extensions_builder = {
        let client = client.clone();
        let network = network.clone();
        let transaction_pool = transaction_pool.clone();
        let sync = sync_service.clone();
        let pubsub_notification_sinks = pubsub_notification_sinks.clone();

        Box::new(move |subscription| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                graph: transaction_pool.clone(),
                network: network.clone(),
                sync: sync.clone(),
                is_authority,
                frontier_backend: match *frontier_backend {
                    fc_db::Backend::KeyValue(ref b) => b.clone(),
                    fc_db::Backend::Sql(ref b) => b.clone(),
                },
                filter_pool: filter_pool.clone(),
                fee_history_limit: FEE_HISTORY_LIMIT,
                fee_history_cache: fee_history_cache.clone(),
                block_data_cache: block_data_cache.clone(),
                storage_override: storage_override.clone(),
                enable_evm_rpc: true, // enable EVM RPC for dev node by default
            };

            crate::rpc::create_full(
                deps,
                subscription,
                pubsub_notification_sinks.clone(),
                crate::rpc::EvmTracingConfig {
                    tracing_requesters: tracing_requesters.clone(),
                    trace_filter_max_count: evm_tracing_config.ethapi_trace_max_count,
                    enable_txpool: ethapi_cmd.contains(&EthApiCmd::TxPool),
                },
            )
            .map_err::<ServiceError, _>(Into::into)
        })
    };

    let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        network: network.clone(),
        client: client.clone(),
        keystore: keystore_container.keystore(),
        task_manager: &mut task_manager,
        transaction_pool: transaction_pool.clone(),
        rpc_builder: rpc_extensions_builder,
        backend,
        system_rpc_tx,
        tx_handler_controller,
        sync_service: sync_service.clone(),
        config,
        telemetry: telemetry.as_mut(),
    })?;

    // === Consensus ===
    let dev_inherents = DevParachainInherents::new(ParaId::from(SHIBUYA_ID));

    let params = sc_consensus_manual_seal::ManualSealParams {
        block_import,
        env: sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|x| x.handle()),
        ),
        client: client.clone(),
        pool: transaction_pool,
        select_chain,
        commands_stream: Box::pin(commands_stream),
        consensus_data_provider: Some(Box::new(aura_digest_provider)),
        create_inherent_data_providers: dev_inherents.create_providers(client.clone()),
    };

    task_manager.spawn_essential_handle().spawn_blocking(
        "manual-seal",
        Some("block-authoring"),
        sc_consensus_manual_seal::run_manual_seal(params),
    );

    Ok(task_manager)
}

/// Spawn task that produces blocks at regular intervals
fn spawn_block_authoring_task(
    spawner: sc_service::SpawnEssentialTaskHandle,
    mut command_sink: futures::channel::mpsc::Sender<sc_consensus_manual_seal::EngineCommand<Hash>>,
    interval_ms: u64,
) {
    spawner.spawn("block-authoring", None, async move {
        let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
        loop {
            interval.tick().await;
            let _ = command_sink.try_send(sc_consensus_manual_seal::EngineCommand::SealNewBlock {
                create_empty: true,
                finalize: true,
                parent_hash: None,
                sender: None,
            });
        }
    });
}
