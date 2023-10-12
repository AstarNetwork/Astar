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

use frame_support::assert_ok;

use crate::test::mock::{Balance, *};
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

#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, Default)]
struct DummyEraAmount {
    amount: Balance,
    era: u32,
}
impl AmountEraPair for DummyEraAmount {
    fn get_amount(&self) -> Balance {
        self.amount
    }
    fn get_era(&self) -> u32 {
        self.era
    }
    fn set_era(&mut self, era: u32) {
        self.era = era;
    }
    fn saturating_accrue(&mut self, increase: Balance) {
        self.amount.saturating_accrue(increase);
    }
    fn saturating_reduce(&mut self, reduction: Balance) {
        self.amount.saturating_reduce(reduction);
    }
}
impl DummyEraAmount {
    pub fn new(amount: Balance, era: u32) -> Self {
        Self { amount, era }
    }
}

#[test]
fn sparse_bounded_amount_era_vec_add_amount_works() {
    get_u32_type!(MaxLen, 5);

    // Sanity check
    let mut vec = SparseBoundedAmountEraVec::<DummyEraAmount, MaxLen>::new();
    assert!(vec.0.is_empty());
    assert_ok!(vec.add_amount(0, 0));
    assert!(vec.0.is_empty());

    // 1st scenario - add to empty vector, should create one entry
    let init_amount = 19;
    let first_era = 3;
    assert_ok!(vec.add_amount(init_amount, first_era));
    assert_eq!(vec.0.len(), 1);
    assert_eq!(vec.0[0], DummyEraAmount::new(init_amount, first_era));

    // 2nd scenario - add to the same era, should update the entry
    assert_ok!(vec.add_amount(init_amount, first_era));
    assert_eq!(vec.0.len(), 1);
    assert_eq!(vec.0[0], DummyEraAmount::new(init_amount * 2, first_era));

    // 3rd scenario - add to the next era, should create a new entry
    let second_era = first_era + 1;
    assert_ok!(vec.add_amount(init_amount, second_era));
    assert_eq!(vec.0.len(), 2);
    assert_eq!(vec.0[0], DummyEraAmount::new(init_amount * 2, first_era));
    assert_eq!(vec.0[1], DummyEraAmount::new(init_amount * 3, second_era));

    // 4th scenario - add to the previous era, should fail and be a noop
    assert_eq!(
        vec.add_amount(init_amount, first_era),
        Err(AccountLedgerError::OldEra)
    );
    assert_eq!(vec.0.len(), 2);
    assert_eq!(vec.0[0], DummyEraAmount::new(init_amount * 2, first_era));
    assert_eq!(vec.0[1], DummyEraAmount::new(init_amount * 3, second_era));

    // 5th scenario - exceed capacity, should fail
    for i in vec.0.len()..MaxLen::get() as usize {
        assert_ok!(vec.add_amount(init_amount, second_era + i as u32));
    }
    assert_eq!(
        vec.add_amount(init_amount, 100),
        Err(AccountLedgerError::NoCapacity)
    );
}

// Test two scenarios:
//
// 1. [amount, era] -> subtract(x, era) -> [amount - x, era]
// 2. [amount, era] -> subtract (amount * 2, era) -> []
#[test]
fn sparse_bounded_amount_era_vec_subtract_amount_basic_scenario_works() {
    get_u32_type!(MaxLen, 5);

    // Sanity check
    let mut vec = SparseBoundedAmountEraVec::<DummyEraAmount, MaxLen>::new();
    assert_ok!(vec.subtract_amount(0, 0));
    assert!(vec.0.is_empty());

    // 1st scenario - only one entry exists, and it's the same era as the unlock
    let init_amount = 19;
    let first_era = 1;
    let sub_amount = 3;
    assert_ok!(vec.add_amount(init_amount, first_era));
    assert_ok!(vec.subtract_amount(sub_amount, first_era));
    assert_eq!(vec.0.len(), 1);
    assert_eq!(
        vec.0[0],
        DummyEraAmount::new(init_amount - sub_amount, first_era),
        "Only single entry and it should be updated."
    );

    // 2nd scenario - subtract everything (and more - underflow!) from the current era, causing full removal. Should cleanup the vector.
    assert_ok!(vec.subtract_amount(init_amount * 2, first_era));
    assert!(vec.0.is_empty(), "Full removal should cleanup the vector.");
}

#[test]
fn sparse_bounded_amount_era_vec_subtract_amount_advanced_consecutive_works() {
    get_u32_type!(MaxLen, 5);
    let mut vec = SparseBoundedAmountEraVec::<DummyEraAmount, MaxLen>::new();

    // 1st scenario - two entries, consecutive eras, subtract from the second era.
    // Only the second entry should be updated.
    let (first_era, second_era) = (1, 2);
    let (first_amount, second_amount) = (19, 23);
    assert_ok!(vec.add_amount(first_amount, first_era));
    assert_ok!(vec.add_amount(second_amount, second_era));

    let sub_amount = 3;
    assert_ok!(vec.subtract_amount(sub_amount, second_era));
    assert_eq!(vec.0.len(), 2);
    assert_eq!(
        vec.0[0],
        DummyEraAmount::new(first_amount, first_era),
        "First entry should remain unchanged."
    );
    assert_eq!(
        vec.0[1],
        DummyEraAmount::new(first_amount + second_amount - sub_amount, second_era),
        "Second entry should have it's amount reduced by the subtracted amount."
    );

    // 2nd scenario - two entries, consecutive eras, subtract from the first era.
    // Both the first and second entry should be updated.
    assert_ok!(vec.subtract_amount(sub_amount, first_era));
    assert_eq!(vec.0.len(), 2);
    assert_eq!(
        vec.0[0],
        DummyEraAmount::new(first_amount - sub_amount, first_era),
        "First entry is updated since it was specified."
    );
    assert_eq!(
        vec.0[1],
        DummyEraAmount::new(first_amount + second_amount - sub_amount * 2, second_era),
        "Second entry is updated because it comes AFTER the first one - same applies to all future entries."
    );

    // 3rd scenario - three entries, consecutive eras, subtract from the second era.
    // Only second and third entry should be updated. First one should remain unchanged.
    let third_era = 3;
    let third_amount = 29;
    assert_ok!(vec.add_amount(third_amount, third_era));
    assert_ok!(vec.subtract_amount(sub_amount, second_era));
    assert_eq!(vec.0.len(), 3);
    assert_eq!(
        vec.0[0],
        DummyEraAmount::new(first_amount - sub_amount, first_era),
        "First entry should remain unchanged, compared to previous scenario."
    );
    assert_eq!(
        vec.0[1],
        DummyEraAmount::new(first_amount + second_amount - sub_amount * 3, second_era),
        "Second entry should be reduced by the subtracted amount, compared to previous scenario."
    );
    assert_eq!(
        vec.0[2],
        DummyEraAmount::new(
            first_amount + second_amount + third_amount - sub_amount * 3,
            third_era
        ),
        "Same as for the second entry."
    );
}

