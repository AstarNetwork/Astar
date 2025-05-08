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
    RuntimeOrigin, DEMOCRACY_ID,
};
use frame_support::dispatch::DispatchResult;
use frame_support::traits::StorePreimage;
use frame_support::{assert_ok, traits::OnRuntimeUpgrade};
use pallet_balances::BalanceLock;
use pallet_democracy::{
    AccountVote, Conviction, ReferendumInfo, ReferendumInfoOf, ReferendumStatus, Vote,
};
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

fn aye(x: u8, balance: u64) -> AccountVote<u64> {
    AccountVote::Standard {
        vote: Vote {
            aye: true,
            conviction: Conviction::try_from(x).unwrap(),
        },
        balance,
    }
}

fn the_lock(amount: u64) -> BalanceLock<u64> {
    BalanceLock {
        id: DEMOCRACY_ID,
        amount,
        reasons: pallet_balances::Reasons::All,
    }
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
            ReferendumInfo::Ongoing(ReferendumStatus {
                end: 26,
                delay: 2,
                ..
            })
        ));
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(3).unwrap(),
            ReferendumInfo::Ongoing(ReferendumStatus {
                end: 28,
                delay: 2,
                ..
            })
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
            ReferendumInfo::Ongoing(ReferendumStatus {
                end: 28,
                delay: 4,
                ..
            })
        ));
        // migration block = 24, referendum ends at 28, remaining = 4
        // New end = 24 + (4*2) = 32
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(3).unwrap(),
            ReferendumInfo::Ongoing(ReferendumStatus {
                end: 32,
                delay: 4,
                ..
            })
        ));
    });
}

#[test]
fn migrate_vote_lock_durations() {
    new_test_ext().execute_with(|| {
        // Create 2 finished referendums
        assert_ok!(propose_set_balance(1, 2, 1));
        run_to_block(2);

        let ref_index = 0;

        assert_ok!(Democracy::vote(
            RuntimeOrigin::signed(1),
            ref_index,
            aye(1, 30)
        ));
        assert_ok!(Democracy::vote(
            RuntimeOrigin::signed(2),
            ref_index,
            aye(6, 40)
        ));

        // run to block 22 so that referendum 22 is approved and finished
        run_to_block(22);
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(0).unwrap(),
            ReferendumInfo::Finished {
                approved: true,
                end: 22
            }
        ));

        assert_ok!(Democracy::remove_vote(RuntimeOrigin::signed(1), ref_index));
        assert_ok!(Democracy::remove_vote(RuntimeOrigin::signed(2), ref_index));

        // Run migration - on block 22
        AllPalletsWithSystem::on_runtime_upgrade();
        run_to_block(23);

        // --- After migration Account 1 ---

        // Account 1 should be able to unlock at: 62
        // remaining: (previous unlock: 42 - migration block: 22) * 2 = 40
        // total: remaining: 40 + 22 = 62

        // Ensure Account 1 cannot unlock before 62:
        run_to_block(61);
        assert_ok!(Democracy::unlock(RuntimeOrigin::signed(1), 1));
        assert_eq!(
            pallet_balances::Locks::<Runtime>::get(&1),
            vec![the_lock(30)]
        );

        // Ensure account 1 can unlock at block 62:
        run_to_block(62);
        assert_ok!(Democracy::unlock(RuntimeOrigin::signed(1), 1));
        assert_eq!(pallet_balances::Locks::<Runtime>::get(&1), vec![]);

        // --- After migration Account 2 ---

        // Account 2 should be able to unlock at: 1302
        // remaining: (previous unlock: 662 - migration block: 22) * 2 = 1280
        // total: remaining: 1280 + 22 = 1302

        // Ensure Account 2 cannot unlock before 1302:
        run_to_block(1301);
        assert_ok!(Democracy::unlock(RuntimeOrigin::signed(2), 2));
        assert_eq!(
            pallet_balances::Locks::<Runtime>::get(&2),
            vec![the_lock(40)]
        );

        // Ensure account 2 can unlock at block 1302:
        run_to_block(1302);
        assert_ok!(Democracy::unlock(RuntimeOrigin::signed(2), 2));
        assert_eq!(pallet_balances::Locks::<Runtime>::get(&2), vec![]);
    });
}

#[test]
fn migrate_vote_lock_durations_of_delegating() {
    new_test_ext().execute_with(|| {
        // Create 2 finished referendums
        assert_ok!(propose_set_balance(1, 2, 1));
        run_to_block(2);
        let ref_index = 0;

        // Account 1 delegates to Account 2 2x, so it has a PriorLock on a delegate voting
        // The first delegation will become the PriorLock.
        // The conviction is 3x, so it will last longer than the nex conviction (that will be none)
        // It ensures prior lock is correctly migrated
        assert_ok!(Democracy::delegate(
            RuntimeOrigin::signed(1),
            2,
            Conviction::Locked3x,
            10
        ));
        assert_ok!(Democracy::vote(
            RuntimeOrigin::signed(2),
            ref_index,
            aye(3, 25)
        ));
        // The second conviction is None, so when we call undelegate, it will unlock directly
        // and only the previous lock will be taking into account
        assert_ok!(Democracy::delegate(
            RuntimeOrigin::signed(1),
            2,
            Conviction::None,
            1
        ));
        assert_ok!(Democracy::vote(
            RuntimeOrigin::signed(2),
            ref_index,
            aye(3, 25)
        ));

        // run to block 22 so that referendum 22 is approved and finished
        run_to_block(22);
        assert!(matches!(
            ReferendumInfoOf::<Runtime>::get(0).unwrap(),
            ReferendumInfo::Finished {
                approved: true,
                end: 22
            }
        ));

        assert_ok!(Democracy::remove_vote(RuntimeOrigin::signed(2), ref_index));

        // Run migration - on block 22
        AllPalletsWithSystem::on_runtime_upgrade();
        run_to_block(23);

        // --- After migration Account 1 Delegate Voting ---

        // Account 1 should be able to unlock at: 142
        // previous unlock: end: 22 + - base lock period: 20 * 3xlock: 3 = 82
        // remaining: (previous unlock: 82 - migration block: 22) * 2 = 120
        // total: remaining: 120 + 22 = 142

        // Ensure Account 1 cannot unlock before 142:
        run_to_block(141);
        assert_ok!(Democracy::undelegate(RuntimeOrigin::signed(1)));
        assert_ok!(Democracy::unlock(RuntimeOrigin::signed(1), 1));
        assert_eq!(
            pallet_balances::Locks::<Runtime>::get(&1),
            vec![the_lock(10)]
        );

        // Ensure account 1 can unlock at block 142:
        run_to_block(142);
        assert_ok!(Democracy::unlock(RuntimeOrigin::signed(1), 1));
        assert_eq!(pallet_balances::Locks::<Runtime>::get(&1), vec![]);
    });
}
