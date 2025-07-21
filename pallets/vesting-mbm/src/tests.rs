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
    new_test_ext, run_to_block, AllPalletsWithSystem, Balances, RuntimeOrigin, Vesting, ALICE, BOB,
    CHARLIE, DAVE,
};
use frame_support::{
    assert_ok,
    traits::{OnRuntimeUpgrade, VestingSchedule},
};
use pallet_balances::{BalanceLock, Reasons};
use pallet_vesting::VestingInfo;

#[test]
fn migrate_to_new_block_time() {
    new_test_ext().execute_with(|| {
        assert_eq!(Vesting::vesting_balance(&ALICE), Some(90)); // 10 vested at block 1, 10 per block
        assert_eq!(Vesting::vesting_balance(&BOB), Some(100));
        assert_eq!(Vesting::vesting_balance(&CHARLIE), Some(1000));
        assert_eq!(Vesting::vesting_balance(&DAVE), Some(400));

        run_to_block(10);

        assert_eq!(Vesting::vesting_balance(&ALICE), Some(0)); // Alice fully vested at block 10
        assert_eq!(Vesting::vesting_balance(&BOB), Some(100)); // starts now, 10 per block
        assert_eq!(Vesting::vesting_balance(&CHARLIE), Some(1000)); // starts at 20
        assert_eq!(Vesting::vesting_balance(&DAVE), Some(300)); // starts vesting at block 5, 20 per block

        assert_eq!(
            Vesting::vesting(BOB).unwrap().to_vec(),
            vec![VestingInfo::new(100, 10, 10)]
        );
        assert_eq!(
            Vesting::vesting(CHARLIE).unwrap().to_vec(),
            vec![VestingInfo::new(1000, 100, 20)]
        );
        assert_eq!(
            Vesting::vesting(DAVE).unwrap().to_vec(),
            vec![VestingInfo::new(400, 20, 5)]
        );

        // onboard MBMs
        AllPalletsWithSystem::on_runtime_upgrade();
        run_to_block(11);

        assert_eq!(Vesting::vesting_balance(&ALICE), Some(0)); // Alice remains the same
        assert_eq!(Vesting::vesting_balance(&BOB), Some(95)); // 5 unlocked
        assert_eq!(Vesting::vesting_balance(&CHARLIE), Some(1000)); // starts at 30 now, doubled remaining blocks
        assert_eq!(Vesting::vesting_balance(&DAVE), Some(290)); // 10 unlocked

        assert_eq!(
            Vesting::vesting(BOB).unwrap().to_vec(),
            vec![VestingInfo::new(100, 5, 10)]
        );
        assert_eq!(
            Vesting::vesting(CHARLIE).unwrap().to_vec(),
            vec![VestingInfo::new(1000, 50, 30)]
        );
        assert_eq!(
            Vesting::vesting(DAVE).unwrap().to_vec(),
            vec![VestingInfo::new(300, 10, 10)]
        );

        // lock is updated when `PalletVesting::vest` is called
        assert_eq!(
            Balances::locks(&BOB).to_vec(),
            vec![BalanceLock {
                id: *b"vesting ",
                amount: 100,
                reasons: Reasons::Misc,
            }]
        );
        // call vest to unlock vested funds
        assert_ok!(Vesting::vest(RuntimeOrigin::signed(BOB)));
        assert_eq!(
            Balances::locks(&BOB).to_vec(),
            vec![BalanceLock {
                id: *b"vesting ",
                amount: 95,
                reasons: Reasons::Misc,
            }]
        );

        run_to_block(29);
        assert_eq!(Vesting::vesting_balance(&BOB), Some(5));
        // Bob will fully vest at 30
        run_to_block(30);
        assert_eq!(Vesting::vesting_balance(&BOB), Some(0));
        // call vest to unlock vested funds
        assert_ok!(Vesting::vest(RuntimeOrigin::signed(BOB)));
        assert_eq!(Balances::locks(&BOB).to_vec(), vec![]);

        run_to_block(39);
        assert_eq!(Vesting::vesting_balance(&CHARLIE), Some(550)); // started vesting at 30
        assert_eq!(Vesting::vesting_balance(&DAVE), Some(10));

        // Dave will fully vest at 40
        run_to_block(40);
        assert_eq!(Vesting::vesting_balance(&DAVE), Some(0));

        run_to_block(49);
        assert_eq!(Vesting::vesting_balance(&CHARLIE), Some(50));
        // Charlie will fully vest at 50, (50 per block starting at block 30)
        run_to_block(50);
        assert_eq!(Vesting::vesting_balance(&CHARLIE), Some(0));
    });
}
