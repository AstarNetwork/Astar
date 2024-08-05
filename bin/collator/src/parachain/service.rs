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
use cumulus_client_consensus_aura::collators::basic as basic_aura;
use cumulus_client_consensus_common::ParachainBlockImport;
use cumulus_client_consensus_relay_chain::Verifier as RelayChainVerifier;
use cumulus_client_service::{
    prepare_node_config, start_relay_chain_tasks, BuildNetworkParams, DARecoveryProfile,
    StartRelayChainTasksParams,
};
use cumulus_primitives_core::ParaId;
use cumulus_relay_chain_inprocess_interface::build_inprocess_relay_chain;
use cumulus_relay_chain_interface::{RelayChainInterface, RelayChainResult};
use cumulus_relay_chain_minimal_node::build_minimal_relay_chain_node_with_rpc;
use fc_consensus::FrontierBlockImport;
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use futures::StreamExt;
use polkadot_service::CollatorPair;
use sc_client_api::BlockchainEvents;
use sc_consensus::{import_queue::BasicQueue, ImportQueue};
use sc_executor::NativeElseWasmExecutor;
use sc_network::NetworkBlock;
use sc_network_sync::SyncingService;
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::{ConstructRuntimeApi, ProvideRuntimeApi};
use sp_consensus_aura::{
    sr25519::AuthorityId as AuraId, sr25519::AuthorityPair as AuraPair, AuraApi,
};
use sp_keystore::KeystorePtr;
use sp_runtime::traits::BlakeTwo256;
use sp_runtime::Percent;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use substrate_prometheus_endpoint::Registry;

use super::shell_upgrade::*;

#[cfg(feature = "evm-tracing")]
use crate::{
    evm_tracing_types::{EthApi as EthApiCmd, EvmTracingConfig},
    rpc::tracing,
};

/// Extra host functions
#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions = (
    frame_benchmarking::benchmarking::HostFunctions,
    moonbeam_primitives_ext::moonbeam_ext::HostFunctions,
    cumulus_client_service::storage_proof_size::HostFunctions,
);

/// Extra host functions
#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = (
    moonbeam_primitives_ext::moonbeam_ext::HostFunctions,
    cumulus_client_service::storage_proof_size::HostFunctions,
);

/// Astar network runtime executor.
pub mod astar {
    use super::HostFunctions;
    pub use astar_runtime::RuntimeApi;

    /// Shibuya runtime executor.
    pub struct Executor;
    impl sc_executor::NativeExecutionDispatch for Executor {
        type ExtendHostFunctions = HostFunctions;

        fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
            astar_runtime::api::dispatch(method, data)
        }

        fn native_version() -> sc_executor::NativeVersion {
            astar_runtime::native_version()
        }
    }
}

/// Shiden network runtime executor.
pub mod shiden {
    use super::HostFunctions;
    pub use shiden_runtime::RuntimeApi;

    /// Shiden runtime executor.
    pub struct Executor;
    impl sc_executor::NativeExecutionDispatch for Executor {
        type ExtendHostFunctions = HostFunctions;

        fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
            shiden_runtime::api::dispatch(method, data)
        }

        fn native_version() -> sc_executor::NativeVersion {
            shiden_runtime::native_version()
        }
    }
}

/// Shibuya network runtime executor.
pub mod shibuya {
    use super::HostFunctions;
    pub use shibuya_runtime::RuntimeApi;

    /// Shibuya runtime executor.
    pub struct Executor;
    impl sc_executor::NativeExecutionDispatch for Executor {
        type ExtendHostFunctions = HostFunctions;

        fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
            shibuya_runtime::api::dispatch(method, data)
        }

