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

pub use crate::parachain::fake_runtime_api::RuntimeApi;
use crate::{
    evm_tracing_types::{EthApi as EthApiCmd, FrontierConfig},
    rpc::tracing,
};
use cumulus_client_parachain_inherent::MockXcmConfig;
use cumulus_primitives_aura::Slot;
use cumulus_primitives_core::{
    relay_chain,
    relay_chain::{well_known_keys, AsyncBackingParams, HeadData, UpgradeGoAhead},
    AbridgedHostConfiguration, CollectCollationInfo, InboundHrmpMessage, ParaId,
    RelayParentOffsetApi,
};
use cumulus_primitives_parachain_inherent::{MessageQueueChain, ParachainInherentData};
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
use fc_consensus::FrontierBlockImport;
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use fc_storage::StorageOverrideHandler;
use futures::{FutureExt, StreamExt};
use parity_scale_codec::Encode;
use polkadot_core_primitives::InboundDownwardMessage;
use polkadot_primitives::PersistedValidationData;
use sc_client_api::{Backend, BlockchainEvents};
use sc_executor::{HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::NetworkBackend;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_inherents::{InherentData, InherentDataProvider};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, UniqueSaturatedInto};
use std::{collections::BTreeMap, marker::PhantomData, ops::Sub, sync::Arc, time::Duration};

use astar_primitives::*;

/// Local pending inherent provider for ETH pending RPC in dev mode.
pub struct LocalPendingInherentDataProvider<B, C> {
    client: Arc<C>,
    para_id: ParaId,
    phantom_data: PhantomData<B>,
}

const RELAY_CHAIN_SLOT_DURATION_MILLIS: u64 = 6000;

/// Inherent data provider that supplies mocked validation data.
/// TODO: Use it from PolkadotSDK again after stable2603 uplift
#[derive(Default)]
pub struct MockValidationDataInherentDataProvider<R = ()> {
    /// The current block number of the local block chain (the parachain).
    pub current_para_block: u32,
    /// The parachain ID of the parachain for that the inherent data is created.
    pub para_id: ParaId,
    /// The current block head data of the local block chain (the parachain).
    pub current_para_block_head: Option<cumulus_primitives_core::relay_chain::HeadData>,
    /// The relay block in which this parachain appeared to start. This will be the relay block
    /// number in para block #P1.
    pub relay_offset: u32,
    /// The relay parent offset that determines how many relay parent descendants are required.
    pub relay_parent_offset: u32,
    /// The number of relay blocks that elapses between each parablock. Probably set this to 1 or 2
    /// to simulate optimistic or realistic relay chain behavior.
    pub relay_blocks_per_para_block: u32,
    /// Number of parachain blocks per relay chain epoch
    /// Mock epoch is computed by dividing `current_para_block` by this value.
    pub para_blocks_per_relay_epoch: u32,
    /// Function to mock BABE one epoch ago randomness.
    pub relay_randomness_config: R,
    /// XCM messages and associated configuration information.
    pub xcm_config: MockXcmConfig,
    /// Inbound downward XCM messages to be injected into the block.
    pub raw_downward_messages: Vec<Vec<u8>>,
    /// Inbound Horizontal messages sorted by channel.
    pub raw_horizontal_messages: Vec<(ParaId, Vec<u8>)>,
    /// Additional key-value pairs that should be injected.
    pub additional_key_values: Option<Vec<(Vec<u8>, Vec<u8>)>>,
    /// Whether upgrade go ahead should be set.
    pub upgrade_go_ahead: Option<UpgradeGoAhead>,
}

/// Something that can generate randomness.
pub trait GenerateRandomness<I> {
    /// Generate the randomness using the given `input`.
    fn generate_randomness(&self, input: I) -> relay_chain::Hash;
}

impl GenerateRandomness<u64> for () {
    /// Default implementation uses relay epoch as randomness value
    /// A more seemingly random implementation may hash the relay epoch instead
    fn generate_randomness(&self, input: u64) -> relay_chain::Hash {
        let mut mock_randomness: [u8; 32] = [0u8; 32];
        mock_randomness[..8].copy_from_slice(&input.to_be_bytes());
        mock_randomness.into()
    }
}

