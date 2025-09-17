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

use super::{pallet::Error, Event, *};
use frame_support::{assert_noop, assert_ok, assert_storage_noop, traits::Hooks};
use mock::*;
use sp_runtime::{
    traits::{AccountIdConversion, BadOrigin, Zero},
    Perquintill, Saturating,
};

#[test]
fn default_params_are_valid() {
    assert!(InflationParameters::default().is_valid());
}

#[test]
fn force_set_inflation_params_work() {
    ExternalityBuilder::build().execute_with(|| {
        let mut new_params = InflationParams::<Test>::get();
        new_params.max_inflation_rate = Perquintill::from_percent(20);
        new_params.decay_rate = Perquintill::from_percent(99);
        assert_ne!(new_params, InflationParams::<Test>::get(), "Sanity check");

        // Execute call, ensure it works
        assert_ok!(Inflation::force_set_inflation_params(
            RuntimeOrigin::root(),
            new_params
        ));
        System::assert_last_event(Event::InflationParametersForceChanged.into());

        assert_eq!(InflationParams::<Test>::get(), new_params);
    })
}

#[test]
fn force_set_inflation_params_fails() {
    ExternalityBuilder::build().execute_with(|| {
        let mut new_params = InflationParams::<Test>::get();
        new_params.base_stakers_part = Zero::zero();
        assert!(
            !new_params.is_valid(),
            "Must be invalid for check to make sense."
        );

        // Make sure it's not possible to force-set invalid params
        assert_noop!(
            Inflation::force_set_inflation_params(RuntimeOrigin::root(), new_params),
            Error::<Test>::InvalidInflationParameters
        );

        // Make sure action is privileged
        assert_noop!(
            Inflation::force_set_inflation_params(RuntimeOrigin::signed(1).into(), new_params,),
            BadOrigin
        );
    })
}

#[test]
fn force_inflation_recalculation_work() {
    ExternalityBuilder::build().execute_with(|| {
        let old_config = ActiveInflationConfig::<Test>::get();

        // Execute call, ensure it works
        let next_era = 100;
        assert_ok!(Inflation::force_inflation_recalculation(
            RuntimeOrigin::root(),
            next_era,
        ));

        let new_config = ActiveInflationConfig::<Test>::get();
        assert_ne!(
            old_config, new_config,
            "Config should change, otherwise test doesn't make sense."
        );

        System::assert_last_event(
            Event::ForcedInflationRecalculation { config: new_config }.into(),
        );
    })
}

#[test]
fn force_inflation_fails_due_to_unprivileged_origin() {
    ExternalityBuilder::build().execute_with(|| {
        // Make sure action is privileged
        assert_noop!(
            Inflation::force_inflation_recalculation(RuntimeOrigin::signed(1), 100),
            BadOrigin
        );
    })
}

#[test]
fn force_readjust_config_works() {
    ExternalityBuilder::build().execute_with(|| {
        // Init values
        let old_params = InflationParams::<Test>::get();
        let old_config = ActiveInflationConfig::<Test>::get();

        // Change params, modifying staker reward parts
        let mut new_params = old_params;
        new_params.base_stakers_part = Perquintill::from_percent(10);
        new_params.adjustable_stakers_part = Perquintill::from_percent(50);
        assert_ne!(new_params, old_params, "Sanity check, must be different.");
        assert!(new_params.is_valid(), "Sanity check.");

        // Force set new params, before calling `force_readjust_config`
        assert_ok!(Inflation::force_set_inflation_params(
            RuntimeOrigin::root(),
            new_params
        ));

        // Force readjust config
        assert_ok!(Inflation::force_readjust_config(RuntimeOrigin::root()));
        let new_config = ActiveInflationConfig::<Test>::get();
        assert_ne!(new_config, old_config, "Config should change.");
        System::assert_last_event(
            Event::ForcedInflationRecalculation { config: new_config }.into(),
        );

        // Value checks of the new config
        // These should remain unchanged
        lenient_balance_assert_eq!(
            new_config.collator_reward_per_block,
            old_config.collator_reward_per_block
        );
        lenient_balance_assert_eq!(
            new_config.treasury_reward_per_block,
            old_config.treasury_reward_per_block
        );
        lenient_balance_assert_eq!(
            new_config.dapp_reward_pool_per_era,
            old_config.dapp_reward_pool_per_era
        );
        lenient_balance_assert_eq!(
            new_config.bonus_reward_pool_per_period,
            old_config.bonus_reward_pool_per_period
        );
        assert_eq!(new_config.ideal_staking_rate, old_config.ideal_staking_rate,);
        assert_eq!(new_config.recalculation_era, old_config.recalculation_era,);

        // These should change
        assert!(
            new_config.base_staker_reward_pool_per_era < old_config.base_staker_reward_pool_per_era
        );
        assert!(
            new_config.adjustable_staker_reward_pool_per_era
                > old_config.adjustable_staker_reward_pool_per_era
        )
    })
}

