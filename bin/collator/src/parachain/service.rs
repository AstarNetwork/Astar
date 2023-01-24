// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
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
use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_aura::{AuraConsensus, BuildAuraConsensusParams, SlotProportion};
use cumulus_client_consensus_common::{ParachainBlockImport, ParachainConsensus};
use cumulus_client_consensus_relay_chain::Verifier as RelayChainVerifier;
use cumulus_client_network::BlockAnnounceValidator;
use cumulus_client_service::{
    prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;
use cumulus_relay_chain_inprocess_interface::build_inprocess_relay_chain;
use cumulus_relay_chain_interface::{RelayChainError, RelayChainInterface, RelayChainResult};
use cumulus_relay_chain_minimal_node::build_minimal_relay_chain_node;
use fc_consensus::FrontierBlockImport;
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use futures::{lock::Mutex, StreamExt};
use polkadot_service::CollatorPair;
use sc_client_api::BlockchainEvents;
use sc_consensus::import_queue::BasicQueue;
use sc_executor::NativeElseWasmExecutor;
use sc_network::{NetworkBlock, NetworkService};
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::ConstructRuntimeApi;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::BlakeTwo256;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use substrate_prometheus_endpoint::Registry;

use super::shell_upgrade::*;
use crate::primitives::*;

/// Astar network runtime executor.
pub mod astar {
    pub use astar_runtime::RuntimeApi;

    /// Shibuya runtime executor.
    pub struct Executor;
    impl sc_executor::NativeExecutionDispatch for Executor {
        #[cfg(not(feature = "runtime-benchmarks"))]
        type ExtendHostFunctions = ();

        #[cfg(feature = "runtime-benchmarks")]
        type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

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
    pub use shiden_runtime::RuntimeApi;

    /// Shiden runtime executor.
    pub struct Executor;
    impl sc_executor::NativeExecutionDispatch for Executor {
        #[cfg(not(feature = "runtime-benchmarks"))]
        type ExtendHostFunctions = ();

        #[cfg(feature = "runtime-benchmarks")]
        type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

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
    pub use shibuya_runtime::RuntimeApi;

    /// Shibuya runtime executor.
    pub struct Executor;
    impl sc_executor::NativeExecutionDispatch for Executor {
        #[cfg(not(feature = "runtime-benchmarks"))]
        type ExtendHostFunctions = ();

        #[cfg(feature = "runtime-benchmarks")]
        type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

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
        sc_consensus::DefaultImportQueue<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
        sc_transaction_pool::FullPool<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
        (
            ParachainBlockImport<
                FrontierBlockImport<
                    Block,
                    Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
                    TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
                >,
            >,
            Option<Telemetry>,
            Option<TelemetryWorkerHandle>,
            Arc<fc_db::Backend<Block>>,
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
        + sp_api::ApiExt<
            Block,
            StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>,
        > + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + fp_rpc::EthereumRuntimeRPCApi<Block>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    BIQ: FnOnce(
        Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
        ParachainBlockImport<
            FrontierBlockImport<
                Block,
                Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
                TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
            >,
        >,
        &Configuration,
        Option<TelemetryHandle>,
        &TaskManager,
    ) -> Result<
        sc_consensus::DefaultImportQueue<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
        sc_service::Error,
    >,
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

    let executor = sc_executor::NativeElseWasmExecutor::<Executor>::new(
        config.wasm_method,
        config.default_heap_pages,
        config.max_runtime_instances,
        config.runtime_cache_size,
    );

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(
            config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
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
    let frontier_block_import =
        FrontierBlockImport::new(client.clone(), client.clone(), frontier_backend.clone());

    let parachain_block_import: ParachainBlockImport<_> =
        ParachainBlockImport::<_>::new(frontier_block_import);

    let import_queue = build_import_queue(
        client.clone(),
        parachain_block_import.clone(),
        config,
        telemetry.as_ref().map(|telemetry| telemetry.handle()),
        &task_manager,
    )?;

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
) -> RelayChainResult<(
    Arc<(dyn RelayChainInterface + 'static)>,
    Option<CollatorPair>,
)> {
    match collator_options.relay_chain_rpc_url {
        Some(relay_chain_url) => {
            build_minimal_relay_chain_node(polkadot_config, task_manager, relay_chain_url).await
        }
        None => build_inprocess_relay_chain(
            polkadot_config,
            parachain_config,
            telemetry_worker_handle,
            task_manager,
            None,
        ),
    }
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<RuntimeApi, Executor, BIQ, BIC>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    id: ParaId,
    enable_evm_rpc: bool,
    build_import_queue: BIQ,
    build_consensus: BIC,
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
        + sp_api::ApiExt<
            Block,
            StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>,
        > + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
        + pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + fp_rpc::ConvertTransactionRuntimeApi<Block>
        + cumulus_primitives_core::CollectCollationInfo<Block>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    BIQ: FnOnce(
        Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
        ParachainBlockImport<
            FrontierBlockImport<
                Block,
                Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
                TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
            >,
        >,
        &Configuration,
        Option<TelemetryHandle>,
        &TaskManager,
    ) -> Result<
        sc_consensus::DefaultImportQueue<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
        sc_service::Error,
    >,
    BIC: FnOnce(
        Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
        ParachainBlockImport<
            FrontierBlockImport<
                Block,
                Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
                TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
            >,
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
        Arc<NetworkService<Block, Hash>>,
        SyncCryptoStorePtr,
        bool,
    ) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
{
    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial::<RuntimeApi, Executor, BIQ>(&parachain_config, build_import_queue)?;
    let (parachain_block_import, mut telemetry, telemetry_worker_handle, frontier_backend) =
        params.other;

    let client = params.client.clone();
    let backend = params.backend.clone();

    let mut task_manager = params.task_manager;
    let (relay_chain_interface, collator_key) = build_relay_chain_interface(
        polkadot_config,
        &parachain_config,
        telemetry_worker_handle,
        &mut task_manager,
        collator_options.clone(),
    )
    .await
    .map_err(|e| match e {
        RelayChainError::ServiceError(polkadot_service::Error::Sub(x)) => x,
        s => format!("{}", s).into(),
    })?;
    let block_announce_validator = BlockAnnounceValidator::new(relay_chain_interface.clone(), id);

    let force_authoring = parachain_config.force_authoring;
    let is_authority = parachain_config.role.is_authority();
    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let transaction_pool = params.transaction_pool.clone();
    let import_queue = cumulus_client_service::SharedImportQueue::new(params.import_queue);
    let (network, system_rpc_tx, tx_handler_controller, start_network) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &parachain_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue: import_queue.clone(),
            block_announce_validator_builder: Some(Box::new(|_| {
                Box::new(block_announce_validator)
            })),
            warp_sync: None,
        })?;

    let filter_pool: FilterPool = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let fee_history_cache: FeeHistoryCache = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let overrides = crate::rpc::overrides_handle(client.clone());

    // Frontier offchain DB task. Essential.
    // Maps emulated ethereum data to substrate native data.
    task_manager.spawn_essential_handle().spawn(
        "frontier-mapping-sync-worker",
        Some("frontier"),
        fc_mapping_sync::MappingSyncWorker::new(
            client.import_notification_stream(),
            Duration::new(6, 0),
            client.clone(),
            backend.clone(),
            frontier_backend.clone(),
            3,
            0,
            fc_mapping_sync::SyncStrategy::Parachain,
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

        Box::new(move |deny_unsafe, subscription| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                graph: transaction_pool.pool().clone(),
                network: network.clone(),
                is_authority,
                deny_unsafe,
                frontier_backend: frontier_backend.clone(),
                filter_pool: filter_pool.clone(),
                fee_history_limit: FEE_HISTORY_LIMIT,
                fee_history_cache: fee_history_cache.clone(),
                block_data_cache: block_data_cache.clone(),
                overrides: overrides.clone(),
                enable_evm_rpc,
            };

            crate::rpc::create_full(deps, subscription).map_err(Into::into)
        })
    };

    // Spawn basic services.
    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder: rpc_extensions_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: params.keystore_container.sync_keystore(),
        backend: backend.clone(),
        network: network.clone(),
        system_rpc_tx,
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
    })?;

    let announce_block = {
        let network = network.clone();
        Arc::new(move |hash, data| network.announce_block(hash, data))
    };

    let relay_chain_slot_duration = Duration::from_secs(6);

    if is_authority {
        let parachain_consensus = build_consensus(
            client.clone(),
            parachain_block_import,
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
            &task_manager,
            relay_chain_interface.clone(),
            transaction_pool,
            network,
            params.keystore_container.sync_keystore(),
            force_authoring,
        )?;

        let spawner = task_manager.spawn_handle();

        let params = StartCollatorParams {
            para_id: id,
            block_status: client.clone(),
            announce_block,
            client: client.clone(),
            task_manager: &mut task_manager,
            relay_chain_interface: relay_chain_interface.clone(),
            spawner,
            parachain_consensus,
            import_queue,
            collator_key: collator_key.expect("Command line arguments do not allow this. qed"),
            relay_chain_slot_duration,
        };

        start_collator(params).await?;
    } else {
        let params = StartFullNodeParams {
            client: client.clone(),
            announce_block,
            task_manager: &mut task_manager,
            para_id: id,
            relay_chain_interface,
            relay_chain_slot_duration,
            import_queue,
        };

        start_full_node(params)?;
    }

    start_network.start_network();

    Ok((task_manager, client))
}

/// Build the import queue.
pub fn build_import_queue<RuntimeApi, Executor>(
    client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
    block_import: ParachainBlockImport<
        FrontierBlockImport<
            Block,
            Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
    >,
    config: &Configuration,
    telemetry_handle: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> Result<
    sc_consensus::DefaultImportQueue<
        Block,
        TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
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
        + sp_api::ApiExt<
            Block,
            StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>,
        > + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + sp_consensus_aura::AuraApi<Block, AuraId>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    let client2 = client.clone();

    let aura_verifier = move || {
        let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client2).unwrap();

        Box::new(cumulus_client_consensus_aura::build_verifier::<
            sp_consensus_aura::sr25519::AuthorityPair,
            _,
            _,
            _,
        >(
            cumulus_client_consensus_aura::BuildVerifierParams {
                client: client2.clone(),
                create_inherent_data_providers: move |_, _| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                    let slot =
                    sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                        *timestamp,
                        slot_duration,
                    );

                    Ok((slot, timestamp))
                },
                telemetry: telemetry_handle,
            },
        )) as Box<_>
    };

    let relay_chain_verifier = Box::new(RelayChainVerifier::new(client.clone(), |_, _| async {
        Ok(())
    })) as Box<_>;

    let verifier = Verifier {
        client,
        relay_chain_verifier,
        aura_verifier: BuildOnAccess::Uninitialized(Some(Box::new(aura_verifier))),
    };

    let registry = config.prometheus_registry();
    let spawner = task_manager.spawn_essential_handle();

    Ok(BasicQueue::new(
        verifier,
        Box::new(block_import),
        None,
        &spawner,
        registry,
    ))
}

