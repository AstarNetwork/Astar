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
        #[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
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
    assert!(acc_ledger.active_locked_amount().is_zero());
    assert!(acc_ledger.lock_era().is_zero());
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
    assert!(acc_ledger.active_locked_amount().is_zero());
    assert!(acc_ledger.total_locked_amount().is_zero());
    assert!(acc_ledger.add_lock_amount(0, first_era).is_ok());
    assert!(acc_ledger.active_locked_amount().is_zero());

    // Adding lock value works as expected
    let init_amount = 20;
    assert!(acc_ledger.add_lock_amount(init_amount, first_era).is_ok());
    assert_eq!(acc_ledger.active_locked_amount(), init_amount);
    assert_eq!(acc_ledger.total_locked_amount(), init_amount);
    assert_eq!(acc_ledger.lock_era(), first_era);
    assert!(!acc_ledger.is_empty());
    assert_eq!(acc_ledger.locked.len(), 1);
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
    assert_eq!(acc_ledger.active_locked_amount(), init_amount + addition);
    assert_eq!(acc_ledger.total_locked_amount(), init_amount + addition);
    assert_eq!(acc_ledger.lock_era(), first_era);
    assert_eq!(acc_ledger.locked.len(), 1);

    // Add up to storage limit
    for i in 2..=LockedDummy::get() {
        assert!(acc_ledger.add_lock_amount(addition, first_era + i).is_ok());
        assert_eq!(
            acc_ledger.active_locked_amount(),
            init_amount + addition * i as u128
        );
        assert_eq!(acc_ledger.lock_era(), first_era + i);
        assert_eq!(acc_ledger.locked.len(), i as usize);
    }

    // Any further additions should fail due to exhausting bounded storage capacity
    let acc_ledger_clone = acc_ledger.clone();
    assert!(acc_ledger
        .add_lock_amount(addition, acc_ledger.lock_era() + 1)
        .is_err());
    assert_eq!(acc_ledger, acc_ledger_clone);
}

#[test]
fn account_ledger_subtract_lock_amount_basic_usage_works() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    // Sanity check scenario
    // Cannot reduce if there is nothing locked, should be a noop
    assert!(acc_ledger.subtract_lock_amount(0, 1).is_ok());
    assert!(acc_ledger.subtract_lock_amount(10, 1).is_ok());
    assert!(acc_ledger.locked.len().is_zero());
    assert!(acc_ledger.is_empty());

    // First basic scenario
    // Add some lock amount, then reduce it for the same era
    let first_era = 1;
    let first_lock_amount = 19;
    let unlock_amount = 7;
    assert!(acc_ledger
        .add_lock_amount(first_lock_amount, first_era)
        .is_ok());
    assert!(acc_ledger
        .subtract_lock_amount(unlock_amount, first_era)
        .is_ok());
    assert_eq!(acc_ledger.locked.len(), 1);
    assert_eq!(
        acc_ledger.total_locked_amount(),
        first_lock_amount - unlock_amount
    );
    assert_eq!(
        acc_ledger.active_locked_amount(),
        first_lock_amount - unlock_amount
    );
    assert_eq!(acc_ledger.unlocking_amount(), 0);

    // Second basic scenario
    // Reduce the lock from the era which isn't latest in the vector
    let first_lock_amount = first_lock_amount - unlock_amount;
    let second_lock_amount = 31;
    let second_era = 2;
    assert!(acc_ledger
        .add_lock_amount(second_lock_amount - first_lock_amount, second_era)
        .is_ok());
    assert_eq!(acc_ledger.active_locked_amount(), second_lock_amount);
    assert_eq!(acc_ledger.locked.len(), 2);

    // Subtract from the first era and verify state is as expected
    assert!(acc_ledger
        .subtract_lock_amount(unlock_amount, first_era)
        .is_ok());
    assert_eq!(acc_ledger.locked.len(), 2);
    assert_eq!(
        acc_ledger.active_locked_amount(),
        second_lock_amount - unlock_amount
    );
    assert_eq!(
        acc_ledger.locked[0].amount,
        first_lock_amount - unlock_amount
    );
    assert_eq!(
        acc_ledger.locked[1].amount,
        second_lock_amount - unlock_amount
    );

    // Third basic scenario
    // Reduce the the latest era, don't expect the first one to change
    assert!(acc_ledger
        .subtract_lock_amount(unlock_amount, second_era)
        .is_ok());
    assert_eq!(acc_ledger.locked.len(), 2);
    assert_eq!(
        acc_ledger.active_locked_amount(),
        second_lock_amount - unlock_amount * 2
    );
    assert_eq!(
        acc_ledger.locked[0].amount,
        first_lock_amount - unlock_amount
    );
    assert_eq!(
        acc_ledger.locked[1].amount,
        second_lock_amount - unlock_amount * 2
    );
}

