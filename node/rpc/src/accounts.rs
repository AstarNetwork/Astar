//! Node-specific RPC methods for Accounts.

use std::sync::Arc;

use client::blockchain::HeaderBackend;
use jsonrpc_core::{Result, Error, ErrorCode};
use jsonrpc_derive::rpc;
use plasm_primitives::{
	AccountId, Index, AccountNonceApi, Block, BlockId,
};
use codec::Encode;
use sr_primitives::traits;
use substrate_primitives::hexdisplay::HexDisplay;
use transaction_pool::txpool::{self, Pool};

pub use self::gen_client::Client as AccountsClient;

const RUNTIME_ERROR: i64 = 1;

/// Accounts RPC methods.
#[rpc]
pub trait AccountsApi {
	/// Returns the next valid index (aka nonce) for given account.
	///
	/// This method takes into consideration all pending transactions
	/// currently in the pool and if no transactions are found in the pool
	/// it fallbacks to query the index from the runtime (aka. state nonce).
	#[rpc(name = "account_nextIndex")]
	fn nonce(&self, account: AccountId) -> Result<Index>;
}

/// An implementation of Accounts specific RPC methods.
pub struct Accounts<P: txpool::ChainApi, C> {
	client: Arc<C>,
	pool: Arc<Pool<P>>,
}

impl<P: txpool::ChainApi, C> Accounts<P, C> {
	/// Create new `Accounts` given client and transaction pool.
	pub fn new(client: Arc<C>, pool: Arc<Pool<P>>) -> Self {
		Accounts {
			client,
			pool
		}
	}
}

impl<P, C> AccountsApi for Accounts<P, C>
where
	C: traits::ProvideRuntimeApi,
	C: HeaderBackend<Block>,
	C: Send + Sync + 'static,
	C::Api: AccountNonceApi<Block>,
	P: txpool::ChainApi + Sync + Send + 'static,
{
	fn nonce(&self, account: AccountId) -> Result<Index> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		let at = BlockId::hash(best);

		let nonce = api.account_nonce(&at, account.clone()).map_err(|e| Error {
			code: ErrorCode::ServerError(RUNTIME_ERROR),
			message: "Unable to query nonce.".into(),
			data: Some(format!("{:?}", e).into()),
		})?;

		log::debug!(target: "rpc", "State nonce for {}: {}", account, nonce);
		// Now we need to query the transaction pool
		// and find transactions originating from the same sender.
		//
		// Since extrinsics are opaque to us, we look for them using
		// `provides` tag. And increment the nonce if we find a transaction
		// that matches the current one.
		let mut current_nonce = nonce;
		let mut current_tag = (account.clone(), nonce).encode();
		for tx in self.pool.ready() {
			log::debug!(
				target: "rpc",
				"Current nonce to {:?}, checking {} vs {:?}",
				current_nonce,
				HexDisplay::from(&current_tag),
				tx.provides.iter().map(|x| format!("{}", HexDisplay::from(x))).collect::<Vec<_>>(),
			);
			// since transactions in `ready()` need to be ordered by nonce
			// it's fine to continue with current iterator.
			if tx.provides.get(0) == Some(&current_tag) {
				current_nonce += 1;
				current_tag = (account.clone(), current_nonce).encode();
			}
		}

		Ok(current_nonce)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	use plasm_runtime::{CheckedExtrinsic, Call, TimestampCall};
	use codec::Decode;
	use plasm_testing::{
		client::{ClientExt, TestClientBuilder, TestClientBuilderExt},
		keyring::{self, alice, signed_extra},
	};

	const VERSION: u32 = plasm_runtime::VERSION.spec_version;

	#[test]
	fn should_return_next_nonce_for_some_account() {
		// given
		let _ = env_logger::try_init();
		let client = Arc::new(TestClientBuilder::new().build());
		let pool = Arc::new(Pool::new(Default::default(), transaction_pool::ChainApi::new(client.clone())));

		let new_transaction = |extra| {
			let ex = CheckedExtrinsic {
				signed: Some((alice().into(), extra)),
				function: Call::Timestamp(TimestampCall::set(5)),
			};
			let xt = keyring::sign(ex, VERSION, client.genesis_hash().into());
			// Convert to OpaqueExtrinsic
			let encoded = xt.encode();
			plasm_primitives::UncheckedExtrinsic::decode(&mut &*encoded).unwrap()
		};
		// Populate the pool
		let ext0 = new_transaction(signed_extra(0, 0));
		pool.submit_one(&BlockId::number(0), ext0).unwrap();
		let ext1 = new_transaction(signed_extra(1, 0));
		pool.submit_one(&BlockId::number(0), ext1).unwrap();

		let accounts = Accounts::new(client, pool);

		// when
		let nonce = accounts.nonce(alice().into());

		// then
		assert_eq!(nonce.unwrap(), 2);
	}
}
