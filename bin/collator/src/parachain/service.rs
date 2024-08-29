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

//! Parachain Service and ServiceFactory implementation.

use astar_primitives::*;
use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_aura::collators::lookahead::{self as aura, Params as AuraParams};
use cumulus_client_consensus_common::ParachainBlockImport;
use cumulus_client_consensus_relay_chain::Verifier as RelayChainVerifier;
use cumulus_client_service::{
    prepare_node_config, start_relay_chain_tasks, BuildNetworkParams, DARecoveryProfile,
    StartRelayChainTasksParams,
};
use cumulus_primitives_core::{
    relay_chain::{CollatorPair, ValidationCode},
    ParaId,
};
use cumulus_relay_chain_inprocess_interface::build_inprocess_relay_chain;
use cumulus_relay_chain_interface::{RelayChainInterface, RelayChainResult};
use cumulus_relay_chain_minimal_node::build_minimal_relay_chain_node_with_rpc;
use fc_consensus::FrontierBlockImport;
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use fc_storage::StorageOverrideHandler;
use futures::StreamExt;
use sc_client_api::BlockchainEvents;
use sc_consensus::{import_queue::BasicQueue, ImportQueue};
use sc_executor::{HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::{config::NetworkBackendType, NetworkBackend, NetworkBlock};
use sc_network_sync::SyncingService;
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_consensus_aura::{
    sr25519::AuthorityId as AuraId, sr25519::AuthorityPair as AuraPair, AuraApi,
};
use sp_keystore::KeystorePtr;
use sp_runtime::{traits::Block as BlockT, Percent};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use substrate_prometheus_endpoint::Registry;

use super::shell_upgrade::*;

use crate::{
    evm_tracing_types::{EthApi as EthApiCmd, EvmTracingConfig},
    rpc::tracing,
};

/// Parachain host functions
pub type HostFunctions = (
    cumulus_client_service::ParachainHostFunctions,
    moonbeam_primitives_ext::moonbeam_ext::HostFunctions,
);

/// Parachain executor
pub type ParachainExecutor = WasmExecutor<HostFunctions>;

type FullClient =
    TFullClient<Block, crate::parachain::fake_runtime_api::RuntimeApi, ParachainExecutor>;

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial(
    config: &Configuration,
) -> Result<
    PartialComponents<
        FullClient,
        TFullBackend<Block>,
        (),
        sc_consensus::DefaultImportQueue<Block>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        (
            ParachainBlockImport<
                Block,
                FrontierBlockImport<Block, Arc<FullClient>, FullClient>,
                TFullBackend<Block>,
            >,
            Option<Telemetry>,
            Option<TelemetryWorkerHandle>,
            Arc<fc_db::kv::Backend<Block, FullClient>>,
        ),
    >,
    sc_service::Error,
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
        sc_service::new_full_parts_record_import::<Block, _, _>(
            config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
            true,
        )?;
    let client = Arc::new(client);

    let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager
            .spawn_handle()
            .spawn("telemetry", None, worker.run());
        telemetry
    });

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    let frontier_backend = crate::rpc::open_frontier_backend(client.clone(), config)?;
    let frontier_block_import = FrontierBlockImport::new(client.clone(), client.clone());

    let parachain_block_import: ParachainBlockImport<_, _, _> =
        ParachainBlockImport::new(frontier_block_import, backend.clone());

    let import_queue = build_import_queue(
        client.clone(),
        parachain_block_import.clone(),
        config,
        telemetry.as_ref().map(|telemetry| telemetry.handle()),
        &task_manager,
    );

    let params = PartialComponents {
        backend,
        client,
        import_queue,
        keystore_container,
        task_manager,
        transaction_pool,
        select_chain: (),
        other: (
            parachain_block_import,
            telemetry,
            telemetry_worker_handle,
            frontier_backend,
        ),
    };

    Ok(params)
}