#[test]
fn sparse_bounded_amount_era_vec_subtract_amount_advanced_non_consecutive_works() {
    get_u32_type!(MaxLen, 5);
    let mut vec = SparseBoundedAmountEraVec::<DummyEraAmount, MaxLen>::new();

    // 1st scenario - two entries, non-consecutive eras, subtract from the mid era.
    // Only the second entry should be updated but a new entry should be created.
    let (first_era, second_era) = (1, 5);
    let (first_amount, second_amount) = (19, 23);
    assert_ok!(vec.add_amount(first_amount, first_era));
    assert_ok!(vec.add_amount(second_amount, second_era));

    let sub_amount = 3;
    let mid_era = second_era - 1;
    assert_ok!(vec.subtract_amount(sub_amount, mid_era));
    assert_eq!(vec.0.len(), 3);
    assert_eq!(
        vec.0[0],
        DummyEraAmount::new(first_amount, first_era),
        "No impact on the first entry expected."
    );
    assert_eq!(
        vec.0[1],
        DummyEraAmount::new(first_amount - sub_amount, mid_era),
        "Newly created entry should be equal to the first amount, minus what was subtracted."
    );
    assert_eq!(
        vec.0[2],
        DummyEraAmount::new(vec.0[1].amount + second_amount, second_era),
        "Previous 'second' entry should be total added minus the subtracted amount."
    );

    // 2nd scenario - fully unlock the mid-entry to create a zero entry.
    assert_ok!(vec.subtract_amount(vec.0[1].amount, mid_era));
    assert_eq!(vec.0.len(), 3);
    assert_eq!(
        vec.0[0],
        DummyEraAmount::new(first_amount, first_era),
        "No impact on the first entry expected."
    );
    assert_eq!(
        vec.0[1],
        DummyEraAmount::new(0, mid_era),
        "Zero entry should be kept since it's in between two non-zero entries."
    );
    assert_eq!(
        vec.0[2],
        DummyEraAmount::new(second_amount, second_era),
        "Only the second staked amount should remain since everything else was unstaked."
    );

    // 3rd scenario - create an additional non-zero chunk as prep for the next scenario.
    let pre_mid_era = mid_era - 1;
    assert!(pre_mid_era > first_era, "Sanity check.");
    assert_ok!(vec.subtract_amount(sub_amount, pre_mid_era));
    assert_eq!(vec.0.len(), 4);
    assert_eq!(
        vec.0[1],
        DummyEraAmount::new(first_amount - sub_amount, pre_mid_era),
        "Newly created entry, derives it's initial value from the first entry."
    );
    assert_eq!(
        vec.0[2],
        DummyEraAmount::new(0, mid_era),
        "Zero entry should be kept at this point since it's still between two non-zero entries."
    );
    assert_eq!(
        vec.0[3],
        DummyEraAmount::new(second_amount - sub_amount, second_era),
        "Last entry should be further reduced by the newly subtracted amount."
    );

    // 4th scenario - create an additional zero entry, but ensure it's cleaned up correctly.
    let final_sub_amount = vec.0[1].amount;
    assert_ok!(vec.subtract_amount(final_sub_amount, pre_mid_era));
    assert_eq!(vec.0.len(), 3);
    assert_eq!(
        vec.0[0],
        DummyEraAmount::new(first_amount, first_era),
        "First entry should still remain unchanged."
    );
    assert_eq!(
        vec.0[1],
        DummyEraAmount::new(0, pre_mid_era),
        "The older zero entry should consume the newer ones, hence the pre_mid_era usage"
    );
    assert_eq!(
        vec.0[2],
        DummyEraAmount::new(second_amount - sub_amount - final_sub_amount, second_era),
        "Last entry should be further reduced by the newly subtracted amount."
    );
}

#[test]
fn sparse_bounded_amount_era_vec_full_subtract_with_single_future_era() {
    get_u32_type!(MaxLen, 5);
    let mut vec = SparseBoundedAmountEraVec::<DummyEraAmount, MaxLen>::new();

    // A scenario where some amount is added, for the first time, for era X.
    // Immediately afterward, the same amount is subtracted from era X - 1.
    let (era_1, era_2) = (1, 2);
    let amount = 19;
    assert_ok!(vec.add_amount(amount, era_2));

    assert_ok!(vec.subtract_amount(amount, era_1));
    assert!(
        vec.0.is_empty(),
        "Future entry should have been cleaned up."
    );
}

#[test]
fn period_type_sanity_check() {
    assert_eq!(PeriodType::Voting.next(), PeriodType::BuildAndEarn);
    assert_eq!(PeriodType::BuildAndEarn.next(), PeriodType::Voting);
}

#[test]
fn period_info_basic_checks() {
    let period_number = 2;
    let ending_era = 5;
    let info = PeriodInfo::new(period_number, PeriodType::Voting, ending_era);

    // Sanity checks
    assert_eq!(info.number, period_number);
    assert_eq!(info.period_type, PeriodType::Voting);
    assert_eq!(info.ending_era, ending_era);

    // Voting period checks
    assert!(!info.is_next_period(ending_era - 1));
    assert!(!info.is_next_period(ending_era));
    assert!(!info.is_next_period(ending_era + 1));
    for era in vec![ending_era - 1, ending_era, ending_era + 1] {
        assert!(
            !info.is_next_period(era),
            "Cannot trigger 'true' in the Voting period type."
        );
    }

    // Build&Earn period checks
    let info = PeriodInfo::new(period_number, PeriodType::BuildAndEarn, ending_era);
    assert!(!info.is_next_period(ending_era - 1));
    assert!(info.is_next_period(ending_era));
    assert!(info.is_next_period(ending_era + 1));
}

#[test]
fn protocol_state_default() {
    let protocol_state = ProtocolState::<BlockNumber>::default();

    assert_eq!(protocol_state.era, 0);
    assert_eq!(
        protocol_state.next_era_start, 1,
        "Era should start immediately on the first block"
    );
}

#[test]
fn protocol_state_basic_checks() {
    let mut protocol_state = ProtocolState::<BlockNumber>::default();
    let period_number = 5;
    let ending_era = 11;
    let next_era_start = 31;
    protocol_state.period_info = PeriodInfo::new(period_number, PeriodType::Voting, ending_era);
    protocol_state.next_era_start = next_era_start;

    assert_eq!(protocol_state.period_number(), period_number);
    assert_eq!(protocol_state.period_type(), PeriodType::Voting);

    // New era check
    assert!(!protocol_state.is_new_era(next_era_start - 1));
    assert!(protocol_state.is_new_era(next_era_start));
    assert!(protocol_state.is_new_era(next_era_start + 1));

    // Toggle new period type check - 'Voting' to 'BuildAndEarn'
    let ending_era_1 = 23;
    let next_era_start_1 = 41;
    protocol_state.next_period_type(ending_era_1, next_era_start_1);
    assert_eq!(protocol_state.period_type(), PeriodType::BuildAndEarn);
    assert_eq!(
        protocol_state.period_number(),
        period_number,
        "Switching from 'Voting' to 'BuildAndEarn' should not trigger period bump."
    );
    assert_eq!(protocol_state.ending_era(), ending_era_1);
    assert!(!protocol_state.is_new_era(next_era_start_1 - 1));
    assert!(protocol_state.is_new_era(next_era_start_1));

    // Toggle from 'BuildAndEarn' over to 'Voting'
    let ending_era_2 = 24;
    let next_era_start_2 = 91;
    protocol_state.next_period_type(ending_era_2, next_era_start_2);
    assert_eq!(protocol_state.period_type(), PeriodType::Voting);
    assert_eq!(
        protocol_state.period_number(),
        period_number + 1,
        "Switching from 'BuildAndEarn' to 'Voting' must trigger period bump."
    );
    assert_eq!(protocol_state.ending_era(), ending_era_2);
    assert!(protocol_state.is_new_era(next_era_start_2));
}

#[test]
fn account_ledger_default() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    assert!(acc_ledger.is_empty());
    assert!(acc_ledger.active_locked_amount().is_zero());
}

