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

//! Testing utilities.

use super::*;

use fp_evm::{IsPrecompileResult, Precompile};
use frame_support::{
    construct_runtime, derive_impl, parameter_types, traits::ConstU64, weights::Weight,
};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

use pallet_evm::{
    AddressMapping, EnsureAddressNever, EnsureAddressRoot, PrecompileResult, PrecompileSet,
};
use sp_core::H160;
use sp_runtime::{traits::IdentityLookup, BuildStorage};

pub type AccountId = TestAccount;
pub type Balance = u128;
pub type Block = frame_system::mocking::MockBlock<Runtime>;

pub const PRECOMPILE_ADDRESS: H160 = H160::repeat_byte(0xBB);

#[derive(
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Clone,
    Encode,
    Decode,
    Debug,
    MaxEncodedLen,
    Serialize,
    Deserialize,
    derive_more::Display,
    TypeInfo,
)]
pub enum TestAccount {
    Alice,
    Bob,
    Charlie,
    Bogus,
    Precompile,
}

impl Default for TestAccount {
    fn default() -> Self {
        Self::Alice
    }
}

impl AddressMapping<TestAccount> for TestAccount {
    fn into_account_id(h160_account: H160) -> TestAccount {
        match h160_account {
            a if a == H160::repeat_byte(0xAA) => Self::Alice,
            a if a == H160::repeat_byte(0xBB) => Self::Bob,
            a if a == H160::repeat_byte(0xCC) => Self::Charlie,
            a if a == PRECOMPILE_ADDRESS => Self::Precompile,
            _ => Self::Bogus,
        }
    }
}

impl From<H160> for TestAccount {
    fn from(x: H160) -> TestAccount {
        TestAccount::into_account_id(x)
    }
}

impl From<TestAccount> for H160 {
    fn from(value: TestAccount) -> H160 {
        match value {
            TestAccount::Alice => H160::repeat_byte(0xAA),
            TestAccount::Bob => H160::repeat_byte(0xBB),
            TestAccount::Charlie => H160::repeat_byte(0xCC),
            TestAccount::Precompile => PRECOMPILE_ADDRESS,
            TestAccount::Bogus => Default::default(),
        }
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = Block;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type AccountData = pallet_balances::AccountData<Balance>;
}

#[derive(Debug, Clone, Copy)]
pub struct TestPrecompileSet<R>(PhantomData<R>);

impl<R> PrecompileSet for TestPrecompileSet<R>
where
    R: pallet_evm::Config,
    Sr25519Precompile<R>: Precompile,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            a if a == PRECOMPILE_ADDRESS => Some(Sr25519Precompile::<R>::execute(handle)),
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
    pub const MinimumPeriod: u64 = 5;
}

#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for Runtime {
    type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

parameter_types! {
    pub const PrecompilesValue: TestPrecompileSet<Runtime> =
        TestPrecompileSet(PhantomData);
    pub const WeightPerGas: Weight = Weight::from_parts(1, 0);
}

pub type PrecompileCall = Sr25519PrecompileCall<Runtime>;

impl pallet_evm::Config for Runtime {
    type FeeCalculator = ();
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type CallOrigin = EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = EnsureAddressNever<AccountId>;
    type AddressMapping = AccountId;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = TestPrecompileSet<Self>;
    type PrecompilesValue = PrecompilesValue;
    type Timestamp = Timestamp;
    type ChainId = ();
    type OnChargeTransaction = ();
    type BlockGasLimit = ();
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type OnCreate = ();
    type FindAuthor = ();
    type WeightInfo = ();
    type GasLimitPovSizeRatio = ConstU64<4>;
    type AccountProvider = pallet_evm::FrameSystemAccountProvider<Self>;
    type GasLimitStorageGrowthRatio = ConstU64<0>;
}

// Configure a mock runtime to test the pallet.
construct_runtime!(
    pub enum Runtime
    {
        System: frame_system,
        Balances: pallet_balances,
        Evm: pallet_evm,
        Timestamp: pallet_timestamp,
    }
);

#[derive(Default)]
pub(crate) struct ExtBuilder;

impl ExtBuilder {
    pub(crate) fn build(self) -> sp_io::TestExternalities {
        let t = frame_system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .expect("Frame system builds valid default genesis config");

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