#[test]
fn force_readjust_config_fails_due_to_unprivileged_origin() {
    ExternalityBuilder::build().execute_with(|| {
        assert_noop!(
            Inflation::force_readjust_config(RuntimeOrigin::signed(1)),
            BadOrigin
        );
    })
}

#[test]
fn inflation_recalculation_occurs_when_expected() {
    ExternalityBuilder::build().execute_with(|| {
        let init_config = ActiveInflationConfig::<Test>::get();

        let recalculation_era = init_config.recalculation_era;


        // Make sure `on_finalize` calls before the expected change are storage noops
        Inflation::block_before_new_era(recalculation_era - 2);
        assert_storage_noop!(Inflation::on_finalize(100));

        Inflation::block_before_new_era(recalculation_era - 1);
        assert_storage_noop!(Inflation::on_finalize(200));

        // One block before recalculation era starts, on_finalize should calculate new inflation config
        Inflation::block_before_new_era(recalculation_era);
        let init_config = ActiveInflationConfig::<Test>::get();
        let init_total_issuance = Balances::total_issuance();

        // Finally trigger inflation recalculation.
        Inflation::on_finalize(300);

        let new_config = ActiveInflationConfig::<Test>::get();
        assert_ne!(
            new_config, init_config,
            "Recalculation must happen at this point."
        );
        System::assert_last_event(Event::NewInflationConfiguration { config: new_config }.into());

        assert_eq!(
            Balances::total_issuance(),
            init_total_issuance,
            "Total issuance must not change when inflation is recalculated - nothing is minted until it's needed."
        );
    })
}

#[test]
fn on_initialize_reward_payout_works() {
    ExternalityBuilder::build().execute_with(|| {
        // Save initial state, before the payout
        let config = ActiveInflationConfig::<Test>::get();

        let init_issuance = Balances::total_issuance();
        let init_collator_pot = Balances::free_balance(&COLLATOR_POT.into_account_truncating());
        let init_treasury_pot = Balances::free_balance(&TREASURY_POT.into_account_truncating());

        // Execute payout
        Inflation::on_initialize(1);

        // Verify state post payout
        let expected_reward = config.collator_reward_per_block + config.treasury_reward_per_block;

        // Balance changes are as expected
        assert_eq!(Balances::total_issuance(), init_issuance + expected_reward);
        assert_eq!(
            Balances::free_balance(&COLLATOR_POT.into_account_truncating()),
            init_collator_pot + config.collator_reward_per_block
        );
        assert_eq!(
            Balances::free_balance(&TREASURY_POT.into_account_truncating()),
            init_treasury_pot + config.treasury_reward_per_block
        );
    })
}