#[test]
fn account_ledger_add_lock_amount_works() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // First step, sanity checks
    assert!(acc_ledger.active_locked_amount().is_zero());
    assert!(acc_ledger.total_locked_amount().is_zero());
    acc_ledger.add_lock_amount(0);
    assert!(acc_ledger.active_locked_amount().is_zero());

    // Adding lock value works as expected
    let init_amount = 20;
    acc_ledger.add_lock_amount(init_amount);
    assert_eq!(acc_ledger.active_locked_amount(), init_amount);
    assert_eq!(acc_ledger.total_locked_amount(), init_amount);
    assert!(!acc_ledger.is_empty());
}

#[test]
fn account_ledger_subtract_lock_amount_basic_usage_works() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Sanity check scenario
    // Cannot reduce if there is nothing locked, should be a noop
    acc_ledger.subtract_lock_amount(0);
    acc_ledger.subtract_lock_amount(10);
    assert!(acc_ledger.is_empty());

    // First basic scenario
    // Add some lock amount, then reduce it
    let first_lock_amount = 19;
    let unlock_amount = 7;
    acc_ledger.add_lock_amount(first_lock_amount);
    acc_ledger.subtract_lock_amount(unlock_amount);
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
    let first_lock_amount = first_lock_amount - unlock_amount;
    let second_lock_amount = 31;
    acc_ledger.add_lock_amount(second_lock_amount - first_lock_amount);
    assert_eq!(acc_ledger.active_locked_amount(), second_lock_amount);

    // Subtract from the first era and verify state is as expected
    acc_ledger.subtract_lock_amount(unlock_amount);
    assert_eq!(
        acc_ledger.active_locked_amount(),
        second_lock_amount - unlock_amount
    );
}

#[test]
fn account_ledger_add_unlocking_chunk_works() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

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
    assert_eq!(
        acc_ledger.add_unlocking_chunk(1, block_number + UnlockingDummy::get() as u64 + 1),
        Err(AccountLedgerError::NoCapacity)
    );
    assert_eq!(acc_ledger, acc_ledger_snapshot);
}

#[test]
fn account_ledger_active_stake_works() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Sanity check
    assert!(acc_ledger.active_stake(0).is_zero());
    assert!(acc_ledger.active_stake(1).is_zero());

    // Period matches
    let amount = 29;
    let period = 5;
    acc_ledger.staked = SparseBoundedAmountEraVec(
        BoundedVec::try_from(vec![StakeChunk { amount, era: 1 }])
            .expect("Only one chunk so creation should succeed."),
    );
    acc_ledger.staked_period = Some(period);
    assert_eq!(acc_ledger.active_stake(period), amount);

    // Period doesn't match
    assert!(acc_ledger.active_stake(period - 1).is_zero());
    assert!(acc_ledger.active_stake(period + 1).is_zero());
}

#[test]
fn account_ledger_stakeable_amount_works() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Sanity check for empty ledger
    assert!(acc_ledger.stakeable_amount(1).is_zero());

    // First scenario - some locked amount, no staking chunks
    let first_period = 1;
    let locked_amount = 19;
    acc_ledger.add_lock_amount(locked_amount);
    assert_eq!(
        acc_ledger.stakeable_amount(first_period),
        locked_amount,
        "Stakeable amount has to be equal to the locked amount"
    );

    // Second scenario - some staked amount is introduced, period is still valid
    let first_era = 1;
    let staked_amount = 7;
    acc_ledger.staked = SparseBoundedAmountEraVec(
        BoundedVec::try_from(vec![StakeChunk {
            amount: staked_amount,
            era: first_era,
        }])
        .expect("Only one chunk so creation should succeed."),
    );
    acc_ledger.staked_period = Some(first_period);

    assert_eq!(
        acc_ledger.stakeable_amount(first_period),
        locked_amount - staked_amount,
        "Total stakeable amount should be equal to the locked amount minus what is already staked."
    );

    // Third scenario - continuation of the previous, but we move to the next period.
    assert_eq!(
        acc_ledger.stakeable_amount(first_period + 1),
        locked_amount,
        "Stakeable amount has to be equal to the locked amount since old period staking isn't valid anymore"
    );
}

#[test]
fn account_ledger_staked_amount_works() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Sanity check for empty ledger
    assert!(acc_ledger.staked_amount(1).is_zero());

    // First scenario - active period matches the ledger
    let first_era = 1;
    let first_period = 1;
    let locked_amount = 19;
    let staked_amount = 13;
    acc_ledger.add_lock_amount(locked_amount);
    acc_ledger.staked = SparseBoundedAmountEraVec(
        BoundedVec::try_from(vec![StakeChunk {
            amount: staked_amount,
            era: first_era,
        }])
        .expect("Only one chunk so creation should succeed."),
    );
    acc_ledger.staked_period = Some(first_period);

    assert_eq!(acc_ledger.staked_amount(first_period), staked_amount);

    // Second scenario - active period doesn't match the ledger
    assert!(acc_ledger.staked_amount(first_period + 1).is_zero());
}

#[test]
fn account_ledger_add_stake_amount_works() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Sanity check
    assert!(acc_ledger.add_stake_amount(0, 0, 0).is_ok());
    assert!(acc_ledger.staked_period.is_none());
    assert!(acc_ledger.staked.0.is_empty());

    // First scenario - stake some amount, and ensure values are as expected
    let first_era = 2;
    let first_period = 1;
    let lock_amount = 17;
    let stake_amount = 11;
    acc_ledger.add_lock_amount(lock_amount);

    assert!(acc_ledger
        .add_stake_amount(stake_amount, first_era, first_period)
        .is_ok());
    assert_eq!(acc_ledger.staked_period, Some(first_period));
    assert_eq!(acc_ledger.staked.0.len(), 1);
    assert_eq!(
        acc_ledger.staked.0[0],
        StakeChunk {
            amount: stake_amount,
            era: first_era,
        }
    );
    assert_eq!(acc_ledger.staked_amount(first_period), stake_amount);

    // Second scenario - stake some more to the same era, only amount should change
    assert!(acc_ledger
        .add_stake_amount(1, first_era, first_period)
        .is_ok());
    assert_eq!(acc_ledger.staked.0.len(), 1);
    assert_eq!(acc_ledger.staked_amount(first_period), stake_amount + 1);

    // Third scenario - stake to the next era, new chunk should be added
    let next_era = first_era + 3;
    let remaining_not_staked = lock_amount - stake_amount - 1;
    assert!(acc_ledger
        .add_stake_amount(remaining_not_staked, next_era, first_period)
        .is_ok());
    assert_eq!(acc_ledger.staked.0.len(), 2);
    assert_eq!(acc_ledger.staked_amount(first_period), lock_amount);
}

#[test]
fn account_ledger_add_stake_amount_invalid_era_fails() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Prep actions
    let first_era = 5;
    let first_period = 2;
    let lock_amount = 13;
    let stake_amount = 7;
    acc_ledger.add_lock_amount(lock_amount);
    assert!(acc_ledger
        .add_stake_amount(stake_amount, first_era, first_period)
        .is_ok());
    let acc_ledger_snapshot = acc_ledger.clone();

    // Try to add to the next era, it should fail
    assert_eq!(
        acc_ledger.add_stake_amount(1, first_era, first_period + 1),
        Err(AccountLedgerError::InvalidPeriod)
    );
    assert_eq!(
        acc_ledger, acc_ledger_snapshot,
        "Previous failed action must be a noop"
    );

    // Try to add to the previous era, it should fail
    assert_eq!(
        acc_ledger.add_stake_amount(1, first_era, first_period - 1),
        Err(AccountLedgerError::InvalidPeriod)
    );
    assert_eq!(
        acc_ledger, acc_ledger_snapshot,
        "Previous failed action must be a noop"
    );
}