#[test]
fn account_ledger_subtract_lock_amount_overflow_fails() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    let first_lock_amount = 17 * 19;
    let era = 1;
    let unlock_amount = 5;
    assert!(acc_ledger.add_lock_amount(first_lock_amount, era).is_ok());
    for idx in 1..=LockedDummy::get() {
        assert!(acc_ledger.subtract_lock_amount(unlock_amount, idx).is_ok());
        assert_eq!(acc_ledger.locked.len(), idx as usize);
        assert_eq!(
            acc_ledger.active_locked_amount(),
            first_lock_amount - unlock_amount * idx as u128
        );
    }

    // Updating existing lock should still work
    let locked_snapshot = acc_ledger.locked.clone();
    for i in 1..10 {
        assert!(acc_ledger
            .subtract_lock_amount(unlock_amount, LockedDummy::get())
            .is_ok());
        assert_eq!(acc_ledger.locked.len(), LockedDummy::get() as usize);

        let last_idx = LockedDummy::get() as usize - 1;
        assert_eq!(
            &acc_ledger.locked[0..last_idx],
            &locked_snapshot[0..last_idx]
        );
        assert_eq!(
            acc_ledger.locked[last_idx].amount as u128 + unlock_amount * i,
            locked_snapshot[last_idx].amount
        );
    }

    // Attempt to add additional chunks should fail, and is a noop.
    let acc_ledger_clone = acc_ledger.clone();
    assert!(acc_ledger
        .subtract_lock_amount(unlock_amount, LockedDummy::get() + 1)
        .is_err());
    assert_eq!(acc_ledger, acc_ledger_clone);
}

#[test]
fn account_ledger_subtract_lock_amount_advanced_example_works() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    // Prepare an example where we have two non-consecutive entries, and we unlock in the era right before the second entry.
    // This covers a scenario where user has called `lock` in the current era,
    // creating an entry for the next era, and then decides to immediately unlock a portion of the locked amount.
    let first_lock_amount = 17;
    let second_lock_amount = 23;
    let first_era = 1;
    let second_era = 5;
    let unlock_era = second_era - 1;
    let unlock_amount = 5;
    assert!(acc_ledger
        .add_lock_amount(first_lock_amount, first_era)
        .is_ok());
    assert!(acc_ledger
        .add_lock_amount(second_lock_amount, second_era)
        .is_ok());
    assert_eq!(acc_ledger.locked.len(), 2);

    assert!(acc_ledger
        .subtract_lock_amount(unlock_amount, unlock_era)
        .is_ok());
    assert_eq!(
        acc_ledger.active_locked_amount(),
        first_lock_amount + second_lock_amount - unlock_amount
    );

    // Check entries in more detail
    assert_eq!(acc_ledger.locked.len(), 3);
    assert_eq!(acc_ledger.locked[0].amount, first_lock_amount,);
    assert_eq!(
        acc_ledger.locked[2].amount,
        first_lock_amount + second_lock_amount - unlock_amount
    );
    // Verify the new entry is as expected
    assert_eq!(
        acc_ledger.locked[1].amount,
        first_lock_amount - unlock_amount
    );
    assert_eq!(acc_ledger.locked[1].era, unlock_era);
}

#[test]
fn account_ledger_subtract_lock_amount_with_only_one_locked_chunk() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    // Scenario: user locks for era 2 while era 1 is active, immediately followed by unlock call.
    // Locked amount should be updated for the next era, but active locked amount should be unchanged (zero).
    let lock_amount = 17;
    let unlock_amount = 5;
    let lock_era = 2;
    let unlock_era = 1;
    assert!(acc_ledger.add_lock_amount(lock_amount, lock_era).is_ok());
    assert!(acc_ledger
        .subtract_lock_amount(unlock_amount, unlock_era)
        .is_ok());

    assert_eq!(acc_ledger.locked.len(), 1);
    assert_eq!(
        acc_ledger.locked[0],
        LockedChunk {
            amount: lock_amount - unlock_amount,
            era: lock_era,
        }
    );
}