/// Start a parachain node for Astar.
pub async fn start_astar_node(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    id: ParaId,
    enable_evm_rpc: bool,
) -> sc_service::error::Result<(
    TaskManager,
    Arc<TFullClient<Block, astar::RuntimeApi, NativeElseWasmExecutor<astar::Executor>>>,
)> {
    start_node_impl::<astar::RuntimeApi, astar::Executor, _, _>(
        parachain_config,
        polkadot_config,
        collator_options,
        id,
        enable_evm_rpc,
        |client,
         block_import,
         config,
         telemetry,
         task_manager| {
            let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

            cumulus_client_consensus_aura::import_queue::<
                sp_consensus_aura::sr25519::AuthorityPair,
                _,
                _,
                _,
                _,
                _,
            >(cumulus_client_consensus_aura::ImportQueueParams {
                block_import,
                client,
                create_inherent_data_providers: move |_, _| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                    let slot =
                        sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *timestamp,
                            slot_duration,
                        );

                    Ok((slot, timestamp))
                },
                registry: config.prometheus_registry(),
                spawner: &task_manager.spawn_essential_handle(),
                telemetry,
            })
            .map_err(Into::into)
        },
        |client,
         block_import,
         prometheus_registry,
         telemetry,
         task_manager,
         relay_chain_interface,
         transaction_pool,
         sync_oracle,
         keystore,
         force_authoring| {
            let spawn_handle = task_manager.spawn_handle();

            let slot_duration =
                cumulus_client_consensus_aura::slot_duration(&*client).unwrap();

            let proposer_factory =
                sc_basic_authorship::ProposerFactory::with_proof_recording(
                    spawn_handle,
                    client.clone(),
                    transaction_pool,
                    prometheus_registry,
                    telemetry.clone(),
                );

            let relay_chain_for_aura = relay_chain_interface.clone();

            Ok(AuraConsensus::build::<
                sp_consensus_aura::sr25519::AuthorityPair,
                _,
                _,
                _,
                _,
                _,
                _,
            >(BuildAuraConsensusParams {
                proposer_factory,
                create_inherent_data_providers:
                    move |_, (relay_parent, validation_data)| {
                        let relay_chain_for_aura = relay_chain_for_aura.clone();
                        async move {
                            let parachain_inherent =
                                cumulus_primitives_parachain_inherent::ParachainInherentData::create_at(
                                    relay_parent,
                                    &relay_chain_for_aura,
                                    &validation_data,
                                    id,
                                ).await;
                            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                            let slot =
                                sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                                    *timestamp,
                                    slot_duration,
                                );

                            let parachain_inherent = parachain_inherent.ok_or_else(|| {
                                Box::<dyn std::error::Error + Send + Sync>::from(
                                    "Failed to create parachain inherent",
                                )
                            })?;
                            Ok((slot, timestamp, parachain_inherent))
                        }
                    },
                block_import: block_import,
                para_client: client,
                backoff_authoring_blocks: Option::<()>::None,
                sync_oracle,
                keystore,
                force_authoring,
                slot_duration,
                // We got around 500ms for proposing
                block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
                // And a maximum of 750ms if slots are skipped
                max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
                telemetry,
            })
        )
    }).await
}

