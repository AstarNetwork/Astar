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

use crate::setup::*;

use frame_support::traits::Currency;
use pallet_evm::{AddressMapping, OnChargeEVMTransaction};
use sp_core::{H160, U256};
use sp_runtime::traits::AccountIdConversion;

#[test]
fn evm_fees_work() {
    new_test_ext().execute_with(|| {
        let address = H160::repeat_byte(0xbe);
        let mapped_address =
            <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(address);
        Balances::make_free_balance_be(&mapped_address, 1_000_000_000_000_000);

        type EvmFeeHandler = <Runtime as pallet_evm::Config>::OnChargeTransaction;

        // 0. Define init values
        let (base_fee, tip, init_fee) = (500, 100, 1000);
        let corrected_fee = base_fee + tip;

        let pot_account = PotId::get().into_account_truncating();
        let init_reward_pot = Balances::free_balance(&pot_account);
        let init_total_issuance = Balances::total_issuance();

        // 1. Withdraw some init fee
        let result = <EvmFeeHandler as OnChargeEVMTransaction<Runtime>>::withdraw_fee(
            &address,
            U256::from(init_fee),
        );
        let already_withdrawn = result.expect("Account is funded, must succeed.");

        // 2. Correct the charged fee
        let calculated_tip =
            <EvmFeeHandler as OnChargeEVMTransaction<Runtime>>::correct_and_deposit_fee(
                &address,
                U256::from(corrected_fee),
                U256::from(base_fee),
                already_withdrawn,
            );
        assert!(calculated_tip.is_some());

        // The expectation is that 20% of the fee was deposited into the reward pot, and the rest was burned.
        assert_eq!(
            init_reward_pot + base_fee / 5,
            Balances::free_balance(&pot_account)
        );
        assert_eq!(
            init_total_issuance - base_fee / 5 * 4,
            Balances::total_issuance()
        );

        // 3. Deposit the tip
        let issuance = Balances::total_issuance();
        let pot = Balances::free_balance(&pot_account);
        <EvmFeeHandler as OnChargeEVMTransaction<Runtime>>::pay_priority_fee(calculated_tip);
        assert_eq!(
            issuance,
            Balances::total_issuance(),
            "Total issuance should not change since tip isn't burned."
        );
        assert_eq!(
            pot + tip,
            Balances::free_balance(&pot_account),
            "Pot should increase by the tip amount."
        );
    })
}