#[test]
fn account_ledger_add_stake_amount_too_large_amount_fails() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Sanity check
    assert_eq!(
        acc_ledger.add_stake_amount(10, 1, 1),
        Err(AccountLedgerError::UnavailableStakeFunds)
    );

    // Lock some amount, and try to stake more than that
    let first_era = 5;
    let first_period = 2;
    let lock_amount = 13;
    acc_ledger.add_lock_amount(lock_amount);
    assert_eq!(
        acc_ledger.add_stake_amount(lock_amount + 1, first_era, first_period),
        Err(AccountLedgerError::UnavailableStakeFunds)
    );

    // Additional check - have some active stake, and then try to overstake
    assert!(acc_ledger
        .add_stake_amount(lock_amount - 2, first_era, first_period)
        .is_ok());
    assert_eq!(
        acc_ledger.add_stake_amount(3, first_era, first_period),
        Err(AccountLedgerError::UnavailableStakeFunds)
    );
}

#[test]
fn account_ledger_add_stake_amount_while_exceeding_capacity_fails() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Try to stake up to the capacity, it should work
    // Lock some amount, and try to stake more than that
    let first_era = 5;
    let first_period = 2;
    let lock_amount = 31;
    let stake_amount = 3;
    acc_ledger.add_lock_amount(lock_amount);
    for inc in 0..StakingDummy::get() {
        assert!(acc_ledger
            .add_stake_amount(stake_amount, first_era + inc, first_period)
            .is_ok());
        assert_eq!(
            acc_ledger.staked_amount(first_period),
            stake_amount * (inc as u128 + 1)
        );
    }

    // Can still stake to the last staked era
    assert!(acc_ledger
        .add_stake_amount(
            stake_amount,
            first_era + StakingDummy::get() - 1,
            first_period
        )
        .is_ok());

    // But staking to the next era must fail with exceeded capacity
    assert_eq!(
        acc_ledger.add_stake_amount(stake_amount, first_era + StakingDummy::get(), first_period),
        Err(AccountLedgerError::NoCapacity)
    );
}

#[test]
fn account_ledger_unstake_amount_works() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Prep actions
    let amount_1 = 19;
    let era_1 = 2;
    let period_1 = 1;
    acc_ledger.add_lock_amount(amount_1);
    assert!(acc_ledger
        .add_stake_amount(amount_1, era_1, period_1)
        .is_ok());

    // Sanity check
    assert!(acc_ledger.unstake_amount(0, era_1, period_1).is_ok());

    // 1st scenario - unstake some amount from the current era.
    let unstake_amount_1 = 3;
    assert!(acc_ledger
        .unstake_amount(unstake_amount_1, era_1, period_1)
        .is_ok());
    assert_eq!(
        acc_ledger.staked_amount(period_1),
        amount_1 - unstake_amount_1
    );
    assert_eq!(
        acc_ledger.staked.0.len(),
        1,
        "Only existing entry should be updated."
    );

    // 2nd scenario - unstake some more, but from the next era
    let era_2 = era_1 + 1;
    assert!(acc_ledger
        .unstake_amount(unstake_amount_1, era_2, period_1)
        .is_ok());
    assert_eq!(
        acc_ledger.staked_amount(period_1),
        amount_1 - unstake_amount_1 * 2
    );
    assert_eq!(
        acc_ledger.staked.0.len(),
        2,
        "New entry must be created to cover the new era stake."
    );

    // 3rd scenario - unstake some more, bump era by a larger number
    let era_3 = era_2 + 3;
    assert!(acc_ledger
        .unstake_amount(unstake_amount_1, era_3, period_1)
        .is_ok());
    assert_eq!(
        acc_ledger.staked_amount(period_1),
        amount_1 - unstake_amount_1 * 3
    );
    assert_eq!(
        acc_ledger.staked.0.len(),
        3,
        "New entry must be created to cover the new era stake."
    );
}

#[test]
fn account_ledger_unstake_from_invalid_era_fails() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Prep actions
    let amount_1 = 13;
    let era_1 = 2;
    let period_1 = 1;
    acc_ledger.add_lock_amount(amount_1);
    assert!(acc_ledger
        .add_stake_amount(amount_1, era_1, period_1)
        .is_ok());

    assert_eq!(
        acc_ledger.unstake_amount(amount_1, era_1 + 1, period_1 + 1),
        Err(AccountLedgerError::InvalidPeriod)
    );
}

#[test]
fn account_ledger_unstake_too_much_fails() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Prep actions
    let amount_1 = 23;
    let era_1 = 2;
    let period_1 = 1;
    acc_ledger.add_lock_amount(amount_1);
    assert!(acc_ledger
        .add_stake_amount(amount_1, era_1, period_1)
        .is_ok());

    assert_eq!(
        acc_ledger.unstake_amount(amount_1 + 1, era_1, period_1),
        Err(AccountLedgerError::UnstakeAmountLargerThanStake)
    );
}

#[test]
fn account_ledger_unstake_exceeds_capacity() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Prep actions
    let amount_1 = 100;
    let era_1 = 2;
    let period_1 = 1;
    acc_ledger.add_lock_amount(amount_1);
    assert!(acc_ledger
        .add_stake_amount(amount_1, era_1, period_1)
        .is_ok());

    for x in 0..StakingDummy::get() {
        assert!(
            acc_ledger.unstake_amount(3, era_1 + x, period_1).is_ok(),
            "Capacity isn't full so unstake must work."
        );
    }

    assert_eq!(
        acc_ledger.unstake_amount(3, era_1 + StakingDummy::get(), period_1),
        Err(AccountLedgerError::NoCapacity)
    );
}

#[test]
fn account_ledger_unlockable_amount_works() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Sanity check scenario
    assert!(acc_ledger.unlockable_amount(0).is_zero());

    // Nothing is staked
    let lock_amount = 29;
    let lock_era = 3;
    acc_ledger.add_lock_amount(lock_amount);
    assert_eq!(acc_ledger.unlockable_amount(0), lock_amount);

    // Some amount is staked, period matches
    let stake_period = 5;
    let stake_amount = 17;
    acc_ledger.staked = SparseBoundedAmountEraVec(
        BoundedVec::try_from(vec![StakeChunk {
            amount: stake_amount,
            era: lock_era,
        }])
        .expect("Only one chunk so creation should succeed."),
    );
    acc_ledger.staked_period = Some(stake_period);
    assert_eq!(
        acc_ledger.unlockable_amount(stake_period),
        lock_amount - stake_amount
    );

    // Period doesn't match
    assert_eq!(acc_ledger.unlockable_amount(stake_period - 1), lock_amount);
    assert_eq!(acc_ledger.unlockable_amount(stake_period + 2), lock_amount);

    // Absurd example, for the sake of completeness - staked without any lock
    acc_ledger.locked = Balance::zero();
    assert!(acc_ledger.unlockable_amount(stake_period).is_zero());
    assert!(acc_ledger.unlockable_amount(stake_period - 2).is_zero());
    assert!(acc_ledger.unlockable_amount(stake_period + 1).is_zero());
}

#[test]
fn account_ledger_claim_unlocked_works() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Sanity check scenario
    assert!(acc_ledger.claim_unlocked(0).is_zero());

    // Add a chunk, assert it can be claimed correctly
    let amount = 19;
    let block_number = 1;
    assert_ok!(acc_ledger.add_unlocking_chunk(amount, block_number));
    assert!(acc_ledger.claim_unlocked(0).is_zero());
    assert_eq!(acc_ledger.claim_unlocked(block_number), amount);
    assert!(acc_ledger.unlocking.is_empty());

    // Add multiple chunks, assert claim works correctly
    let (amount1, amount2, amount3) = (7, 13, 19);
    let (block1, block2, block3) = (1, 3, 5);

    // Prepare unlocking chunks
    assert_ok!(acc_ledger.add_unlocking_chunk(amount1, block1));
    assert_ok!(acc_ledger.add_unlocking_chunk(amount2, block2));
    assert_ok!(acc_ledger.add_unlocking_chunk(amount3, block3));

    // Only claim 1 chunk
    assert_eq!(acc_ledger.claim_unlocked(block1 + 1), amount1);
    assert_eq!(acc_ledger.unlocking.len(), 2);

    // Claim remaining two chunks
    assert_eq!(acc_ledger.claim_unlocked(block3 + 1), amount2 + amount3);
    assert!(acc_ledger.unlocking.is_empty());
}