        fn native_version() -> sc_executor::NativeVersion {
            shibuya_runtime::native_version()
        }
    }
}

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial<RuntimeApi, Executor, BIQ>(
    config: &Configuration,
    build_import_queue: BIQ,
) -> Result<
    PartialComponents<
        TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        TFullBackend<Block>,
        (),
        sc_consensus::DefaultImportQueue<Block>,
        sc_transaction_pool::FullPool<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
        (
            ParachainBlockImport<
                Block,
                FrontierBlockImport<
                    Block,
                    Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
                    TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
                >,
                TFullBackend<Block>,
            >,
            Option<Telemetry>,
            Option<TelemetryWorkerHandle>,
            Arc<fc_db::kv::Backend<Block>>,
        ),
    >,
    sc_service::Error,
>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + fp_rpc::EthereumRuntimeRPCApi<Block>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>:
        sc_client_api::backend::StateBackend<BlakeTwo256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    BIQ: FnOnce(
        Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
        ParachainBlockImport<
            Block,
            FrontierBlockImport<
                Block,
                Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
                TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
            >,
            TFullBackend<Block>,
        >,
        &Configuration,
        Option<TelemetryHandle>,
        &TaskManager,
    ) -> sc_consensus::DefaultImportQueue<Block>,
{
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

    let executor = sc_service::new_native_or_wasm_executor(&config);

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts_record_import::<Block, RuntimeApi, _>(
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
/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[cfg(not(feature = "evm-tracing"))]
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<RuntimeApi, Executor, BIQ, SC>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    id: ParaId,
    additional_config: AdditionalConfig,
    build_import_queue: BIQ,
    start_consensus: SC,
) -> sc_service::error::Result<(
    TaskManager,
    Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
)>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
        + pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + fp_rpc::ConvertTransactionRuntimeApi<Block>
        + cumulus_primitives_core::CollectCollationInfo<Block>
        + AuraApi<Block, AuraId>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>:
        sc_client_api::backend::StateBackend<BlakeTwo256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    BIQ: FnOnce(
        Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
        ParachainBlockImport<
            Block,
            FrontierBlockImport<
                Block,
                Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
                TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
            >,
            TFullBackend<Block>,
        >,
        &Configuration,
        Option<TelemetryHandle>,
        &TaskManager,
    ) -> sc_consensus::DefaultImportQueue<Block>,
    SC: FnOnce(
        Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
        ParachainBlockImport<
            Block,
            FrontierBlockImport<
                Block,
                Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
                TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
            >,
            TFullBackend<Block>,
        >,
        Option<&Registry>,
        Option<TelemetryHandle>,
        &TaskManager,
        Arc<dyn RelayChainInterface>,
        Arc<
            sc_transaction_pool::FullPool<
                Block,
                TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
            >,
        >,
        Arc<SyncingService<Block>>,
        KeystorePtr,
        ParaId,
        CollatorPair,
        AdditionalConfig,
    ) -> Result<(), sc_service::Error>,
{
    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial::<RuntimeApi, Executor, BIQ>(&parachain_config, build_import_queue)?;
    let (parachain_block_import, mut telemetry, telemetry_worker_handle, frontier_backend) =
        params.other;
    let net_config = sc_network::config::FullNetworkConfiguration::new(&parachain_config.network);

    let client = params.client.clone();
    let backend = params.backend.clone();

    let mut task_manager = params.task_manager;
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
    let transaction_pool = params.transaction_pool.clone();
    let import_queue_service = params.import_queue.service();
    let (network, system_rpc_tx, tx_handler_controller, start_network, sync_service) =
        cumulus_client_service::build_network(BuildNetworkParams {
            parachain_config: &parachain_config,
            net_config,
            para_id: id,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue: params.import_queue,
            relay_chain_interface: relay_chain_interface.clone(),
            sybil_resistance_level: cumulus_client_service::CollatorSybilResistance::Resistant,
        })
        .await?;

    let filter_pool: FilterPool = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let fee_history_cache: FeeHistoryCache = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let overrides = fc_storage::overrides_handle(client.clone());

    // Sinks for pubsub notifications.
    // Everytime a new subscription is created, a new mpsc channel is added to the sink pool.
    // The MappingSyncWorker sends through the channel on block import and the subscription emits a notification to the subscriber on receiving a message through this channel.
    // This way we avoid race conditions when using native substrate block import notification stream.
    let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
        fc_mapping_sync::EthereumBlockNotification<Block>,
    > = Default::default();
    let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

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
            overrides.clone(),
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
            overrides.clone(),
            fee_history_cache.clone(),
            FEE_HISTORY_LIMIT,
        ),
    );

    let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
        task_manager.spawn_handle(),
        overrides.clone(),
        50,
        50,
        prometheus_registry.clone(),
    ));

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
                overrides: overrides.clone(),
                enable_evm_rpc: additional_config.enable_evm_rpc,
                #[cfg(feature = "manual-seal")]
                command_sink: None,
            };

            crate::rpc::create_full(deps, subscription, pubsub_notification_sinks.clone())
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
        keystore: params.keystore_container.keystore(),
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

    let relay_chain_slot_duration = Duration::from_secs(6);

    let overseer_handle = relay_chain_interface
        .overseer_handle()
        .map_err(|e| sc_service::Error::Application(Box::new(e)))?;

    start_relay_chain_tasks(StartRelayChainTasksParams {
        client: client.clone(),
        announce_block: announce_block.clone(),
        task_manager: &mut task_manager,
        para_id: id,
        relay_chain_interface: relay_chain_interface.clone(),
        relay_chain_slot_duration,
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
        start_consensus(
            client.clone(),
            parachain_block_import,
            prometheus_registry.as_ref(),
            telemetry.map(|t| t.handle()),
            &mut task_manager,
            relay_chain_interface,
            transaction_pool,
            sync_service,
            params.keystore_container.keystore(),
            id,
            collator_key.expect("Command line arguments do not allow this. qed"),
            additional_config,
        )?;
    }

    start_network.start_network();

    Ok((task_manager, client))
}

