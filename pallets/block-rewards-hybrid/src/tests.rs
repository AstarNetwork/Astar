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

use super::{pallet::Error, Event, *};
use frame_support::{assert_noop, assert_ok, traits::OnTimestampSet};
use mock::{Balance, *};
use sp_runtime::{
    traits::{AccountIdConversion, BadOrigin, Zero},
    Perbill,
};

#[test]
fn default_reward_distribution_config_is_consitent() {
    let reward_config = RewardDistributionConfig::default();
    assert!(reward_config.is_consistent());
}

#[test]
fn reward_distribution_config_is_consistent() {
    // 1
    let reward_config = RewardDistributionConfig {
        treasury_percent: Perbill::from_percent(100),
        base_staker_percent: Zero::zero(),
        dapps_percent: Zero::zero(),
        collators_percent: Zero::zero(),
        adjustable_percent: Zero::zero(),
        ideal_dapps_staking_tvl: Zero::zero(),
    };
    assert!(reward_config.is_consistent());

    // 2
    let reward_config = RewardDistributionConfig {
        treasury_percent: Zero::zero(),
        base_staker_percent: Perbill::from_percent(100),
        dapps_percent: Zero::zero(),
        collators_percent: Zero::zero(),
        adjustable_percent: Zero::zero(),
        ideal_dapps_staking_tvl: Zero::zero(),
    };
    assert!(reward_config.is_consistent());

    // 3
    let reward_config = RewardDistributionConfig {
        treasury_percent: Zero::zero(),
        base_staker_percent: Zero::zero(),
        dapps_percent: Zero::zero(),
        collators_percent: Zero::zero(),
        adjustable_percent: Perbill::from_percent(100),
        ideal_dapps_staking_tvl: Perbill::from_percent(13),
    };
    assert!(reward_config.is_consistent());

    // 4
    // 100%
    let reward_config = RewardDistributionConfig {
        treasury_percent: Perbill::from_rational(4663701u32, 100000000u32),
        base_staker_percent: Perbill::from_rational(2309024u32, 10000000u32),
        dapps_percent: Perbill::from_rational(173094531u32, 1000000000u32),
        collators_percent: Perbill::from_rational(29863296u32, 1000000000u32),
        adjustable_percent: Perbill::from_rational(519502763u32, 1000000000u32),
        ideal_dapps_staking_tvl: Perbill::from_percent(60),
    };
    assert!(reward_config.is_consistent());
}

#[test]
fn reward_distribution_config_not_consistent() {
    // 1
    let reward_config = RewardDistributionConfig {
        treasury_percent: Perbill::from_percent(100),
        ..Default::default()
    };
    assert!(!reward_config.is_consistent());

    // 2
    let reward_config = RewardDistributionConfig {
        adjustable_percent: Perbill::from_percent(100),
        ..Default::default()
    };
    assert!(!reward_config.is_consistent());

    // 3
    // 99%
    let reward_config = RewardDistributionConfig {
        treasury_percent: Perbill::from_percent(10),
        base_staker_percent: Perbill::from_percent(20),
        dapps_percent: Perbill::from_percent(20),
        collators_percent: Perbill::from_percent(30),
        adjustable_percent: Perbill::from_percent(19),
        ideal_dapps_staking_tvl: Zero::zero(),
    };
    assert!(!reward_config.is_consistent());

    // 4
    // 101%
    let reward_config = RewardDistributionConfig {
        treasury_percent: Perbill::from_percent(10),
        base_staker_percent: Perbill::from_percent(20),
        dapps_percent: Perbill::from_percent(20),
        collators_percent: Perbill::from_percent(31),
        adjustable_percent: Perbill::from_percent(20),
        ideal_dapps_staking_tvl: Zero::zero(),
    };
    assert!(!reward_config.is_consistent());
}

#[test]
fn set_configuration_fails() {
    ExternalityBuilder::build().execute_with(|| {
        // 1
        assert_noop!(
            BlockReward::set_configuration(RuntimeOrigin::signed(1), Default::default()),
            BadOrigin
        );

        // 2
        let reward_config = RewardDistributionConfig {
            treasury_percent: Perbill::from_percent(100),
            ..Default::default()
        };
        assert!(!reward_config.is_consistent());
        assert_noop!(
            BlockReward::set_configuration(RuntimeOrigin::root(), reward_config),
            Error::<TestRuntime>::InvalidDistributionConfiguration,
        );
    })
}

#[test]
fn set_configuration_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // custom config so it differs from the default one
        let reward_config = RewardDistributionConfig {
            treasury_percent: Perbill::from_percent(3),
            base_staker_percent: Perbill::from_percent(14),
            dapps_percent: Perbill::from_percent(18),
            collators_percent: Perbill::from_percent(31),
            adjustable_percent: Perbill::from_percent(34),
            ideal_dapps_staking_tvl: Perbill::from_percent(87),
        };
        assert!(reward_config.is_consistent());

        assert_ok!(BlockReward::set_configuration(
            RuntimeOrigin::root(),
            reward_config.clone()
        ));
        System::assert_last_event(mock::RuntimeEvent::BlockReward(
            Event::DistributionConfigurationChanged(reward_config.clone()),
        ));

        assert_eq!(
            RewardDistributionConfigStorage::<TestRuntime>::get(),
            reward_config
        );
    })
}