#[test]
fn account_ledger_consume_unlocking_chunks_works() {
    get_u32_type!(UnlockingDummy, 5);
    get_u32_type!(StakingDummy, 8);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy, StakingDummy>::default();

    // Sanity check scenario
    assert!(acc_ledger.consume_unlocking_chunks().is_zero());

    // Add multiple chunks, cal should return correct amount
    let (amount1, amount2) = (7, 13);
    assert_ok!(acc_ledger.add_unlocking_chunk(amount1, 1));
    assert_ok!(acc_ledger.add_unlocking_chunk(amount2, 2));

    assert_eq!(acc_ledger.consume_unlocking_chunks(), amount1 + amount2);
    assert!(acc_ledger.unlocking.is_empty());
}

#[test]
fn era_info_lock_unlock_works() {
    let mut era_info = EraInfo::default();

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

    // Claim unlocked chunks
    let old_era_info = era_info.clone();
    era_info.unlocking_removed(1);
    assert_eq!(era_info.unlocking, old_era_info.unlocking - 1);
    assert_eq!(era_info.active_era_locked, old_era_info.active_era_locked);
}

#[test]
fn era_info_stake_works() {
    let mut era_info = EraInfo::default();

    // Sanity check
    assert!(era_info.total_locked.is_zero());

    // Add some voting period stake
    let vp_stake_amount = 7;
    era_info.add_stake_amount(vp_stake_amount, PeriodType::Voting);
    assert_eq!(era_info.total_staked_amount_next_era(), vp_stake_amount);
    assert_eq!(
        era_info.staked_amount_next_era(PeriodType::Voting),
        vp_stake_amount
    );
    assert!(
        era_info.total_staked_amount().is_zero(),
        "Calling stake makes it available only from the next era."
    );

    // Add some build&earn period stake
    let bep_stake_amount = 13;
    era_info.add_stake_amount(bep_stake_amount, PeriodType::BuildAndEarn);
    assert_eq!(
        era_info.total_staked_amount_next_era(),
        vp_stake_amount + bep_stake_amount
    );
    assert_eq!(
        era_info.staked_amount_next_era(PeriodType::BuildAndEarn),
        bep_stake_amount
    );
    assert!(
        era_info.total_staked_amount().is_zero(),
        "Calling stake makes it available only from the next era."
    );
}

#[test]
fn era_info_unstake_works() {
    let mut era_info = EraInfo::default();

    // Make dummy era info with stake amounts
    let vp_stake_amount = 15;
    let bep_stake_amount_1 = 23;
    let bep_stake_amount_2 = bep_stake_amount_1 + 6;
    era_info.current_stake_amount = StakeAmount::new(vp_stake_amount, bep_stake_amount_1);
    era_info.next_stake_amount = StakeAmount::new(vp_stake_amount, bep_stake_amount_2);
    let total_staked = era_info.total_staked_amount();
    let total_staked_next_era = era_info.total_staked_amount_next_era();

    // 1st scenario - unstake some amount, no overflow
    let unstake_amount_1 = bep_stake_amount_1;
    era_info.unstake_amount(unstake_amount_1, PeriodType::BuildAndEarn);

    // Current era
    assert_eq!(
        era_info.total_staked_amount(),
        total_staked - unstake_amount_1
    );
    assert_eq!(era_info.staked_amount(PeriodType::Voting), vp_stake_amount);
    assert!(era_info.staked_amount(PeriodType::BuildAndEarn).is_zero());

    // Next era
    assert_eq!(
        era_info.total_staked_amount_next_era(),
        total_staked_next_era - unstake_amount_1
    );
    assert_eq!(
        era_info.staked_amount_next_era(PeriodType::Voting),
        vp_stake_amount
    );
    assert_eq!(
        era_info.staked_amount_next_era(PeriodType::BuildAndEarn),
        bep_stake_amount_2 - unstake_amount_1
    );

    // 2nd scenario - unstake some more, but with overflow
    let overflow = 2;
    let unstake_amount_2 = bep_stake_amount_2 - unstake_amount_1 + overflow;
    era_info.unstake_amount(unstake_amount_2, PeriodType::BuildAndEarn);

    // Current era
    assert_eq!(
        era_info.total_staked_amount(),
        total_staked - unstake_amount_1 - unstake_amount_2
    );

    // Next era
    assert_eq!(
        era_info.total_staked_amount_next_era(),
        vp_stake_amount - overflow
    );
    assert_eq!(
        era_info.staked_amount_next_era(PeriodType::Voting),
        vp_stake_amount - overflow
    );
    assert!(era_info
        .staked_amount_next_era(PeriodType::BuildAndEarn)
        .is_zero());
}

#[test]
fn stake_amount_works() {
    let mut stake_amount = StakeAmount::default();

    // Sanity check
    assert!(stake_amount.total().is_zero());
    assert!(stake_amount.for_type(PeriodType::Voting).is_zero());
    assert!(stake_amount.for_type(PeriodType::BuildAndEarn).is_zero());

    // Stake some amount in voting period
    let vp_stake_1 = 11;
    stake_amount.stake(vp_stake_1, PeriodType::Voting);
    assert_eq!(stake_amount.total(), vp_stake_1);
    assert_eq!(stake_amount.for_type(PeriodType::Voting), vp_stake_1);
    assert!(stake_amount.for_type(PeriodType::BuildAndEarn).is_zero());

    // Stake some amount in build&earn period
    let bep_stake_1 = 13;
    stake_amount.stake(bep_stake_1, PeriodType::BuildAndEarn);
    assert_eq!(stake_amount.total(), vp_stake_1 + bep_stake_1);
    assert_eq!(stake_amount.for_type(PeriodType::Voting), vp_stake_1);
    assert_eq!(stake_amount.for_type(PeriodType::BuildAndEarn), bep_stake_1);

    // Unstake some amount from voting period
    let vp_unstake_1 = 5;
    stake_amount.unstake(5, PeriodType::Voting);
    assert_eq!(
        stake_amount.total(),
        vp_stake_1 + bep_stake_1 - vp_unstake_1
    );
    assert_eq!(
        stake_amount.for_type(PeriodType::Voting),
        vp_stake_1 - vp_unstake_1
    );
    assert_eq!(stake_amount.for_type(PeriodType::BuildAndEarn), bep_stake_1);

    // Unstake some amount from build&earn period
    let bep_unstake_1 = 2;
    stake_amount.unstake(bep_unstake_1, PeriodType::BuildAndEarn);
    assert_eq!(
        stake_amount.total(),
        vp_stake_1 + bep_stake_1 - vp_unstake_1 - bep_unstake_1
    );
    assert_eq!(
        stake_amount.for_type(PeriodType::Voting),
        vp_stake_1 - vp_unstake_1
    );
    assert_eq!(
        stake_amount.for_type(PeriodType::BuildAndEarn),
        bep_stake_1 - bep_unstake_1
    );

    // Unstake some more from build&earn period, and chip away from the voting period
    let total_stake = vp_stake_1 + bep_stake_1 - vp_unstake_1 - bep_unstake_1;
    let bep_unstake_2 = bep_stake_1 - bep_unstake_1 + 1;
    stake_amount.unstake(bep_unstake_2, PeriodType::BuildAndEarn);
    assert_eq!(stake_amount.total(), total_stake - bep_unstake_2);
    assert_eq!(
        stake_amount.for_type(PeriodType::Voting),
        vp_stake_1 - vp_unstake_1 - 1
    );
    assert!(stake_amount.for_type(PeriodType::BuildAndEarn).is_zero());
}

