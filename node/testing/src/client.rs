//! Utilites to build a `TestClient` for `plasm-runtime`.

use sr_primitives::BuildStorage;

/// Re-export test-client utilities.
pub use test_client::*;

/// Call executor for `plasm-runtime` `TestClient`.
pub type Executor = substrate_executor::NativeExecutor<plasm_executor::Executor>;

/// Default backend type.
pub type Backend = client_db::Backend<plasm_primitives::Block>;

/// Test client type.
pub type Client = client::Client<
	Backend,
	client::LocalCallExecutor<Backend, Executor>,
	plasm_primitives::Block,
	plasm_runtime::RuntimeApi,
>;

/// Genesis configuration parameters for `TestClient`.
#[derive(Default)]
pub struct GenesisParameters {
	support_changes_trie: bool,
}

impl test_client::GenesisInit for GenesisParameters {
	fn genesis_storage(&self) -> (StorageOverlay, ChildrenStorageOverlay) {
		crate::genesis::config(self.support_changes_trie, None).build_storage().unwrap()
	}
}

/// A `test-runtime` extensions to `TestClientBuilder`.
pub trait TestClientBuilderExt: Sized {
	/// Create test client builder.
	fn new() -> Self;

	/// Build the test client.
	fn build(self) -> Client;
}

impl TestClientBuilderExt for test_client::TestClientBuilder<
	client::LocalCallExecutor<Backend, Executor>,
	Backend,
	GenesisParameters,
> {
	fn new() -> Self{
		Self::default()
	}

	fn build(self) -> Client {
		self.build_with_native_executor(None).0
	}
}