#[test]
fn inflation_and_total_issuance_as_expected() {
    ExternalityBuilder::build().execute_with(|| {
        let init_issuance = <TestRuntime as Config>::Currency::total_issuance();

        for block in 0..10 {
            assert_eq!(
                <TestRuntime as Config>::Currency::total_issuance(),
                block * BLOCK_REWARD + init_issuance
            );
            BlockReward::on_timestamp_set(0);
            assert_eq!(
                <TestRuntime as Config>::Currency::total_issuance(),
                (block + 1) * BLOCK_REWARD + init_issuance
            );
        }
    })
}

#[test]
fn reward_distribution_as_expected() {
    ExternalityBuilder::build().execute_with(|| {
        // Ensure that initially, all beneficiaries have no free balance
        let init_balance_snapshot = FreeBalanceSnapshot::new();
        assert!(init_balance_snapshot.is_zero());

        // Prepare a custom config (easily discernable percentages for visual verification)
        let reward_config = RewardDistributionConfig {
            treasury_percent: Perbill::from_percent(10),
            base_staker_percent: Perbill::from_percent(20),
            dapps_percent: Perbill::from_percent(25),
            collators_percent: Perbill::from_percent(5),
            adjustable_percent: Perbill::from_percent(40),
            ideal_dapps_staking_tvl: Perbill::from_percent(50),
        };
        assert!(reward_config.is_consistent());
        assert_ok!(BlockReward::set_configuration(
            RuntimeOrigin::root(),
            reward_config.clone()
        ));

        // Issue rewards a couple of times and verify distribution is as expected
        // also ensure that the non distributed reward amount is burn
        // (that the total issuance is only increased by the amount that has been rewarded)
        for _block in 1..=100 {
            // TVL amount is updated every block
            // to ensure TVL ratio as expected
            adjust_tvl(30);
            let init_balance_state = FreeBalanceSnapshot::new();
            let total_issuance_before = <TestRuntime as Config>::Currency::total_issuance();
            let distributed_rewards = Rewards::calculate(&reward_config);

            BlockReward::on_timestamp_set(0);

            let final_balance_state = FreeBalanceSnapshot::new();
            init_balance_state.assert_distribution(&final_balance_state, &distributed_rewards);

            assert_eq!(
                <TestRuntime as Config>::Currency::total_issuance(),
                total_issuance_before + distributed_rewards.sum()
            );
        }
    })
}

#[test]
fn non_distributed_reward_amount_is_burned() {
    ExternalityBuilder::build().execute_with(|| {
        // Ensure that initially, all beneficiaries have no free balance
        let init_balance_snapshot = FreeBalanceSnapshot::new();
        assert!(init_balance_snapshot.is_zero());

        // Prepare a custom config (easily discernible percentages for visual verification)
        let reward_config = RewardDistributionConfig {
            treasury_percent: Perbill::from_percent(10),
            base_staker_percent: Perbill::from_percent(20),
            dapps_percent: Perbill::from_percent(25),
            collators_percent: Perbill::from_percent(5),
            adjustable_percent: Perbill::from_percent(40),
            ideal_dapps_staking_tvl: Perbill::from_percent(50),
        };
        assert!(reward_config.is_consistent());
        assert_ok!(BlockReward::set_configuration(
            RuntimeOrigin::root(),
            reward_config.clone()
        ));

        for tvl in [30, 50, 70, 100] {
            for _block in 1..=100 {
                // TVL amount is updated every block
                // to ensure TVL ratio as expected
                adjust_tvl(tvl);
                let total_issuance_before = <TestRuntime as Config>::Currency::total_issuance();
                let distributed_rewards = Rewards::calculate(&reward_config);
                let burned_amount = BLOCK_REWARD - distributed_rewards.sum();

                BlockReward::on_timestamp_set(0);

                assert_eq!(
                    <TestRuntime as Config>::Currency::total_issuance(),
                    total_issuance_before + BLOCK_REWARD - burned_amount
                );
            }
        }
    })
}

#[test]
fn reward_distribution_no_adjustable_part() {
    ExternalityBuilder::build().execute_with(|| {
        let reward_config = RewardDistributionConfig {
            treasury_percent: Perbill::from_percent(10),
            base_staker_percent: Perbill::from_percent(45),
            dapps_percent: Perbill::from_percent(40),
            collators_percent: Perbill::from_percent(5),
            adjustable_percent: Perbill::zero(),
            ideal_dapps_staking_tvl: Perbill::from_percent(50), // this is irrelevant
        };
        assert!(reward_config.is_consistent());
        assert_ok!(BlockReward::set_configuration(
            RuntimeOrigin::root(),
            reward_config.clone()
        ));

        // no adjustable part so we don't expect rewards to change with TVL percentage
        let const_rewards = Rewards::calculate(&reward_config);

        for _block in 1..=100 {
            let init_balance_state = FreeBalanceSnapshot::new();
            let rewards = Rewards::calculate(&reward_config);

            assert_eq!(rewards, const_rewards);

            BlockReward::on_timestamp_set(0);

            let final_balance_state = FreeBalanceSnapshot::new();
            init_balance_state.assert_distribution(&final_balance_state, &rewards);
        }
    })
}

