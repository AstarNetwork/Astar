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
        Inflation::on_finalize(init_config.recalculation_block - 1);
        assert!(
            InflationConfig::<Test>::get() != init_config,
            "Recalculation should have happened."
        );

        // TODO: should there be an event to mark this?
    })
}