#[test]
fn inflation_parameters_validity_check_works() {
    // Params to be used as anchor for the tests
    let base_params = INIT_PARAMS;
    assert!(base_params.is_valid(), "Sanity check.");

    // Reduction of some param, it should invalidate the whole config
    let mut params = base_params;
    params.base_stakers_part = params.base_stakers_part - Perquintill::from_percent(1);
    assert!(!params.is_valid(), "Sum is below 100%, must fail.");

    // Increase of some param, it should invalidate the whole config
    let mut params = base_params;
    params.base_stakers_part = params.base_stakers_part + Perquintill::from_percent(1);
    assert!(!params.is_valid(), "Sum is above 100%, must fail.");

    // Excessive increase of some param, it should invalidate the whole config
    let mut params = base_params;
    params.treasury_part = Perquintill::from_percent(100);
    assert!(!params.is_valid(), "Sum is above 100%, must fail.");

    // Some param can be zero, as long as sum remains 100%
    let mut params = base_params;
    params.base_stakers_part = params.base_stakers_part + params.adjustable_stakers_part;
    params.adjustable_stakers_part = Zero::zero();
    assert!(params.is_valid());
}

#[test]
fn inflation_recalculation_works() {
    ExternalityBuilder::build().execute_with(|| {
        let total_issuance = Balances::total_issuance();
        let params = InflationParams::<Test>::get();
        let now = System::block_number();

        // Calculate new config
        let decay_factor = Perquintill::one();
        let new_config = Inflation::recalculate_inflation(now, decay_factor);
        let max_emission = params.max_inflation_rate * total_issuance;

        // Verify basics are ok
        assert_eq!(
            new_config.recalculation_era,
            now + <Test as Config>::CycleConfiguration::eras_per_cycle()
        );
        assert_eq!(
            new_config.decay_factor, decay_factor,
            "Default decay factor expected."
        );

        // Verify collator rewards are as expected
        assert!(
            !new_config.collator_reward_per_block.is_zero(),
            "Not wrong, but all test values should be non-zero."
        );
        assert_eq!(
            new_config.collator_reward_per_block,
            params.collators_part * max_emission
                / Balance::from(<Test as Config>::CycleConfiguration::blocks_per_cycle()),
        );

        // Verify treasury rewards are as expected
        assert!(
            !new_config.treasury_reward_per_block.is_zero(),
            "Not wrong, but all test values should be non-zero."
        );
        assert_eq!(
            new_config.treasury_reward_per_block,
            params.treasury_part * max_emission
                / Balance::from(<Test as Config>::CycleConfiguration::blocks_per_cycle()),
        );

        // Verify dApp rewards are as expected
        assert!(
            !new_config.dapp_reward_pool_per_era.is_zero(),
            "Not wrong, but all test values should be non-zero."
        );
        assert_eq!(
            new_config.dapp_reward_pool_per_era,
            params.dapps_part * max_emission
                / Balance::from(
                    <Test as Config>::CycleConfiguration::build_and_earn_eras_per_cycle()
                ),
        );

        // Verify base & adjustable staker rewards are as expected
        assert!(
            !new_config.base_staker_reward_pool_per_era.is_zero(),
            "Not wrong, but all test values should be non-zero."
        );
        assert_eq!(
            new_config.base_staker_reward_pool_per_era,
            params.base_stakers_part * max_emission
                / Balance::from(
                    <Test as Config>::CycleConfiguration::build_and_earn_eras_per_cycle()
                ),
        );
        assert!(
            !new_config.adjustable_staker_reward_pool_per_era.is_zero(),
            "Not wrong, but all test values should be non-zero."
        );
        assert_eq!(
            new_config.adjustable_staker_reward_pool_per_era,
            params.adjustable_stakers_part * max_emission
                / Balance::from(
                    <Test as Config>::CycleConfiguration::build_and_earn_eras_per_cycle()
                ),
        );

        // Verify bonus rewards are as expected
        assert!(
            !new_config.bonus_reward_pool_per_period.is_zero(),
            "Not wrong, but all test values should be non-zero."
        );
        assert_eq!(
            new_config.bonus_reward_pool_per_period,
            params.bonus_part * max_emission
                / Balance::from(<Test as Config>::CycleConfiguration::periods_per_cycle()),
        );
    })
}