#[async_trait::async_trait]
impl<R: Send + Sync + GenerateRandomness<u64>> InherentDataProvider
    for MockValidationDataInherentDataProvider<R>
{
    async fn provide_inherent_data(
        &self,
        inherent_data: &mut InherentData,
    ) -> Result<(), sp_inherents::Error> {
        // Use the "sproof" (spoof proof) builder to build valid mock state root and proof.
        let mut sproof_builder = RelayStateSproofBuilder {
            para_id: self.para_id,
            ..Default::default()
        };

        // Calculate the mocked relay block based on the current para block
        let relay_parent_number =
            self.relay_offset + self.relay_blocks_per_para_block * self.current_para_block;
        sproof_builder.current_slot = Slot::from(relay_parent_number as u64);

        sproof_builder.upgrade_go_ahead = self.upgrade_go_ahead;
        // Process the downward messages and set up the correct head
        let mut downward_messages = Vec::new();
        let mut dmq_mqc = MessageQueueChain::new(self.xcm_config.starting_dmq_mqc_head);
        for msg in &self.raw_downward_messages {
            let wrapped = InboundDownwardMessage {
                sent_at: relay_parent_number,
                msg: msg.clone(),
            };

            dmq_mqc.extend_downward(&wrapped);
            downward_messages.push(wrapped);
        }
        sproof_builder.dmq_mqc_head = Some(dmq_mqc.head());

        // Process the hrmp messages and set up the correct heads
        // Begin by collecting them into a Map
        let mut horizontal_messages = BTreeMap::<ParaId, Vec<InboundHrmpMessage>>::new();
        for (para_id, msg) in &self.raw_horizontal_messages {
            let wrapped = InboundHrmpMessage {
                sent_at: relay_parent_number,
                data: msg.clone(),
            };

            horizontal_messages
                .entry(*para_id)
                .or_default()
                .push(wrapped);
        }

        // Now iterate again, updating the heads as we go
        for (para_id, messages) in &horizontal_messages {
            let mut channel_mqc = MessageQueueChain::new(
                *self
                    .xcm_config
                    .starting_hrmp_mqc_heads
                    .get(para_id)
                    .unwrap_or(&relay_chain::Hash::default()),
            );
            for message in messages {
                channel_mqc.extend_hrmp(message);
            }
            sproof_builder.upsert_inbound_channel(*para_id).mqc_head = Some(channel_mqc.head());
        }

        // Epoch is set equal to current para block / blocks per epoch
        sproof_builder.current_epoch = if self.para_blocks_per_relay_epoch == 0 {
            // do not divide by 0 => set epoch to para block number
            self.current_para_block.into()
        } else {
            (self.current_para_block / self.para_blocks_per_relay_epoch).into()
        };
        // Randomness is set by randomness generator
        sproof_builder.randomness = self
            .relay_randomness_config
            .generate_randomness(self.current_para_block.into());

        if let Some(key_values) = &self.additional_key_values {
            sproof_builder.additional_key_values = key_values.clone()
        }

        // Inject current para block head, if any
        sproof_builder.included_para_head = self.current_para_block_head.clone();

        let (relay_parent_storage_root, proof, relay_parent_descendants) =
            sproof_builder.into_state_root_proof_and_descendants(self.relay_parent_offset.into());
        let parachain_inherent_data = ParachainInherentData {
            validation_data: PersistedValidationData {
                parent_head: Default::default(),
                relay_parent_storage_root,
                relay_parent_number,
                max_pov_size: Default::default(),
            },
            downward_messages,
            horizontal_messages,
            relay_chain_state: proof,
            relay_parent_descendants,
            collator_peer_id: None,
        };

        parachain_inherent_data
            .provide_inherent_data(inherent_data)
            .await
    }

    // Copied from the real implementation
    async fn try_handle_error(
        &self,
        _: &sp_inherents::InherentIdentifier,
        _: &[u8],
    ) -> Option<Result<(), sp_inherents::Error>> {
        None
    }
}

fn build_local_mock_inherent_data(
    para_id: ParaId,
    current_para_block: u32,
    current_para_block_head: Option<HeadData>,
    relay_blocks_per_para_block: u32,
    relay_slot: u64,
    relay_parent_offset: u32,
    upgrade_go_ahead: Option<UpgradeGoAhead>,
) -> (
    sp_timestamp::InherentDataProvider,
    MockValidationDataInherentDataProvider<()>,
) {
    let relay_offset = relay_parent_offset.saturating_add(
        (relay_slot as u32)
            .saturating_sub(relay_blocks_per_para_block.saturating_mul(current_para_block)),
    );

    let local_host_config = AbridgedHostConfiguration {
        max_code_size: 16 * 1024 * 1024, // 16 MiB (local dev only)
        max_head_data_size: 1024 * 1024,
        max_upward_queue_count: 8,
        max_upward_queue_size: 1024,
        max_upward_message_size: 256,
        max_upward_message_num_per_candidate: 5,
        hrmp_max_message_num_per_candidate: 5,
        validation_upgrade_cooldown: 6,
        validation_upgrade_delay: 6,
        async_backing_params: AsyncBackingParams {
            allowed_ancestry_len: 0,
            max_candidate_depth: 0,
        },
    };

    let mocked_parachain = MockValidationDataInherentDataProvider::<()> {
        current_para_block,
        para_id,
        current_para_block_head,
        relay_blocks_per_para_block,
        relay_offset,
        relay_parent_offset,
        para_blocks_per_relay_epoch: 10,
        upgrade_go_ahead,
        additional_key_values: Some(vec![(
            well_known_keys::ACTIVE_CONFIG.to_vec(),
            local_host_config.encode(),
        )]),
        ..Default::default()
    };

    let timestamp = relay_slot
        .saturating_add(u64::from(relay_parent_offset))
        .saturating_mul(RELAY_CHAIN_SLOT_DURATION_MILLIS);
    let timestamp_provider = sp_timestamp::InherentDataProvider::new(timestamp.into());

    (timestamp_provider, mocked_parachain)
}