#[derive(Clone)]
/// To add additional config to start_xyz_node functions
pub struct AdditionalConfig {
    #[cfg(feature = "evm-tracing")]
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
#[cfg(feature = "evm-tracing")]
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<RuntimeApi, Executor, BIQ, SC>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    id: ParaId,
    additional_config: AdditionalConfig,
    build_import_queue: BIQ,
    start_consensus: SC,
) -> sc_service::error::Result<(
    TaskManager,
    Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
)>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
        + pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
        + moonbeam_rpc_primitives_debug::DebugRuntimeApi<Block>
        + moonbeam_rpc_primitives_txpool::TxPoolRuntimeApi<Block>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + fp_rpc::ConvertTransactionRuntimeApi<Block>
        + cumulus_primitives_core::CollectCollationInfo<Block>
        + AuraApi<Block, AuraId>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>:
        sc_client_api::backend::StateBackend<BlakeTwo256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    BIQ: FnOnce(
        Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
        ParachainBlockImport<
            Block,
            FrontierBlockImport<
                Block,
                Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
                TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
            >,
            TFullBackend<Block>,
        >,
        &Configuration,
        Option<TelemetryHandle>,
        &TaskManager,
    ) -> sc_consensus::DefaultImportQueue<Block>,
    SC: FnOnce(
        Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
        ParachainBlockImport<
            Block,
            FrontierBlockImport<
                Block,
                Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
                TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
            >,
            TFullBackend<Block>,
        >,
        Option<&Registry>,
        Option<TelemetryHandle>,
        &TaskManager,
        Arc<dyn RelayChainInterface>,
        Arc<
            sc_transaction_pool::FullPool<
                Block,
                TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
            >,
        >,
        Arc<SyncingService<Block>>,
        KeystorePtr,
        ParaId,
        CollatorPair,
        AdditionalConfig,
    ) -> Result<(), sc_service::Error>,
{
    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial::<RuntimeApi, Executor, BIQ>(&parachain_config, build_import_queue)?;
    let (parachain_block_import, mut telemetry, telemetry_worker_handle, frontier_backend) =
        params.other;
    let net_config = sc_network::config::FullNetworkConfiguration::new(&parachain_config.network);

    let client = params.client.clone();
    let backend = params.backend.clone();

    let mut task_manager = params.task_manager;
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
    let transaction_pool = params.transaction_pool.clone();
    let import_queue_service = params.import_queue.service();
    let (network, system_rpc_tx, tx_handler_controller, start_network, sync_service) =
        cumulus_client_service::build_network(BuildNetworkParams {
            parachain_config: &parachain_config,
            net_config,
            para_id: id,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue: params.import_queue,
            relay_chain_interface: relay_chain_interface.clone(),
            sybil_resistance_level: cumulus_client_service::CollatorSybilResistance::Resistant,
        })
        .await?;

    let filter_pool: FilterPool = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let fee_history_cache: FeeHistoryCache = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let overrides = fc_storage::overrides_handle(client.clone());

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
                    overrides: overrides.clone(),
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
            overrides.clone(),
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
            overrides.clone(),
            fee_history_cache.clone(),
            FEE_HISTORY_LIMIT,
        ),
    );

    let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
        task_manager.spawn_handle(),
        overrides.clone(),
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
                overrides: overrides.clone(),
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
        keystore: params.keystore_container.keystore(),
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

    let relay_chain_slot_duration = Duration::from_secs(6);

    let overseer_handle = relay_chain_interface
        .overseer_handle()
        .map_err(|e| sc_service::Error::Application(Box::new(e)))?;

    start_relay_chain_tasks(StartRelayChainTasksParams {
        client: client.clone(),
        announce_block: announce_block.clone(),
        task_manager: &mut task_manager,
        para_id: id,
        relay_chain_interface: relay_chain_interface.clone(),
        relay_chain_slot_duration,
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
        start_consensus(
            client.clone(),
            parachain_block_import,
            prometheus_registry.as_ref(),
            telemetry.map(|t| t.handle()),
            &mut task_manager,
            relay_chain_interface,
            transaction_pool,
            sync_service,
            params.keystore_container.keystore(),
            id,
            collator_key.expect("Command line arguments do not allow this. qed"),
            additional_config,
        )?;
    }

    start_network.start_network();

    Ok((task_manager, client))
}

