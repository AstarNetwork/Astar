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

use crate::test::mock::*;
use crate::*;

// Helper to generate custom `Get` types for testing the `AccountLedger` struct.
macro_rules! get_u32_type {
    ($struct_name:ident, $value:expr) => {
        struct $struct_name;
        impl Get<u32> for $struct_name {
            fn get() -> u32 {
                $value
            }
        }
    };
}

#[test]
fn protocol_state_default() {
    let protoc_state = ProtocolState::<BlockNumber>::default();

    assert_eq!(protoc_state.era, 0);
    assert_eq!(
        protoc_state.next_era_start, 1,
        "Era should start immediately on the first block"
    );
}

#[test]
fn account_ledger_default() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let acc_ledger = AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    assert!(acc_ledger.is_empty());
    assert!(acc_ledger.locked_amount().is_zero());
    assert!(acc_ledger.era().is_zero());
    assert!(acc_ledger.latest_locked_chunk().is_none());
}

#[test]
fn account_ledger_add_lock_amount_works() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    // First step, sanity checks
    let first_era = 1;
    assert!(acc_ledger.locked_amount().is_zero());
    assert!(acc_ledger.add_lock_amount(0, first_era).is_ok());
    assert!(acc_ledger.locked_amount().is_zero());

    // Adding lock value works as expected
    let init_amount = 20;
    assert!(acc_ledger.add_lock_amount(init_amount, first_era).is_ok());
    assert_eq!(acc_ledger.locked_amount(), init_amount);
    assert_eq!(acc_ledger.era(), first_era);
    assert!(!acc_ledger.is_empty());
    assert_eq!(
        acc_ledger.latest_locked_chunk(),
        Some(&LockedChunk::<Balance> {
            amount: init_amount,
            era: first_era,
        })
    );

    // Add to the same era
    let addition = 7;
    assert!(acc_ledger.add_lock_amount(addition, first_era).is_ok());
    assert_eq!(acc_ledger.locked_amount(), init_amount + addition);
    assert_eq!(acc_ledger.era(), first_era);

    // Add up to storage limit
    for i in 2..=LockedDummy::get() {
        assert!(acc_ledger.add_lock_amount(addition, first_era + i).is_ok());
        assert_eq!(
            acc_ledger.locked_amount(),
            init_amount + addition * i as u128
        );
        assert_eq!(acc_ledger.era(), first_era + i);
    }

    // Any further additions should fail due to exhausting bounded storage capacity
    assert!(acc_ledger
        .add_lock_amount(addition, acc_ledger.era() + 1)
        .is_err());
    assert!(!acc_ledger.is_empty());
}