impl<B, C> LocalPendingInherentDataProvider<B, C> {
    /// Creates a new instance with the given client and parachain ID.
    pub fn new(client: Arc<C>, para_id: ParaId) -> Self {
        Self {
            client,
            para_id,
            phantom_data: Default::default(),
        }
    }
}

#[async_trait::async_trait]
impl<B, C> sp_inherents::CreateInherentDataProviders<B, ()>
    for LocalPendingInherentDataProvider<B, C>
where
    B: BlockT,
    C: ProvideRuntimeApi<B> + HeaderBackend<B> + Send + Sync,
    C::Api: RelayParentOffsetApi<B>,
{
    type InherentDataProviders = (
        sp_timestamp::InherentDataProvider,
        MockValidationDataInherentDataProvider<()>,
    );

    async fn create_inherent_data_providers(
        &self,
        parent: B::Hash,
        _extra_args: (),
    ) -> Result<Self::InherentDataProviders, Box<dyn std::error::Error + Send + Sync>> {
        let relay_slot = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Current time is always after UNIX_EPOCH; qed")
            .as_millis() as u64
            / RELAY_CHAIN_SLOT_DURATION_MILLIS;

        let current_para_block = self
            .client
            .header(parent)?
            .map(|header| {
                UniqueSaturatedInto::<u32>::unique_saturated_into(*header.number())
                    .saturating_add(1)
            })
            .unwrap_or(1);

        let current_para_block_head = self
            .client
            .header(parent)?
            .map(|header| header.encode().into());

        let relay_parent_offset = self.client.runtime_api().relay_parent_offset(parent)?;

        let (timestamp_provider, mocked_parachain) = build_local_mock_inherent_data(
            self.para_id,
            current_para_block,
            current_para_block_head,
            1,
            relay_slot,
            relay_parent_offset,
            None,
        );

        Ok((timestamp_provider, mocked_parachain))
    }
}

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

type FullClient = sc_service::TFullClient<Block, RuntimeApi, ParachainExecutor>;
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

/// Builds a new local development service (parachain-oriented).
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
                    None,
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

    let role = config.role.clone();
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
    let local_para_id = ParaId::from(
        crate::parachain::chain_spec::Extensions::try_get(&*config.chain_spec)
            .map(|e| e.para_id)
            .unwrap_or(2000),
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
                command_sink: Some(command_sink.clone()),
            };

            crate::rpc::create_full_local_dev(
                deps,
                subscription,
                pubsub_notification_sinks.clone(),
                local_para_id,
                crate::rpc::EvmTracingConfig {
                    tracing_requesters: tracing_requesters.clone(),
                    trace_filter_max_count: evm_tracing_config.ethapi_trace_max_count,
                    trace_filter_max_block_range: evm_tracing_config.trace_filter_max_block_range,
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
        tracing_execute_block: None,
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

        let para_id = local_para_id;
        let initial_relay_slot = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Current time is always after UNIX_EPOCH; qed")
            .sub(Duration::from_secs(2 * 60 * 60))
            .as_millis() as u64
            / RELAY_CHAIN_SLOT_DURATION_MILLIS;

        let aura =
            sc_consensus_manual_seal::run_manual_seal(sc_consensus_manual_seal::ManualSealParams {
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
                create_inherent_data_providers: move |parent_hash, ()| {
                    let client = client.clone();
                    async move {
                        let current_para_head = client
                            .header(parent_hash)
                            .expect("Header lookup should succeed")
                            .expect("Header passed in as parent should be present in backend.");

                        let should_send_go_ahead = client
                            .runtime_api()
                            .collect_collation_info(parent_hash, &current_para_head)
                            .map(|info| info.new_validation_code.is_some())
                            .unwrap_or_default();

                        let current_para_block = UniqueSaturatedInto::<u32>::unique_saturated_into(
                            *current_para_head.number(),
                        ) + 1;

                        let relay_blocks_per_para_block =
                            (slot_duration.as_millis() / RELAY_CHAIN_SLOT_DURATION_MILLIS).max(1)
                                as u32;
                        let current_para_block_u64 = u64::from(current_para_block);
                        let relay_blocks_per_para_block_u64 =
                            u64::from(relay_blocks_per_para_block);
                        let target_relay_slot = initial_relay_slot.saturating_add(
                            current_para_block_u64.saturating_mul(relay_blocks_per_para_block_u64),
                        );

                        let current_para_block_head = Some(current_para_head.encode().into());
                        let relay_parent_offset =
                            client.runtime_api().relay_parent_offset(parent_hash)?;

                        let (timestamp_provider, mocked_parachain) = build_local_mock_inherent_data(
                            para_id,
                            current_para_block,
                            current_para_block_head,
                            relay_blocks_per_para_block,
                            target_relay_slot,
                            relay_parent_offset,
                            should_send_go_ahead.then_some(UpgradeGoAhead::GoAhead),
                        );

                        Ok((timestamp_provider, mocked_parachain))
                    }
                },
            });

        task_manager
            .spawn_essential_handle()
            .spawn_blocking("aura", Some("block-authoring"), aura);
    }

    Ok(task_manager)
}