/// Build aura import queue with fallback to relay-chain verifier.
/// Starts with relay-chain verifier until aura becomes available.
pub fn build_import_queue_fallback<RuntimeApi, Executor>(
    client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
    block_import: ParachainBlockImport<
        Block,
        FrontierBlockImport<
            Block,
            Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
        TFullBackend<Block>,
    >,
    config: &Configuration,
    telemetry_handle: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> sc_consensus::DefaultImportQueue<Block>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + AuraApi<Block, AuraId>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>:
        sc_client_api::StateBackend<BlakeTwo256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    let verifier_client = client.clone();

    let aura_verifier = cumulus_client_consensus_aura::build_verifier::<AuraPair, _, _, _>(
        cumulus_client_consensus_aura::BuildVerifierParams {
            client: verifier_client.clone(),
            create_inherent_data_providers: move |parent_hash, _| {
                let cidp_client = verifier_client.clone();
                async move {
                    let slot_duration = cumulus_client_consensus_aura::slot_duration_at(
                        &*cidp_client,
                        parent_hash,
                    )?;
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
        },
    );

    let relay_chain_verifier = Box::new(RelayChainVerifier::new(client.clone(), |_, _| async {
        Ok(())
    })) as Box<_>;

    let verifier = Verifier {
        client,
        relay_chain_verifier,
        aura_verifier: Box::new(aura_verifier),
    };

    let registry = config.prometheus_registry();
    let spawner = task_manager.spawn_essential_handle();

    BasicQueue::new(verifier, Box::new(block_import), None, &spawner, registry)
}

/// Build aura only import queue.
pub fn build_import_queue<RuntimeApi, Executor>(
    client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
    block_import: ParachainBlockImport<
        Block,
        FrontierBlockImport<
            Block,
            Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
        TFullBackend<Block>,
    >,
    config: &Configuration,
    telemetry_handle: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> sc_consensus::DefaultImportQueue<Block>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + AuraApi<Block, AuraId>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>:
        sc_client_api::StateBackend<BlakeTwo256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)
        .expect("AuraApi slot_duration failed!");

    cumulus_client_consensus_aura::equivocation_import_queue::fully_verifying_import_queue::<
        AuraPair,
        _,
        _,
        _,
        _,
    >(
        client,
        block_import,
        move |_, _| async move {
            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

            let slot =
                sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                    *timestamp,
                    slot_duration,
                );

            Ok((slot, timestamp))
        },
        slot_duration,
        &task_manager.spawn_essential_handle(),
        config.prometheus_registry(),
        telemetry_handle,
    )
}

