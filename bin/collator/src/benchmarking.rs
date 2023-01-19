use crate::primitives::Block;
use codec::Encode;
use sc_executor::NativeElseWasmExecutor;
use sc_service::TFullClient;
use sp_api::ConstructRuntimeApi;
use sp_core::{Pair, H256};
use sp_keyring::Sr25519Keyring;
use sp_runtime::OpaqueExtrinsic;
use std::sync::Arc;

/// Generates `System::Remark` extrinsics for the benchmarks.
///
/// Note: Should only be used for benchmarking.
pub struct RemarkBuilder<RuntimeApi, Executor>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
}

impl<RuntimeApi, Executor> RemarkBuilder<RuntimeApi, Executor>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    /// Creates a new [`Self`] from the given client.
    pub fn new(
        client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
    ) -> Self {
        Self { client }
    }
}

impl<RuntimeApi, Executor> frame_benchmarking_cli::ExtrinsicBuilder
    for RemarkBuilder<RuntimeApi, Executor>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    fn pallet(&self) -> &str {
        "system"
    }

    fn extrinsic(&self) -> &str {
        "remark"
    }

    fn build(&self, nonce: u32) -> std::result::Result<OpaqueExtrinsic, &'static str> {
        use local_runtime::RuntimeCall;
        use sc_client_api::UsageProvider;

        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
        let signer = Sr25519Keyring::Bob.pair();
        let period = local_runtime::BlockHashCount::get()
            .checked_next_power_of_two()
            .map(|c| c / 2)
            .unwrap_or(2) as u64;
        let genesis = self.client.usage_info().chain.best_hash;

        Ok(self
            .client
            .sign_call(call, nonce, 0, period, genesis, signer))
    }
}

/// Helper trait to implement [`frame_benchmarking_cli::ExtrinsicBuilder`].
///
/// Should only be used for benchmarking since it makes strong assumptions
/// about the chain state that these calls will be valid for.
trait BenchmarkCallSigner<RuntimeCall: Encode + Clone, Signer: Pair> {
    /// Signs a call together with the signed extensions of the specific runtime.
    ///
    /// Only works if the current block is the genesis block since the
    /// `CheckMortality` check is mocked by using the genesis block.
    fn sign_call(
        &self,
        call: RuntimeCall,
        nonce: u32,
        current_block: u64,
        period: u64,
        genesis: H256,
        acc: Signer,
    ) -> OpaqueExtrinsic;
}

impl<RuntimeApi, Executor> BenchmarkCallSigner<local_runtime::RuntimeCall, sp_core::sr25519::Pair>
    for TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    fn sign_call(
        &self,
        call: local_runtime::RuntimeCall,
        nonce: u32,
        current_block: u64,
        period: u64,
        genesis: H256,
        acc: sp_core::sr25519::Pair,
    ) -> OpaqueExtrinsic {
        use local_runtime as runtime;

        let extra: runtime::SignedExtra = (
            frame_system::CheckSpecVersion::<runtime::Runtime>::new(),
            frame_system::CheckTxVersion::<runtime::Runtime>::new(),
            frame_system::CheckGenesis::<runtime::Runtime>::new(),
            frame_system::CheckMortality::<runtime::Runtime>::from(
                sp_runtime::generic::Era::mortal(period, current_block),
            ),
            frame_system::CheckNonce::<runtime::Runtime>::from(nonce),
            frame_system::CheckWeight::<runtime::Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<runtime::Runtime>::from(0),
        );

        let payload = runtime::SignedPayload::from_raw(
            call.clone(),
            extra.clone(),
            (
                runtime::VERSION.spec_version,
                runtime::VERSION.transaction_version,
                genesis.clone(),
                genesis,
                (),
                (),
                (),
            ),
        );

        let signature = payload.using_encoded(|p| acc.sign(p));
        runtime::UncheckedExtrinsic::new_signed(
            call,
            sp_runtime::AccountId32::from(acc.public()).into(),
            runtime::Signature::Sr25519(signature.clone()),
            extra,
        )
        .into()
    }
}

/// Generates inherent data for benchmarking Astar, Shiden and Shibuya.
///
/// Not to be used outside of benchmarking since it returns mocked values.
pub fn benchmark_inherent_data(
) -> std::result::Result<sp_inherents::InherentData, sp_inherents::Error> {
    use sp_inherents::InherentDataProvider;
    let mut inherent_data = sp_inherents::InherentData::new();

    // Assume that all runtimes have the `timestamp` pallet.
    let d = std::time::Duration::from_millis(0);
    let timestamp = sp_timestamp::InherentDataProvider::new(d.into());
    timestamp.provide_inherent_data(&mut inherent_data)?;

    Ok(inherent_data)
}
