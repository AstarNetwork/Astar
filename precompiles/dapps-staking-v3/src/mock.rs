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

use super::*;

use fp_evm::{IsPrecompileResult, Precompile};
use frame_support::{
    construct_runtime, parameter_types,
    traits::{fungible::Mutate, ConstU128, ConstU64},
    weights::{RuntimeDbWeight, Weight},
};
use pallet_evm::{
    AddressMapping, EnsureAddressNever, EnsureAddressRoot, PrecompileResult, PrecompileSet,
};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_arithmetic::fixed_point::FixedU64;
use sp_core::{H160, H256};
use sp_io::TestExternalities;
use sp_runtime::{
    traits::{BlakeTwo256, ConstU32, IdentityLookup},
    AccountId32,
};
extern crate alloc;

use astar_primitives::{
    dapp_staking::{CycleConfiguration, StakingRewardHandler},
    testing::Header,
    Balance, BlockNumber,
};
use pallet_dapp_staking_v3::PriceProvider;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

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
    scale_info::TypeInfo,
)]

pub enum TestAccount {
    Empty,
    Alex,
    Bobo,
    Dino,
}

impl Default for TestAccount {
    fn default() -> Self {
        Self::Empty
    }
}

// needed for associated type in pallet_evm
impl AddressMapping<AccountId32> for TestAccount {
    fn into_account_id(h160_account: H160) -> AccountId32 {
        match h160_account {
            a if a == H160::repeat_byte(0x01) => TestAccount::Alex.into(),
            a if a == H160::repeat_byte(0x02) => TestAccount::Bobo.into(),
            a if a == H160::repeat_byte(0x03) => TestAccount::Dino.into(),
            _ => TestAccount::Empty.into(),
        }
    }
}

impl From<TestAccount> for H160 {
    fn from(x: TestAccount) -> H160 {
        match x {
            TestAccount::Alex => H160::repeat_byte(0x01),
            TestAccount::Bobo => H160::repeat_byte(0x02),
            TestAccount::Dino => H160::repeat_byte(0x03),
            _ => Default::default(),
        }
    }
}

trait H160Conversion {
    fn to_h160(&self) -> H160;
}

impl H160Conversion for AccountId32 {
    fn to_h160(&self) -> H160 {
        let x = self.encode()[31];
        H160::repeat_byte(x)
    }
}

impl From<TestAccount> for AccountId32 {
    fn from(x: TestAccount) -> Self {
        match x {
            TestAccount::Alex => AccountId32::from([1u8; 32]),
            TestAccount::Bobo => AccountId32::from([2u8; 32]),
            TestAccount::Dino => AccountId32::from([3u8; 32]),
            _ => AccountId32::from([0u8; 32]),
        }
    }
}

pub const READ_WEIGHT: u64 = 3;
pub const WRITE_WEIGHT: u64 = 7;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
        pub const TestWeights: RuntimeDbWeight = RuntimeDbWeight {
            read: READ_WEIGHT,
            write: WRITE_WEIGHT,
        };
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Index = u64;
    type RuntimeCall = RuntimeCall;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId32;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<4>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type HoldIdentifier = ();
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<1>;
    type WeightInfo = ();
}

pub fn precompile_address() -> H160 {
    H160::from_low_u64_be(0x5001)
}

#[derive(Debug, Clone, Copy)]
pub struct DappStakingPrecompile<R>(PhantomData<R>);
impl<R> PrecompileSet for DappStakingPrecompile<R>
where
    R: pallet_evm::Config,
    DappStakingV3Precompile<R>: Precompile,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            a if a == precompile_address() => Some(DappStakingV3Precompile::<R>::execute(handle)),
            _ => None,
        }
    }

    fn is_precompile(&self, address: sp_core::H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: address == precompile_address(),
            extra_cost: 0,
        }
    }
}

parameter_types! {
    pub PrecompilesValue: DappStakingPrecompile<Test> = DappStakingPrecompile(Default::default());
    pub WeightPerGas: Weight = Weight::from_parts(1, 0);
}

