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
    evm_tracing_types::{EthApi as EthApiCmd, EvmTracingConfig},
    rpc::tracing,
};
use fc_consensus::FrontierBlockImport;
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use fc_storage::StorageOverrideHandler;
use futures::{FutureExt, StreamExt};
use sc_client_api::{Backend, BlockBackend, BlockchainEvents};
use sc_consensus_grandpa::SharedVoterState;
use sc_executor::{HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::NetworkBackend;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
#[cfg(not(feature = "manual-seal"))]
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sp_runtime::traits::Block as BlockT;
use std::{collections::BTreeMap, sync::Arc, time::Duration};

pub use local_runtime::RuntimeApi;

use astar_primitives::*;

/// The minimum period of blocks on which justifications will be
/// imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

/// Parachain host functions
pub type HostFunctions = (
    cumulus_client_service::ParachainHostFunctions,
    moonbeam_primitives_ext::moonbeam_ext::HostFunctions,
);

type ParachainExecutor = WasmExecutor<HostFunctions>;

type FullClient = sc_service::TFullClient<Block, RuntimeApi, ParachainExecutor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

/// Build a partial chain component config
pub fn new_partial(
    config: &Configuration,
) -> Result<
    sc_service::PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        sc_consensus::DefaultImportQueue<Block>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        (
            FrontierBlockImport<
                Block,
                sc_consensus_grandpa::GrandpaBlockImport<
                    FullBackend,
                    Block,
                    FullClient,
                    FullSelectChain,
                >,
                FullClient,
            >,
            sc_consensus_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
            Option<Telemetry>,
            Arc<fc_db::kv::Backend<Block, FullClient>>,
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
        .default_heap_pages
        .map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static {
            extra_pages: h as _,
        });

    let executor = ParachainExecutor::builder()
        .with_execution_method(config.wasm_method)
        .with_onchain_heap_alloc_strategy(heap_pages)
        .with_offchain_heap_alloc_strategy(heap_pages)
        .with_max_runtime_instances(config.max_runtime_instances)
        .with_runtime_cache_size(config.runtime_cache_size)
        .build();

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts_record_import::<Block, RuntimeApi, _>(
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
    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );
    let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
        client.clone(),
        GRANDPA_JUSTIFICATION_PERIOD,
        &(client.clone() as Arc<_>),
        select_chain.clone(),
        telemetry.as_ref().map(|x| x.handle()),
    )?;
    let frontier_backend = crate::rpc::open_frontier_backend(client.clone(), config)?;
    let frontier_block_import =
        FrontierBlockImport::new(grandpa_block_import.clone(), client.clone());

    #[cfg(feature = "manual-seal")]
    let import_queue = sc_consensus_manual_seal::import_queue(
        Box::new(client.clone()),
        &task_manager.spawn_essential_handle(),
        config.prometheus_registry(),
    );

    #[cfg(not(feature = "manual-seal"))]
    let import_queue = {
        let slot_duration = sc_consensus_aura::slot_duration(&*client)?;
        sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(
            sc_consensus_aura::ImportQueueParams {
                block_import: frontier_block_import.clone(),
                justification_import: Some(Box::new(grandpa_block_import)),
                client: client.clone(),
                create_inherent_data_providers: move |_, ()| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                    let slot =
                        sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *timestamp,
                            slot_duration,
                        );
                    Ok((slot, timestamp))
                },
                spawner: &task_manager.spawn_essential_handle(),
                registry: config.prometheus_registry(),
                check_for_equivocation: Default::default(),
                telemetry: telemetry.as_ref().map(|x| x.handle()),
                compatibility_mode: Default::default(),
            },
        )?
    };

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (
            frontier_block_import,
            grandpa_link,
            telemetry,
            frontier_backend,
        ),
    })
}

