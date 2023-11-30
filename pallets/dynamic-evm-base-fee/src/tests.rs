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

#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{
    assert_noop, assert_ok,
    traits::{Get, OnFinalize},
};
use num_traits::Bounded;
use sp_runtime::{
    traits::{BadOrigin, One, Zero},
    FixedU128,
};

use fp_evm::FeeCalculator;

#[test]
fn default_base_fee_per_gas_works() {
    ExtBuilder::build().execute_with(|| {
        // Genesis state check
        assert_eq!(
            BaseFeePerGas::<TestRuntime>::get(),
            <TestRuntime as pallet::Config>::DefaultBaseFeePerGas::get(),
            "Init bfpg should be equal to the specified default one."
        )
    });
}

#[test]
fn set_base_fee_per_gas_works() {
    ExtBuilder::build().execute_with(|| {
        // sanity check
        assert_eq!(
            BaseFeePerGas::<TestRuntime>::get(),
            <TestRuntime as pallet::Config>::DefaultBaseFeePerGas::get()
        );

        // Ensure we can change the bfpg value via root
        for new_base_fee_per_gas in [
            <TestRuntime as pallet::Config>::MinBaseFeePerGas::get(),
            <TestRuntime as pallet::Config>::MaxBaseFeePerGas::get(),
        ] {
            assert_ok!(DynamicEvmBaseFee::set_base_fee_per_gas(
                RuntimeOrigin::root(),
                new_base_fee_per_gas
            ));
            System::assert_last_event(mock::RuntimeEvent::DynamicEvmBaseFee(
                Event::NewBaseFeePerGas {
                    fee: new_base_fee_per_gas,
                },
            ));
            assert_eq!(BaseFeePerGas::<TestRuntime>::get(), new_base_fee_per_gas);
        }
    });
}

#[test]
fn set_base_fee_per_gas_value_out_of_bounds_fails() {
    ExtBuilder::build().execute_with(|| {
        // Out of bound values
        let too_small_base_fee_per_gas =
            <TestRuntime as pallet::Config>::MinBaseFeePerGas::get() - 1;
        let too_big_base_fee_per_gas = <TestRuntime as pallet::Config>::MaxBaseFeePerGas::get() + 1;

        assert_noop!(
            DynamicEvmBaseFee::set_base_fee_per_gas(
                RuntimeOrigin::root(),
                too_small_base_fee_per_gas
            ),
            Error::<TestRuntime>::ValueOutOfBounds
        );
        assert_noop!(
            DynamicEvmBaseFee::set_base_fee_per_gas(
                RuntimeOrigin::root(),
                too_big_base_fee_per_gas
            ),
            Error::<TestRuntime>::ValueOutOfBounds
        );
    });
}

#[test]
fn set_base_fee_per_gas_non_root_fails() {
    ExtBuilder::build().execute_with(|| {
        assert_noop!(
            DynamicEvmBaseFee::set_base_fee_per_gas(
                RuntimeOrigin::signed(1),
                <TestRuntime as pallet::Config>::MinBaseFeePerGas::get()
            ),
            BadOrigin
        );
    });
}

#[test]
fn min_gas_price_works() {
    ExtBuilder::build().execute_with(|| {
        let new_base_fee_per_gas =
            <TestRuntime as pallet::Config>::MinBaseFeePerGas::get() + 19 * 17;
        assert_ok!(DynamicEvmBaseFee::set_base_fee_per_gas(
            RuntimeOrigin::root(),
            new_base_fee_per_gas
        ));

        let expected_weight: Weight =
            <<TestRuntime as pallet::Config>::WeightInfo as weights::WeightInfo>::min_gas_price();
        assert_eq!(
            DynamicEvmBaseFee::min_gas_price(),
            (new_base_fee_per_gas, expected_weight)
        );
    });
}

