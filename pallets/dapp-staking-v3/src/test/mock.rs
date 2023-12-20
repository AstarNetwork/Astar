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

use crate::{
    self as pallet_dapp_staking,
    test::testing_utils::{assert_block_bump, assert_on_idle_cleanup, MemorySnapshot},
    *,
};

use frame_support::{
    construct_runtime, parameter_types,
    traits::{fungible::Mutate as FunMutate, ConstU128, ConstU32},
    weights::Weight,
};
use sp_arithmetic::fixed_point::FixedU64;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    Permill,
};
use sp_std::cell::RefCell;

use astar_primitives::{dapp_staking::SmartContract, testing::Header, Balance, BlockNumber};

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
        DappStaking: pallet_dapp_staking,
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
impl PriceProvider for DummyPriceProvider {
    fn average_price() -> FixedU64 {
        FixedU64::from_rational(1, 10)
    }
}

thread_local! {
    pub(crate) static DOES_PAYOUT_SUCCEED: RefCell<bool> = RefCell::new(false);
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
        if DOES_PAYOUT_SUCCEED.with(|v| v.borrow().clone()) {
            let _ = Balances::mint_into(beneficiary, reward);
            Ok(())
        } else {
            Err(())
        }
    }
}

pub(crate) type MockSmartContract = SmartContract<AccountId>;

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchmarkHelper<SC, ACC>(sp_std::marker::PhantomData<(SC, ACC)>);
#[cfg(feature = "runtime-benchmarks")]
impl crate::BenchmarkHelper<MockSmartContract, AccountId>
    for BenchmarkHelper<MockSmartContract, AccountId>
{
    fn get_smart_contract(id: u32) -> MockSmartContract {
        MockSmartContract::wasm(id as AccountId)
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

impl pallet_dapp_staking::Config for Test {
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
    type WeightInfo = weights::SubstrateWeight<Test>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = BenchmarkHelper<MockSmartContract, AccountId>;
}

pub struct ExtBuilder;
impl ExtBuilder {
    pub fn build() -> TestExternalities {
        // Normal behavior is for reward payout to succeed
        DOES_PAYOUT_SUCCEED.with(|v| *v.borrow_mut() = true);

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

            let era_length = <Test as Config>::CycleConfiguration::blocks_per_era();
            let voting_period_length_in_eras =
                <Test as Config>::CycleConfiguration::eras_per_voting_subperiod();

            // Init protocol state
            pallet_dapp_staking::ActiveProtocolState::<Test>::put(ProtocolState {
                era: 1,
                next_era_start: era_length.saturating_mul(voting_period_length_in_eras.into()) + 1,
                period_info: PeriodInfo {
                    number: 1,
                    subperiod: Subperiod::Voting,
                    next_subperiod_start_era: 2,
                },
                maintenance: false,
            });
            pallet_dapp_staking::CurrentEraInfo::<Test>::put(EraInfo {
                total_locked: 0,
                unlocking: 0,
                current_stake_amount: StakeAmount {
                    voting: 0,
                    build_and_earn: 0,
                    era: 1,
                    period: 1,
                },
                next_stake_amount: StakeAmount {
                    voting: 0,
                    build_and_earn: 0,
                    era: 2,
                    period: 1,
                },
            });

            // Init tier params
            let tier_params = TierParameters::<<Test as Config>::NumberOfTiers> {
                reward_portion: BoundedVec::try_from(vec![
                    Permill::from_percent(40),
                    Permill::from_percent(30),
                    Permill::from_percent(20),
                    Permill::from_percent(10),
                ])
                .unwrap(),
                slot_distribution: BoundedVec::try_from(vec![
                    Permill::from_percent(10),
                    Permill::from_percent(20),
                    Permill::from_percent(30),
                    Permill::from_percent(40),
                ])
                .unwrap(),
                tier_thresholds: BoundedVec::try_from(vec![
                    TierThreshold::DynamicTvlAmount {
                        amount: 100,
                        minimum_amount: 80,
                    },
                    TierThreshold::DynamicTvlAmount {
                        amount: 50,
                        minimum_amount: 40,
                    },
                    TierThreshold::DynamicTvlAmount {
                        amount: 20,
                        minimum_amount: 20,
                    },
                    TierThreshold::FixedTvlAmount { amount: 15 },
                ])
                .unwrap(),
            };

            // Init tier config, based on the initial params
            let init_tier_config = TiersConfiguration::<<Test as Config>::NumberOfTiers> {
                number_of_slots: 40,
                slots_per_tier: BoundedVec::try_from(vec![2, 5, 13, 20]).unwrap(),
                reward_portion: tier_params.reward_portion.clone(),
                tier_thresholds: tier_params.tier_thresholds.clone(),
            };

            pallet_dapp_staking::StaticTierParams::<Test>::put(tier_params);
            pallet_dapp_staking::TierConfig::<Test>::put(init_tier_config.clone());

            DappStaking::on_initialize(System::block_number());
        });

        ext
    }
}

/// Run to the specified block number.
/// Function assumes first block has been initialized.
pub(crate) fn run_to_block(n: BlockNumber) {
    while System::block_number() < n {
        DappStaking::on_finalize(System::block_number());
        assert_on_idle_cleanup();
        System::set_block_number(System::block_number() + 1);
        // This is performed outside of dapps staking but we expect it before on_initialize

        let pre_snapshot = MemorySnapshot::new();
        DappStaking::on_initialize(System::block_number());
        assert_block_bump(&pre_snapshot);
    }
}

/// Run for the specified number of blocks.
/// Function assumes first block has been initialized.
pub(crate) fn run_for_blocks(n: BlockNumber) {
    run_to_block(System::block_number() + n);
}

/// Advance blocks until the specified era has been reached.
///
/// Function has no effect if era is already passed.
pub(crate) fn advance_to_era(era: EraNumber) {
    assert!(era >= ActiveProtocolState::<Test>::get().era);
    while ActiveProtocolState::<Test>::get().era < era {
        run_for_blocks(1);
    }
}

/// Advance blocks until next era has been reached.
pub(crate) fn advance_to_next_era() {
    advance_to_era(ActiveProtocolState::<Test>::get().era + 1);
}

/// Advance blocks until the specified period has been reached.
///
/// Function has no effect if period is already passed.
pub(crate) fn advance_to_period(period: PeriodNumber) {
    assert!(period >= ActiveProtocolState::<Test>::get().period_number());
    while ActiveProtocolState::<Test>::get().period_number() < period {
        run_for_blocks(1);
    }
}

/// Advance blocks until next period has been reached.
pub(crate) fn advance_to_next_period() {
    advance_to_period(ActiveProtocolState::<Test>::get().period_number() + 1);
}

/// Advance blocks until next period type has been reached.
pub(crate) fn advance_to_next_subperiod() {
    let subperiod = ActiveProtocolState::<Test>::get().subperiod();
    while ActiveProtocolState::<Test>::get().subperiod() == subperiod {
        run_for_blocks(1);
    }
}

// Return all dApp staking events from the event buffer.
pub fn dapp_staking_events() -> Vec<crate::Event<Test>> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| <Test as Config>::RuntimeEvent::from(e).try_into().ok())
        .collect::<Vec<_>>()
}
