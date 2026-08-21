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
use frame_support::{
    migrations::SteppedMigrations,
    traits::fungible::{Inspect, InspectHold, MutateHold},
    weights::WeightMeter,
};
use sp_runtime::DispatchError;

/// New WASM code uploads are frozen.
#[test]
fn upload_code_is_disabled() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Contracts::upload_code(
                RuntimeOrigin::signed(ALICE),
                vec![],
                None,
                pallet_contracts::Determinism::Enforced,
            ),
            DispatchError::BadOrigin,
        );
    });
}

/// Deploying a new contract with inline code is frozen.
#[test]
fn instantiate_with_code_is_disabled() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Contracts::instantiate_with_code(
                RuntimeOrigin::signed(ALICE),
                0u128,
                Weight::zero(),
                None,
                vec![],
                vec![],
                vec![],
            ),
            DispatchError::BadOrigin,
        );
    });
}

/// Instantiating from an existing code hash is frozen.
#[test]
fn instantiate_is_disabled() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Contracts::instantiate(
                RuntimeOrigin::signed(ALICE),
                0u128,
                Weight::zero(),
                None,
                Default::default(),
                vec![],
                vec![],
            ),
            DispatchError::BadOrigin,
        );
    });
}

/// The escrow collecting the settled contract deposits must stay the foundation account.
#[test]
fn deposit_escrow_is_the_foundation_account() {
    // XPDSbfc3fcoVWEtPsxQXFDvWqnZgQfsxXv6MW8dd7G3GkZt
    assert_eq!(
        ContractsDepositEscrow::get(),
        AccountId32::new(hex_literal::hex!(
            "400048a4f3672511dfcf2ddfcb34bafb80ee2f28bbec8cbe0283e90573e93474"
        )),
    );
}

/// End-to-end check against the real runtime rather than a mock: a hold carrying the actual
/// `pallet_contracts` reason - i.e. the real `RuntimeHoldReason` variant, at the real pallet
/// index - is swept to the configured escrow by the migration tuple the runtime actually runs.
#[test]
fn contracts_holds_are_swept_to_the_escrow() {
    new_test_ext().execute_with(|| {
        let reason: RuntimeHoldReason = pallet_contracts::HoldReason::StorageDepositReserve.into();
        let escrow = ContractsDepositEscrow::get();
        let amount = 1_000 * UNIT;

        assert_ok!(Balances::hold(&reason, &ALICE, amount));
        assert_eq!(Balances::balance_on_hold(&reason, &ALICE), amount);
        let escrow_before = Balances::balance(&escrow);

        // Driven the way `pallet-migrations` drives it, through the runtime's own tuple.
        let mut cursor = None;
        loop {
            let mut meter = WeightMeter::new();
            match MultiBlockMigrationsList::nth_step(0, cursor, &mut meter)
                .expect("the tuple has a migration at index 0")
                .expect("the step succeeds")
            {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }

        assert_eq!(Balances::balance_on_hold(&reason, &ALICE), 0);
        assert_eq!(Balances::balance(&escrow), escrow_before + amount);
    });
}