#[test]
fn unit_adjustment_factor_no_change() {
    ExtBuilder::build().execute_with(|| {
        // Prep init values - ideal bfpg, and unit adjustment factor
        let init_bfpg = get_ideal_bfpg();
        BaseFeePerGas::<TestRuntime>::set(init_bfpg);
        set_adjustment_factor(FixedU128::one());

        DynamicEvmBaseFee::on_finalize(1);
        assert_eq!(
            BaseFeePerGas::<TestRuntime>::get(),
            init_bfpg,
            "bfpg should remain the same"
        );
    });
}

#[test]
fn bfpg_bounds_are_respected() {
    ExtBuilder::build().execute_with(|| {
        // Lower bound
        let min_bfpg = <TestRuntime as pallet::Config>::MinBaseFeePerGas::get();
        BaseFeePerGas::<TestRuntime>::set(min_bfpg);

        // This should bring the ideal bfpg value to zero
        set_adjustment_factor(FixedU128::zero());
        assert!(get_ideal_bfpg().is_zero(), "Sanity check");

        DynamicEvmBaseFee::on_finalize(1);
        assert_eq!(
            BaseFeePerGas::<TestRuntime>::get(),
            min_bfpg,
            "bfpg must not go below lower threshold."
        );

        // Upper limit
        let upper_bfpg = <TestRuntime as pallet::Config>::MaxBaseFeePerGas::get();
        BaseFeePerGas::<TestRuntime>::set(upper_bfpg);

        // This should bring the ideal bfpg very high, well above max value
        set_adjustment_factor(FixedU128::max_value());
        assert!(get_ideal_bfpg() > upper_bfpg, "Sanity check");

        DynamicEvmBaseFee::on_finalize(2);
        assert_eq!(
            BaseFeePerGas::<TestRuntime>::get(),
            upper_bfpg,
            "bfpg must not go above threshold"
        );
    });
}

#[test]
fn step_limit_ratio_is_respected() {
    ExtBuilder::build().execute_with(|| {
        // Lower bound, high adjustment factor
        let min_bfpg = <TestRuntime as pallet::Config>::MinBaseFeePerGas::get();
        BaseFeePerGas::<TestRuntime>::set(min_bfpg);
        set_adjustment_factor(FixedU128::max_value());
        let step_limit = get_max_step_limit();

        DynamicEvmBaseFee::on_finalize(1);
        assert_eq!(
            BaseFeePerGas::<TestRuntime>::get(),
            min_bfpg + step_limit,
            "Step limit ratio in ascending direction was not respected."
        );

        // Upper bound, low adjustment factor
        let max_bfpg = <TestRuntime as pallet::Config>::MaxBaseFeePerGas::get();
        BaseFeePerGas::<TestRuntime>::set(max_bfpg);
        set_adjustment_factor(FixedU128::zero());
        let step_limit = get_max_step_limit();

        DynamicEvmBaseFee::on_finalize(2);
        assert_eq!(
            BaseFeePerGas::<TestRuntime>::get(),
            max_bfpg - step_limit,
            "Step limit ratio in descending direction was not respected."
        );
    });
}

#[test]
fn bfpg_full_spectrum_change_works() {
    ExtBuilder::build().execute_with(|| {
        // Set bfpg to lowest possible, and adjustment factor to highest possible
        let min_bfpg = <TestRuntime as pallet::Config>::MinBaseFeePerGas::get();
        BaseFeePerGas::<TestRuntime>::set(min_bfpg);
        set_adjustment_factor(FixedU128::max_value());

        // Run for limited amount of iterations until upper bound is reached
        let target_bfpg = <TestRuntime as pallet::Config>::MaxBaseFeePerGas::get();
        let mut counter = 1;
        let iter_limit = 500_000; // safety limit to avoid endless loop
        while counter <= iter_limit && BaseFeePerGas::<TestRuntime>::get() < target_bfpg {
            DynamicEvmBaseFee::on_finalize(counter);
            counter += 1;
        }

        assert_eq!(BaseFeePerGas::<TestRuntime>::get(), target_bfpg,
            "bfpg upper bound not reached - either it's not enough iterations or some precision loss occurs.");
    });
}

