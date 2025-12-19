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

use crate::{
    self as pallet_dapp_staking,
    test::testing_utils::{assert_block_bump, assert_on_idle_cleanup, MemorySnapshot},
    *,
};

use frame_support::{
    construct_runtime, derive_impl,
    migrations::MultiStepMigrator,
    ord_parameter_types, parameter_types,
    traits::{fungible::Mutate as FunMutate, ConstBool, ConstU128, ConstU32, EitherOfDiverse},
    weights::Weight,
};
use sp_arithmetic::fixed_point::FixedU128;
use sp_io::TestExternalities;
use sp_runtime::{BuildStorage, Permill};
use sp_std::cell::RefCell;

use astar_primitives::{
    dapp_staking::{
        Observer as DappStakingObserver, SmartContract, StandardTierSlots, STANDARD_TIER_SLOTS_ARGS,
    },
    Balance, BlockNumber,
};
use frame_system::{EnsureRoot, EnsureSignedBy};

pub(crate) type AccountId = u64;

pub(crate) const EXISTENTIAL_DEPOSIT: Balance = 2;
pub(crate) const MINIMUM_LOCK_AMOUNT: Balance = 10;

type Block = frame_system::mocking::MockBlockU32<Test>;

construct_runtime!(
    pub struct Test {
        System: frame_system,
        Balances: pallet_balances,
        DappStaking: pallet_dapp_staking,
        MultiBlockMigrations: pallet_migrations,
    }
);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountData = pallet_balances::AccountData<Balance>;
    type MultiBlockMigrator = MultiBlockMigrations;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type Balance = Balance;
    type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
    type AccountStore = System;
    type FreezeIdentifier = RuntimeFreezeReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type MaxFreezes = ConstU32<1>;
}

parameter_types! {
    pub const MaxServiceWeight: Weight = Weight::from_parts(1_000_000_000, 1_000_000);
}

#[derive_impl(pallet_migrations::config_preludes::TestDefaultConfig)]
impl pallet_migrations::Config for Test {
    #[cfg(not(feature = "runtime-benchmarks"))]
    type Migrations =
        (crate::migration::LazyMigration<Test, crate::weights::SubstrateWeight<Test>>,);
    #[cfg(feature = "runtime-benchmarks")]
    type Migrations = pallet_migrations::mock_helpers::MockedMigrations;
    type MaxServiceWeight = MaxServiceWeight;
}

pub struct DummyPriceProvider;
impl PriceProvider for DummyPriceProvider {
    fn average_price() -> FixedU128 {
        NATIVE_PRICE.with(|v| v.borrow().clone())
    }
}

thread_local! {
    pub(crate) static DOES_PAYOUT_SUCCEED: RefCell<bool> = RefCell::new(false);
    pub(crate) static BLOCK_BEFORE_NEW_ERA: RefCell<EraNumber> = RefCell::new(0);
    pub(crate) static NATIVE_PRICE: RefCell<FixedU128> = RefCell::new(BaseNativeCurrencyPrice::get());
    pub(crate) static MAX_BONUS_SAFE_MOVES: RefCell<u8> = RefCell::new(0);
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

pub struct DummyDappStakingObserver;
impl DappStakingObserver for DummyDappStakingObserver {
    fn block_before_new_era(next_era: EraNumber) -> Weight {
        BLOCK_BEFORE_NEW_ERA.with(|v| *v.borrow_mut() = next_era);
        Weight::from_parts(1, 2)
    }
}

pub(crate) const BLACKLISTED_ACCOUNT: AccountId = 789456123;
pub struct DummyAccountCheck;
impl AccountCheck<AccountId> for DummyAccountCheck {
    fn allowed_to_stake(account: &AccountId) -> bool {
        *account != BLACKLISTED_ACCOUNT
    }
}

pub struct DynamicMaxBonusSafeMovesPerPeriod;
impl Get<u8> for DynamicMaxBonusSafeMovesPerPeriod {
    fn get() -> u8 {
        MAX_BONUS_SAFE_MOVES.with(|v| *v.borrow())
    }
}

parameter_types! {
    pub const BaseNativeCurrencyPrice: FixedU128 = FixedU128::from_rational(5, 100);
}
ord_parameter_types! {
    pub const ContractRegisterAccount: AccountId = 1337;
    pub const ContractUnregisterAccount: AccountId = 1779;
    pub const ManagerAccount: AccountId = 25711;
}

impl pallet_dapp_staking::Config for Test {
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type Currency = Balances;
    type SmartContract = MockSmartContract;
    type ContractRegisterOrigin =
        EitherOfDiverse<EnsureRoot<AccountId>, EnsureSignedBy<ContractRegisterAccount, AccountId>>;
    type ContractUnregisterOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        EnsureSignedBy<ContractUnregisterAccount, AccountId>,
    >;
    type ManagerOrigin =
        EitherOfDiverse<EnsureRoot<AccountId>, EnsureSignedBy<ManagerAccount, AccountId>>;
    type NativePriceProvider = DummyPriceProvider;
    type StakingRewardHandler = DummyStakingRewardHandler;
    type CycleConfiguration = DummyCycleConfiguration;
    type Observers = DummyDappStakingObserver;
    type AccountCheck = DummyAccountCheck;
    type TierSlots = StandardTierSlots;
    type BaseNativeCurrencyPrice = BaseNativeCurrencyPrice;
    type EraRewardSpanLength = ConstU32<8>;
    type RewardRetentionInPeriods = ConstU32<2>;
    type MaxNumberOfContracts = ConstU32<10>;
    type MaxUnlockingChunks = ConstU32<5>;
    type MinimumLockedAmount = ConstU128<MINIMUM_LOCK_AMOUNT>;
    type UnlockingPeriod = ConstU32<2>;
    type MaxNumberOfStakedContracts = ConstU32<5>;
    type MinimumStakeAmount = ConstU128<3>;
    type NumberOfTiers = ConstU32<4>;
    type RankingEnabled = ConstBool<true>;
    type MaxBonusSafeMovesPerPeriod = DynamicMaxBonusSafeMovesPerPeriod;
    type WeightInfo = weights::SubstrateWeight<Test>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = BenchmarkHelper<MockSmartContract, AccountId>;
}

pub struct ExtBuilder {}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {}
    }
}

