//! Astar RPCs implementation.

use fc_rpc::{
    Eth, EthApiServer, EthBlockDataCacheTask, EthFilter, EthFilterApiServer, EthPubSub,
    EthPubSubApiServer, Net, NetApiServer, OverrideHandle, RuntimeApiStorageOverride,
    SchemaV1Override, SchemaV2Override, SchemaV3Override, StorageOverride, Web3, Web3ApiServer,
};
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use fp_storage::EthereumStorageSchema;
use jsonrpsee::RpcModule;
use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
use sc_client_api::{AuxStore, Backend, BlockchainEvents, StateBackend, StorageProvider};
use sc_network::NetworkService;
pub use sc_rpc::{DenyUnsafe, SubscriptionTaskExecutor};
use sc_transaction_pool::{ChainApi, Pool};
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{
    Backend as BlockchainBackend, Error as BlockChainError, HeaderBackend, HeaderMetadata,
};
use sp_runtime::traits::BlakeTwo256;
use std::collections::BTreeMap;
use std::sync::Arc;
use substrate_frame_rpc_system::{SystemApi, SystemApiServer};

use crate::primitives::*;

// TODO This is copied from frontier. It should be imported instead after
// https://github.com/paritytech/frontier/issues/333 is solved
pub fn open_frontier_backend(
    config: &sc_service::Configuration,
) -> Result<Arc<fc_db::Backend<Block>>, String> {
    let config_dir = config
        .base_path
        .as_ref()
        .map(|base_path| base_path.config_dir(config.chain_spec.id()))
        .unwrap_or_else(|| {
            sc_service::BasePath::from_project("", "", "astar").config_dir(config.chain_spec.id())
        });
    let path = config_dir.join("frontier").join("db");

    Ok(Arc::new(fc_db::Backend::<Block>::new(
        &fc_db::DatabaseSettings {
            source: fc_db::DatabaseSource::RocksDb {
                path,
                cache_size: 0,
            },
        },
    )?))
}

pub fn overrides_handle<C, BE>(client: Arc<C>) -> Arc<OverrideHandle<Block>>
where
    C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
    C: Send + Sync + 'static,
    C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
    BE: Backend<Block> + 'static,
    BE::State: StateBackend<BlakeTwo256>,
{
    let mut overrides_map = BTreeMap::new();
    overrides_map.insert(
        EthereumStorageSchema::V1,
        Box::new(SchemaV1Override::new(client.clone()))
            as Box<dyn StorageOverride<_> + Send + Sync>,
    );
    overrides_map.insert(
        EthereumStorageSchema::V2,
        Box::new(SchemaV2Override::new(client.clone()))
            as Box<dyn StorageOverride<_> + Send + Sync>,
    );
    overrides_map.insert(
        EthereumStorageSchema::V3,
        Box::new(SchemaV3Override::new(client.clone()))
            as Box<dyn StorageOverride<_> + Send + Sync>,
    );

    Arc::new(OverrideHandle {
        schemas: overrides_map,
        fallback: Box::new(RuntimeApiStorageOverride::new(client.clone())),
    })
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
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    /// The Node authority flag
    pub is_authority: bool,
    /// Frontier Backend.
    pub frontier_backend: Arc<fc_db::Backend<Block>>,
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
}

/// Instantiate all RPC extensions.
pub fn create_full<C, P, BE, A>(
    deps: FullDeps<C, P, A>,
    subscription_task_executor: SubscriptionTaskExecutor,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + AuxStore
        + StorageProvider<Block, BE>
        + HeaderMetadata<Block, Error = BlockChainError>
        + BlockchainEvents<Block>
        + Send
        + Sync
        + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
        + pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
        + fp_rpc::ConvertTransactionRuntimeApi<Block>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + BlockBuilder<Block>,
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
        deny_unsafe,
        is_authority,
        frontier_backend,
        filter_pool,
        fee_history_limit,
        fee_history_cache,
        overrides,
        block_data_cache,
    } = deps;

    // TODO: why does frontier explicitly use `Arc::clone` instead of just `.clone()`?

    io.merge(SystemApi::new(Arc::clone(&client), Arc::clone(&pool), deny_unsafe).into_rpc())?;
    io.merge(TransactionPayment::new(Arc::clone(&client)).into_rpc())?;

    io.merge(
        Eth::new(
            Arc::clone(&client),
            Arc::clone(&pool),
            graph,
            None,
            Arc::clone(&network),
            Default::default(),
            Arc::clone(&overrides),
            Arc::clone(&frontier_backend),
            is_authority,
            Arc::clone(&block_data_cache),
            fee_history_cache,
            fee_history_limit,
        )
        .into_rpc(),
    )?;

    let max_past_logs: u32 = 10_000;
    let max_stored_filters: usize = 500;
    io.merge(
        EthFilter::new(
            Arc::clone(&client),
            Arc::clone(&frontier_backend),
            filter_pool,
            max_stored_filters,
            max_past_logs,
            Arc::clone(&block_data_cache),
        )
        .into_rpc(),
    )?;

    io.merge(Net::new(Arc::clone(&client), Arc::clone(&network), true).into_rpc())?;

    io.merge(Web3::new(Arc::clone(&client)).into_rpc())?;

    io.merge(
        EthPubSub::new(
            Arc::clone(&pool),
            Arc::clone(&client),
            Arc::clone(&network),
            subscription_task_executor,
            Arc::clone(&overrides),
        )
        .into_rpc(),
    )?;

    io
}