/// Start collating with the `shell` runtime while waiting for an upgrade to an Aura compatible runtime.
fn start_aura_consensus_fallback<RuntimeApi, Executor>(
    client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
    parachain_block_import: ParachainBlockImport<
        Block,
        FrontierBlockImport<
            Block,
            Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
        TFullBackend<Block>,
    >,
    prometheus_registry: Option<&Registry>,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
    relay_chain_interface: Arc<dyn RelayChainInterface>,
    transaction_pool: Arc<
        sc_transaction_pool::FullPool<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
    >,
    sync_oracle: Arc<SyncingService<Block>>,
    keystore: KeystorePtr,
    para_id: ParaId,
    collator_key: CollatorPair,
    additional_config: AdditionalConfig,
) -> Result<(), sc_service::Error>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + AuraApi<Block, AuraId>
        + cumulus_primitives_core::CollectCollationInfo<Block>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>:
        sc_client_api::StateBackend<BlakeTwo256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
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

    let spawner = task_manager.spawn_handle();
    let client_ = client.clone();

    let collation_future = Box::pin(async move {
        use parity_scale_codec::Decode;
        use sp_api::ApiExt;
        use sp_runtime::traits::Block as BlockT;

        let client = client_.clone();

        // Start collating with the `shell` runtime while waiting for an upgrade to an Aura
        // compatible runtime.
        let mut request_stream = cumulus_client_collator::relay_chain_driven::init(
            collator_key.clone(),
            para_id.clone(),
            overseer_handle.clone(),
        )
        .await;

        while let Some(request) = request_stream.next().await {
            let pvd = request.persisted_validation_data().clone();
            let last_head_hash =
                match <Block as BlockT>::Header::decode(&mut &pvd.parent_head.0[..]) {
                    Ok(header) => header.hash(),
                    Err(e) => {
                        log::error!("Could not decode the head data: {e}");
                        request.complete(None);
                        continue;
                    }
                };

            // Check if we have upgraded to an Aura compatible runtime and transition if
            // necessary.
            if client
                .runtime_api()
                .has_api::<dyn AuraApi<Block, AuraId>>(last_head_hash)
                .unwrap_or_default()
            {
                // Respond to this request before transitioning to Aura.
                request.complete(None);
                break;
            }
        }

        // Move to Aura consensus.
        let slot_duration =
            cumulus_client_consensus_aura::slot_duration(&*client).expect("aura is present; qed");

        let announce_block = {
            let sync_service = sync_oracle.clone();
            Arc::new(move |hash, data| sync_service.announce_block(hash, data))
        };

        let collator_service = cumulus_client_collator::service::CollatorService::new(
            client.clone(),
            Arc::new(spawner),
            announce_block,
            client.clone(),
        );

        basic_aura::run::<Block, AuraPair, _, _, _, _, _, _, _>(basic_aura::Params {
            create_inherent_data_providers: move |_, ()| async move { Ok(()) },
            block_import: parachain_block_import.clone(),
            para_client: client.clone(),
            relay_client: relay_chain_interface.clone(),
            sync_oracle: sync_oracle.clone(),
            keystore,
            collator_key,
            para_id,
            overseer_handle,
            slot_duration,
            relay_chain_slot_duration: Duration::from_secs(6),
            proposer: cumulus_client_consensus_proposer::Proposer::new(proposer_factory),
            collator_service,
            // We got around 500ms for proposing
            authoring_duration: Duration::from_millis(500),
            collation_request_receiver: Some(request_stream),
        })
        .await
    });

    task_manager
        .spawn_essential_handle()
        .spawn("aura", None, collation_future);
    Ok(())
}