/// Start a parachain node for Shiden.
pub async fn start_shiden_node(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    id: ParaId,
    enable_evm_rpc: bool,
) -> sc_service::error::Result<(
    TaskManager,
    Arc<TFullClient<Block, shiden::RuntimeApi, NativeElseWasmExecutor<shiden::Executor>>>,
)> {
    start_node_impl::<shiden::RuntimeApi, shiden::Executor, _, _>(
        parachain_config,
        polkadot_config,
        collator_options,
        id,
        enable_evm_rpc,
        build_import_queue,
        |client,
         block_import,
         prometheus_registry,
         telemetry,
         task_manager,
         relay_chain_interface,
         transaction_pool,
         sync_oracle,
         keystore,
         force_authoring| {
            let client2 = client.clone();
            let spawn_handle = task_manager.spawn_handle();
            let transaction_pool2 = transaction_pool.clone();
            let telemetry2 = telemetry.clone();
            let prometheus_registry2 = prometheus_registry.map(|r| (*r).clone());
            let relay_chain_for_aura = relay_chain_interface.clone();
            let block_import2 = block_import.clone();
            let sync_oracle2 = sync_oracle.clone();
            let keystore2 = keystore.clone();

            let aura_consensus = BuildOnAccess::Uninitialized(Some(
                Box::new(move || {
                    let slot_duration =
                        cumulus_client_consensus_aura::slot_duration(&*client2).unwrap();

                    let proposer_factory =
                        sc_basic_authorship::ProposerFactory::with_proof_recording(
                            spawn_handle,
                            client2.clone(),
                            transaction_pool2,
                            prometheus_registry2.as_ref(),
                            telemetry2.clone(),
                        );

                    AuraConsensus::build::<
                        sp_consensus_aura::sr25519::AuthorityPair,
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                    >(BuildAuraConsensusParams {
                        proposer_factory,
                        create_inherent_data_providers:
                            move |_, (relay_parent, validation_data)| {
                                let relay_chain_for_aura = relay_chain_for_aura.clone();
                                async move {
                                    let parachain_inherent =
                                        cumulus_primitives_parachain_inherent::ParachainInherentData::create_at(
                                            relay_parent,
                                            &relay_chain_for_aura,
                                            &validation_data,
                                            id,
                                        ).await;
                                    let timestamp =
                                        sp_timestamp::InherentDataProvider::from_system_time();

                                    let slot =
                                    sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                                        *timestamp,
                                        slot_duration,
                                    );

                                    let parachain_inherent =
                                        parachain_inherent.ok_or_else(|| {
                                            Box::<dyn std::error::Error + Send + Sync>::from(
                                                "Failed to create parachain inherent",
                                            )
                                        })?;
                                    Ok((slot, timestamp, parachain_inherent))
                                }
                            },
                        block_import: block_import2.clone(),
                        para_client: client2.clone(),
                        backoff_authoring_blocks: Option::<()>::None,
                        sync_oracle: sync_oracle2,
                        keystore: keystore2,
                        force_authoring,
                        slot_duration,
                        // We got around 500ms for proposing
                        block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
                        // And a maximum of 750ms if slots are skipped
                        max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
                        telemetry: telemetry2,
                    })
                }),
            ));

            let proposer_factory =
                sc_basic_authorship::ProposerFactory::with_proof_recording(
                    task_manager.spawn_handle(),
                    client.clone(),
                    transaction_pool,
                    prometheus_registry,
                    telemetry.clone(),
                );

            let relay_chain_consensus =
                cumulus_client_consensus_relay_chain::build_relay_chain_consensus(
                    cumulus_client_consensus_relay_chain::BuildRelayChainConsensusParams {
                        para_id: id,
                        proposer_factory,
                        block_import: block_import, //client.clone(),
                        relay_chain_interface: relay_chain_interface.clone(),
                        create_inherent_data_providers:
                            move |_, (relay_parent, validation_data)| {
                                let relay_chain_for_aura = relay_chain_interface.clone();
                                async move {
                                    let parachain_inherent =
                                        cumulus_primitives_parachain_inherent::ParachainInherentData::create_at(
                                            relay_parent,
                                            &relay_chain_for_aura,
                                            &validation_data,
                                            id,
                                        ).await;
                                    let parachain_inherent =
                                        parachain_inherent.ok_or_else(|| {
                                            Box::<dyn std::error::Error + Send + Sync>::from(
                                                "Failed to create parachain inherent",
                                            )
                                        })?;
                                    Ok(parachain_inherent)
                                }
                            },
                    },
                );

            let parachain_consensus = Box::new(WaitForAuraConsensus {
                client,
                aura_consensus: Arc::new(Mutex::new(aura_consensus)),
                relay_chain_consensus: Arc::new(Mutex::new(relay_chain_consensus)),
            });

            Ok(parachain_consensus)
    }).await
}

