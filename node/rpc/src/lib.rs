
//! A collection of node-specific RPC methods.
//!
//! Since `substrate` core functionality makes no assumptions
//! about the modules used inside the runtime, so do
//! RPC methods defined in `substrate-rpc` crate.
//! It means that `core/rpc` can't have any methods that
//! need some strong assumptions about the particular runtime.
//!
//! The RPCs available in this crate however can make some assumptions
//! about how the runtime is constructed and what `SRML` modules
//! are part of it. Therefore all node-runtime-specific RPCs can
//! be placed here or imported from corresponding `SRML` RPC definitions.

#![warn(missing_docs)]

use std::sync::Arc;

use plasm_primitives::{Block, AccountId, Index, Balance};
use plasm_runtime::UncheckedExtrinsic;
use sr_primitives::traits::ProvideRuntimeApi;
use transaction_pool::txpool::{ChainApi, Pool};

/// Instantiate all RPC extensions.
pub fn create<C, P, M>(client: Arc<C>, pool: Arc<Pool<P>>) -> jsonrpc_core::IoHandler<M> where
	C: ProvideRuntimeApi,
	C: client::blockchain::HeaderBackend<Block>,
	C: Send + Sync + 'static,
	C::Api: srml_system_rpc::AccountNonceApi<Block, AccountId, Index>,
	C::Api: srml_contracts_rpc::ContractsRuntimeApi<Block, AccountId, Balance>,
	C::Api: srml_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance, UncheckedExtrinsic>,
	P: ChainApi + Sync + Send + 'static,
	M: jsonrpc_core::Metadata + Default,
{
	use srml_system_rpc::{System, SystemApi};
	use srml_contracts_rpc::{Contracts, ContractsApi};
	use srml_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};

	let mut io = jsonrpc_core::IoHandler::default();
	io.extend_with(
		SystemApi::to_delegate(System::new(client.clone(), pool))
	);
	io.extend_with(
		ContractsApi::to_delegate(Contracts::new(client.clone()))
	);
	io.extend_with(
		TransactionPaymentApi::to_delegate(TransactionPayment::new(client))
	);
	io
}
