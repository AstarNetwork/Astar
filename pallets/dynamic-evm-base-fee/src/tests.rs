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

use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;

use fp_evm::FeeCalculator;

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

        assert_eq!(
            DynamicEvmBaseFee::min_gas_price(),
            (
                new_base_fee_per_gas,
                <TestRuntime as frame_system::Config>::DbWeight::get().reads(1)
            )
        );
    });
}