#[test]
fn singular_staking_info_basics_are_ok() {
    let period_number = 3;
    let period_type = PeriodType::Voting;
    let mut staking_info = SingularStakingInfo::new(period_number, period_type);

    // Sanity checks
    assert_eq!(staking_info.period_number(), period_number);
    assert!(staking_info.is_loyal());
    assert!(staking_info.total_staked_amount().is_zero());
    assert!(!SingularStakingInfo::new(period_number, PeriodType::BuildAndEarn).is_loyal());

    // Add some staked amount during `Voting` period
    let vote_stake_amount_1 = 11;
    staking_info.stake(vote_stake_amount_1, PeriodType::Voting);
    assert_eq!(staking_info.total_staked_amount(), vote_stake_amount_1);
    assert_eq!(
        staking_info.staked_amount(PeriodType::Voting),
        vote_stake_amount_1
    );
    assert!(staking_info
        .staked_amount(PeriodType::BuildAndEarn)
        .is_zero());

    // Add some staked amount during `BuildAndEarn` period
    let bep_stake_amount_1 = 23;
    staking_info.stake(bep_stake_amount_1, PeriodType::BuildAndEarn);
    assert_eq!(
        staking_info.total_staked_amount(),
        vote_stake_amount_1 + bep_stake_amount_1
    );
    assert_eq!(
        staking_info.staked_amount(PeriodType::Voting),
        vote_stake_amount_1
    );
    assert_eq!(
        staking_info.staked_amount(PeriodType::BuildAndEarn),
        bep_stake_amount_1
    );
}

#[test]
fn singular_staking_info_unstake_during_voting_is_ok() {
    let period_number = 3;
    let period_type = PeriodType::Voting;
    let mut staking_info = SingularStakingInfo::new(period_number, period_type);

    // Prep actions
    let vote_stake_amount_1 = 11;
    staking_info.stake(vote_stake_amount_1, PeriodType::Voting);

    // Unstake some amount during `Voting` period, loyalty should remain as expected.
    let unstake_amount_1 = 5;
    assert_eq!(
        staking_info.unstake(unstake_amount_1, PeriodType::Voting),
        (unstake_amount_1, Balance::zero())
    );
    assert_eq!(
        staking_info.total_staked_amount(),
        vote_stake_amount_1 - unstake_amount_1
    );
    assert!(staking_info.is_loyal());

    // Fully unstake, attempting to undersaturate, and ensure loyalty flag is still true.
    let remaining_stake = staking_info.total_staked_amount();
    assert_eq!(
        staking_info.unstake(remaining_stake + 1, PeriodType::Voting),
        (remaining_stake, Balance::zero())
    );
    assert!(staking_info.total_staked_amount().is_zero());
    assert!(staking_info.is_loyal());
}

#[test]
fn singular_staking_info_unstake_during_bep_is_ok() {
    let period_number = 3;
    let period_type = PeriodType::Voting;
    let mut staking_info = SingularStakingInfo::new(period_number, period_type);

    // Prep actions
    let vote_stake_amount_1 = 11;
    staking_info.stake(vote_stake_amount_1, PeriodType::Voting);
    let bep_stake_amount_1 = 23;
    staking_info.stake(bep_stake_amount_1, PeriodType::BuildAndEarn);

    // 1st scenario - Unstake some of the amount staked during B&E period
    let unstake_1 = 5;
    assert_eq!(
        staking_info.unstake(5, PeriodType::BuildAndEarn),
        (Balance::zero(), unstake_1)
    );
    assert_eq!(
        staking_info.total_staked_amount(),
        vote_stake_amount_1 + bep_stake_amount_1 - unstake_1
    );
    assert_eq!(
        staking_info.staked_amount(PeriodType::Voting),
        vote_stake_amount_1
    );
    assert_eq!(
        staking_info.staked_amount(PeriodType::BuildAndEarn),
        bep_stake_amount_1 - unstake_1
    );
    assert!(staking_info.is_loyal());

    // 2nd scenario - unstake all of the amount staked during B&E period, and then some more.
    // The point is to take a chunk from the voting period stake too.
    let current_total_stake = staking_info.total_staked_amount();
    let current_bep_stake = staking_info.staked_amount(PeriodType::BuildAndEarn);
    let voting_stake_overflow = 2;
    let unstake_2 = current_bep_stake + voting_stake_overflow;

    assert_eq!(
        staking_info.unstake(unstake_2, PeriodType::BuildAndEarn),
        (voting_stake_overflow, current_bep_stake)
    );
    assert_eq!(
        staking_info.total_staked_amount(),
        current_total_stake - unstake_2
    );
    assert_eq!(
        staking_info.staked_amount(PeriodType::Voting),
        vote_stake_amount_1 - voting_stake_overflow
    );
    assert!(staking_info
        .staked_amount(PeriodType::BuildAndEarn)
        .is_zero());
    assert!(
        !staking_info.is_loyal(),
        "Loyalty flag should have been removed due to non-zero voting period unstake"
    );
}

#[test]
fn contract_stake_info_is_ok() {
    let period = 2;
    let era = 3;
    let mut contract_stake_info = ContractStakingInfo::new(era, period);

    // Sanity check
    assert_eq!(contract_stake_info.period(), period);
    assert_eq!(contract_stake_info.era(), era);
    assert!(contract_stake_info.total_staked_amount().is_zero());
    assert!(contract_stake_info.is_empty());

    // 1st scenario - Add some staked amount to the voting period
    let vote_stake_amount_1 = 11;
    contract_stake_info.stake(vote_stake_amount_1, PeriodType::Voting);
    assert_eq!(
        contract_stake_info.total_staked_amount(),
        vote_stake_amount_1
    );
    assert_eq!(
        contract_stake_info.staked_amount(PeriodType::Voting),
        vote_stake_amount_1
    );
    assert!(!contract_stake_info.is_empty());

    // 2nd scenario - add some staked amount to the B&E period
    let bep_stake_amount_1 = 23;
    contract_stake_info.stake(bep_stake_amount_1, PeriodType::BuildAndEarn);
    assert_eq!(
        contract_stake_info.total_staked_amount(),
        vote_stake_amount_1 + bep_stake_amount_1
    );
    assert_eq!(
        contract_stake_info.staked_amount(PeriodType::Voting),
        vote_stake_amount_1
    );
    assert_eq!(
        contract_stake_info.staked_amount(PeriodType::BuildAndEarn),
        bep_stake_amount_1
    );

    // 3rd scenario - reduce some of the staked amount from both periods and verify it's as expected.
    let total_staked = contract_stake_info.total_staked_amount();
    let vp_reduction = 3;
    contract_stake_info.unstake(vp_reduction, PeriodType::Voting);
    assert_eq!(
        contract_stake_info.total_staked_amount(),
        total_staked - vp_reduction
    );
    assert_eq!(
        contract_stake_info.staked_amount(PeriodType::Voting),
        vote_stake_amount_1 - vp_reduction
    );

    let bp_reduction = 7;
    contract_stake_info.unstake(bp_reduction, PeriodType::BuildAndEarn);
    assert_eq!(
        contract_stake_info.total_staked_amount(),
        total_staked - vp_reduction - bp_reduction
    );
    assert_eq!(
        contract_stake_info.staked_amount(PeriodType::BuildAndEarn),
        bep_stake_amount_1 - bp_reduction
    );

    // 4th scenario - unstake everything, and some more, from Build&Earn period, chiping away from the voting period.
    let overflow = 1;
    let overflow_reduction = contract_stake_info.staked_amount(PeriodType::BuildAndEarn) + overflow;
    contract_stake_info.unstake(overflow_reduction, PeriodType::BuildAndEarn);
    assert_eq!(
        contract_stake_info.total_staked_amount(),
        vote_stake_amount_1 - vp_reduction - overflow
    );
    assert!(contract_stake_info
        .staked_amount(PeriodType::BuildAndEarn)
        .is_zero());
    assert_eq!(
        contract_stake_info.staked_amount(PeriodType::Voting),
        vote_stake_amount_1 - vp_reduction - overflow
    );
}

