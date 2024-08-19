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

#![allow(missing_docs)]

use astar_primitives::{AccountId, Balance, Block, Nonce};
use frame_support::weights::Weight;
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{OpaqueMetadata, H160, H256, U256};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
    traits::Block as BlockT,
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, Permill,
};
use sp_version::RuntimeVersion;

pub struct Runtime;

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            unimplemented!()
        }

        fn execute_block(_block: Block) {
            unimplemented!()
        }

        fn initialize_block(_header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            unimplemented!()
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            unimplemented!()
        }

        fn metadata_at_version(_version: u32) -> Option<OpaqueMetadata> {
            unimplemented!()
        }

        fn metadata_versions() -> sp_std::vec::Vec<u32> {
            unimplemented!()
        }
    }

    impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
        fn collect_collation_info(_header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
            unimplemented!()
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            unimplemented!()
        }

        fn authorities() -> Vec<AuraId> {
            unimplemented!()
        }
    }

    impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
        fn can_build_upon(_included_hash: <Block as BlockT>::Hash, _slot: cumulus_primitives_aura::Slot) -> bool {
            unimplemented!()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(_extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            unimplemented!()
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            unimplemented!()
        }

        fn inherent_extrinsics(_data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            unimplemented!()
        }

        fn check_inherents(_block: Block, _data: InherentData) -> CheckInherentsResult {
            unimplemented!()
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(_account: AccountId) -> Nonce {
            unimplemented!()
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(_seed: Option<Vec<u8>>) -> Vec<u8> {
            unimplemented!()
        }

        fn decode_session_keys(_encoded: Vec<u8>) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
            unimplemented!()
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
        Block,
        Balance,
    > for Runtime {
        fn query_info(_uxt: <Block as BlockT>::Extrinsic, _len: u32) -> RuntimeDispatchInfo<Balance> {
            unimplemented!()
        }
        fn query_fee_details(_uxt: <Block as BlockT>::Extrinsic, _len: u32) -> FeeDetails<Balance> {
            unimplemented!()
        }
        fn query_weight_to_fee(_weight: Weight) -> Balance {
            unimplemented!()
        }
        fn query_length_to_fee(_length: u32) -> Balance {
            unimplemented!()
        }
    }

    impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
        fn chain_id() -> u64 {
            unimplemented!()
        }

        fn account_basic(_address: H160) -> pallet_evm::Account {
            unimplemented!()
        }

        fn gas_price() -> U256 {
            unimplemented!()
        }

        fn account_code_at(_address: H160) -> Vec<u8> {
            unimplemented!()
        }

        fn author() -> H160 {
            unimplemented!()
        }

        fn storage_at(_address: H160, _index: U256) -> H256 {
            unimplemented!()
        }

        fn call(
            _from: H160,
            _to: H160,
            _data: Vec<u8>,
            _value: U256,
            _gas_limit: U256,
            _max_fee_per_gas: Option<U256>,
            _max_priority_fee_per_gas: Option<U256>,
            _nonce: Option<U256>,
            _estimate: bool,
            _access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
            unimplemented!()
        }

        fn create(
            _from: H160,
            _data: Vec<u8>,
            _value: U256,
            _gas_limit: U256,
            _max_fee_per_gas: Option<U256>,
            _max_priority_fee_per_gas: Option<U256>,
            _nonce: Option<U256>,
            _estimate: bool,
            _access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
            unimplemented!()
        }

        fn current_transaction_statuses() -> Option<Vec<fp_rpc::TransactionStatus>> {
            unimplemented!()
        }

        fn current_block() -> Option<pallet_ethereum::Block> {
            unimplemented!()
        }

        fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
            unimplemented!()
        }

        fn current_all() -> (
            Option<pallet_ethereum::Block>,
            Option<Vec<pallet_ethereum::Receipt>>,
            Option<Vec<fp_rpc::TransactionStatus>>,
        ) {
            unimplemented!()
        }

        fn extrinsic_filter(_xts: Vec<<Block as BlockT>::Extrinsic>) -> Vec<pallet_ethereum::Transaction> {
            unimplemented!()
        }

        fn elasticity() -> Option<Permill> {
            unimplemented!()
        }

        fn gas_limit_multiplier_support() {}

        fn pending_block(_xts: Vec<<Block as BlockT>::Extrinsic>) -> (Option<pallet_ethereum::Block>, Option<Vec<fp_rpc::TransactionStatus>>) {
            unimplemented!()
        }
    }

    impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
        fn convert_transaction(_transaction: pallet_ethereum::Transaction) -> <Block as BlockT>::Extrinsic {
            unimplemented!()
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(_source: TransactionSource, _tx: <Block as BlockT>::Extrinsic, _block_hash: <Block as BlockT>::Hash) -> TransactionValidity {
            unimplemented!()
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(_header: &<Block as BlockT>::Header) {
            unimplemented!()
        }
    }

    impl moonbeam_rpc_primitives_debug::DebugRuntimeApi<Block> for Runtime {
        fn trace_transaction(_extrinsics: Vec<<Block as BlockT>::Extrinsic>, _traced_transaction: &pallet_ethereum::Transaction, _header: &<Block as BlockT>::Header) -> Result<(), sp_runtime::DispatchError> {
            unimplemented!()
        }

        fn trace_block(_extrinsics: Vec<<Block as BlockT>::Extrinsic>, _known_transactions: Vec<H256>, _header: &<Block as BlockT>::Header) -> Result<(), sp_runtime::DispatchError> {
            unimplemented!()
        }

        fn trace_call(
            _header: &<Block as BlockT>::Header,
            _from: H160,
            _to: H160,
            _data: Vec<u8>,
            _value: U256,
            _gas_limit: U256,
            _max_fee_per_gas: Option<U256>,
            _max_priority_fee_per_gas: Option<U256>,
            _nonce: Option<U256>,
            _access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<(), sp_runtime::DispatchError> {
            unimplemented!()
        }
    }

    impl moonbeam_rpc_primitives_txpool::TxPoolRuntimeApi<Block> for Runtime {
        fn extrinsic_filter(_xts_ready: Vec<<Block as BlockT>::Extrinsic>, _xts_future: Vec<<Block as BlockT>::Extrinsic>) -> moonbeam_rpc_primitives_txpool::TxPoolResponse {
            unimplemented!()
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(_extra: bool) -> (Vec<frame_benchmarking::BenchmarkList>, Vec<frame_support::traits::StorageInfo>) {
            unimplemented!()
        }

        fn dispatch_benchmark(_config: frame_benchmarking::BenchmarkConfig) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            unimplemented!()
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(_checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
            unimplemented!()
        }

        fn execute_block(_block: Block, _state_root_check: bool, _signature_check: bool, _select: frame_try_runtime::TryStateSelect) -> Weight {
            unimplemented!()
        }
    }
}
