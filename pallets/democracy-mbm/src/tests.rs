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

#![cfg(all(test, not(feature = "runtime-benchmarks")))]

use crate::mock::{
    new_test_ext, run_to_block, AllPalletsWithSystem, Democracy, Preimage, Runtime, RuntimeCall,
    RuntimeOrigin,
};
use frame_support::dispatch::DispatchResult;
use frame_support::traits::StorePreimage;
use frame_support::{assert_ok, traits::OnRuntimeUpgrade};
use pallet_democracy::{ReferendumInfo, ReferendumInfoOf, ReferendumStatus};
use pallet_scheduler::BoundedCallOf;

fn set_balance_proposal(value: u64) -> BoundedCallOf<Runtime> {
    let inner = pallet_balances::Call::force_set_balance {
        who: 42,
        new_free: value,
    };
    let outer = RuntimeCall::Balances(inner);
    Preimage::bound(outer).unwrap()
}

fn propose_set_balance(who: u64, value: u64, delay: u64) -> DispatchResult {
    Democracy::propose(
        RuntimeOrigin::signed(who),
        set_balance_proposal(value),
        delay,
    )
}

#[test]
fn migrate_multiple_referendums() {
    new_test_ext().execute_with(|| {
        // Create 2 finished referendums
        assert_ok!(propose_set_balance(1, 2, 1));
        run_to_block(2);
        assert_ok!(propose_set_balance(1, 3, 1));
        run_to_block(3);

        // Create 2 Ongoing referendums
        assert_ok!(propose_set_balance(1, 4, 1));
        run_to_block(4);
        assert_ok!(propose_set_balance(1, 5, 1));
        run_to_block(5);

        // Run to block 24 so the first 2 referendums are finished,
        // and the last 2 are still ongoing
        run_to_block(24);

        // Verify the state before migration
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(0).unwrap(),
            ReferendumInfo::Finished {
                approved: false,
                end: 22
            }
        ));
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(1).unwrap(),
            ReferendumInfo::Finished {
                approved: false,
                end: 24
            }
        ));
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(2).unwrap(),
            ReferendumInfo::Ongoing(ReferendumStatus { end: 26, .. })
        ));
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(3).unwrap(),
            ReferendumInfo::Ongoing(ReferendumStatus { end: 28, .. })
        ));

        // Run migration - on block 24
        AllPalletsWithSystem::on_runtime_upgrade();
        run_to_block(25);

        // Verify finished referendums were not migrated
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(0).unwrap(),
            ReferendumInfo::Finished {
                approved: false,
                end: 22
            }
        ));
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(1).unwrap(),
            ReferendumInfo::Finished {
                approved: false,
                end: 24
            }
        ));

        // Verify ongoing referendums were migrated
        // migration block = 24; referendum ends at 26, remaining = 2
        // New end = 24 + (2*2) = 28
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(2).unwrap(),
            ReferendumInfo::Ongoing(ReferendumStatus { end: 28, .. })
        ));
        // migration block = 24, referendum ends at 28, remaining = 4
        // New end = 24 + (4*2) = 32
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(3).unwrap(),
            ReferendumInfo::Ongoing(ReferendumStatus { end: 32, .. })
        ));
    });
}