#[test]
fn stakers_and_dapp_reward_pool_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let total_issuance = Balances::total_issuance();
        let config = ActiveInflationConfig::<Test>::get();

        // 1st scenario - no staked value
        let (staker_pool, dapp_pool) = Inflation::staker_and_dapp_reward_pools(Zero::zero());
        assert_eq!(staker_pool, config.base_staker_reward_pool_per_era);
        assert_eq!(dapp_pool, config.dapp_reward_pool_per_era);

        // 2nd scenario - there is some staked value, larger than zero, but less than ideal
        let test_rate = config.ideal_staking_rate - Perquintill::from_percent(11);
        let (staker_pool, dapp_pool) =
            Inflation::staker_and_dapp_reward_pools(test_rate * total_issuance);

        assert_eq!(
            staker_pool,
            config.base_staker_reward_pool_per_era
                + test_rate / config.ideal_staking_rate
                    * config.adjustable_staker_reward_pool_per_era
        );
        assert_eq!(dapp_pool, config.dapp_reward_pool_per_era);

        // 3rd scenario - we're exactly at the ideal staking rate
        let (staker_pool, dapp_pool) =
            Inflation::staker_and_dapp_reward_pools(config.ideal_staking_rate * total_issuance);

        assert_eq!(
            staker_pool,
            config.base_staker_reward_pool_per_era + config.adjustable_staker_reward_pool_per_era
        );
        assert_eq!(dapp_pool, config.dapp_reward_pool_per_era);

        // 4th scenario - we're above ideal staking rate, should be the same as at the ideal staking rate regarding the pools
        let test_rate = config.ideal_staking_rate + Perquintill::from_percent(13);
        let (staker_pool, dapp_pool) =
            Inflation::staker_and_dapp_reward_pools(test_rate * total_issuance);

        assert_eq!(
            staker_pool,
            config.base_staker_reward_pool_per_era + config.adjustable_staker_reward_pool_per_era
        );
        assert_eq!(dapp_pool, config.dapp_reward_pool_per_era);

        // 5th scenario - ideal staking rate is zero, entire adjustable amount is always used.
        ActiveInflationConfig::<Test>::mutate(|config| {
            config.ideal_staking_rate = Zero::zero();
        });

        let (staker_pool, dapp_pool) =
            Inflation::staker_and_dapp_reward_pools(Perquintill::from_percent(5) * total_issuance);

        assert_eq!(
            staker_pool,
            config.base_staker_reward_pool_per_era + config.adjustable_staker_reward_pool_per_era
        );
        assert_eq!(dapp_pool, config.dapp_reward_pool_per_era);
    })
}

#[test]
fn bonus_reward_pool_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let config = ActiveInflationConfig::<Test>::get();

        let bonus_pool = Inflation::bonus_reward_pool();
        assert_eq!(bonus_pool, config.bonus_reward_pool_per_period);
    })
}

#[test]
fn basic_payout_reward_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // Prepare reward payout params
        let config = ActiveInflationConfig::<Test>::get();
        let account = 1;
        let reward = config.issuance_safety_cap - Balances::total_issuance();
        let init_balance = Balances::free_balance(&account);
        let init_issuance = Balances::total_issuance();

        // Payout reward and verify balances are as expected
        assert_ok!(Inflation::payout_reward(&account, reward));

        assert_eq!(Balances::free_balance(&account), init_balance + reward);
        assert_eq!(Balances::total_issuance(), init_issuance + reward);
    })
}

#[test]
fn cycle_configuration_works() {
    ExternalityBuilder::build().execute_with(|| {
        type CycleConfig = <Test as Config>::CycleConfiguration;

        let eras_per_period = CycleConfig::eras_per_build_and_earn_subperiod() + 1;
        assert_eq!(CycleConfig::eras_per_period(), eras_per_period);

        let period_in_era_lengths = CycleConfig::eras_per_voting_subperiod()
            + CycleConfig::eras_per_build_and_earn_subperiod();
        assert_eq!(CycleConfig::period_in_era_lengths(), period_in_era_lengths);

        let eras_per_cycle = eras_per_period * CycleConfig::periods_per_cycle();
        assert_eq!(CycleConfig::eras_per_cycle(), eras_per_cycle);

        let cycle_in_era_lengths = period_in_era_lengths * CycleConfig::periods_per_cycle();
        assert_eq!(CycleConfig::cycle_in_era_lengths(), cycle_in_era_lengths);

        let blocks_per_cycle = cycle_in_era_lengths * CycleConfig::blocks_per_era();
        assert_eq!(CycleConfig::blocks_per_cycle(), blocks_per_cycle);

        let build_and_earn_eras_per_cycle =
            CycleConfig::eras_per_build_and_earn_subperiod() * CycleConfig::periods_per_cycle();
        assert_eq!(
            CycleConfig::build_and_earn_eras_per_cycle(),
            build_and_earn_eras_per_cycle
        );
    })
}