#[test]
fn account_ledger_subtract_lock_amount_correct_zero_cleanup() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    // Ensure that zero entries are cleaned up correctly when required.
    // There are a couple of distinct scenarios:
    //    1. There is only one entry, and it's zero. The vector should be cleared & empty.
    //    2. There are multiple entries, and the last one is zero. It's valid since it marks when someone fully unlocked.
    //    3. Zero entry can exist in between two non-zero entries (not covered in this UT).

    // 1st scenario (A) - only one zero entry, unlock is in the same era
    let lock_amount = 17;
    let lock_era = 2;
    assert!(acc_ledger.add_lock_amount(lock_amount, lock_era).is_ok());
    assert!(acc_ledger
        .subtract_lock_amount(lock_amount, lock_era)
        .is_ok());
    assert!(acc_ledger.locked.is_empty());

    // 1st scenario (B) - only one zero entry, unlock is in the previous era
    assert!(acc_ledger.add_lock_amount(lock_amount, lock_era).is_ok());
    assert!(acc_ledger
        .subtract_lock_amount(lock_amount, lock_era - 1)
        .is_ok());
    assert!(acc_ledger.locked.is_empty());

    // 2nd scenario - last entry is zero
    let first_lock_era = 3;
    let second_lock_era = 11;
    let unlock_era = second_lock_era + 2;
    assert!(acc_ledger
        .add_lock_amount(lock_amount, first_lock_era)
        .is_ok());
    assert!(acc_ledger
        .add_lock_amount(lock_amount, second_lock_era)
        .is_ok());
    // Following should add new entry, to mark when the user fully unlocked
    assert!(acc_ledger
        .subtract_lock_amount(acc_ledger.active_locked_amount(), unlock_era)
        .is_ok());
    assert_eq!(acc_ledger.locked.len(), 3);
    assert!(acc_ledger.active_locked_amount().is_zero());
}

#[test]
fn account_ledger_subtract_lock_amount_zero_entry_between_two_non_zero() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    let (first_lock_amount, second_lock_amount, third_lock_amount) = (17, 23, 29);
    let (first_lock_era, second_lock_era, third_lock_era) = (1, 3, 7);

    // Prepare scenario with 3 locked chunks
    assert!(acc_ledger
        .add_lock_amount(first_lock_amount, first_lock_era)
        .is_ok());
    assert!(acc_ledger
        .add_lock_amount(second_lock_amount, second_lock_era)
        .is_ok());
    assert!(acc_ledger
        .add_lock_amount(third_lock_amount, third_lock_era)
        .is_ok());
    assert_eq!(acc_ledger.locked.len(), 3);

    // Unlock everything for the era right before the latest chunk era
    // This should result in scenario like:
    // [17, 17 + 23, 0, 29]
    assert!(acc_ledger
        .subtract_lock_amount(first_lock_amount + second_lock_amount, third_lock_era - 1)
        .is_ok());
    assert_eq!(acc_ledger.locked.len(), 4);
    assert_eq!(acc_ledger.active_locked_amount(), third_lock_amount);
    assert_eq!(
        acc_ledger.locked[0],
        LockedChunk {
            amount: first_lock_amount,
            era: first_lock_era
        }
    );
    assert_eq!(
        acc_ledger.locked[1],
        LockedChunk {
            amount: first_lock_amount + second_lock_amount,
            era: second_lock_era
        }
    );
    assert_eq!(
        acc_ledger.locked[2],
        LockedChunk {
            amount: 0,
            era: third_lock_era - 1
        }
    );
    assert_eq!(
        acc_ledger.locked[3],
        LockedChunk {
            amount: third_lock_amount,
            era: third_lock_era
        }
    );
}

#[test]
fn account_ledger_subtract_lock_amount_consecutive_zeroes_merged() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    // Prepare scenario with 3 locked chunks, where the middle one is zero
    let lock_amount = 61;
    let last_era = 11;
    assert!(acc_ledger.add_lock_amount(lock_amount, 2).is_ok());
    assert!(acc_ledger.subtract_lock_amount(lock_amount, 5).is_ok());
    assert!(acc_ledger.add_lock_amount(lock_amount, last_era).is_ok());
    let second_chunk = acc_ledger.locked[1];

    // Unlock everything in the era right before the latest chunk era, but that chunk should not persist
    // [61, 0, 61] --> [61, 0, 0, 61] shouldn't happen since the 2nd zero is redundant.
    assert!(acc_ledger
        .subtract_lock_amount(lock_amount, last_era - 1)
        .is_ok());
    assert_eq!(acc_ledger.locked.len(), 3);
    assert_eq!(acc_ledger.locked[1], second_chunk);
}