async fn build_relay_chain_interface(
    polkadot_config: Configuration,
    parachain_config: &Configuration,
    telemetry_worker_handle: Option<TelemetryWorkerHandle>,
    task_manager: &mut TaskManager,
    collator_options: CollatorOptions,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> RelayChainResult<(
    Arc<(dyn RelayChainInterface + 'static)>,
    Option<CollatorPair>,
)> {
    if let cumulus_client_cli::RelayChainMode::ExternalRpc(rpc_target_urls) =
        collator_options.relay_chain_mode
    {
        build_minimal_relay_chain_node_with_rpc(polkadot_config, task_manager, rpc_target_urls)
            .await
    } else {
        build_inprocess_relay_chain(
            polkadot_config,
            parachain_config,
            telemetry_worker_handle,
            task_manager,
            hwbench,
        )
    }
}

#[derive(Clone)]
/// To add additional config to start_xyz_node functions
pub struct AdditionalConfig {
    /// EVM tracing configuration
    pub evm_tracing_config: EvmTracingConfig,

    /// Whether EVM RPC be enabled
    pub enable_evm_rpc: bool,

    /// Maxium allowed block size limit to propose
    pub proposer_block_size_limit: usize,

    /// Soft deadline limit used by `Proposer`
    pub proposer_soft_deadline_percent: u8,

    /// Hardware benchmarks score
    pub hwbench: Option<sc_sysinfo::HwBench>,
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<N>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    para_id: ParaId,
    additional_config: AdditionalConfig,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient>)>
where
    N: NetworkBackend<Block, <Block as BlockT>::Hash>,
{
    let parachain_config = prepare_node_config(parachain_config);

    let PartialComponents {
        client,
        backend,
        mut task_manager,
        keystore_container,
        select_chain: _,
        import_queue,
        transaction_pool,
        other: (parachain_block_import, mut telemetry, telemetry_worker_handle, frontier_backend),
    } = new_partial(&parachain_config)?;

    let net_config =
        sc_network::config::FullNetworkConfiguration::<_, _, N>::new(&parachain_config.network);

    let (relay_chain_interface, collator_key) = build_relay_chain_interface(
        polkadot_config,
        &parachain_config,
        telemetry_worker_handle,
        &mut task_manager,
        collator_options.clone(),
        additional_config.hwbench.clone(),
    )
    .await
    .map_err(|e| sc_service::Error::Application(Box::new(e) as Box<_>))?;

    let is_authority = parachain_config.role.is_authority();
    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let import_queue_service = import_queue.service();
    let (network, system_rpc_tx, tx_handler_controller, start_network, sync_service) =
        cumulus_client_service::build_network(BuildNetworkParams {
            parachain_config: &parachain_config,
            net_config,
            para_id,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            relay_chain_interface: relay_chain_interface.clone(),
            sybil_resistance_level: cumulus_client_service::CollatorSybilResistance::Resistant,
        })
        .await?;

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

    let ethapi_cmd = additional_config.evm_tracing_config.ethapi.clone();
    let tracing_requesters =
        if ethapi_cmd.contains(&EthApiCmd::Debug) || ethapi_cmd.contains(&EthApiCmd::Trace) {
            tracing::spawn_tracing_tasks(
                &additional_config.evm_tracing_config,
                prometheus_registry.clone(),
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

    let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
        task_manager.spawn_handle(),
        storage_override.clone(),
        50,
        50,
        prometheus_registry.clone(),
    ));

    let rpc_extensions_builder = {
        let client = client.clone();
        let network = network.clone();
        let transaction_pool = transaction_pool.clone();
        let rpc_config = crate::rpc::EvmTracingConfig {
            tracing_requesters,
            trace_filter_max_count: additional_config.evm_tracing_config.ethapi_trace_max_count,
            enable_txpool: ethapi_cmd.contains(&EthApiCmd::TxPool),
        };
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
                enable_evm_rpc: additional_config.enable_evm_rpc,
                #[cfg(feature = "manual-seal")]
                command_sink: None,
            };

            crate::rpc::create_full(
                deps,
                subscription,
                pubsub_notification_sinks.clone(),
                rpc_config.clone(),
            )
            .map_err(Into::into)
        })
    };

    // Spawn basic services.
    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder: rpc_extensions_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: keystore_container.keystore(),
        backend: backend.clone(),
        network: network.clone(),
        system_rpc_tx,
        sync_service: sync_service.clone(),
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
    })?;

    if let Some(hwbench) = additional_config.hwbench.clone() {
        sc_sysinfo::print_hwbench(&hwbench);
        if is_authority {
            warn_if_slow_hardware(&hwbench);
        }

        if let Some(ref mut telemetry) = telemetry {
            let telemetry_handle = telemetry.handle();
            task_manager.spawn_handle().spawn(
                "telemetry_hwbench",
                None,
                sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
            );
        }
    }

    let announce_block = {
        let sync_service = sync_service.clone();
        Arc::new(move |hash, data| sync_service.announce_block(hash, data))
    };

    let overseer_handle = relay_chain_interface
        .overseer_handle()
        .map_err(|e| sc_service::Error::Application(Box::new(e)))?;

    start_relay_chain_tasks(StartRelayChainTasksParams {
        client: client.clone(),
        announce_block: announce_block.clone(),
        task_manager: &mut task_manager,
        para_id,
        relay_chain_interface: relay_chain_interface.clone(),
        relay_chain_slot_duration: Duration::from_secs(6),
        import_queue: import_queue_service,
        recovery_handle: Box::new(overseer_handle.clone()),
        sync_service: sync_service.clone(),
        da_recovery_profile: if is_authority {
            DARecoveryProfile::Collator
        } else {
            DARecoveryProfile::FullNode
        },
    })?;

    if is_authority {
        start_aura_consensus(
            client.clone(),
            backend,
            parachain_block_import,
            prometheus_registry.as_ref(),
            telemetry.map(|t| t.handle()),
            &mut task_manager,
            relay_chain_interface,
            transaction_pool,
            sync_service,
            keystore_container.keystore(),
            para_id,
            collator_key.expect("Command line arguments do not allow this. qed"),
            additional_config,
        )?;
    }

    start_network.start_network();

    Ok((task_manager, client))
}