impl ExtBuilder {
    pub fn build(self) -> TestExternalities {
        // Normal behavior is for reward payout to succeed
        DOES_PAYOUT_SUCCEED.with(|v| *v.borrow_mut() = true);

        let mut storage = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();

        let balances = vec![1000; 9]
            .into_iter()
            .enumerate()
            .map(|(idx, amount)| (idx as u64 + 1, amount))
            .collect();

        pallet_balances::GenesisConfig::<Test> {
            balances,
            ..Default::default()
        }
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
                    TierThreshold::DynamicPercentage {
                        percentage: Perbill::from_parts(11_112_000), // 1.1112%
                        minimum_required_percentage: Perbill::from_parts(8_889_000), // 0.8889%
                        maximum_possible_percentage: Perbill::from_parts(13_000_000), // 1.3%
                    },
                    TierThreshold::DynamicPercentage {
                        percentage: Perbill::from_parts(5_556_000), // 0.5556%
                        minimum_required_percentage: Perbill::from_parts(4_400_000), // 0.44%
                        maximum_possible_percentage: Perbill::from_percent(100),
                    },
                    TierThreshold::DynamicPercentage {
                        percentage: Perbill::from_parts(2_223_000), // 0.2223%
                        minimum_required_percentage: Perbill::from_parts(2_223_000), // 0.2223%
                        maximum_possible_percentage: Perbill::from_percent(100),
                    },
                    TierThreshold::FixedPercentage {
                        required_percentage: Perbill::from_parts(1_667_000), // 0.1667%
                    },
                ])
                .unwrap(),
                slot_number_args: STANDARD_TIER_SLOTS_ARGS,
            };

            let total_issuance = <Test as Config>::Currency::total_issuance();
            let tier_thresholds = tier_params
                .tier_thresholds
                .iter()
                .map(|t| t.threshold(total_issuance))
                .collect::<Vec<Balance>>()
                .try_into()
                .expect("Invalid number of tier thresholds provided.");

            // Init tier config, based on the initial params. Needs to be adjusted to the init price.
            let init_tier_config = TiersConfiguration::<
                <Test as Config>::NumberOfTiers,
                <Test as Config>::TierSlots,
                <Test as Config>::BaseNativeCurrencyPrice,
            > {
                slots_per_tier: BoundedVec::try_from(vec![2, 5, 13, 20]).unwrap(),
                reward_portion: tier_params.reward_portion.clone(),
                tier_thresholds,
                _phantom: Default::default(),
            }
            .calculate_new(
                &tier_params,
                NATIVE_PRICE.with(|v| v.borrow().clone()),
                total_issuance,
            );

            pallet_dapp_staking::StaticTierParams::<Test>::put(tier_params);
            pallet_dapp_staking::TierConfig::<Test>::put(init_tier_config.clone());
            pallet_dapp_staking::Safeguard::<Test>::put(false);

            DappStaking::on_initialize(System::block_number());
        });

        ext
    }

    pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
        self.build().execute_with(|| {
            test();
            DappStaking::do_try_state().unwrap();
        })
    }

    pub fn with_max_bonus_safe_moves(self, value: u8) -> Self {
        MAX_BONUS_SAFE_MOVES.with(|v| *v.borrow_mut() = value);
        self
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

        // Done by Executive:
        <Test as frame_system::Config>::MultiBlockMigrator::step();

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
        .filter_map(|e| e.try_into().ok())
        .collect::<Vec<_>>()
}
