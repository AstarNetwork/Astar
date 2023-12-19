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

use crate::{self as pallet_dapp_staking_migration, *};

use frame_support::{
    assert_ok, construct_runtime, parameter_types,
    traits::{fungible::Mutate as FunMutate, ConstBool, ConstU128, ConstU32, Currency},
    weights::Weight,
    PalletId,
};
use sp_arithmetic::fixed_point::FixedU64;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::traits::{BlakeTwo256, IdentityLookup};

use astar_primitives::{
    dapp_staking::{CycleConfiguration, SmartContract, StakingRewardHandler},
    testing::Header,
    Balance, BlockNumber,
};

pub(crate) type AccountId = u64;

pub(crate) const EXISTENTIAL_DEPOSIT: Balance = 2;
pub(crate) const MINIMUM_LOCK_AMOUNT: Balance = 10;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub struct Test
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Balances: pallet_balances,
        DappStaking: pallet_dapp_staking_v3,
        DappsStaking: pallet_dapps_staking,
        DappStakingMigration: pallet_dapp_staking_migration,
    }
);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
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
    type AccountId = AccountId;
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
    type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
    type AccountStore = System;
    type HoldIdentifier = ();
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<1>;
    type WeightInfo = ();
}

pub struct DummyPriceProvider;
impl pallet_dapp_staking_v3::PriceProvider for DummyPriceProvider {
    fn average_price() -> FixedU64 {
        FixedU64::from_rational(1, 10)
    }
}

pub struct DummyStakingRewardHandler;
impl StakingRewardHandler<AccountId> for DummyStakingRewardHandler {
    fn staker_and_dapp_reward_pools(_total_staked_value: Balance) -> (Balance, Balance) {
        (
            Balance::from(1_000_000_000_000_u128),
            Balance::from(1_000_000_000_u128),
        )
    }

    fn bonus_reward_pool() -> Balance {
        Balance::from(3_000_000_u128)
    }

    fn payout_reward(beneficiary: &AccountId, reward: Balance) -> Result<(), ()> {
        let _ = Balances::mint_into(beneficiary, reward);
        Ok(())
    }
}

pub(crate) type MockSmartContract = SmartContract<AccountId>;

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchmarkHelper<SC, ACC>(sp_std::marker::PhantomData<(SC, ACC)>);
#[cfg(feature = "runtime-benchmarks")]
impl pallet_dapp_staking_v3::BenchmarkHelper<MockSmartContract, AccountId>
    for BenchmarkHelper<MockSmartContract, AccountId>
{
    fn get_smart_contract(id: u32) -> MockSmartContract {
        MockSmartContract::Wasm(id as AccountId)
    }

    fn set_balance(account: &AccountId, amount: Balance) {
        use frame_support::traits::fungible::Unbalanced as FunUnbalanced;
        Balances::write_balance(account, amount)
            .expect("Must succeed in test/benchmark environment.");
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
    type SmartContract = MockSmartContract;
    type ManagerOrigin = frame_system::EnsureRoot<AccountId>;
    type NativePriceProvider = DummyPriceProvider;
    type StakingRewardHandler = DummyStakingRewardHandler;
    type CycleConfiguration = DummyCycleConfiguration;
    type EraRewardSpanLength = ConstU32<8>;
    type RewardRetentionInPeriods = ConstU32<2>;
    type MaxNumberOfContracts = ConstU32<10>;
    type MaxUnlockingChunks = ConstU32<5>;
    type MinimumLockedAmount = ConstU128<MINIMUM_LOCK_AMOUNT>;
    type UnlockingPeriod = ConstU32<2>;
    type MaxNumberOfStakedContracts = ConstU32<5>;
    type MinimumStakeAmount = ConstU128<3>;
    type NumberOfTiers = ConstU32<4>;
    type WeightInfo = pallet_dapp_staking_v3::weights::SubstrateWeight<Test>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = BenchmarkHelper<MockSmartContract, AccountId>;
}

parameter_types! {
    pub const DappsStakingPalletId: PalletId = PalletId(*b"mokdpstk");
}

impl pallet_dapps_staking::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BlockPerEra = ConstU32<10>;
    type RegisterDeposit = ConstU128<100>;
    type SmartContract = MockSmartContract;
    type WeightInfo = pallet_dapps_staking::weights::SubstrateWeight<Test>;
    type MaxNumberOfStakersPerContract = ConstU32<10>;
    type MinimumStakingAmount = ConstU128<MINIMUM_LOCK_AMOUNT>;
    type PalletId = DappsStakingPalletId;
    type MinimumRemainingAmount = ConstU128<1>;
    type MaxUnlockingChunks = ConstU32<5>;
    type UnbondingPeriod = ConstU32<3>;
    type MaxEraStakeValues = ConstU32<10>;
    type UnregisteredDappRewardRetention = ConstU32<10>;
    type ForcePalletDisabled = ConstBool<false>;
}

impl pallet_dapp_staking_migration::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = crate::weights::SubstrateWeight<Test>;
}

pub struct ExtBuilder;
impl ExtBuilder {
    pub fn build() -> TestExternalities {
        // Normal behavior is for reward payout to succeed
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let balances = vec![1000; 9]
            .into_iter()
            .enumerate()
            .map(|(idx, amount)| (idx as u64 + 1, amount))
            .collect();

        pallet_balances::GenesisConfig::<Test> { balances: balances }
            .assimilate_storage(&mut storage)
            .ok();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| {
            System::set_block_number(1);
        });

        ext
    }
}

/// Initialize old dApps staking storage.
///
/// This is kept outside of the test ext creation since the same mock is reused
/// in the benchmarks code.
pub fn init() {
    let dapps_number = 10_u32;
    let staker = dapps_number.into();
    Balances::make_free_balance_be(&staker, 1_000_000_000_000_000_000);

    // Add some dummy dApps to the old pallet & stake on them.
    for idx in 0..dapps_number {
        let developer = idx.into();
        Balances::make_free_balance_be(&developer, 1_000_000_000_000);
        let smart_contract = MockSmartContract::Wasm(idx.into());
        assert_ok!(pallet_dapps_staking::Pallet::<Test>::register(
            RawOrigin::Root.into(),
            developer,
            smart_contract.clone(),
        ));
        assert_ok!(pallet_dapps_staking::Pallet::<Test>::bond_and_stake(
            RawOrigin::Signed(staker.clone()).into(),
            smart_contract,
            1_000,
        ));
    }
}
