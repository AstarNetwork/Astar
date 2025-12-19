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

use super::*;
use std::marker::PhantomData;

use fp_evm::{IsPrecompileResult, Precompile};
use frame_support::{
    construct_runtime, derive_impl, parameter_types, traits::ConstU64, weights::Weight,
};
pub use pallet_evm::{
    AddressMapping, EnsureAddressNever, EnsureAddressRoot, PrecompileResult, PrecompileSet,
};
use sp_core::{keccak_256, H160};
use sp_runtime::{
    traits::{ConstU32, IdentityLookup},
    AccountId32,
};

use frame_support::traits::Contains;

use astar_primitives::precompiles::DispatchFilterValidate;
pub type AccountId = AccountId32;
pub type Balance = u128;
pub type Block = frame_system::mocking::MockBlock<TestRuntime>;
pub const PRECOMPILE_ADDRESS: H160 = H160::repeat_byte(0x7B);

pub const ONE: u128 = 1_000_000_000_000_000_000;
pub const ALICE: AccountId32 = AccountId32::new([1u8; 32]);
pub const DUMMY: AccountId32 = AccountId32::new([2u8; 32]);

pub fn alice_secret() -> libsecp256k1::SecretKey {
    libsecp256k1::SecretKey::parse(&keccak_256(b"Alice")).unwrap()
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for TestRuntime {
    type Block = Block;
    type AccountId = AccountId32;
    type Lookup = IdentityLookup<AccountId32>;
    type AccountData = pallet_balances::AccountData<Balance>;
}

pub struct WhitelistedCalls;

impl Contains<RuntimeCall> for WhitelistedCalls {
    fn contains(call: &RuntimeCall) -> bool {
        match call {
            RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive { .. }) => true,
            RuntimeCall::System(frame_system::Call::remark { .. }) => true,
            RuntimeCall::Utility(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TestPrecompileSet<R>(PhantomData<R>);

impl<R> PrecompileSet for TestPrecompileSet<R>
where
    R: pallet_evm::Config,
    DispatchLockdrop<R, DispatchFilterValidate<RuntimeCall, WhitelistedCalls>>: Precompile,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            a if a == PRECOMPILE_ADDRESS => Some(DispatchLockdrop::<
                R,
                DispatchFilterValidate<RuntimeCall, WhitelistedCalls>,
            >::execute(handle)),
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: address == PRECOMPILE_ADDRESS,
            extra_cost: 0,
        }
    }
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for TestRuntime {
    type Balance = Balance;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for TestRuntime {
    type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
    pub const PrecompilesValue: TestPrecompileSet<TestRuntime> =
        TestPrecompileSet(PhantomData);
    pub WeightPerGas: Weight = Weight::from_parts(1, 0);
}

pub type PrecompileCall = DispatchLockdropCall<
    TestRuntime,
    DispatchFilterValidate<<TestRuntime as Config>::RuntimeCall, WhitelistedCalls>,
    ConstU32<8>,
>;

pub struct AddressMapper;
impl AddressMapping<astar_primitives::AccountId> for AddressMapper {
    fn into_account_id(account: H160) -> astar_primitives::AccountId {
        let mut account_id = [0u8; 32];
        account_id[0..20].clone_from_slice(&account.as_bytes());

        account_id
            .try_into()
            .expect("H160 is 20 bytes long so it must fit into 32 bytes; QED")
    }
}

impl pallet_evm::Config for TestRuntime {
    type FeeCalculator = ();
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type CallOrigin = EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = EnsureAddressNever<AccountId>;
    type AddressMapping = AddressMapper;
    type Currency = Balances;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = TestPrecompileSet<Self>;
    type PrecompilesValue = PrecompilesValue;
    type Timestamp = Timestamp;
    type ChainId = ChainId;
    type OnChargeTransaction = ();
    type BlockGasLimit = ();
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type FindAuthor = ();
    type OnCreate = ();
    type WeightInfo = ();
    type GasLimitPovSizeRatio = ConstU64<4>;
    type AccountProvider = pallet_evm::FrameSystemAccountProvider<Self>;
    type GasLimitStorageGrowthRatio = ConstU64<0>;
    type CreateOriginFilter = ();
    type CreateInnerOriginFilter = ();
}

impl pallet_utility::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = ();
}

parameter_types! {
    // 2 storage items with value size 20 and 32
    pub const AccountMappingStorageFee: u128 = 0;
    pub ChainId: u64 = 1024;
}

// Configure a mock runtime to test the pallet.
construct_runtime!(
    pub enum TestRuntime
    {
        System: frame_system,
        Evm: pallet_evm,
        Balances : pallet_balances,
        Timestamp: pallet_timestamp,
        Utility: pallet_utility,
    }
);

#[derive(Default)]
pub(crate) struct ExtBuilder;

impl ExtBuilder {
    pub(crate) fn build(self) -> sp_io::TestExternalities {
        use sp_runtime::BuildStorage;
        let t = frame_system::GenesisConfig::<TestRuntime>::default()
            .build_storage()
            .expect("Frame system builds valid default genesis config");

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
