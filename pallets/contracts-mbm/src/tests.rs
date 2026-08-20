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
    migrations_in_progress, new_test_ext, run_to_block, AllPalletsWithSystem, Balances, HoldReason,
    MaxServiceWeight, Runtime, ESCROW,
};
use frame_support::traits::{
    fungible::{Inspect, InspectHold, Mutate, MutateHold},
    OnRuntimeUpgrade,
};

fn fund_and_hold(who: u64, free: u64, reason: HoldReason, amount: u64) {
    Balances::set_balance(&who, free);
    Balances::hold(&reason, &who, amount).expect("can hold");
}

fn held(who: u64, reason: HoldReason) -> u64 {
    Balances::balance_on_hold(&reason, &who)
}

/// Onboards the multi block migrations the way a runtime upgrade does, then runs blocks until
/// `pallet-migrations` reports it is done. Returns how many blocks that took.
fn run_migration_to_completion() -> u64 {
    AllPalletsWithSystem::on_runtime_upgrade();

    let start = frame_system::Pallet::<Runtime>::block_number();
    let mut current = start;
    while migrations_in_progress() {
        current += 1;
        run_to_block(current);
        assert!(current - start < 1_000, "migration did not converge");
    }
    current - start
}

#[test]
fn both_contracts_hold_reasons_are_swept_to_the_escrow() {
    new_test_ext().execute_with(|| {
        // A real account holding a code upload deposit.
        fund_and_hold(1, 1_000, HoldReason::CodeUploadDepositReserve, 400);
        // A keyless contract account holding a storage deposit on top of its own free balance.
        fund_and_hold(2, 1_000, HoldReason::StorageDepositReserve, 700);

        run_migration_to_completion();

        assert_eq!(held(1, HoldReason::CodeUploadDepositReserve), 0);
        assert_eq!(held(2, HoldReason::StorageDepositReserve), 0);
        assert_eq!(Balances::balance(&ESCROW), 1_100);
        // Only the deposits move; the free balances stay where they are.
        assert_eq!(Balances::balance(&1), 600);
        assert_eq!(Balances::balance(&2), 300);
    });
}

#[test]
fn holds_of_other_pallets_are_left_alone() {
    new_test_ext().execute_with(|| {
        fund_and_hold(3, 1_000, HoldReason::Unrelated, 250);

        run_migration_to_completion();

        assert_eq!(held(3, HoldReason::Unrelated), 250);
        assert_eq!(Balances::balance(&ESCROW), 0);
    });
}

#[test]
fn settles_every_account_across_several_blocks() {
    new_test_ext().execute_with(|| {
        // A budget that only affords a couple of accounts per block, so the migration has to
        // suspend and resume - which round trips the account cursor through `pallet-migrations`.
        // Derived from the weights rather than hardcoded, so it survives a regeneration of
        // `weights.rs` on different hardware. `pallet-migrations` charges its own per block
        // overhead against the same meter and raises a defensive failure if what is left cannot
        // afford a single step, so that overhead has to be budgeted for explicitly.
        let per_account =
            <crate::weights::SubstrateWeight<Runtime> as crate::WeightInfo>::release_deposit();
        let migrations_overhead = <() as pallet_migrations::WeightInfo>::progress_mbms_none()
            .saturating_add(pallet_migrations::Pallet::<Runtime>::exec_migration_max_weight());
        MaxServiceWeight::set(migrations_overhead.saturating_add(per_account.saturating_mul(3)));

        for who in 10..40u64 {
            fund_and_hold(who, 1_000, HoldReason::StorageDepositReserve, 100);
        }

        let blocks = run_migration_to_completion();

        assert!(
            blocks > 1,
            "should have needed several blocks, took {blocks}"
        );
        for who in 10..40u64 {
            assert_eq!(
                held(who, HoldReason::StorageDepositReserve),
                0,
                "account {who}"
            );
        }
        assert_eq!(Balances::balance(&ESCROW), 30 * 100);
    });
}

#[test]
fn is_a_no_op_when_there_are_no_holds() {
    new_test_ext().execute_with(|| {
        run_migration_to_completion();

        assert_eq!(Balances::balance(&ESCROW), 0);
        assert!(!migrations_in_progress());
    });
}

#[cfg(feature = "try-runtime")]
#[test]
fn post_upgrade_catches_a_surviving_hold() {
    use crate::{
        mock::{CodeDeposit, EscrowAccount, StorageDeposit},
        ReleaseContractsDeposits,
    };
    use frame_support::migrations::SteppedMigration;

    type Release = ReleaseContractsDeposits<
        Runtime,
        CodeDeposit,
        StorageDeposit,
        EscrowAccount,
        crate::weights::SubstrateWeight<Runtime>,
    >;

    new_test_ext().execute_with(|| {
        fund_and_hold(1, 1_000, HoldReason::StorageDepositReserve, 400);
        assert!(Release::post_upgrade(Vec::new()).is_err());

        run_migration_to_completion();
        assert!(Release::post_upgrade(Vec::new()).is_ok());
    });
}