fn start_aura_consensus<RuntimeApi, Executor>(
    client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
    parachain_block_import: ParachainBlockImport<
        Block,
        FrontierBlockImport<
            Block,
            Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
        TFullBackend<Block>,
    >,
    prometheus_registry: Option<&Registry>,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
    relay_chain_interface: Arc<dyn RelayChainInterface>,
    transaction_pool: Arc<
        sc_transaction_pool::FullPool<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
    >,
    sync_oracle: Arc<SyncingService<Block>>,
    keystore: KeystorePtr,
    para_id: ParaId,
    collator_key: CollatorPair,
    additional_config: AdditionalConfig,
) -> Result<(), sc_service::Error>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + AuraApi<Block, AuraId>
        + cumulus_primitives_core::CollectCollationInfo<Block>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>:
        sc_client_api::StateBackend<BlakeTwo256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
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

    let fut = basic_aura::run::<Block, AuraPair, _, _, _, _, _, _, _>(basic_aura::Params {
        create_inherent_data_providers: move |_, ()| async move { Ok(()) },
        block_import: parachain_block_import.clone(),
        para_client: client.clone(),
        relay_client: relay_chain_interface.clone(),
        sync_oracle: sync_oracle.clone(),
        keystore,
        collator_key,
        para_id,
        overseer_handle,
        slot_duration: cumulus_client_consensus_aura::slot_duration(&*client)?,
        relay_chain_slot_duration: Duration::from_secs(6),
        proposer: cumulus_client_consensus_proposer::Proposer::new(proposer_factory),
        collator_service,
        // We got around 500ms for proposing
        authoring_duration: Duration::from_millis(500),
        collation_request_receiver: None,
    });

    task_manager
        .spawn_essential_handle()
        .spawn("aura", None, fut);

    Ok(())
}

/// Start a parachain node for Astar.
pub async fn start_astar_node(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    id: ParaId,
    additional_config: AdditionalConfig,
) -> sc_service::error::Result<(
    TaskManager,
    Arc<TFullClient<Block, astar::RuntimeApi, NativeElseWasmExecutor<astar::Executor>>>,
)> {
    start_node_impl::<astar::RuntimeApi, astar::Executor, _, _>(
        parachain_config,
        polkadot_config,
        collator_options,
        id,
        additional_config.clone(),
        build_import_queue,
        start_aura_consensus,
    )
    .await
}

/// Start a parachain node for Shiden.
pub async fn start_shiden_node(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    id: ParaId,
    additional_config: AdditionalConfig,
) -> sc_service::error::Result<(
    TaskManager,
    Arc<TFullClient<Block, shiden::RuntimeApi, NativeElseWasmExecutor<shiden::Executor>>>,
)> {
    start_node_impl::<shiden::RuntimeApi, shiden::Executor, _, _>(
        parachain_config,
        polkadot_config,
        collator_options,
        id,
        additional_config.clone(),
        build_import_queue_fallback,
        start_aura_consensus_fallback,
    )
    .await
}

/// Start a parachain node for Shibuya.
pub async fn start_shibuya_node(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    id: ParaId,
    additional_config: AdditionalConfig,
) -> sc_service::error::Result<(
    TaskManager,
    Arc<TFullClient<Block, shibuya::RuntimeApi, NativeElseWasmExecutor<shibuya::Executor>>>,
)> {
    start_node_impl::<shibuya::RuntimeApi, shibuya::Executor, _, _>(
        parachain_config,
        polkadot_config,
        collator_options,
        id,
        additional_config.clone(),
        build_import_queue,
        start_aura_consensus,
    )
    .await
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