#[test]
fn contract_staking_info_series_get_works() {
    let info_1 = ContractStakingInfo::new(4, 2);
    let mut info_2 = ContractStakingInfo::new(7, 3);
    info_2.stake(11, PeriodType::Voting);
    let mut info_3 = ContractStakingInfo::new(9, 3);
    info_3.stake(13, PeriodType::BuildAndEarn);

    let series = ContractStakingInfoSeries::new(vec![info_1, info_2, info_3]);

    // Sanity check
    assert_eq!(series.len(), 3);
    assert!(!series.is_empty());

    // 1st scenario - get existing entries
    assert_eq!(series.get(4, 2), Some(info_1));
    assert_eq!(series.get(7, 3), Some(info_2));
    assert_eq!(series.get(9, 3), Some(info_3));

    // 2nd scenario - get non-existing entries for covered eras
    {
        let era_1 = 6;
        let entry_1 = series.get(era_1, 2).expect("Has to be Some");
        assert!(entry_1.total_staked_amount().is_zero());
        assert_eq!(entry_1.era(), era_1);
        assert_eq!(entry_1.period(), 2);

        let era_2 = 8;
        let entry_1 = series.get(era_2, 3).expect("Has to be Some");
        assert_eq!(entry_1.total_staked_amount(), 11);
        assert_eq!(entry_1.era(), era_2);
        assert_eq!(entry_1.period(), 3);
    }

    // 3rd scenario - get non-existing entries for covered eras but mismatching period
    assert!(series.get(8, 2).is_none());

    // 4th scenario - get non-existing entries for non-covered eras
    assert!(series.get(3, 2).is_none());
}

#[test]
fn contract_staking_info_series_stake_is_ok() {
    let mut series = ContractStakingInfoSeries::default();

    // Sanity check
    assert!(series.is_empty());
    assert!(series.len().is_zero());

    // 1st scenario - stake some amount and verify state change
    let era_1 = 3;
    let period_1 = 5;
    let period_info_1 = PeriodInfo::new(period_1, PeriodType::Voting, 20);
    let amount_1 = 31;
    assert!(series.stake(amount_1, period_info_1, era_1).is_ok());

    assert_eq!(series.len(), 1);
    assert!(!series.is_empty());

    let entry_1_1 = series.get(era_1, period_1).unwrap();
    assert_eq!(entry_1_1.era(), era_1);
    assert_eq!(entry_1_1.total_staked_amount(), amount_1);

    // 2nd scenario - stake some more to the same era but different period type, and verify state change.
    let period_info_1 = PeriodInfo::new(period_1, PeriodType::BuildAndEarn, 20);
    assert!(series.stake(amount_1, period_info_1, era_1).is_ok());
    assert_eq!(
        series.len(),
        1,
        "No new entry should be created since it's the same era."
    );
    let entry_1_2 = series.get(era_1, period_1).unwrap();
    assert_eq!(entry_1_2.era(), era_1);
    assert_eq!(entry_1_2.total_staked_amount(), amount_1 * 2);

    // 3rd scenario - stake more to the next era, while still in the same period.
    let era_2 = era_1 + 2;
    let amount_2 = 37;
    assert!(series.stake(amount_2, period_info_1, era_2).is_ok());
    assert_eq!(series.len(), 2);
    let entry_2_1 = series.get(era_1, period_1).unwrap();
    let entry_2_2 = series.get(era_2, period_1).unwrap();
    assert_eq!(entry_2_1, entry_1_2, "Old entry must remain unchanged.");
    assert_eq!(entry_2_2.era(), era_2);
    assert_eq!(entry_2_2.period(), period_1);
    assert_eq!(
        entry_2_2.total_staked_amount(),
        entry_2_1.total_staked_amount() + amount_2,
        "Since it's the same period, stake amount must carry over from the previous entry."
    );

    // 4th scenario - stake some more to the next era, but this time also bump the period.
    let era_3 = era_2 + 3;
    let period_2 = period_1 + 1;
    let period_info_2 = PeriodInfo::new(period_2, PeriodType::BuildAndEarn, 20);
    let amount_3 = 41;

    assert!(series.stake(amount_3, period_info_2, era_3).is_ok());
    assert_eq!(series.len(), 3);
    let entry_3_1 = series.get(era_1, period_1).unwrap();
    let entry_3_2 = series.get(era_2, period_1).unwrap();
    let entry_3_3 = series.get(era_3, period_2).unwrap();
    assert_eq!(entry_3_1, entry_2_1, "Old entry must remain unchanged.");
    assert_eq!(entry_3_2, entry_2_2, "Old entry must remain unchanged.");
    assert_eq!(entry_3_3.era(), era_3);
    assert_eq!(entry_3_3.period(), period_2);
    assert_eq!(
        entry_3_3.total_staked_amount(),
        amount_3,
        "No carry over from previous entry since period has changed."
    );

    // 5th scenario - stake to the next era, expect cleanup of oldest entry
    let era_4 = era_3 + 1;
    let amount_4 = 5;
    assert!(series.stake(amount_4, period_info_2, era_4).is_ok());
    assert_eq!(series.len(), 3);
    let entry_4_1 = series.get(era_2, period_1).unwrap();
    let entry_4_2 = series.get(era_3, period_2).unwrap();
    let entry_4_3 = series.get(era_4, period_2).unwrap();
    assert_eq!(entry_4_1, entry_3_2, "Old entry must remain unchanged.");
    assert_eq!(entry_4_2, entry_3_3, "Old entry must remain unchanged.");
    assert_eq!(entry_4_3.era(), era_4);
    assert_eq!(entry_4_3.period(), period_2);
    assert_eq!(entry_4_3.total_staked_amount(), amount_3 + amount_4);
}

#[test]
fn contract_staking_info_series_stake_with_inconsistent_data_fails() {
    let mut series = ContractStakingInfoSeries::default();

    // Create an entry with some staked amount
    let era = 5;
    let period_info = PeriodInfo {
        number: 7,
        period_type: PeriodType::Voting,
        ending_era: 31,
    };
    let amount = 37;
    assert!(series.stake(amount, period_info, era).is_ok());

    // 1st scenario - attempt to stake using old era
    assert!(series.stake(amount, period_info, era - 1).is_err());

    // 2nd scenario - attempt to stake using old period
    let period_info = PeriodInfo {
        number: period_info.number - 1,
        period_type: PeriodType::Voting,
        ending_era: 31,
    };
    assert!(series.stake(amount, period_info, era).is_err());
}