/// Build aura import queue with fallback to relay-chain verifier.
/// Starts with relay-chain verifier until aura becomes available.
pub fn build_import_queue(
    client: Arc<FullClient>,
    block_import: ParachainBlockImport<
        Block,
        FrontierBlockImport<Block, Arc<FullClient>, FullClient>,
        TFullBackend<Block>,
    >,
    config: &Configuration,
    telemetry_handle: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> sc_consensus::DefaultImportQueue<Block> {
    let verifier_client = client.clone();

    let aura_verifier = Box::new(cumulus_client_consensus_aura::build_verifier::<
        AuraPair,
        _,
        _,
        _,
    >(cumulus_client_consensus_aura::BuildVerifierParams {
        client: verifier_client.clone(),
        create_inherent_data_providers: move |parent_hash, _| {
            let cidp_client = verifier_client.clone();
            async move {
                let slot_duration =
                    cumulus_client_consensus_aura::slot_duration_at(&*cidp_client, parent_hash)?;
                let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                let slot =
                            sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                                *timestamp,
                                slot_duration,
                            );

                Ok((slot, timestamp))
            }
        },
        telemetry: telemetry_handle,
    }));

    let relay_chain_verifier = Box::new(RelayChainVerifier::new(client.clone(), |_, _| async {
        Ok(())
    })) as Box<_>;

    let verifier = Verifier {
        client,
        relay_chain_verifier,
        aura_verifier,
    };

    let registry = config.prometheus_registry();
    let spawner = task_manager.spawn_essential_handle();

    BasicQueue::new(verifier, Box::new(block_import), None, &spawner, registry)
}

