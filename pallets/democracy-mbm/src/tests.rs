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
fn migrate_ongoing_referendum() {
    new_test_ext().execute_with(|| {
        // create a referendum
        assert_ok!(propose_set_balance(1, 2, 1));
        run_to_block(2);
        let ref_index = 0;
        // sanity check: ensure the referendum is Ongoing with and end at 22
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(ref_index).unwrap(),
            ReferendumInfo::Ongoing(ReferendumStatus { end: 22, .. })
        ));

        // run multiblock migration
        AllPalletsWithSystem::on_runtime_upgrade();
        run_to_block(3);

        // the end value should be updated from 22 to 40:
        // current block number = 2
        // remaining: end:22 - current_block_number:2 = 20
        // multiply it by 2: 40
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(ref_index).unwrap(),
            ReferendumInfo::Ongoing(ReferendumStatus { end: 40, .. })
        ));
    });
}

#[test]
fn finished_referendum_is_not_migrated() {
    new_test_ext().execute_with(|| {
        // create a referendum
        assert_ok!(propose_set_balance(1, 2, 1));
        run_to_block(2);
        let ref_index = 0;
        // sanity check: ensure the referendum is Ongoing with and end at 22
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(ref_index).unwrap(),
            ReferendumInfo::Ongoing(ReferendumStatus { end: 22, .. })
        ));
        // run to block 22 - which is the end value of the referendum
        // and ensure it is Finished
        run_to_block(22);
        assert_eq!(
            ReferendumInfoOf::<Runtime>::get(ref_index).unwrap(),
            ReferendumInfo::Finished {
                approved: false,
                end: 22
            }
        );

        // run multiblock migration
        AllPalletsWithSystem::on_runtime_upgrade();
        run_to_block(23);

        assert_eq!(
            ReferendumInfoOf::<Runtime>::get(ref_index).unwrap(),
            ReferendumInfo::Finished {
                approved: false,
                end: 22
            }
        );
    });
}