#[test]
fn test_genesis_build() {
    ExternalityBuilder::build().execute_with(|| {
        let genesis_config = InflationConfig::default();
        assert!(genesis_config.params.is_valid());

        // Prep actions
        ActiveInflationConfig::<Test>::kill();
        InflationParams::<Test>::kill();

        // Execute genesis build
        <pallet::GenesisConfig<Test> as BuildGenesisConfig>::build(&genesis_config);

        // Verify state is as expected
        assert_eq!(InflationParams::<Test>::get(), genesis_config.params);
        assert!(ActiveInflationConfig::<Test>::get().recalculation_era > 0);
    })
}

#[test]
fn on_initialize_decay_and_payout_works() {
    ExternalityBuilder::build().execute_with(|| {
        // no decay
        ActiveInflationConfig::<Test>::mutate(|config| {
            config.decay_factor = Perquintill::one();
            config.decay_rate = Perquintill::one();
            config.collator_reward_per_block = 10;
            config.treasury_reward_per_block = 5;
        });

        let base_rewards = 10 + 5;
        let issuance_before = Balances::total_issuance();
        Inflation::on_initialize(1);
        let issuance_after = Balances::total_issuance();
        let paid_out = issuance_after - issuance_before;
        assert_eq!(paid_out, 10 + 5, "Full payout expected");

        // 50% decay
        let decay_rate = Perquintill::from_percent(50);
        ActiveInflationConfig::<Test>::mutate(|config| {
            config.decay_rate = decay_rate;
        });

        let initial_issuance = Balances::total_issuance();
        let blocks_to_run = 10;
        let mut expected_factor = Perquintill::one();
        let mut total_expected_payout = 0;
        for _ in 0..blocks_to_run {
            Inflation::on_initialize(1);
            expected_factor = expected_factor.saturating_mul(decay_rate);
            total_expected_payout += expected_factor * base_rewards;
        }
        let issuance_now = Balances::total_issuance();
        lenient_balance_assert_eq!(issuance_now, initial_issuance + total_expected_payout);

        // Config checks
        let cfg = ActiveInflationConfig::<Test>::get();
        let expected_factor = decay_rate.saturating_pow(blocks_to_run);
        assert_eq!(cfg.decay_factor, expected_factor);
        assert_eq!(cfg.decay_rate, decay_rate);
        assert_eq!(cfg.collator_reward_per_block, 10);
        assert_eq!(cfg.treasury_reward_per_block, 5);
    });
}

#[test]
fn set_decay_factor_works() {
    ExternalityBuilder::build().execute_with(|| {
        // Sanity Check
        assert_eq!(
            ActiveInflationConfig::<Test>::get().decay_factor,
            Perquintill::one()
        );

        assert_noop!(
            Inflation::force_set_decay_factor(RuntimeOrigin::signed(1), Perquintill::one()),
            BadOrigin
        );

        let new_decay_factor = Perquintill::from_percent(98);
        assert_ok!(Inflation::force_set_decay_factor(
            RuntimeOrigin::root(),
            new_decay_factor
        ));
        System::assert_last_event(
            Event::DecayFactorUpdated {
                decay_factor: new_decay_factor,
            }
            .into(),
        );
        assert_eq!(
            ActiveInflationConfig::<Test>::get().decay_factor,
            new_decay_factor
        );
    })
}

