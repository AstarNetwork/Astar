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
use frame_support::{
    assert_noop, assert_ok, assert_storage_noop,
    traits::{GenesisBuild, Hooks},
};
use mock::*;
use sp_runtime::{
    traits::{AccountIdConversion, BadOrigin, Zero},
    Perquintill,
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
        assert!(new_params != InflationParams::<Test>::get(), "Sanity check");

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
fn force_set_inflation_config_work() {
    ExternalityBuilder::build().execute_with(|| {
        let mut new_config = ActiveInflationConfig::<Test>::get();
        new_config.recalculation_block = new_config.recalculation_block + 50;

        // Execute call, ensure it works
        assert_ok!(Inflation::force_set_inflation_config(
            RuntimeOrigin::root(),
            new_config
        ));
        System::assert_last_event(
            Event::InflationConfigurationForceChanged { config: new_config }.into(),
        );

        assert_eq!(ActiveInflationConfig::<Test>::get(), new_config);
    })
}

#[test]
fn force_set_inflation_config_fails() {
    ExternalityBuilder::build().execute_with(|| {
        let mut new_config = ActiveInflationConfig::<Test>::get();
        new_config.recalculation_block = new_config.recalculation_block + 50;

        // Make sure action is privileged
        assert_noop!(
            Inflation::force_set_inflation_config(RuntimeOrigin::signed(1), new_config),
            BadOrigin
        );
    })
}

#[test]
fn force_inflation_recalculation_work() {
    ExternalityBuilder::build().execute_with(|| {
        let old_config = ActiveInflationConfig::<Test>::get();

        // Execute call, ensure it works
        assert_ok!(Inflation::force_inflation_recalculation(
            RuntimeOrigin::root(),
        ));

        let new_config = ActiveInflationConfig::<Test>::get();
        assert!(
            old_config != new_config,
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
            Inflation::force_inflation_recalculation(RuntimeOrigin::signed(1)),
            BadOrigin
        );
    })
}

#[test]
fn inflation_recalculation_occurs_when_exepcted() {
    ExternalityBuilder::build().execute_with(|| {
        let init_config = ActiveInflationConfig::<Test>::get();

        // Make sure `on_finalize` calls before the expected change are storage noops
        advance_to_block(init_config.recalculation_block - 3);
        assert_storage_noop!(Inflation::on_finalize(init_config.recalculation_block - 3));
        Inflation::on_initialize(
            init_config.recalculation_block - 2
        );
        assert_storage_noop!(Inflation::on_finalize(init_config.recalculation_block - 2));
        Inflation::on_initialize(
            init_config.recalculation_block - 1
        );

        // One block before recalculation, on_finalize should calculate new inflation config
        let init_config = ActiveInflationConfig::<Test>::get();
        let init_total_issuance = Balances::total_issuance();

        // Finally trigger inflation recalculation.
        Inflation::on_finalize(init_config.recalculation_block - 1);

        let new_config = ActiveInflationConfig::<Test>::get();
        assert!(
            new_config != init_config,
            "Recalculation must happen at this point."
        );
        System::assert_last_event(Event::NewInflationConfiguration { config: new_config }.into());

        assert_eq!(
            Balances::total_issuance(),
            init_total_issuance,
            "Total issuance must not change when inflation is recalculated - nothing is minted until it's needed."
        );

        assert_eq!(new_config.issuance_safety_cap, init_total_issuance + InflationParams::<Test>::get().max_inflation_rate * init_total_issuance);
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
fn inflation_recalucation_works() {
    ExternalityBuilder::build().execute_with(|| {
        let total_issuance = Balances::total_issuance();
        let params = InflationParams::<Test>::get();
        let now = System::block_number();

        // Calculate new config
        let new_config = Inflation::recalculate_inflation(now);
        let max_emission = params.max_inflation_rate * total_issuance;

        // Verify basics are ok
        assert_eq!(
            new_config.recalculation_block,
            now + <Test as Config>::CycleConfiguration::blocks_per_cycle()
        );
        assert_eq!(
            new_config.issuance_safety_cap,
            total_issuance + max_emission,
        );

        // Verify collator rewards are as expected
        assert_eq!(
            new_config.collator_reward_per_block,
            params.collators_part * max_emission
                / Balance::from(<Test as Config>::CycleConfiguration::blocks_per_cycle()),
        );

        // Verify treasury rewards are as expected
        assert_eq!(
            new_config.treasury_reward_per_block,
            params.treasury_part * max_emission
                / Balance::from(<Test as Config>::CycleConfiguration::blocks_per_cycle()),
        );

        // Verify dApp rewards are as expected
        assert_eq!(
            new_config.dapp_reward_pool_per_era,
            params.dapps_part * max_emission
                / Balance::from(
                    <Test as Config>::CycleConfiguration::build_and_earn_eras_per_cycle()
                ),
        );

        // Verify base & adjustable staker rewards are as expected
        assert_eq!(
            new_config.base_staker_reward_pool_per_era,
            params.base_stakers_part * max_emission
                / Balance::from(
                    <Test as Config>::CycleConfiguration::build_and_earn_eras_per_cycle()
                ),
        );
        assert_eq!(
            new_config.adjustable_staker_reward_pool_per_era,
            params.adjustable_stakers_part * max_emission
                / Balance::from(
                    <Test as Config>::CycleConfiguration::build_and_earn_eras_per_cycle()
                ),
        );

        // Verify bonus rewards are as expected
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
fn payout_reward_with_exceeded_cap_but_not_exceeded_relaxed_cap_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // Prepare reward payout params
        let config = ActiveInflationConfig::<Test>::get();
        let account = 1;

        let relaxed_cap = config.issuance_safety_cap * 101 / 100;
        let reward = relaxed_cap - Balances::total_issuance();
        let init_balance = Balances::free_balance(&account);
        let init_issuance = Balances::total_issuance();

        // Payout reward and verify balances are as expected
        assert_ok!(Inflation::payout_reward(&account, reward));

        assert_eq!(Balances::free_balance(&account), init_balance + reward);
        assert_eq!(Balances::total_issuance(), init_issuance + reward);
    })
}

#[test]
fn payout_reward_fails_when_relaxed_cap_is_exceeded() {
    ExternalityBuilder::build().execute_with(|| {
        // Prepare reward payout params
        let config = ActiveInflationConfig::<Test>::get();
        let account = 1;

        let relaxed_cap = config.issuance_safety_cap * 101 / 100;
        let reward = relaxed_cap - Balances::total_issuance() + 1;

        // Payout should be a failure, with storage noop.
        assert_noop!(Inflation::payout_reward(&account, reward), ());
    })
}

#[test]
fn cylcle_configuration_works() {
    ExternalityBuilder::build().execute_with(|| {
        type CycleConfig = <Test as Config>::CycleConfiguration;

        let eras_per_period = CycleConfig::eras_per_voting_subperiod()
            + CycleConfig::eras_per_build_and_earn_subperiod();
        assert_eq!(CycleConfig::eras_per_period(), eras_per_period);

        let eras_per_cycle = eras_per_period * CycleConfig::periods_per_cycle();
        assert_eq!(CycleConfig::eras_per_cycle(), eras_per_cycle);

        let blocks_per_cycle = eras_per_cycle * CycleConfig::blocks_per_era();
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
        <pallet::GenesisConfig as GenesisBuild<Test>>::build(&genesis_config);

        // Verify state is as expected
        assert_eq!(InflationParams::<Test>::get(), genesis_config.params);
        assert!(ActiveInflationConfig::<Test>::get().recalculation_block > 0);
    })
}