/// Start collating with the `shell` runtime while waiting for an upgrade to an Aura compatible runtime.
fn start_aura_consensus(
    client: Arc<FullClient>,
    backend: Arc<TFullBackend<Block>>,
    parachain_block_import: ParachainBlockImport<
        Block,
        FrontierBlockImport<Block, Arc<FullClient>, FullClient>,
        TFullBackend<Block>,
    >,
    prometheus_registry: Option<&Registry>,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
    relay_chain_interface: Arc<dyn RelayChainInterface>,
    transaction_pool: Arc<sc_transaction_pool::FullPool<Block, FullClient>>,
    sync_oracle: Arc<SyncingService<Block>>,
    keystore: KeystorePtr,
    para_id: ParaId,
    collator_key: CollatorPair,
    additional_config: AdditionalConfig,
) -> Result<(), sc_service::Error> {
    let mut proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
        task_manager.spawn_handle(),
        client.clone(),
        transaction_pool,
        prometheus_registry,
        telemetry,
    );

    proposer_factory.set_default_block_size_limit(additional_config.proposer_block_size_limit);
    proposer_factory.set_soft_deadline(Percent::from_percent(
        additional_config.proposer_soft_deadline_percent,
    ));

    let overseer_handle = relay_chain_interface
        .overseer_handle()
        .map_err(|e| sc_service::Error::Application(Box::new(e)))?;

    let announce_block = {
        let sync_service = sync_oracle.clone();
        Arc::new(move |hash, data| sync_service.announce_block(hash, data))
    };

    let collator_service = cumulus_client_collator::service::CollatorService::new(
        client.clone(),
        Arc::new(task_manager.spawn_handle()),
        announce_block,
        client.clone(),
    );

    let params = AuraParams {
        create_inherent_data_providers: move |_, ()| async move { Ok(()) },
        block_import: parachain_block_import.clone(),
        para_client: client.clone(),
        para_backend: backend,
        relay_client: relay_chain_interface.clone(),
        code_hash_provider: {
            let client = client.clone();
            move |block_hash| {
                client
                    .code_at(block_hash)
                    .ok()
                    .map(|c| ValidationCode::from(c).hash())
            }
        },
        sync_oracle: sync_oracle.clone(),
        keystore,
        collator_key,
        para_id,
        overseer_handle,
        relay_chain_slot_duration: Duration::from_secs(6),
        proposer: cumulus_client_consensus_proposer::Proposer::new(proposer_factory),
        collator_service,
        authoring_duration: Duration::from_millis(2000),
        reinitialize: false,
    };

    let fut = async move {
        wait_for_aura(client).await;
        aura::run::<Block, AuraPair, _, _, _, _, _, _, _, _, _>(params).await
    };

    task_manager
        .spawn_essential_handle()
        .spawn("aura", None, fut);
    Ok(())
}

/// Wait for the Aura runtime API to appear on chain.
/// This is useful for chains that started out without Aura. Components that
/// are depending on Aura functionality will wait until Aura appears in the runtime.
async fn wait_for_aura(client: Arc<FullClient>) {
    let finalized_hash = client.chain_info().finalized_hash;
    if client
        .runtime_api()
        .has_api::<dyn AuraApi<Block, AuraId>>(finalized_hash)
        .unwrap_or_default()
    {
        return;
    };

    let mut stream = client.finality_notification_stream();
    while let Some(notification) = stream.next().await {
        if client
            .runtime_api()
            .has_api::<dyn AuraApi<Block, AuraId>>(notification.hash)
            .unwrap_or_default()
        {
            return;
        }
    }
}

/// Checks that the hardware meets the requirements and print a warning otherwise.
fn warn_if_slow_hardware(hwbench: &sc_sysinfo::HwBench) {
    // Polkadot para-chains should generally use these requirements to ensure that the relay-chain
    // will not take longer than expected to import its blocks.
    if let Err(err) = frame_benchmarking_cli::SUBSTRATE_REFERENCE_HARDWARE.check_hardware(hwbench) {
        log::warn!(
            "⚠️  The hardware does not meet the minimal requirements {} for role 'Authority' find out more at:\n\
            https://wiki.polkadot.network/docs/maintain-guides-how-to-validate-polkadot#reference-hardware",
            err
        );
    }
}

/// Start a parachain node.
pub async fn start_node(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    para_id: ParaId,
    additional_config: AdditionalConfig,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient>)> {
    match parachain_config.network.network_backend {
        NetworkBackendType::Libp2p => {
            start_node_impl::<sc_network::NetworkWorker<_, _>>(
                parachain_config,
                polkadot_config,
                collator_options,
                para_id,
                additional_config,
            )
            .await
        }
        NetworkBackendType::Litep2p => {
            start_node_impl::<sc_network::Litep2pNetworkBackend>(
                parachain_config,
                polkadot_config,
                collator_options,
                para_id,
                additional_config,
            )
            .await
        }
    }
}