/// Builds a new service.
pub fn start_node<N>(
    config: Configuration,
    evm_tracing_config: EvmTracingConfig,
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
        other: (block_import, grandpa_link, mut telemetry, frontier_backend),
    } = new_partial(&config)?;

    let protocol_name = sc_consensus_grandpa::protocol_standard_name(
        &client
            .block_hash(0)
            .ok()
            .flatten()
            .expect("Genesis block exists; qed"),
        &config.chain_spec,
    );
    let mut net_config =
        sc_network::config::FullNetworkConfiguration::<_, _, N>::new(&config.network);

    let metrics = N::register_notification_metrics(
        config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
    );
    let peer_store_handle = net_config.peer_store_handle();

    let (grandpa_protocol_config, grandpa_notification_service) =
        sc_consensus_grandpa::grandpa_peers_set_config::<_, N>(
            protocol_name.clone(),
            metrics.clone(),
            Arc::clone(&peer_store_handle),
        );
    net_config.add_notification_protocol(grandpa_protocol_config);

    let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_params: None,
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
            })
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
                    filter_pool: Some(filter_pool.clone()),
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
    task_manager.spawn_essential_handle().spawn(
        "frontier-mapping-sync-worker",
        Some("frontier"),
        fc_mapping_sync::kv::MappingSyncWorker::new(
            client.import_notification_stream(),
            Duration::new(6, 0),
            client.clone(),
            backend.clone(),
            storage_override.clone(),
            frontier_backend.clone(),
            3,
            0,
            fc_mapping_sync::SyncStrategy::Parachain,
            sync_service.clone(),
            pubsub_notification_sinks.clone(),
        )
        .for_each(|()| futures::future::ready(())),
    );

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

    #[cfg(not(feature = "manual-seal"))]
    let force_authoring = config.force_authoring;
    #[cfg(not(feature = "manual-seal"))]
    let backoff_authoring_blocks: Option<()> = None;

    let role = config.role.clone();
    let name = config.network.node_name.clone();
    let enable_grandpa = !config.disable_grandpa;
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
    #[cfg(feature = "manual-seal")]
    let (command_sink, commands_stream) = futures::channel::mpsc::channel(1024);

    let rpc_extensions_builder = {
        let client = client.clone();
        let network = network.clone();
        let transaction_pool = transaction_pool.clone();
        let sync = sync_service.clone();
        let pubsub_notification_sinks = pubsub_notification_sinks.clone();

        Box::new(move |deny_unsafe, subscription| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                graph: transaction_pool.pool().clone(),
                network: network.clone(),
                sync: sync.clone(),
                is_authority,
                deny_unsafe,
                frontier_backend: frontier_backend.clone(),
                filter_pool: filter_pool.clone(),
                fee_history_limit: FEE_HISTORY_LIMIT,
                fee_history_cache: fee_history_cache.clone(),
                block_data_cache: block_data_cache.clone(),
                storage_override: storage_override.clone(),
                enable_evm_rpc: true, // enable EVM RPC for dev node by default
                #[cfg(feature = "manual-seal")]
                command_sink: Some(command_sink.clone()),
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

    if role.is_authority() {
        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|x| x.handle()),
        );

        let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

        #[cfg(feature = "manual-seal")]
        let aura = sc_consensus_manual_seal::run_manual_seal(
            sc_consensus_manual_seal::ManualSealParams {
                block_import,
                env: proposer_factory,
                client: client.clone(),
                pool: transaction_pool.clone(),
                commands_stream,
                select_chain,
                consensus_data_provider: Some(Box::new(
                    sc_consensus_manual_seal::consensus::aura::AuraConsensusDataProvider::new(
                        client.clone(),
                    ),
                )),
                create_inherent_data_providers: move |_, ()| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                    let slot =
                        sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *timestamp,
                            slot_duration.clone(),
                        );
                    Ok((slot, timestamp))
                },
            },
        );

        #[cfg(not(feature = "manual-seal"))]
        let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
            sc_consensus_aura::StartAuraParams {
                slot_duration,
                client,
                select_chain,
                block_import,
                proposer_factory,
                create_inherent_data_providers: move |_, ()| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                    let slot =
                        sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *timestamp,
                            slot_duration,
                        );

                    Ok((slot, timestamp))
                },
                force_authoring,
                backoff_authoring_blocks,
                keystore: keystore_container.keystore(),
                sync_oracle: sync_service.clone(),
                justification_sync_link: sync_service.clone(),
                block_proposal_slot_portion: sc_consensus_aura::SlotProportion::new(2f32 / 3f32),
                max_block_proposal_slot_portion: None,
                telemetry: telemetry.as_ref().map(|x| x.handle()),
                compatibility_mode: Default::default(),
            },
        )?;

        // the AURA authoring task is considered essential, i.e. if it
        // fails we take down the service with it.
        task_manager
            .spawn_essential_handle()
            .spawn_blocking("aura", Some("block-authoring"), aura);
    }

    // if the node isn't actively participating in consensus then it doesn't
    // need a keystore, regardless of which protocol we use below.
    let keystore = if role.is_authority() {
        Some(keystore_container.keystore())
    } else {
        None
    };

    let grandpa_config = sc_consensus_grandpa::Config {
        // FIXME #1578 make this available through chainspec
        gossip_duration: Duration::from_millis(333),
        justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
        name: Some(name),
        observer_enabled: false,
        keystore,
        local_role: role,
        telemetry: telemetry.as_ref().map(|x| x.handle()),
        protocol_name,
    };

    if enable_grandpa {
        // start the full GRANDPA voter
        // NOTE: non-authorities could run the GRANDPA observer protocol, but at
        // this point the full voter should provide better guarantees of block
        // and vote data availability than the observer. The observer has not
        // been tested extensively yet and having most nodes in a network run it
        // could lead to finality stalls.
        let grandpa_config = sc_consensus_grandpa::GrandpaParams {
            config: grandpa_config,
            link: grandpa_link,
            network,
            sync: Arc::new(sync_service),
            notification_service: grandpa_notification_service,
            voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry,
            shared_voter_state: SharedVoterState::empty(),
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool),
        };

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        task_manager.spawn_essential_handle().spawn_blocking(
            "grandpa-voter",
            None,
            sc_consensus_grandpa::run_grandpa_voter(grandpa_config)?,
        );
    }

    network_starter.start_network();
    Ok(task_manager)
}
