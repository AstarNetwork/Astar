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
    traits::{Hooks, OnTimestampSet},
};
use mock::*;
use sp_runtime::{
    traits::{AccountIdConversion, BadOrigin, Zero},
    Perquintill,
};

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
        let mut new_config = InflationConfig::<Test>::get();
        new_config.recalculation_block = new_config.recalculation_block + 50;

        // Execute call, ensure it works
        assert_ok!(Inflation::force_set_inflation_config(
            RuntimeOrigin::root(),
            new_config
        ));
        System::assert_last_event(
            Event::InflationConfigurationForceChanged { config: new_config }.into(),
        );

        assert_eq!(InflationConfig::<Test>::get(), new_config);
    })
}

#[test]
fn force_set_inflation_config_fails() {
    ExternalityBuilder::build().execute_with(|| {
        let mut new_config = InflationConfig::<Test>::get();
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
        let old_config = InflationConfig::<Test>::get();

        // Execute call, ensure it works
        assert_ok!(Inflation::force_inflation_recalculation(
            RuntimeOrigin::root(),
        ));

        let new_config = InflationConfig::<Test>::get();
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
fn inflation_recalculation_occurs_when_exepcted() {
    ExternalityBuilder::build().execute_with(|| {
        let init_config = InflationConfig::<Test>::get();

        // Make sure calls before the expected change are storage noops
        advance_to_block(init_config.recalculation_block - 3);
        assert_storage_noop!(Inflation::on_finalize(init_config.recalculation_block - 3));
        assert_storage_noop!(Inflation::on_initialize(
            init_config.recalculation_block - 2
        ));
        assert_storage_noop!(Inflation::on_finalize(init_config.recalculation_block - 2));
        assert_storage_noop!(Inflation::on_initialize(
            init_config.recalculation_block - 1
        ));

        // One block before recalculation, on_finalize should calculate new inflation config
        let init_config = InflationConfig::<Test>::get();
        let init_tracker = SafetyInflationTracker::<Test>::get();
        let init_total_issuance = Balances::total_issuance();

        // Finally trigger inflation recalculation.
        Inflation::on_finalize(init_config.recalculation_block - 1);

        let new_config = InflationConfig::<Test>::get();
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

        let new_tracker = SafetyInflationTracker::<Test>::get();
        assert_eq!(new_tracker.issued, init_tracker.issued);
        assert_eq!(new_tracker.cap, init_tracker.cap + InflationParams::<Test>::get().max_inflation_rate * init_total_issuance);
    })
}

#[test]
fn on_timestamp_set_payout_works() {
    ExternalityBuilder::build().execute_with(|| {
        // Save initial state, before the payout
        let config = InflationConfig::<Test>::get();
        let init_tracker = SafetyInflationTracker::<Test>::get();

        let init_issuance = Balances::total_issuance();
        let init_collator_pot = Balances::free_balance(&COLLATOR_POT.into_account_truncating());
        let init_treasury_pot = Balances::free_balance(&TREASURY_POT.into_account_truncating());

        // Execute payout
        Inflation::on_timestamp_set(1);

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

        // Safety tracker has been properly updated
        let post_tracker = SafetyInflationTracker::<Test>::get();
        assert_eq!(post_tracker.cap, init_tracker.cap);
        assert_eq!(post_tracker.issued, init_tracker.issued + expected_reward);
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
        let (max_emission, new_config) = Inflation::recalculate_inflation(now);

        // Verify basics are ok
        assert_eq!(max_emission, params.max_inflation_rate * total_issuance);
        assert_eq!(
            new_config.recalculation_block,
            now + <Test as Config>::CycleConfiguration::blocks_per_cycle()
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