// Test that the recalculation uses the original max_emission, not the decayed values
#[test]
fn force_readjust_config_with_decay_works() {
    ExternalityBuilder::build().execute_with(|| {
        let params = InflationParams::<Test>::get();
        let init_total_issuance = Balances::total_issuance();
        let original_max_emission = params.max_inflation_rate * init_total_issuance;

        // Prerequisite: Set decay and run a few blocks to decay the config
        let decay_rate = Perquintill::from_percent(99);
        ActiveInflationConfig::<Test>::mutate(|config| {
            config.decay_rate = decay_rate;
        });
        let blocks_to_run = 500;
        for _ in 0..blocks_to_run {
            Inflation::on_initialize(1);
        }
        let expected_factor = decay_rate.saturating_pow(blocks_to_run);

        assert_ok!(Inflation::force_readjust_config(RuntimeOrigin::root()));

        let new_config = ActiveInflationConfig::<Test>::get();
        lenient_perquintill_assert_eq!(new_config.decay_factor, expected_factor);

        // New config is based on original max emission since the decay factor is applied on payouts
        let new_max_emission_from_config = new_config.collator_reward_per_block
            * Balance::from(<Test as Config>::CycleConfiguration::blocks_per_cycle())
            + new_config.treasury_reward_per_block
                * Balance::from(<Test as Config>::CycleConfiguration::blocks_per_cycle())
            + new_config.dapp_reward_pool_per_era
                * Balance::from(
                    <Test as Config>::CycleConfiguration::build_and_earn_eras_per_cycle(),
                )
            + new_config.base_staker_reward_pool_per_era
                * Balance::from(
                    <Test as Config>::CycleConfiguration::build_and_earn_eras_per_cycle(),
                )
            + new_config.adjustable_staker_reward_pool_per_era
                * Balance::from(
                    <Test as Config>::CycleConfiguration::build_and_earn_eras_per_cycle(),
                )
            + new_config.bonus_reward_pool_per_period
                * Balance::from(<Test as Config>::CycleConfiguration::periods_per_cycle());

        lenient_balance_assert_eq!(original_max_emission, new_max_emission_from_config);
    })
}

#[test]
fn force_update_decay_rate_and_reset_factor_works() {
    ExternalityBuilder::build().execute_with(|| {
        // 1. Initial setup with 90% decay rate
        let initial_decay = Perquintill::from_percent(90);
        ActiveInflationConfig::<Test>::mutate(|config| {
            config.decay_rate = initial_decay;
            config.collator_reward_per_block = 100;
            config.treasury_reward_per_block = 50;
        });

        let base_rewards = 100 + 50;
        let mut expected_factor = Perquintill::one();

        // Run 10 blocks with 90% decay
        for block in 1..=10 {
            Inflation::on_initialize(block);
            expected_factor = expected_factor.saturating_mul(initial_decay);
        }

        let cfg = ActiveInflationConfig::<Test>::get();
        assert_eq!(cfg.decay_rate, initial_decay);
        assert_eq!(cfg.decay_factor, expected_factor);

        // 2. Root forced changes
        // Force-set inflation params to remove decay rate = 100%
        let new_decay_rate = Perquintill::one();
        assert_ok!(Inflation::force_set_inflation_params(
            RuntimeOrigin::root(),
            InflationParameters {
                decay_rate: new_decay_rate,
                ..Default::default()
            }
        ));

        // Update decay factor manually to 50%
        let new_decay_factor = Perquintill::from_percent(50);
        assert_ok!(Inflation::force_set_decay_factor(
            RuntimeOrigin::root(),
            new_decay_factor
        ));

        // Force readjust config
        assert_ok!(Inflation::force_readjust_config(RuntimeOrigin::root()));

        // Check updates
        let cfg_after = ActiveInflationConfig::<Test>::get();
        assert_eq!(
            cfg_after.decay_rate, new_decay_rate,
            "Decay rate should be reset to default"
        );
        assert_eq!(
            cfg_after.decay_factor, new_decay_factor,
            "Decay factor should be updated"
        );

        let issuance_before = Balances::total_issuance();
        Inflation::on_initialize(1);
        let total_expected_payout = new_decay_factor.mul_floor(base_rewards);
        let issuance_now = Balances::total_issuance();
        lenient_balance_assert_eq!(issuance_now, issuance_before + total_expected_payout);
    });
}
