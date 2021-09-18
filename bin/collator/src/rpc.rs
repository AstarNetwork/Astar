//! Astar RPCs implementation.

use fc_rpc::{
    EthApi, EthApiServer, EthFilterApi, EthFilterApiServer, EthPubSubApi, EthPubSubApiServer,
    HexEncodedIdProvider, NetApi, NetApiServer, OverrideHandle, RuntimeApiStorageOverride,
    SchemaV1Override, StorageOverride, Web3Api, Web3ApiServer,
};
use fc_rpc_core::types::{FilterPool, PendingTransactions};
use frame_rpc_system::{FullSystem, SystemApi};
use jsonrpc_pubsub::manager::SubscriptionManager;
use pallet_ethereum::EthereumStorageSchema;
use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
use sc_client_api::{AuxStore, Backend, BlockchainEvents, StateBackend, StorageProvider};
use sc_network::NetworkService;
pub use sc_rpc::{DenyUnsafe, SubscriptionTaskExecutor};
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{
    Backend as BlockchainBackend, Error as BlockChainError, HeaderBackend, HeaderMetadata,
};
use sp_runtime::traits::BlakeTwo256;
use std::collections::BTreeMap;
use std::sync::Arc;

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
            source: fc_db::DatabaseSettingsSrc::RocksDb {
                path,
                cache_size: 0,
            },
        },
    )?))
}

/// Full client dependencies
pub struct FullDeps<C, P> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Network service
    pub network: Arc<NetworkService<Block, Hash>>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    /// The Node authority flag
    pub is_authority: bool,
    /// Frontier Backend.
    pub frontier_backend: Arc<fc_db::Backend<Block>>,
    /// Ethereum pending transactions.
    pub pending_transactions: PendingTransactions,
    /// EthFilterApi pool.
    pub filter_pool: FilterPool,
}

/// Instantiate all RPC extensions.
pub fn create_full<C, P, BE>(
    deps: FullDeps<C, P>,
    subscription_task_executor: SubscriptionTaskExecutor,
) -> jsonrpc_core::IoHandler<sc_rpc::Metadata>
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
    C::Api: frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
        + pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
        + fp_rpc::EthereumRuntimeRPCApi<Block>
        + BlockBuilder<Block>,
    P: TransactionPool<Block = Block> + Sync + Send + 'static,
    BE: Backend<Block> + 'static,
    BE::State: StateBackend<BlakeTwo256>,
    BE::Blockchain: BlockchainBackend<Block>,
{
    let mut io = jsonrpc_core::IoHandler::default();
    let FullDeps {
        client,
        pool,
        network,
        deny_unsafe,
        is_authority,
        frontier_backend,
        pending_transactions,
        filter_pool,
    } = deps;

    io.extend_with(SystemApi::to_delegate(FullSystem::new(
        client.clone(),
        pool.clone(),
        deny_unsafe,
    )));

    io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
        client.clone(),
    )));

    let mut overrides_map = BTreeMap::new();
    overrides_map.insert(
        EthereumStorageSchema::V1,
        Box::new(SchemaV1Override::new(client.clone()))
            as Box<dyn StorageOverride<_> + Send + Sync>,
    );

    let overrides = Arc::new(OverrideHandle {
        schemas: overrides_map,
        fallback: Box::new(RuntimeApiStorageOverride::new(client.clone())),
    });

    let transaction_converter = shiden_runtime::TransactionConverter;
    let max_past_logs: u32 = 10_000;
    let max_stored_filters: usize = 500;

    io.extend_with(EthApiServer::to_delegate(EthApi::new(
        client.clone(),
        pool.clone(),
        transaction_converter,
        network.clone(),
        pending_transactions,
        Default::default(),
        overrides.clone(),
        frontier_backend.clone(),
        is_authority,
        max_past_logs,
    )));

    io.extend_with(EthFilterApiServer::to_delegate(EthFilterApi::new(
        client.clone(),
        frontier_backend,
        filter_pool,
        max_stored_filters,
        overrides.clone(),
        max_past_logs,
    )));

    io.extend_with(NetApiServer::to_delegate(NetApi::new(
        client.clone(),
        network.clone(),
        true,
    )));

    io.extend_with(Web3ApiServer::to_delegate(Web3Api::new(client.clone())));

    io.extend_with(EthPubSubApiServer::to_delegate(EthPubSubApi::new(
        pool,
        client.clone(),
        network,
        SubscriptionManager::<HexEncodedIdProvider>::with_id_provider(
            HexEncodedIdProvider::default(),
            Arc::new(subscription_task_executor),
        ),
        overrides,
    )));

    io
}