#[test]
fn contract_staking_info_series_unstake_is_ok() {
    let mut series = ContractStakingInfoSeries::default();

    // Prep action - create a stake entry
    let era_1 = 2;
    let period = 3;
    let period_info = PeriodInfo::new(period, PeriodType::Voting, 20);
    let stake_amount = 100;
    assert!(series.stake(stake_amount, period_info, era_1).is_ok());

    // 1st scenario - unstake in the same era
    let amount_1 = 5;
    assert!(series.unstake(amount_1, period_info, era_1).is_ok());
    assert_eq!(series.len(), 1);
    assert_eq!(series.total_staked_amount(period), stake_amount - amount_1);
    assert_eq!(
        series.staked_amount(period, PeriodType::Voting),
        stake_amount - amount_1
    );

    // 2nd scenario - unstake in the future era, creating a 'gap' in the series
    // [(era: 2)] ---> [(era: 2), (era: 5)]
    let period_info = PeriodInfo::new(period, PeriodType::BuildAndEarn, 40);
    let era_2 = era_1 + 3;
    let amount_2 = 7;
    assert!(series.unstake(amount_2, period_info, era_2).is_ok());
    assert_eq!(series.len(), 2);
    assert_eq!(
        series.total_staked_amount(period),
        stake_amount - amount_1 - amount_2
    );
    assert_eq!(
        series.staked_amount(period, PeriodType::Voting),
        stake_amount - amount_1 - amount_2
    );

    // 3rd scenario - unstake in the era right before the last, inserting the new value in-between the old ones
    // [(era: 2), (era: 5)] ---> [(era: 2), (era: 4), (era: 5)]
    let era_3 = era_2 - 1;
    let amount_3 = 11;
    assert!(series.unstake(amount_3, period_info, era_3).is_ok());
    assert_eq!(series.len(), 3);
    assert_eq!(
        series.total_staked_amount(period),
        stake_amount - amount_1 - amount_2 - amount_3
    );
    assert_eq!(
        series.staked_amount(period, PeriodType::Voting),
        stake_amount - amount_1 - amount_2 - amount_3
    );

    // Check concrete entries
    assert_eq!(
        series.get(era_1, period).unwrap().total_staked_amount(),
        stake_amount - amount_1,
        "Oldest entry must remain unchanged."
    );
    assert_eq!(
        series.get(era_2, period).unwrap().total_staked_amount(),
        stake_amount - amount_1 - amount_2 - amount_3,
        "Future era entry must be updated with all of the reductions."
    );
    assert_eq!(
        series.get(era_3, period).unwrap().total_staked_amount(),
        stake_amount - amount_1 - amount_3,
        "Second to last era entry must be updated with first & last reduction\
     because it derives its initial value from the oldest entry."
    );
}

#[test]
fn contract_staking_info_unstake_with_worst_case_scenario_for_capacity_overflow() {
    let (era_1, era_2, era_3) = (4, 7, 9);
    let (period_1, period_2) = (2, 3);
    let info_1 = ContractStakingInfo::new(era_1, period_1);
    let mut info_2 = ContractStakingInfo::new(era_2, period_2);
    let stake_amount_2 = 11;
    info_2.stake(stake_amount_2, PeriodType::Voting);
    let mut info_3 = ContractStakingInfo::new(era_3, period_2);
    let stake_amount_3 = 13;
    info_3.stake(stake_amount_3, PeriodType::BuildAndEarn);

    // A gap between 2nd and 3rd era, and from that gap unstake will be done.
    // This will force a new entry to be created, potentially overflowing the vector capacity.
    let mut series = ContractStakingInfoSeries::new(vec![info_1, info_2, info_3]);

    // Unstake between era 2 & 3, in attempt to overflow the inner vector capacity
    let period_info = PeriodInfo {
        number: period_2,
        period_type: PeriodType::BuildAndEarn,
        ending_era: 51,
    };
    let unstake_amount = 3;
    assert!(series.unstake(3, period_info, era_2 + 1).is_ok());
    assert_eq!(series.len(), 3);

    assert_eq!(
        series.get(era_1, period_1),
        None,
        "Oldest entry should have been prunned"
    );
    assert_eq!(
        series
            .get(era_2, period_2)
            .expect("Entry must exist.")
            .total_staked_amount(),
        stake_amount_2
    );
    assert_eq!(
        series
            .get(era_2 + 1, period_2)
            .expect("Entry must exist.")
            .total_staked_amount(),
        stake_amount_2 - unstake_amount
    );
    assert_eq!(
        series
            .get(era_3, period_2)
            .expect("Entry must exist.")
            .total_staked_amount(),
        stake_amount_3 - unstake_amount
    );
}

#[test]
fn contract_staking_info_series_unstake_with_inconsistent_data_fails() {
    let mut series = ContractStakingInfoSeries::default();
    let era = 5;
    let period = 2;
    let period_info = PeriodInfo {
        number: period,
        period_type: PeriodType::Voting,
        ending_era: 31,
    };

    // 1st - Unstake from empty series
    assert!(series.unstake(1, period_info, era).is_err());

    // 2nd - Unstake with old period
    let amount = 37;
    assert!(series.stake(amount, period_info, era).is_ok());

    let old_period_info = {
        let mut temp = period_info.clone();
        temp.number -= 1;
        temp
    };
    assert!(series.unstake(1, old_period_info, era - 1).is_err());

    // 3rd - Unstake with 'too' old era
    assert!(series.unstake(1, period_info, era - 2).is_err());
    assert!(series.unstake(1, period_info, era - 1).is_ok());
}

#[test]
fn era_reward_span_push_and_get_works() {
    get_u32_type!(SpanLength, 8);
    let mut era_reward_span = EraRewardSpan::<SpanLength>::new();

    // Sanity checks
    assert!(era_reward_span.is_empty());
    assert!(era_reward_span.len().is_zero());
    assert!(era_reward_span.first_era().is_zero());
    assert!(era_reward_span.last_era().is_zero());

    // Insert some values and verify state change
    let era_1 = 5;
    let era_reward_1 = EraReward::new(23, 41);
    assert!(era_reward_span.push(era_1, era_reward_1).is_ok());
    assert_eq!(era_reward_span.len(), 1);
    assert_eq!(era_reward_span.first_era(), era_1);
    assert_eq!(era_reward_span.last_era(), era_1);

    // Insert another value and verify state change
    let era_2 = era_1 + 1;
    let era_reward_2 = EraReward::new(37, 53);
    assert!(era_reward_span.push(era_2, era_reward_2).is_ok());
    assert_eq!(era_reward_span.len(), 2);
    assert_eq!(era_reward_span.first_era(), era_1);
    assert_eq!(era_reward_span.last_era(), era_2);

    // Get the values and verify they are as expected
    assert_eq!(era_reward_span.get(era_1), Some(&era_reward_1));
    assert_eq!(era_reward_span.get(era_2), Some(&era_reward_2));
}

#[test]
fn era_reward_span_fails_when_expected() {
    // Capacity is only 2 to make testing easier
    get_u32_type!(SpanLength, 2);
    let mut era_reward_span = EraRewardSpan::<SpanLength>::new();

    // Push first values to get started
    let era_1 = 5;
    let era_reward = EraReward::new(23, 41);
    assert!(era_reward_span.push(era_1, era_reward).is_ok());

    // Attempting to push incorrect era results in an error
    for wrong_era in &[era_1 - 1, era_1, era_1 + 2] {
        assert_eq!(
            era_reward_span.push(era_1 - 1, era_reward),
            Err(EraRewardSpanError::InvalidEra)
        );
    }

    // Pushing above capacity results in an error
    let era_2 = era_1 + 1;
    assert!(era_reward_span.push(era_2, era_reward).is_ok());
    let era_3 = era_2 + 1;
    assert_eq!(
        era_reward_span.push(era_3, era_reward),
        Err(EraRewardSpanError::NoCapacity)
    );
}
