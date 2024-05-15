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

//! Astar RPCs implementation.

use fc_rpc::{
    Eth, EthApiServer, EthBlockDataCacheTask, EthFilter, EthFilterApiServer, EthPubSub,
    EthPubSubApiServer, Net, NetApiServer, OverrideHandle, Web3, Web3ApiServer,
};
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use jsonrpsee::RpcModule;
use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};

use sc_client_api::{
    AuxStore, Backend, BlockchainEvents, StateBackend, StorageProvider, UsageProvider,
};
use sc_network::NetworkService;
use sc_network_sync::SyncingService;
use sc_rpc::dev::DevApiServer;
pub use sc_rpc::{DenyUnsafe, SubscriptionTaskExecutor};
use sc_transaction_pool::{ChainApi, Pool};
use sc_transaction_pool_api::TransactionPool;
use sp_api::{CallApiAt, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_blockchain::{
    Backend as BlockchainBackend, Error as BlockChainError, HeaderBackend, HeaderMetadata,
};
use sp_consensus_aura::{sr25519::AuthorityId as AuraId, AuraApi};
use sp_runtime::traits::BlakeTwo256;
use std::sync::Arc;
use substrate_frame_rpc_system::{System, SystemApiServer};

#[cfg(feature = "evm-tracing")]
use moonbeam_rpc_debug::{Debug, DebugServer};
#[cfg(feature = "evm-tracing")]
use moonbeam_rpc_trace::{Trace, TraceServer};
// TODO: get rid of this completely now that it's part of frontier?
#[cfg(feature = "evm-tracing")]
use moonbeam_rpc_txpool::{TxPool as MoonbeamTxPool, TxPoolServer};

use astar_primitives::*;

#[cfg(feature = "evm-tracing")]
pub mod tracing;

#[cfg(feature = "evm-tracing")]
#[derive(Clone)]
pub struct EvmTracingConfig {
    pub tracing_requesters: tracing::RpcRequesters,
    pub trace_filter_max_count: u32,
    pub enable_txpool: bool,
}

// TODO This is copied from frontier. It should be imported instead after
// https://github.com/paritytech/frontier/issues/333 is solved
pub fn open_frontier_backend<C>(
    client: Arc<C>,
    config: &sc_service::Configuration,
) -> Result<Arc<fc_db::kv::Backend<Block>>, String>
where
    C: HeaderBackend<Block>,
{
    let config_dir = config.base_path.config_dir(config.chain_spec.id());
    let path = config_dir.join("frontier").join("db");

    Ok(Arc::new(fc_db::kv::Backend::<Block>::new(
        client,
        &fc_db::kv::DatabaseSettings {
            source: fc_db::DatabaseSource::RocksDb {
                path,
                cache_size: 0,
            },
        },
    )?))
}

pub struct AstarEthConfig<C, BE>(std::marker::PhantomData<(C, BE)>);

impl<C, BE> fc_rpc::EthConfig<Block, C> for AstarEthConfig<C, BE>
where
    C: StorageProvider<Block, BE> + Sync + Send + 'static,
    BE: Backend<Block> + 'static,
{
    // Use to override (adapt) evm call to precompiles for proper gas estimation.
    // We are not aware of any of our precompile that require this.
    type EstimateGasAdapter = ();
    // This assumes the use of HashedMapping<BlakeTwo256> for address mapping
    type RuntimeStorageOverride =
        fc_rpc::frontier_backend_client::SystemAccountId32StorageOverride<Block, C, BE>;
}

/// Full client dependencies
pub struct FullDeps<C, P, A: ChainApi> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Graph pool instance.
    pub graph: Arc<Pool<A>>,
    /// Network service
    pub network: Arc<NetworkService<Block, Hash>>,
    /// Chain syncing service
    pub sync: Arc<SyncingService<Block>>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    /// The Node authority flag
    pub is_authority: bool,
    /// Frontier Backend.
    pub frontier_backend: Arc<dyn fc_api::Backend<Block>>,
    /// EthFilterApi pool.
    pub filter_pool: FilterPool,
    /// Maximum fee history cache size.
    pub fee_history_limit: u64,
    /// Fee history cache.
    pub fee_history_cache: FeeHistoryCache,
    /// Ethereum data access overrides.
    pub overrides: Arc<OverrideHandle<Block>>,
    /// Cache for Ethereum block data.
    pub block_data_cache: Arc<EthBlockDataCacheTask<Block>>,
    /// Enable EVM RPC servers
    pub enable_evm_rpc: bool,
}

