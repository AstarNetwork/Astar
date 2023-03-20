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

use crate::primitives::{AccountId, Balance, Block, BlockId};
use cumulus_primitives_core::PersistedValidationData;
use cumulus_primitives_parachain_inherent::{ParachainInherentData, INHERENT_IDENTIFIER};
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
use parity_scale_codec::Encode;
use polkadot_runtime_common::BlockHashCount;
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
        with_runtime! {
            self.client.as_ref(), {
                use runtime::{RuntimeCall, SystemCall};
                use sc_client_api::UsageProvider;

                let call = RuntimeCall::System(SystemCall::remark { remark: vec![] });
                let signer = Sr25519Keyring::Bob.pair();
                let period = BlockHashCount::get()
                    .checked_next_power_of_two()
                    .map(|c| c / 2)
                    .unwrap_or(2) as u64;
                let genesis = self.client.usage_info().chain.best_hash;

                Ok(self.client.sign_call(call, nonce, 0, period, genesis, signer))
            }
        }
    }
}

/// Generates `Balances::TransferKeepAlive` extrinsics for the benchmarks.
///
/// Note: Should only be used for benchmarking.
pub struct TransferKeepAliveBuilder<RuntimeApi, Executor>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
    dest: AccountId,
    value: Balance,
}

impl<RuntimeApi, Executor> TransferKeepAliveBuilder<RuntimeApi, Executor>
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
        dest: AccountId,
        value: Balance,
    ) -> Self {
        Self {
            client,
            dest,
            value,
        }
    }
}

impl<RuntimeApi, Executor> frame_benchmarking_cli::ExtrinsicBuilder
    for TransferKeepAliveBuilder<RuntimeApi, Executor>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    fn pallet(&self) -> &str {
        "balances"
    }

    fn extrinsic(&self) -> &str {
        "transfer_keep_alive"
    }

    fn build(&self, nonce: u32) -> std::result::Result<OpaqueExtrinsic, &'static str> {
        with_runtime! {
            self.client.as_ref(), {
                use runtime::{RuntimeCall, BalancesCall};
                use sc_client_api::UsageProvider;

                let call = RuntimeCall::Balances(BalancesCall::transfer_keep_alive {
                    dest: self.dest.clone().into(),
                    value: self.value.into(),
                });
                let signer = Sr25519Keyring::Bob.pair();

                let period = BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
                let genesis = self.client.usage_info().chain.best_hash;

                Ok(self.client.sign_call(call, nonce, 0, period, genesis, signer))
            }
        }
    }
}

/// Helper trait to implement [`frame_benchmarking_cli::ExtrinsicBuilder`].
///
/// Should only be used for benchmarking.
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

impl<RuntimeApi, Executor> BenchmarkCallSigner<astar_runtime::RuntimeCall, sp_core::sr25519::Pair>
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
        call: astar_runtime::RuntimeCall,
        nonce: u32,
        current_block: u64,
        period: u64,
        genesis: H256,
        acc: sp_core::sr25519::Pair,
    ) -> OpaqueExtrinsic {
        use astar_runtime as runtime;

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

impl<RuntimeApi, Executor> BenchmarkCallSigner<shiden_runtime::RuntimeCall, sp_core::sr25519::Pair>
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
        call: shiden_runtime::RuntimeCall,
        nonce: u32,
        current_block: u64,
        period: u64,
        genesis: H256,
        acc: sp_core::sr25519::Pair,
    ) -> OpaqueExtrinsic {
        use shiden_runtime as runtime;

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

impl<RuntimeApi, Executor> BenchmarkCallSigner<shibuya_runtime::RuntimeCall, sp_core::sr25519::Pair>
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
        call: shibuya_runtime::RuntimeCall,
        nonce: u32,
        current_block: u64,
        period: u64,
        genesis: H256,
        acc: sp_core::sr25519::Pair,
    ) -> OpaqueExtrinsic {
        use shibuya_runtime as runtime;

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

/// Provides the existential deposit that is only needed for benchmarking.
pub trait ExistentialDepositProvider {
    /// Returns the existential deposit.
    fn existential_deposit(&self) -> Balance;
}

impl<RuntimeApi, Executor> ExistentialDepositProvider
    for TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    fn existential_deposit(&self) -> Balance {
        with_runtime! {
            self,
            runtime::ExistentialDeposit::get()
        }
    }
}

/// Generates inherent data for benchmarking local node.
///
/// Not to be used outside of benchmarking since it returns mocked values.
pub fn local_benchmark_inherent_data(
) -> std::result::Result<sp_inherents::InherentData, sp_inherents::Error> {
    use sp_inherents::InherentDataProvider;
    let mut inherent_data = sp_inherents::InherentData::new();

    // Assume that all runtimes have the `timestamp` pallet.
    let d = std::time::Duration::from_millis(0);
    let timestamp = sp_timestamp::InherentDataProvider::new(d.into());
    futures::executor::block_on(timestamp.provide_inherent_data(&mut inherent_data))?;

    Ok(inherent_data)
}

/// Generates inherent data for benchmarking Astar, Shiden and Shibuya.
///
/// Not to be used outside of benchmarking since it returns mocked values.
pub fn para_benchmark_inherent_data(
) -> std::result::Result<sp_inherents::InherentData, sp_inherents::Error> {
    use sp_inherents::InherentDataProvider;
    let mut inherent_data = sp_inherents::InherentData::new();

    // Assume that all runtimes have the `timestamp` pallet.
    let d = std::time::Duration::from_millis(0);
    let timestamp = sp_timestamp::InherentDataProvider::new(d.into());
    futures::executor::block_on(timestamp.provide_inherent_data(&mut inherent_data))?;

    let sproof_builder = RelayStateSproofBuilder::default();
    let (relay_parent_storage_root, relay_chain_state) = sproof_builder.into_state_root_and_proof();
    // `relay_parent_number` should be bigger than 0 for benchmarking.
    // It is mocked value, any number except 0 is valid.
    let validation_data = PersistedValidationData {
        relay_parent_number: 1,
        relay_parent_storage_root,
        ..Default::default()
    };

    // Parachain blocks needs to include ParachainInherentData, otherwise block is invalid.
    let para_data = ParachainInherentData {
        validation_data,
        relay_chain_state,
        downward_messages: Default::default(),
        horizontal_messages: Default::default(),
    };

    inherent_data.put_data(INHERENT_IDENTIFIER, &para_data)?;

    Ok(inherent_data)
}

/// Interpret client's native runtime version name (at genesis block).
/// This is valid only for benchmarking, since genesis block is alreways refered to.
macro_rules! with_runtime {
	{
		$client:expr,
		$code:expr
	} => {
        match $client.runtime_version_at(&BlockId::Number(0)).unwrap().spec_name.to_string().as_str() {
            "astar" => {
                #[allow(unused_imports)]
				use astar_runtime as runtime;
				$code
            },
            "shiden" => {
                #[allow(unused_imports)]
				use shiden_runtime as runtime;
				$code
            },
            "shibuya" => {
                #[allow(unused_imports)]
				use shibuya_runtime as runtime;
				$code
            },
            _ => {
                #[allow(unused_imports)]
				use local_runtime as runtime;
				$code
            }
        }
	}
}

use with_runtime;