/// Start a parachain node for Shibuya.
pub async fn start_shibuya_node(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    id: ParaId,
    enable_evm_rpc: bool,
) -> sc_service::error::Result<(
    TaskManager,
    Arc<TFullClient<Block, shibuya::RuntimeApi, NativeElseWasmExecutor<shibuya::Executor>>>,
)> {
    start_node_impl::<shibuya::RuntimeApi, shibuya::Executor, _, _>(
        parachain_config,
        polkadot_config,
        collator_options,
        id,
        enable_evm_rpc,
        |client,
         block_import,
         config,
         telemetry,
         task_manager| {
            let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

            cumulus_client_consensus_aura::import_queue::<
                sp_consensus_aura::sr25519::AuthorityPair,
                _,
                _,
                _,
                _,
                _,
            >(cumulus_client_consensus_aura::ImportQueueParams {
                block_import,
                client,
                create_inherent_data_providers: move |_, _| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                    let slot =
                        sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *timestamp,
                            slot_duration,
                        );

                    Ok((slot, timestamp))
                },
                registry: config.prometheus_registry(),
                spawner: &task_manager.spawn_essential_handle(),
                telemetry,
            })
            .map_err(Into::into)
        },
        |client,
         block_import,
         prometheus_registry,
         telemetry,
         task_manager,
         relay_chain_interface,
         transaction_pool,
         sync_oracle,
         keystore,
         force_authoring| {
            let spawn_handle = task_manager.spawn_handle();

            let slot_duration =
                cumulus_client_consensus_aura::slot_duration(&*client).unwrap();

            let proposer_factory =
                sc_basic_authorship::ProposerFactory::with_proof_recording(
                    spawn_handle,
                    client.clone(),
                    transaction_pool,
                    prometheus_registry,
                    telemetry.clone(),
                );

            Ok(AuraConsensus::build::<
                sp_consensus_aura::sr25519::AuthorityPair,
                _,
                _,
                _,
                _,
                _,
                _,
            >(BuildAuraConsensusParams {
                proposer_factory,
                create_inherent_data_providers:
                    move |_, (relay_parent, validation_data)| {
                        let relay_chain_for_aura = relay_chain_interface.clone();
                        async move {
                            let parachain_inherent =
                                cumulus_primitives_parachain_inherent::ParachainInherentData::create_at(
                                    relay_parent,
                                    &relay_chain_for_aura,
                                    &validation_data,
                                    id,
                                ).await;
                            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                            let slot =
                                sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                                    *timestamp,
                                    slot_duration,
                                );

                            let parachain_inherent = parachain_inherent.ok_or_else(|| {
                                Box::<dyn std::error::Error + Send + Sync>::from(
                                    "Failed to create parachain inherent",
                                )
                            })?;
                            Ok((slot, timestamp, parachain_inherent))
                        }
                    },
                block_import: block_import,
                para_client: client,
                backoff_authoring_blocks: Option::<()>::None,
                sync_oracle,
                keystore,
                force_authoring,
                slot_duration,
                // We got around 500ms for proposing
                block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
                // And a maximum of 750ms if slots are skipped
                max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
                telemetry,
            })
        )
    }).await
}