/// Instantiate all RPC extensions.
pub fn create_full<C, P, BE, A>(
    deps: FullDeps<C, P, A>,
    subscription_task_executor: SubscriptionTaskExecutor,
    pubsub_notification_sinks: Arc<
        fc_mapping_sync::EthereumBlockNotificationSinks<
            fc_mapping_sync::EthereumBlockNotification<Block>,
        >,
    >,
    #[cfg(feature = "evm-tracing")] tracing_config: EvmTracingConfig,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + UsageProvider<Block>
        + CallApiAt<Block>
        + AuxStore
        + StorageProvider<Block, BE>
        + HeaderMetadata<Block, Error = BlockChainError>
        + BlockchainEvents<Block>
        + Send
        + Sync
        + 'static,
    C: sc_client_api::BlockBackend<Block>,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
        + pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
        + fp_rpc::ConvertTransactionRuntimeApi<Block>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + BlockBuilder<Block>
        + AuraApi<Block, AuraId>,
    P: TransactionPool<Block = Block> + Sync + Send + 'static,
    BE: Backend<Block> + 'static,
    BE::State: StateBackend<BlakeTwo256>,
    BE::Blockchain: BlockchainBackend<Block>,
    A: ChainApi<Block = Block> + 'static,
{
    let mut io = RpcModule::new(());
    let FullDeps {
        client,
        pool,
        graph,
        network,
        sync,
        deny_unsafe,
        is_authority,
        frontier_backend,
        filter_pool,
        fee_history_limit,
        fee_history_cache,
        overrides,
        block_data_cache,
        enable_evm_rpc,
    } = deps;

    io.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
    io.merge(TransactionPayment::new(client.clone()).into_rpc())?;
    io.merge(sc_rpc::dev::Dev::new(client.clone(), deny_unsafe).into_rpc())?;

    if !enable_evm_rpc {
        return Ok(io);
    }

    let no_tx_converter: Option<fp_rpc::NoTransactionConverter> = None;

    io.merge(
        Eth::<_, _, _, _, _, _, _, ()>::new(
            client.clone(),
            pool.clone(),
            graph.clone(),
            no_tx_converter,
            sync.clone(),
            Default::default(),
            overrides.clone(),
            frontier_backend.clone(),
            is_authority,
            block_data_cache.clone(),
            fee_history_cache,
            fee_history_limit,
            // Allow 10x max allowed weight for non-transactional calls
            10,
            None,
            crate::parachain::PendingCrateInherentDataProvider::new(client.clone()),
            Some(Box::new(
                crate::parachain::AuraConsensusDataProviderFallback::new(client.clone()),
            )),
        )
        .replace_config::<AstarEthConfig<C, BE>>()
        .into_rpc(),
    )?;

    let max_past_logs: u32 = 10_000;
    let max_stored_filters: usize = 500;
    io.merge(
        EthFilter::new(
            client.clone(),
            frontier_backend,
            graph.clone(),
            filter_pool,
            max_stored_filters,
            max_past_logs,
            block_data_cache,
        )
        .into_rpc(),
    )?;

    io.merge(Net::new(client.clone(), network.clone(), true).into_rpc())?;

    io.merge(Web3::new(client.clone()).into_rpc())?;

    io.merge(
        EthPubSub::new(
            pool,
            client.clone(),
            sync,
            subscription_task_executor,
            overrides,
            pubsub_notification_sinks,
        )
        .into_rpc(),
    )?;

    #[cfg(feature = "evm-tracing")]
    {
        if tracing_config.enable_txpool {
            io.merge(MoonbeamTxPool::new(client.clone(), graph.clone()).into_rpc())?;
        }

        if let Some(trace_filter_requester) = tracing_config.tracing_requesters.trace {
            io.merge(
                Trace::new(
                    client,
                    trace_filter_requester,
                    tracing_config.trace_filter_max_count,
                )
                .into_rpc(),
            )?;
        }

        if let Some(debug_requester) = tracing_config.tracing_requesters.debug {
            io.merge(Debug::new(debug_requester).into_rpc())?;
        }
    }

    Ok(io)
}