#[test]
fn reward_distribution_all_zero_except_one() {
    ExternalityBuilder::build().execute_with(|| {
        let reward_config = RewardDistributionConfig {
            treasury_percent: Perbill::zero(),
            base_staker_percent: Perbill::zero(),
            dapps_percent: Perbill::zero(),
            collators_percent: Perbill::zero(),
            adjustable_percent: Perbill::one(),
            ideal_dapps_staking_tvl: Perbill::from_percent(50), // this is irrelevant
        };
        assert!(reward_config.is_consistent());
        assert_ok!(BlockReward::set_configuration(
            RuntimeOrigin::root(),
            reward_config.clone()
        ));

        for _block in 1..=10 {
            let init_balance_state = FreeBalanceSnapshot::new();
            let rewards = Rewards::calculate(&reward_config);

            BlockReward::on_timestamp_set(0);

            let final_balance_state = FreeBalanceSnapshot::new();
            init_balance_state.assert_distribution(&final_balance_state, &rewards);
        }
    })
}

/// Represents free balance snapshot at a specific point in time
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
struct FreeBalanceSnapshot {
    treasury: Balance,
    collators: Balance,
    stakers: Balance,
    dapps: Balance,
}

impl FreeBalanceSnapshot {
    /// Creates a new free balance snapshot using current balance state.
    ///
    /// Future balance changes won't be reflected in this instance.
    fn new() -> Self {
        Self {
            treasury: <TestRuntime as Config>::Currency::free_balance(
                &TREASURY_POT.into_account_truncating(),
            ),
            collators: <TestRuntime as Config>::Currency::free_balance(
                &COLLATOR_POT.into_account_truncating(),
            ),
            stakers: <TestRuntime as Config>::Currency::free_balance(
                &STAKERS_POT.into_account_truncating(),
            ),
            dapps: <TestRuntime as Config>::Currency::free_balance(
                &DAPPS_POT.into_account_truncating(),
            ),
        }
    }

    /// `true` if all free balances equal `Zero`, `false` otherwise
    fn is_zero(&self) -> bool {
        self.treasury.is_zero()
            && self.collators.is_zero()
            && self.stakers.is_zero()
            && self.dapps.is_zero()
    }

    /// Asserts that `post_reward_state` is as expected.
    ///
    /// Increase in balances, based on `rewards` values, is verified.
    ///
    fn assert_distribution(&self, post_reward_state: &Self, rewards: &Rewards) {
        assert_eq!(
            self.treasury + rewards.treasury_reward,
            post_reward_state.treasury
        );
        assert_eq!(
            self.stakers + rewards.staker_reward,
            post_reward_state.stakers
        );
        assert_eq!(
            self.collators + rewards.collators_reward,
            post_reward_state.collators
        );
        assert_eq!(self.dapps + rewards.dapps_reward, post_reward_state.dapps);
    }
}

impl Rewards {
    /// Pre-calculates the reward distribution, using the provided `RewardDistributionConfig`.
    /// Method assumes that total issuance will be increased by `BLOCK_REWARD`.
    ///
    /// Both current `total_issuance` and `TVL` are used. If these are changed after calling this function,
    /// they won't be reflected in the struct.
    ///
    fn calculate(reward_config: &RewardDistributionConfig) -> Self {
        // Calculate `tvl-independent` portions
        let treasury_reward = reward_config.treasury_percent * BLOCK_REWARD;
        let base_staker_reward = reward_config.base_staker_percent * BLOCK_REWARD;
        let dapps_reward = reward_config.dapps_percent * BLOCK_REWARD;
        let collators_reward = reward_config.collators_percent * BLOCK_REWARD;
        let adjustable_reward = reward_config.adjustable_percent * BLOCK_REWARD;

        // Calculate `tvl-dependent` portions
        let total_issuance = <TestRuntime as Config>::Currency::total_issuance();
        let tvl = <TestRuntime as Config>::DappsStakingTvlProvider::get();
        let tvl_percentage = Perbill::from_rational(tvl, total_issuance);

        // Calculate factor for adjusting staker reward portion
        let factor = if reward_config.ideal_dapps_staking_tvl.is_zero() {
            Perbill::one()
        } else {
            tvl_percentage / reward_config.ideal_dapps_staking_tvl
        };

        // Adjustable reward portions
        let adjustable_staker_reward = factor * adjustable_reward;

        let staker_reward = base_staker_reward + adjustable_staker_reward;

        Self {
            treasury_reward,
            staker_reward,
            dapps_reward,
            collators_reward,
        }
    }
}

fn adjust_tvl(desired_percent: u128) {
    set_tvl(<TestRuntime as Config>::Currency::total_issuance() / 100 * desired_percent);
}