#[test]
fn bfpg_matches_expected_value_for_so_called_average_transaction() {
    ExtBuilder::build().execute_with(|| {
        // The new proposed models suggests to use the following formula to calculate the base fee per gas:
        //
        // bfpg = (adj_factor *  weight_factor * 25_000) / 9_897_4000
        let init_bfpg = get_ideal_bfpg();
        BaseFeePerGas::<TestRuntime>::set(init_bfpg);
        let init_adj_factor = <TestRuntime as pallet::Config>::AdjustmentFactor::get();

        // Slighly increase the adjustment factor, and calculate the new base fee per gas
        //
        // To keep it closer to reality, let's assume we're using the proposed variability factor of 0.000_015.
        // Let's also assume that block fullness difference is 0.01 (1%).
        // This should result in the adjustment factor of 0.000_001_5.
        //
        // NOTE: it's important to keep the increase small so that the step doesn't saturate
        let change = FixedU128::from_rational(1500, 1_000_000_000);
        let new_adj_factor = init_adj_factor + change;
        assert!(new_adj_factor > init_adj_factor, "Sanity check");
        set_adjustment_factor(new_adj_factor);

        // Calculate the new expected base fee per gas
        let weight_factor: u128 = <TestRuntime as pallet::Config>::WeightFactor::get();
        let expected_bfpg =
            U256::from(new_adj_factor.saturating_mul_int(weight_factor) * 25_000 / 9_897_4000);

        // Calculate the new base fee per gas in the pallet
        DynamicEvmBaseFee::on_finalize(1);

        // Assert calculated value is as expected
        let new_bfpg = BaseFeePerGas::<TestRuntime>::get();
        assert!(new_bfpg > init_bfpg, "Sanity check");
        assert_eq!(new_bfpg, expected_bfpg);

        // Also check the opposite direction
        let new_adj_factor = init_adj_factor - change;
        set_adjustment_factor(new_adj_factor);
        let expected_bfpg =
            U256::from(new_adj_factor.saturating_mul_int(weight_factor) * 25_000 / 9_897_4000);

        // Calculate the new base fee per gas in the pallet
        DynamicEvmBaseFee::on_finalize(2);
        // Assert calculated value is as expected
        let new_bfpg = BaseFeePerGas::<TestRuntime>::get();
        assert!(new_bfpg < init_bfpg, "Sanity check");
        assert_eq!(new_bfpg, expected_bfpg);
    });
}

#[test]
fn lower_upper_bounds_ignored_if_bfpg_is_outside() {
    ExtBuilder::build().execute_with(|| {
        // Set the initial bfpg to be outside of the allowed range.
        // It's important reduction is sufficient so we're still below the minimum limit after the adjustment.
        let delta = 100_000_000;

        // First test when bfpg is too little
        let too_small_bfpg = <TestRuntime as pallet::Config>::MinBaseFeePerGas::get() - delta;
        BaseFeePerGas::<TestRuntime>::set(too_small_bfpg);
        DynamicEvmBaseFee::on_finalize(1);

        assert!(
            BaseFeePerGas::<TestRuntime>::get() > too_small_bfpg,
            "Bfpg should have increased slightly."
        );
        assert!(
            BaseFeePerGas::<TestRuntime>::get()
                < <TestRuntime as pallet::Config>::MinBaseFeePerGas::get(),
            "For this test, bfpg should still be below the minimum limit."
        );

        // Repeat the same test but this time bfpg is too big
        let too_big_bfpg = <TestRuntime as pallet::Config>::MaxBaseFeePerGas::get() + delta;
        BaseFeePerGas::<TestRuntime>::set(too_big_bfpg);
        DynamicEvmBaseFee::on_finalize(2);

        assert!(
            BaseFeePerGas::<TestRuntime>::get() < too_big_bfpg,
            "Bfpg should have decreased slightly."
        );
        assert!(
            BaseFeePerGas::<TestRuntime>::get()
                < <TestRuntime as pallet::Config>::MaxBaseFeePerGas::get(),
            "For this test, bfpg should still be above the maximum limit."
        );
    });
}