#[test]
fn account_ledger_add_unlocking_chunk_works() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    // Sanity check scenario
    // Cannot reduce if there is nothing locked, should be a noop
    assert!(acc_ledger.add_unlocking_chunk(0, 0).is_ok());
    assert!(acc_ledger.unlocking.len().is_zero());
    assert!(acc_ledger.is_empty());

    // Basic scenario
    let unlock_amount = 17;
    let block_number = 29;
    assert!(acc_ledger
        .add_unlocking_chunk(unlock_amount, block_number)
        .is_ok());
    assert_eq!(
        acc_ledger.unlocking,
        vec![UnlockingChunk {
            amount: unlock_amount,
            unlock_block: block_number
        }]
    );
    assert_eq!(acc_ledger.unlocking_amount(), unlock_amount);

    // Unlock additional amount in the same block
    assert!(acc_ledger
        .add_unlocking_chunk(unlock_amount, block_number)
        .is_ok());
    assert_eq!(
        acc_ledger.unlocking,
        vec![UnlockingChunk {
            amount: unlock_amount * 2,
            unlock_block: block_number
        }]
    );
    assert_eq!(acc_ledger.unlocking_amount(), unlock_amount * 2);

    // Add unlocking chunks up to vector capacity
    let mut total_unlocking = acc_ledger.unlocking_amount();
    for i in 2..=UnlockingDummy::get() {
        let new_unlock_amount = unlock_amount + i as u128;
        assert!(acc_ledger
            .add_unlocking_chunk(new_unlock_amount, block_number + i as u64)
            .is_ok());
        total_unlocking += new_unlock_amount;
        assert_eq!(acc_ledger.unlocking_amount(), total_unlocking);
        assert_eq!(
            acc_ledger.unlocking[i as usize - 1].amount,
            new_unlock_amount
        );
    }

    // Any further addition should fail, resulting in a noop
    let acc_ledger_snapshot = acc_ledger.clone();
    assert!(acc_ledger
        .add_unlocking_chunk(1, block_number + UnlockingDummy::get() as u64 + 1)
        .is_err());
    assert_eq!(acc_ledger, acc_ledger_snapshot);
}

#[test]
fn active_stake_works() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    // Sanity check
    assert!(acc_ledger.active_stake(0).is_zero());
    assert!(acc_ledger.active_stake(1).is_zero());

    // Period matches
    let amount = 29;
    let period = 5;
    acc_ledger.staked = StakeInfo { amount, period };
    assert_eq!(acc_ledger.active_stake(period), amount);

    // Period doesn't match
    assert!(acc_ledger.active_stake(period - 1).is_zero());
    assert!(acc_ledger.active_stake(period + 1).is_zero());
}

#[test]
fn unlockable_amount_works() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    // Sanity check scenario
    assert!(acc_ledger.unlockable_amount(0).is_zero());

    // Nothing is staked
    let lock_amount = 29;
    let lock_era = 3;
    assert!(acc_ledger.add_lock_amount(lock_amount, lock_era).is_ok());
    assert_eq!(acc_ledger.unlockable_amount(0), lock_amount);

    // Some amount is staked, period matches
    let stake_period = 5;
    let stake_amount = 17;
    acc_ledger.staked = StakeInfo {
        amount: stake_amount,
        period: stake_period,
    };
    assert_eq!(
        acc_ledger.unlockable_amount(stake_period),
        lock_amount - stake_amount
    );

    // Period doesn't match
    assert_eq!(acc_ledger.unlockable_amount(stake_period - 1), lock_amount);
    assert_eq!(acc_ledger.unlockable_amount(stake_period + 2), lock_amount);

    // Absurd example, for the sake of completeness - staked without any lock
    acc_ledger.locked = Default::default();
    assert!(acc_ledger.unlockable_amount(stake_period).is_zero());
    assert!(acc_ledger.unlockable_amount(stake_period - 2).is_zero());
    assert!(acc_ledger.unlockable_amount(stake_period + 1).is_zero());
}

#[test]
fn era_info_manipulation_works() {
    let mut era_info = EraInfo::<Balance>::default();

    // Sanity check
    assert!(era_info.total_locked.is_zero());
    assert!(era_info.active_era_locked.is_zero());
    assert!(era_info.unlocking.is_zero());

    // Basic add lock
    let lock_amount = 7;
    era_info.add_locked(lock_amount);
    assert_eq!(era_info.total_locked, lock_amount);
    era_info.add_locked(lock_amount);
    assert_eq!(era_info.total_locked, lock_amount * 2);

    // Basic unlocking started
    let unlock_amount = 2;
    era_info.total_locked = 17;
    era_info.active_era_locked = 13;
    let era_info_snapshot = era_info;

    // First unlock & checks
    era_info.unlocking_started(unlock_amount);
    assert_eq!(
        era_info.total_locked,
        era_info_snapshot.total_locked - unlock_amount
    );
    assert_eq!(
        era_info.active_era_locked,
        era_info_snapshot.active_era_locked - unlock_amount
    );
    assert_eq!(era_info.unlocking, unlock_amount);

    // Second unlock and checks
    era_info.unlocking_started(unlock_amount);
    assert_eq!(
        era_info.total_locked,
        era_info_snapshot.total_locked - unlock_amount * 2
    );
    assert_eq!(
        era_info.active_era_locked,
        era_info_snapshot.active_era_locked - unlock_amount * 2
    );
    assert_eq!(era_info.unlocking, unlock_amount * 2);
}