impl pallet_evm::Config for Test {
    type FeeCalculator = ();
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type CallOrigin = EnsureAddressRoot<AccountId32>;
    type WithdrawOrigin = EnsureAddressNever<AccountId32>;
    type AddressMapping = TestAccount;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = DappStakingPrecompile<Test>;
    type PrecompilesValue = PrecompilesValue;
    type Timestamp = Timestamp;
    type ChainId = ();
    type OnChargeTransaction = ();
    type BlockGasLimit = ();
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type FindAuthor = ();
    type OnCreate = ();
    type WeightInfo = ();
    type GasLimitPovSizeRatio = ConstU64<4>;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<5>;
    type WeightInfo = ();
}

#[derive(
    PartialEq, Eq, Copy, Clone, Encode, Decode, Debug, scale_info::TypeInfo, MaxEncodedLen,
)]
pub enum MockSmartContract<AccountId32> {
    Evm(sp_core::H160),
    Wasm(AccountId32),
}

impl<AccountId32> Default for MockSmartContract<AccountId32> {
    fn default() -> Self {
        MockSmartContract::Evm(H160::repeat_byte(0x00))
    }
}

pub struct DummyPriceProvider;
impl PriceProvider for DummyPriceProvider {
    fn average_price() -> FixedU64 {
        FixedU64::from_rational(1, 10)
    }
}

pub struct DummyStakingRewardHandler;
impl StakingRewardHandler<AccountId32> for DummyStakingRewardHandler {
    fn staker_and_dapp_reward_pools(_total_staked_value: Balance) -> (Balance, Balance) {
        (
            Balance::from(1_000_000_000_000_u128),
            Balance::from(1_000_000_000_u128),
        )
    }

    fn bonus_reward_pool() -> Balance {
        Balance::from(3_000_000_u128)
    }

    fn payout_reward(beneficiary: &AccountId32, reward: Balance) -> Result<(), ()> {
        let _ = Balances::mint_into(beneficiary, reward);
        Ok(())
    }
}

pub struct DummyCycleConfiguration;
impl CycleConfiguration for DummyCycleConfiguration {
    fn periods_per_cycle() -> u32 {
        4
    }

    fn eras_per_voting_subperiod() -> u32 {
        8
    }

    fn eras_per_build_and_earn_subperiod() -> u32 {
        16
    }

    fn blocks_per_era() -> u32 {
        10
    }
}

impl pallet_dapp_staking_v3::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type Currency = Balances;
    type SmartContract = MockSmartContract<Self::AccountId>;
    type ManagerOrigin = frame_system::EnsureRoot<AccountId32>;
    type NativePriceProvider = DummyPriceProvider;
    type StakingRewardHandler = DummyStakingRewardHandler;
    type CycleConfiguration = DummyCycleConfiguration;
    type EraRewardSpanLength = ConstU32<8>;
    type RewardRetentionInPeriods = ConstU32<2>;
    type MaxNumberOfContracts = ConstU32<10>;
    type MaxUnlockingChunks = ConstU32<5>;
    type MinimumLockedAmount = ConstU128<10>;
    type UnlockingPeriod = ConstU32<2>;
    type MaxNumberOfStakedContracts = ConstU32<5>;
    type MinimumStakeAmount = ConstU128<3>;
    type NumberOfTiers = ConstU32<4>;
    type WeightInfo = pallet_dapp_staking_v3::weights::SubstrateWeight<Test>;
}

pub struct _ExternalityBuilder;
impl _ExternalityBuilder {
    pub fn _build(self) -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let balances = vec![10000; 9]
            .into_iter()
            .enumerate()
            .map(|(idx, amount)| ([idx as u8; 32].into(), amount))
            .collect();

        pallet_balances::GenesisConfig::<Test> { balances: balances }
            .assimilate_storage(&mut storage)
            .ok();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

construct_runtime!(
    pub struct Test
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Balances: pallet_balances,
        Evm: pallet_evm,
        Timestamp: pallet_timestamp,
        DappsStaking: pallet_dapp_staking_v3,
    }
);
