//! A collection of node-specific RPC methods.

// Our drop-in replacements for Frontier's RPC servers.
mod pubsub_hotfixes;
mod server_hotfixes;

use std::{fmt, sync::Arc};

use frontier_rpc::HexEncodedIdProvider;
use jsonrpc_pubsub::manager::SubscriptionManager;
use plasm_primitives::{AccountId, Balance, Block, BlockNumber, Hash, Index};
use sc_client_api::{
    backend::{AuxStore, Backend, StateBackend, StorageProvider},
    client::BlockchainEvents,
};
use sc_network::NetworkService;
use sc_rpc::SubscriptionTaskExecutor;
use sc_rpc_api::DenyUnsafe;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_runtime::traits::BlakeTwo256;
use sp_transaction_pool::TransactionPool;

/// Light client extra dependencies.
pub struct LightDeps<C, F, P> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Remote access to the blockchain (async).
    pub remote_blockchain: Arc<dyn sc_client_api::light::RemoteBlockchain<Block>>,
    /// Fetcher instance.
    pub fetcher: Arc<F>,
}

/// Full client dependencies.
pub struct FullDeps<C, P> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    /// The Node authority flag
    pub is_authority: bool,
    /// Network service
    pub network: Arc<NetworkService<Block, Hash>>,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, BE>(
    deps: FullDeps<C, P>,
    subscription_task_executor: SubscriptionTaskExecutor,
) -> jsonrpc_core::IoHandler<sc_rpc::Metadata>
where
    BE: Backend<Block> + 'static,
    BE::State: StateBackend<BlakeTwo256>,
    C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
    C: BlockchainEvents<Block>,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
    C: Send + Sync + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    C::Api: BlockBuilder<Block>,
    C::Api: pallet_contracts_rpc::ContractsRuntimeApi<Block, AccountId, Balance, BlockNumber>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: frontier_rpc_primitives::EthereumRuntimeRPCApi<Block>,
    <C::Api as sp_api::ApiErrorExt>::Error: fmt::Debug,
    P: TransactionPool<Block = Block> + 'static,
{
    use frontier_rpc::{EthApiServer, EthPubSubApiServer, NetApi, NetApiServer};
    use pallet_contracts_rpc::{Contracts, ContractsApi};
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
    use substrate_frame_rpc_system::{FullSystem, SystemApi};
    // Our drop in replacements for the Eth APIs. These can be removed after
    // https://github.com/paritytech/frontier/pull/199 lands
    use pubsub_hotfixes::EthPubSubApi;
    use server_hotfixes::EthApi;

    let mut io = jsonrpc_core::IoHandler::default();
    let FullDeps {
        client,
        pool,
        deny_unsafe,
        is_authority,
        network,
    } = deps;

    io.extend_with(SystemApi::to_delegate(FullSystem::new(
        client.clone(),
        pool.clone(),
        deny_unsafe,
    )));
    // Making synchronous calls in light client freezes the browser currently,
    // more context: https://github.com/paritytech/substrate/pull/3480
    // These RPCs should use an asynchronous caller instead.
    io.extend_with(ContractsApi::to_delegate(Contracts::new(client.clone())));
    io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
        client.clone(),
    )));

    // We currently don't want to support signing in the node. Users should prefer external tools
    // for transaction signing. So just pass in an empty vector of signers.
    let signers = Vec::new();
    io.extend_with(EthApiServer::to_delegate(EthApi::new(
        client.clone(),
        pool.clone(),
        plasm_runtime::TransactionConverter,
        network.clone(),
        signers,
        is_authority,
    )));
    io.extend_with(NetApiServer::to_delegate(NetApi::new(
        client.clone(),
        network.clone(),
    )));
    io.extend_with(EthPubSubApiServer::to_delegate(EthPubSubApi::new(
        pool.clone(),
        client.clone(),
        network.clone(),
        SubscriptionManager::<HexEncodedIdProvider>::with_id_provider(
            HexEncodedIdProvider::default(),
            Arc::new(subscription_task_executor),
        ),
    )));

    io
}

/// Instantiate all Light RPC extensions.
pub fn create_light<C, P, M, F>(deps: LightDeps<C, F, P>) -> jsonrpc_core::IoHandler<M>
where
    C: sp_blockchain::HeaderBackend<Block>,
    C: Send + Sync + 'static,
    F: sc_client_api::light::Fetcher<Block> + 'static,
    P: TransactionPool + 'static,
    M: jsonrpc_core::Metadata + Default,
{
    use substrate_frame_rpc_system::{LightSystem, SystemApi};

    let LightDeps {
        client,
        pool,
        remote_blockchain,
        fetcher,
    } = deps;
    let mut io = jsonrpc_core::IoHandler::default();
    io.extend_with(SystemApi::<Hash, AccountId, Index>::to_delegate(
        LightSystem::new(client, remote_blockchain, fetcher, pool),
    ));

    io
}
