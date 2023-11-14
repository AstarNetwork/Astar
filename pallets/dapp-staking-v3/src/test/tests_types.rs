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

use astar_primitives::{Balance, BlockNumber};
use frame_support::assert_ok;
use sp_arithmetic::fixed_point::FixedU64;
use sp_runtime::Permill;

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
fn subperiod_sanity_check() {
    assert_eq!(Subperiod::Voting.next(), Subperiod::BuildAndEarn);
    assert_eq!(Subperiod::BuildAndEarn.next(), Subperiod::Voting);
}

#[test]
fn period_info_basic_checks() {
    let period_number = 2;
    let subperiod_end_era = 5;
    let info = PeriodInfo {
        number: period_number,
        subperiod: Subperiod::Voting,
        subperiod_end_era: subperiod_end_era,
    };

    // Sanity checks
    assert_eq!(info.number, period_number);
    assert_eq!(info.subperiod, Subperiod::Voting);
    assert_eq!(info.subperiod_end_era, subperiod_end_era);

    // Voting period checks
    assert!(!info.is_next_period(subperiod_end_era - 1));
    assert!(!info.is_next_period(subperiod_end_era));
    assert!(!info.is_next_period(subperiod_end_era + 1));
    for era in vec![
        subperiod_end_era - 1,
        subperiod_end_era,
        subperiod_end_era + 1,
    ] {
        assert!(
            !info.is_next_period(era),
            "Cannot trigger 'true' in the Voting period type."
        );
    }

    // Build&Earn period checks
    let info = PeriodInfo {
        number: period_number,
        subperiod: Subperiod::BuildAndEarn,
        subperiod_end_era: subperiod_end_era,
    };
    assert!(!info.is_next_period(subperiod_end_era - 1));
    assert!(info.is_next_period(subperiod_end_era));
    assert!(info.is_next_period(subperiod_end_era + 1));
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
    let subperiod_end_era = 11;
    let next_era_start = 31;
    protocol_state.period_info = PeriodInfo {
        number: period_number,
        subperiod: Subperiod::Voting,
        subperiod_end_era: subperiod_end_era,
    };
    protocol_state.next_era_start = next_era_start;

    assert_eq!(protocol_state.period_number(), period_number);
    assert_eq!(protocol_state.subperiod(), Subperiod::Voting);

    // New era check
    assert!(!protocol_state.is_new_era(next_era_start - 1));
    assert!(protocol_state.is_new_era(next_era_start));
    assert!(protocol_state.is_new_era(next_era_start + 1));

    // Toggle new period type check - 'Voting' to 'BuildAndEarn'
    let subperiod_end_era_1 = 23;
    let next_era_start_1 = 41;
    protocol_state.advance_to_next_subperiod(subperiod_end_era_1, next_era_start_1);
    assert_eq!(protocol_state.subperiod(), Subperiod::BuildAndEarn);
    assert_eq!(
        protocol_state.period_number(),
        period_number,
        "Switching from 'Voting' to 'BuildAndEarn' should not trigger period bump."
    );
    assert_eq!(protocol_state.period_end_era(), subperiod_end_era_1);
    assert!(!protocol_state.is_new_era(next_era_start_1 - 1));
    assert!(protocol_state.is_new_era(next_era_start_1));

    // Toggle from 'BuildAndEarn' over to 'Voting'
    let subperiod_end_era_2 = 24;
    let next_era_start_2 = 91;
    protocol_state.advance_to_next_subperiod(subperiod_end_era_2, next_era_start_2);
    assert_eq!(protocol_state.subperiod(), Subperiod::Voting);
    assert_eq!(
        protocol_state.period_number(),
        period_number + 1,
        "Switching from 'BuildAndEarn' to 'Voting' must trigger period bump."
    );
    assert_eq!(protocol_state.period_end_era(), subperiod_end_era_2);
    assert!(protocol_state.is_new_era(next_era_start_2));
}

#[test]
fn dapp_info_basic_checks() {
    let owner = 1;
    let beneficiary = 3;

    let mut dapp_info = DAppInfo {
        owner,
        id: 7,
        state: DAppState::Registered,
        reward_destination: None,
    };

    // Owner receives reward in case no beneficiary is set
    assert_eq!(*dapp_info.reward_beneficiary(), owner);

    // Beneficiary receives rewards in case it is set
    dapp_info.reward_destination = Some(beneficiary);
    assert_eq!(*dapp_info.reward_beneficiary(), beneficiary);
}

#[test]
fn account_ledger_default() {
    get_u32_type!(UnlockingDummy, 5);
    let acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    assert!(acc_ledger.is_empty());
    assert!(acc_ledger.active_locked_amount().is_zero());
}

#[test]
fn account_ledger_add_lock_amount_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

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
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

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
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    // Base sanity check
    let default_unlocking_chunk = UnlockingChunk::<BlockNumber>::default();
    assert!(default_unlocking_chunk.amount.is_zero());
    assert!(default_unlocking_chunk.unlock_block.is_zero());

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
            .add_unlocking_chunk(new_unlock_amount, block_number + i)
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
        acc_ledger.add_unlocking_chunk(1, block_number + UnlockingDummy::get() + 1),
        Err(AccountLedgerError::NoCapacity)
    );
    assert_eq!(acc_ledger, acc_ledger_snapshot);
}

#[test]
fn account_ledger_staked_amount_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    // Sanity check
    assert!(acc_ledger.staked_amount(0).is_zero());
    assert!(acc_ledger.staked_amount(1).is_zero());

    // Period matches
    let amount_1 = 29;
    let period = 5;
    acc_ledger.staked = StakeAmount::new(amount_1, 0, 1, period);
    assert_eq!(acc_ledger.staked_amount(period), amount_1);

    // Period doesn't match
    assert!(acc_ledger.staked_amount(period - 1).is_zero());
    assert!(acc_ledger.staked_amount(period + 1).is_zero());

    // Add future entry
    let amount_2 = 17;
    acc_ledger.staked_future = Some(StakeAmount::new(0, amount_2, 2, period));
    assert_eq!(acc_ledger.staked_amount(period), amount_2);
    assert!(acc_ledger.staked_amount(period - 1).is_zero());
    assert!(acc_ledger.staked_amount(period + 1).is_zero());
}

#[test]
fn account_ledger_staked_amount_for_type_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    // 1st scenario - 'current' entry is set, 'future' is None
    let (voting_1, build_and_earn_1, period) = (31, 43, 2);
    acc_ledger.staked = StakeAmount {
        voting: voting_1,
        build_and_earn: build_and_earn_1,
        era: 10,
        period,
    };
    acc_ledger.staked_future = None;

    // Correct period should return staked amounts
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::Voting, period),
        voting_1
    );
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::BuildAndEarn, period),
        build_and_earn_1
    );

    // Inocrrect period should simply return 0
    assert!(acc_ledger
        .staked_amount_for_type(Subperiod::Voting, period - 1)
        .is_zero());
    assert!(acc_ledger
        .staked_amount_for_type(Subperiod::BuildAndEarn, period - 1)
        .is_zero());

    // 2nd scenario - both entries are set, but 'future' must be relevant one.
    let (voting_2, build_and_earn_2, period) = (13, 19, 2);
    acc_ledger.staked_future = Some(StakeAmount {
        voting: voting_2,
        build_and_earn: build_and_earn_2,
        era: 20,
        period,
    });

    // Correct period should return staked amounts
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::Voting, period),
        voting_2
    );
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::BuildAndEarn, period),
        build_and_earn_2
    );

    // Inocrrect period should simply return 0
    assert!(acc_ledger
        .staked_amount_for_type(Subperiod::Voting, period - 1)
        .is_zero());
    assert!(acc_ledger
        .staked_amount_for_type(Subperiod::BuildAndEarn, period - 1)
        .is_zero());
}

#[test]
fn account_ledger_stakeable_amount_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    // Sanity check for empty ledger
    assert!(acc_ledger.stakeable_amount(1).is_zero());

    // 1st scenario - some locked amount, no staking chunks
    let period_1 = 1;
    let locked_amount = 19;
    acc_ledger.add_lock_amount(locked_amount);
    assert_eq!(
        acc_ledger.stakeable_amount(period_1),
        locked_amount,
        "Stakeable amount has to be equal to the locked amount"
    );

    // Second scenario - some staked amount is introduced, period is still valid
    let first_era = 1;
    let staked_amount = 7;
    acc_ledger.staked = StakeAmount::new(0, staked_amount, first_era, period_1);

    assert_eq!(
        acc_ledger.stakeable_amount(period_1),
        locked_amount - staked_amount,
        "Total stakeable amount should be equal to the locked amount minus what is already staked."
    );

    // Third scenario - continuation of the previous, but we move to the next period.
    assert_eq!(
        acc_ledger.stakeable_amount(period_1 + 1),
        locked_amount,
        "Stakeable amount has to be equal to the locked amount since old period staking isn't valid anymore"
    );
}

#[test]
fn account_ledger_staked_era_period_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    let (era_1, period) = (10, 2);
    let stake_amount_1 = StakeAmount {
        voting: 13,
        build_and_earn: 17,
        era: era_1,
        period,
    };

    // Sanity check, empty ledger
    assert!(acc_ledger.staked_period().is_none());
    assert!(acc_ledger.earliest_staked_era().is_none());

    // 1st scenario - only 'current' entry is set
    acc_ledger.staked = stake_amount_1;
    acc_ledger.staked_future = None;

    assert_eq!(acc_ledger.staked_period(), Some(period));
    assert_eq!(acc_ledger.earliest_staked_era(), Some(era_1));

    // 2nd scenario - only 'future' is set
    let era_2 = era_1 + 7;
    let stake_amount_2 = StakeAmount {
        voting: 13,
        build_and_earn: 17,
        era: era_2,
        period,
    };
    acc_ledger.staked = Default::default();
    acc_ledger.staked_future = Some(stake_amount_2);

    assert_eq!(acc_ledger.staked_period(), Some(period));
    assert_eq!(acc_ledger.earliest_staked_era(), Some(era_2));

    // 3rd scenario - both entries are set
    acc_ledger.staked = stake_amount_1;
    acc_ledger.staked_future = Some(stake_amount_2);

    assert_eq!(acc_ledger.staked_period(), Some(period));
    assert_eq!(acc_ledger.earliest_staked_era(), Some(era_1));
}

#[test]
fn account_ledger_add_stake_amount_basic_example_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    // Sanity check
    let period_number = 2;
    assert!(acc_ledger
        .add_stake_amount(
            0,
            0,
            PeriodInfo {
                number: period_number,
                subperiod: Subperiod::Voting,
                subperiod_end_era: 0
            }
        )
        .is_ok());
    assert!(acc_ledger.staked.is_empty());
    assert!(acc_ledger.staked_future.is_none());

    // 1st scenario - stake some amount in Voting period, and ensure values are as expected.
    let first_era = 1;
    let period_1 = 1;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::Voting,
        subperiod_end_era: 100,
    };
    let lock_amount = 17;
    let stake_amount = 11;
    acc_ledger.add_lock_amount(lock_amount);

    assert!(acc_ledger
        .add_stake_amount(stake_amount, first_era, period_info_1)
        .is_ok());

    assert!(
        acc_ledger.staked.is_empty(),
        "Current era must remain unchanged."
    );
    assert_eq!(
        acc_ledger
            .staked_future
            .expect("Must exist after stake.")
            .period,
        period_1
    );
    assert_eq!(acc_ledger.staked_future.unwrap().voting, stake_amount);
    assert!(acc_ledger.staked_future.unwrap().build_and_earn.is_zero());
    assert_eq!(acc_ledger.staked_amount(period_1), stake_amount);
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::Voting, period_1),
        stake_amount
    );
    assert!(acc_ledger
        .staked_amount_for_type(Subperiod::BuildAndEarn, period_1)
        .is_zero());

    // Second scenario - stake some more, but to the next period type
    let snapshot = acc_ledger.staked;
    let period_info_2 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::BuildAndEarn,
        subperiod_end_era: 100,
    };
    assert!(acc_ledger
        .add_stake_amount(1, first_era, period_info_2)
        .is_ok());
    assert_eq!(acc_ledger.staked_amount(period_1), stake_amount + 1);
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::Voting, period_1),
        stake_amount
    );
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::BuildAndEarn, period_1),
        1
    );
    assert_eq!(acc_ledger.staked, snapshot);
}

#[test]
fn account_ledger_add_stake_amount_advanced_example_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    // 1st scenario - stake some amount, and ensure values are as expected.
    let first_era = 1;
    let period_1 = 1;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::Voting,
        subperiod_end_era: 100,
    };
    let lock_amount = 17;
    let stake_amount_1 = 11;
    acc_ledger.add_lock_amount(lock_amount);

    // We only have entry for the current era
    acc_ledger.staked = StakeAmount::new(stake_amount_1, 0, first_era, period_1);

    let stake_amount_2 = 2;
    let acc_ledger_snapshot = acc_ledger.clone();
    assert!(acc_ledger
        .add_stake_amount(stake_amount_2, first_era, period_info_1)
        .is_ok());
    assert_eq!(
        acc_ledger.staked_amount(period_1),
        stake_amount_1 + stake_amount_2
    );
    assert_eq!(
        acc_ledger.staked, acc_ledger_snapshot.staked,
        "This entry must remain unchanged."
    );
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::Voting, period_1),
        stake_amount_1 + stake_amount_2
    );
    assert_eq!(
        acc_ledger
            .staked_future
            .unwrap()
            .for_type(Subperiod::Voting),
        stake_amount_1 + stake_amount_2
    );
    assert_eq!(acc_ledger.staked_future.unwrap().era, first_era + 1);
}

#[test]
fn account_ledger_add_stake_amount_invalid_era_or_period_fails() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    // Prep actions
    let first_era = 5;
    let period_1 = 2;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::Voting,
        subperiod_end_era: 100,
    };
    let lock_amount = 13;
    let stake_amount = 7;
    acc_ledger.add_lock_amount(lock_amount);
    assert!(acc_ledger
        .add_stake_amount(stake_amount, first_era, period_info_1)
        .is_ok());

    // Try to add to the next era, it should fail.
    assert_eq!(
        acc_ledger.add_stake_amount(1, first_era + 1, period_info_1),
        Err(AccountLedgerError::InvalidEra)
    );

    // Try to add to the next period, it should fail.
    assert_eq!(
        acc_ledger.add_stake_amount(
            1,
            first_era,
            PeriodInfo {
                number: period_1 + 1,
                subperiod: Subperiod::Voting,
                subperiod_end_era: 100
            }
        ),
        Err(AccountLedgerError::InvalidPeriod)
    );

    // Alternative situation - no future entry, only current era
    acc_ledger.staked = StakeAmount::new(0, stake_amount, first_era, period_1);
    acc_ledger.staked_future = None;

    assert_eq!(
        acc_ledger.add_stake_amount(1, first_era + 1, period_info_1),
        Err(AccountLedgerError::InvalidEra)
    );
    assert_eq!(
        acc_ledger.add_stake_amount(
            1,
            first_era,
            PeriodInfo {
                number: period_1 + 1,
                subperiod: Subperiod::Voting,
                subperiod_end_era: 100
            }
        ),
        Err(AccountLedgerError::InvalidPeriod)
    );
}

#[test]
fn account_ledger_add_stake_amount_too_large_amount_fails() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    // Sanity check
    assert_eq!(
        acc_ledger.add_stake_amount(
            10,
            1,
            PeriodInfo {
                number: 1,
                subperiod: Subperiod::Voting,
                subperiod_end_era: 100
            }
        ),
        Err(AccountLedgerError::UnavailableStakeFunds)
    );

    // Lock some amount, and try to stake more than that
    let first_era = 5;
    let period_1 = 2;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::Voting,
        subperiod_end_era: 100,
    };
    let lock_amount = 13;
    acc_ledger.add_lock_amount(lock_amount);
    assert_eq!(
        acc_ledger.add_stake_amount(lock_amount + 1, first_era, period_info_1),
        Err(AccountLedgerError::UnavailableStakeFunds)
    );

    // Additional check - have some active stake, and then try to overstake
    assert!(acc_ledger
        .add_stake_amount(lock_amount - 2, first_era, period_info_1)
        .is_ok());
    assert_eq!(
        acc_ledger.add_stake_amount(3, first_era, period_info_1),
        Err(AccountLedgerError::UnavailableStakeFunds)
    );
}

#[test]
fn account_ledger_unstake_amount_basic_scenario_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    // Prep actions
    let amount_1 = 19;
    let era_1 = 2;
    let period_1 = 1;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::BuildAndEarn,
        subperiod_end_era: 100,
    };
    acc_ledger.add_lock_amount(amount_1);

    let mut acc_ledger_2 = acc_ledger.clone();

    // 'Current' staked entry will remain empty.
    assert!(acc_ledger
        .add_stake_amount(amount_1, era_1, period_info_1)
        .is_ok());

    // Only 'current' entry has some values, future is set to None.
    acc_ledger_2.staked = StakeAmount::new(0, amount_1, era_1, period_1);
    acc_ledger_2.staked_future = None;

    for mut acc_ledger in vec![acc_ledger, acc_ledger_2] {
        // Sanity check
        assert!(acc_ledger.unstake_amount(0, era_1, period_info_1).is_ok());

        // 1st scenario - unstake some amount from the current era.
        let unstake_amount_1 = 3;
        assert!(acc_ledger
            .unstake_amount(unstake_amount_1, era_1, period_info_1)
            .is_ok());
        assert_eq!(
            acc_ledger.staked_amount(period_1),
            amount_1 - unstake_amount_1
        );

        // 2nd scenario - perform full unstake
        assert!(acc_ledger
            .unstake_amount(amount_1 - unstake_amount_1, era_1, period_info_1)
            .is_ok());
        assert!(acc_ledger.staked_amount(period_1).is_zero());
        assert!(acc_ledger.staked.is_empty());
        assert_eq!(acc_ledger.staked, StakeAmount::default());
        assert!(acc_ledger.staked_future.is_none());
    }
}
#[test]
fn account_ledger_unstake_amount_advanced_scenario_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    // Prep actions
    let amount_1 = 19;
    let era_1 = 2;
    let period_1 = 1;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::BuildAndEarn,
        subperiod_end_era: 100,
    };
    acc_ledger.add_lock_amount(amount_1);

    // We have two entries at once
    acc_ledger.staked = StakeAmount::new(amount_1 - 1, 0, era_1, period_1);
    acc_ledger.staked_future = Some(StakeAmount::new(amount_1 - 1, 1, era_1 + 1, period_1));

    // 1st scenario - unstake some amount from the current era, both entries should be affected.
    let unstake_amount_1 = 3;
    assert!(acc_ledger
        .unstake_amount(unstake_amount_1, era_1, period_info_1)
        .is_ok());
    assert_eq!(
        acc_ledger.staked_amount(period_1),
        amount_1 - unstake_amount_1
    );

    assert_eq!(
        acc_ledger.staked.for_type(Subperiod::Voting),
        amount_1 - 1 - 3
    );
    assert_eq!(
        acc_ledger
            .staked_future
            .unwrap()
            .for_type(Subperiod::Voting),
        amount_1 - 3
    );
    assert!(acc_ledger
        .staked_future
        .unwrap()
        .for_type(Subperiod::BuildAndEarn)
        .is_zero());

    // 2nd scenario - perform full unstake
    assert!(acc_ledger
        .unstake_amount(amount_1 - unstake_amount_1, era_1, period_info_1)
        .is_ok());
    assert!(acc_ledger.staked_amount(period_1).is_zero());
    assert_eq!(acc_ledger.staked, StakeAmount::default());
    assert!(acc_ledger.staked_future.is_none());

    // 3rd scenario - try to stake again, ensure it works
    let era_2 = era_1 + 7;
    let amount_2 = amount_1 - 5;
    assert!(acc_ledger
        .add_stake_amount(amount_2, era_2, period_info_1)
        .is_ok());
    assert_eq!(acc_ledger.staked_amount(period_1), amount_2);
    assert_eq!(acc_ledger.staked, StakeAmount::default());
    assert_eq!(
        acc_ledger
            .staked_future
            .unwrap()
            .for_type(Subperiod::BuildAndEarn),
        amount_2
    );
}

#[test]
fn account_ledger_unstake_from_invalid_era_fails() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    // Prep actions
    let amount_1 = 13;
    let era_1 = 2;
    let period_1 = 1;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::BuildAndEarn,
        subperiod_end_era: 100,
    };
    acc_ledger.add_lock_amount(amount_1);
    assert!(acc_ledger
        .add_stake_amount(amount_1, era_1, period_info_1)
        .is_ok());

    // Try to unstake from the next era, it should fail.
    assert_eq!(
        acc_ledger.unstake_amount(1, era_1 + 1, period_info_1),
        Err(AccountLedgerError::InvalidEra)
    );

    // Try to unstake from the next period, it should fail.
    assert_eq!(
        acc_ledger.unstake_amount(
            1,
            era_1,
            PeriodInfo {
                number: period_1 + 1,
                subperiod: Subperiod::Voting,
                subperiod_end_era: 100
            }
        ),
        Err(AccountLedgerError::InvalidPeriod)
    );

    // Alternative situation - no future entry, only current era
    acc_ledger.staked = StakeAmount::new(0, 1, era_1, period_1);
    acc_ledger.staked_future = None;

    assert_eq!(
        acc_ledger.unstake_amount(1, era_1 + 1, period_info_1),
        Err(AccountLedgerError::InvalidEra)
    );
    assert_eq!(
        acc_ledger.unstake_amount(
            1,
            era_1,
            PeriodInfo {
                number: period_1 + 1,
                subperiod: Subperiod::Voting,
                subperiod_end_era: 100
            }
        ),
        Err(AccountLedgerError::InvalidPeriod)
    );
}

#[test]
fn account_ledger_unstake_too_much_fails() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

    // Prep actions
    let amount_1 = 23;
    let era_1 = 2;
    let period_1 = 1;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::BuildAndEarn,
        subperiod_end_era: 100,
    };
    acc_ledger.add_lock_amount(amount_1);
    assert!(acc_ledger
        .add_stake_amount(amount_1, era_1, period_info_1)
        .is_ok());

    assert_eq!(
        acc_ledger.unstake_amount(amount_1 + 1, era_1, period_info_1),
        Err(AccountLedgerError::UnstakeAmountLargerThanStake)
    );
}

#[test]
fn account_ledger_unlockable_amount_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

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
    let period_info = PeriodInfo {
        number: stake_period,
        subperiod: Subperiod::Voting,
        subperiod_end_era: 100,
    };
    assert!(acc_ledger
        .add_stake_amount(stake_amount, lock_era, period_info)
        .is_ok());
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
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

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
    let mut acc_ledger = AccountLedger::<BlockNumber, UnlockingDummy>::default();

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
fn account_ledger_claim_up_to_era_works() {
    // TODO!!!
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
    era_info.add_stake_amount(vp_stake_amount, Subperiod::Voting);
    assert_eq!(era_info.total_staked_amount_next_era(), vp_stake_amount);
    assert_eq!(
        era_info.staked_amount_next_era(Subperiod::Voting),
        vp_stake_amount
    );
    assert!(
        era_info.total_staked_amount().is_zero(),
        "Calling stake makes it available only from the next era."
    );

    // Add some build&earn period stake
    let bep_stake_amount = 13;
    era_info.add_stake_amount(bep_stake_amount, Subperiod::BuildAndEarn);
    assert_eq!(
        era_info.total_staked_amount_next_era(),
        vp_stake_amount + bep_stake_amount
    );
    assert_eq!(
        era_info.staked_amount_next_era(Subperiod::BuildAndEarn),
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
    let period_number = 1;
    let era = 2;
    era_info.current_stake_amount =
        StakeAmount::new(vp_stake_amount, bep_stake_amount_1, era, period_number);
    era_info.next_stake_amount =
        StakeAmount::new(vp_stake_amount, bep_stake_amount_2, era + 1, period_number);
    let total_staked = era_info.total_staked_amount();
    let total_staked_next_era = era_info.total_staked_amount_next_era();

    // 1st scenario - unstake some amount, no overflow
    let unstake_amount_1 = bep_stake_amount_1;
    era_info.unstake_amount(unstake_amount_1, Subperiod::BuildAndEarn);

    // Current era
    assert_eq!(
        era_info.total_staked_amount(),
        total_staked - unstake_amount_1
    );
    assert_eq!(era_info.staked_amount(Subperiod::Voting), vp_stake_amount);
    assert!(era_info.staked_amount(Subperiod::BuildAndEarn).is_zero());

    // Next era
    assert_eq!(
        era_info.total_staked_amount_next_era(),
        total_staked_next_era - unstake_amount_1
    );
    assert_eq!(
        era_info.staked_amount_next_era(Subperiod::Voting),
        vp_stake_amount
    );
    assert_eq!(
        era_info.staked_amount_next_era(Subperiod::BuildAndEarn),
        bep_stake_amount_2 - unstake_amount_1
    );

    // 2nd scenario - unstake some more, but with overflow
    let overflow = 2;
    let unstake_amount_2 = bep_stake_amount_2 - unstake_amount_1 + overflow;
    era_info.unstake_amount(unstake_amount_2, Subperiod::BuildAndEarn);

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
        era_info.staked_amount_next_era(Subperiod::Voting),
        vp_stake_amount - overflow
    );
    assert!(era_info
        .staked_amount_next_era(Subperiod::BuildAndEarn)
        .is_zero());
}

#[test]
fn stake_amount_works() {
    let mut stake_amount = StakeAmount::default();

    // Sanity check
    assert!(stake_amount.total().is_zero());
    assert!(stake_amount.for_type(Subperiod::Voting).is_zero());
    assert!(stake_amount.for_type(Subperiod::BuildAndEarn).is_zero());

    // Stake some amount in voting period
    let vp_stake_1 = 11;
    stake_amount.add(vp_stake_1, Subperiod::Voting);
    assert_eq!(stake_amount.total(), vp_stake_1);
    assert_eq!(stake_amount.for_type(Subperiod::Voting), vp_stake_1);
    assert!(stake_amount.for_type(Subperiod::BuildAndEarn).is_zero());

    // Stake some amount in build&earn period
    let bep_stake_1 = 13;
    stake_amount.add(bep_stake_1, Subperiod::BuildAndEarn);
    assert_eq!(stake_amount.total(), vp_stake_1 + bep_stake_1);
    assert_eq!(stake_amount.for_type(Subperiod::Voting), vp_stake_1);
    assert_eq!(stake_amount.for_type(Subperiod::BuildAndEarn), bep_stake_1);

    // Unstake some amount from voting period
    let vp_unstake_1 = 5;
    stake_amount.subtract(5, Subperiod::Voting);
    assert_eq!(
        stake_amount.total(),
        vp_stake_1 + bep_stake_1 - vp_unstake_1
    );
    assert_eq!(
        stake_amount.for_type(Subperiod::Voting),
        vp_stake_1 - vp_unstake_1
    );
    assert_eq!(stake_amount.for_type(Subperiod::BuildAndEarn), bep_stake_1);

    // Unstake some amount from build&earn period
    let bep_unstake_1 = 2;
    stake_amount.subtract(bep_unstake_1, Subperiod::BuildAndEarn);
    assert_eq!(
        stake_amount.total(),
        vp_stake_1 + bep_stake_1 - vp_unstake_1 - bep_unstake_1
    );
    assert_eq!(
        stake_amount.for_type(Subperiod::Voting),
        vp_stake_1 - vp_unstake_1
    );
    assert_eq!(
        stake_amount.for_type(Subperiod::BuildAndEarn),
        bep_stake_1 - bep_unstake_1
    );

    // Unstake some more from build&earn period, and chip away from the voting period
    let total_stake = vp_stake_1 + bep_stake_1 - vp_unstake_1 - bep_unstake_1;
    let bep_unstake_2 = bep_stake_1 - bep_unstake_1 + 1;
    stake_amount.subtract(bep_unstake_2, Subperiod::BuildAndEarn);
    assert_eq!(stake_amount.total(), total_stake - bep_unstake_2);
    assert_eq!(
        stake_amount.for_type(Subperiod::Voting),
        vp_stake_1 - vp_unstake_1 - 1
    );
    assert!(stake_amount.for_type(Subperiod::BuildAndEarn).is_zero());
}

#[test]
fn singular_staking_info_basics_are_ok() {
    let period_number = 3;
    let subperiod = Subperiod::Voting;
    let mut staking_info = SingularStakingInfo::new(period_number, subperiod);

    // Sanity checks
    assert_eq!(staking_info.period_number(), period_number);
    assert!(staking_info.is_loyal());
    assert!(staking_info.total_staked_amount().is_zero());
    assert!(!SingularStakingInfo::new(period_number, Subperiod::BuildAndEarn).is_loyal());

    // Add some staked amount during `Voting` period
    let vote_stake_amount_1 = 11;
    staking_info.stake(vote_stake_amount_1, Subperiod::Voting);
    assert_eq!(staking_info.total_staked_amount(), vote_stake_amount_1);
    assert_eq!(
        staking_info.staked_amount(Subperiod::Voting),
        vote_stake_amount_1
    );
    assert!(staking_info
        .staked_amount(Subperiod::BuildAndEarn)
        .is_zero());

    // Add some staked amount during `BuildAndEarn` period
    let bep_stake_amount_1 = 23;
    staking_info.stake(bep_stake_amount_1, Subperiod::BuildAndEarn);
    assert_eq!(
        staking_info.total_staked_amount(),
        vote_stake_amount_1 + bep_stake_amount_1
    );
    assert_eq!(
        staking_info.staked_amount(Subperiod::Voting),
        vote_stake_amount_1
    );
    assert_eq!(
        staking_info.staked_amount(Subperiod::BuildAndEarn),
        bep_stake_amount_1
    );
}

#[test]
fn singular_staking_info_unstake_during_voting_is_ok() {
    let period_number = 3;
    let subperiod = Subperiod::Voting;
    let mut staking_info = SingularStakingInfo::new(period_number, subperiod);

    // Prep actions
    let vote_stake_amount_1 = 11;
    staking_info.stake(vote_stake_amount_1, Subperiod::Voting);

    // Unstake some amount during `Voting` period, loyalty should remain as expected.
    let unstake_amount_1 = 5;
    assert_eq!(
        staking_info.unstake(unstake_amount_1, Subperiod::Voting),
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
        staking_info.unstake(remaining_stake + 1, Subperiod::Voting),
        (remaining_stake, Balance::zero())
    );
    assert!(staking_info.total_staked_amount().is_zero());
    assert!(staking_info.is_loyal());
}

#[test]
fn singular_staking_info_unstake_during_bep_is_ok() {
    let period_number = 3;
    let subperiod = Subperiod::Voting;
    let mut staking_info = SingularStakingInfo::new(period_number, subperiod);

    // Prep actions
    let vote_stake_amount_1 = 11;
    staking_info.stake(vote_stake_amount_1, Subperiod::Voting);
    let bep_stake_amount_1 = 23;
    staking_info.stake(bep_stake_amount_1, Subperiod::BuildAndEarn);

    // 1st scenario - Unstake some of the amount staked during B&E period
    let unstake_1 = 5;
    assert_eq!(
        staking_info.unstake(5, Subperiod::BuildAndEarn),
        (Balance::zero(), unstake_1)
    );
    assert_eq!(
        staking_info.total_staked_amount(),
        vote_stake_amount_1 + bep_stake_amount_1 - unstake_1
    );
    assert_eq!(
        staking_info.staked_amount(Subperiod::Voting),
        vote_stake_amount_1
    );
    assert_eq!(
        staking_info.staked_amount(Subperiod::BuildAndEarn),
        bep_stake_amount_1 - unstake_1
    );
    assert!(staking_info.is_loyal());

    // 2nd scenario - unstake all of the amount staked during B&E period, and then some more.
    // The point is to take a chunk from the voting period stake too.
    let current_total_stake = staking_info.total_staked_amount();
    let current_bep_stake = staking_info.staked_amount(Subperiod::BuildAndEarn);
    let voting_stake_overflow = 2;
    let unstake_2 = current_bep_stake + voting_stake_overflow;

    assert_eq!(
        staking_info.unstake(unstake_2, Subperiod::BuildAndEarn),
        (voting_stake_overflow, current_bep_stake)
    );
    assert_eq!(
        staking_info.total_staked_amount(),
        current_total_stake - unstake_2
    );
    assert_eq!(
        staking_info.staked_amount(Subperiod::Voting),
        vote_stake_amount_1 - voting_stake_overflow
    );
    assert!(staking_info
        .staked_amount(Subperiod::BuildAndEarn)
        .is_zero());
    assert!(
        !staking_info.is_loyal(),
        "Loyalty flag should have been removed due to non-zero voting period unstake"
    );
}

#[test]
fn contract_stake_info_get_works() {
    let info_1 = StakeAmount::new(0, 0, 4, 2);
    let info_2 = StakeAmount::new(11, 0, 7, 3);

    let contract_stake = ContractStakeAmount {
        staked: info_1,
        staked_future: Some(info_2),
    };

    // Sanity check
    assert!(!contract_stake.is_empty());

    // 1st scenario - get existing entries
    assert_eq!(contract_stake.get(4, 2), Some(info_1));
    assert_eq!(contract_stake.get(7, 3), Some(info_2));

    // 2nd scenario - get non-existing entries for covered eras
    {
        let era_1 = 6;
        let entry_1 = contract_stake.get(era_1, 2).expect("Has to be Some");
        assert!(entry_1.total().is_zero());
        assert_eq!(entry_1.era, era_1);
        assert_eq!(entry_1.period, 2);

        let era_2 = 8;
        let entry_1 = contract_stake.get(era_2, 3).expect("Has to be Some");
        assert_eq!(entry_1.total(), 11);
        assert_eq!(entry_1.era, era_2);
        assert_eq!(entry_1.period, 3);
    }

    // 3rd scenario - get non-existing entries for covered eras but mismatching period
    assert!(contract_stake.get(8, 2).is_none());

    // 4th scenario - get non-existing entries for non-covered eras
    assert!(contract_stake.get(3, 2).is_none());
}

#[test]
fn contract_stake_info_stake_is_ok() {
    let mut contract_stake = ContractStakeAmount::default();

    // 1st scenario - stake some amount and verify state change
    let era_1 = 3;
    let stake_era_1 = era_1 + 1;
    let period_1 = 5;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::Voting,
        subperiod_end_era: 20,
    };
    let amount_1 = 31;
    contract_stake.stake(amount_1, period_info_1, era_1);
    assert!(!contract_stake.is_empty());

    assert!(
        contract_stake.get(era_1, period_1).is_none(),
        "Entry for current era must not exist."
    );
    let entry_1_1 = contract_stake.get(stake_era_1, period_1).unwrap();
    assert_eq!(
        entry_1_1.era, stake_era_1,
        "Stake is only valid from next era."
    );
    assert_eq!(entry_1_1.total(), amount_1);

    // 2nd scenario - stake some more to the same era but different period type, and verify state change.
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::BuildAndEarn,
        subperiod_end_era: 20,
    };
    contract_stake.stake(amount_1, period_info_1, era_1);
    let entry_1_2 = contract_stake.get(stake_era_1, period_1).unwrap();
    assert_eq!(entry_1_2.era, stake_era_1);
    assert_eq!(entry_1_2.total(), amount_1 * 2);

    // 3rd scenario - stake more to the next era, while still in the same period.
    let era_2 = era_1 + 2;
    let stake_era_2 = era_2 + 1;
    let amount_2 = 37;
    contract_stake.stake(amount_2, period_info_1, era_2);
    let entry_2_1 = contract_stake.get(stake_era_1, period_1).unwrap();
    let entry_2_2 = contract_stake.get(stake_era_2, period_1).unwrap();
    assert_eq!(entry_2_1, entry_1_2, "Old entry must remain unchanged.");
    assert_eq!(entry_2_2.era, stake_era_2);
    assert_eq!(entry_2_2.period, period_1);
    assert_eq!(
        entry_2_2.total(),
        entry_2_1.total() + amount_2,
        "Since it's the same period, stake amount must carry over from the previous entry."
    );

    // 4th scenario - stake some more to the next era, but this time also bump the period.
    let era_3 = era_2 + 3;
    let stake_era_3 = era_3 + 1;
    let period_2 = period_1 + 1;
    let period_info_2 = PeriodInfo {
        number: period_2,
        subperiod: Subperiod::BuildAndEarn,
        subperiod_end_era: 20,
    };
    let amount_3 = 41;

    contract_stake.stake(amount_3, period_info_2, era_3);
    assert!(
        contract_stake.get(stake_era_1, period_1).is_none(),
        "Old period must be removed."
    );
    assert!(
        contract_stake.get(stake_era_2, period_1).is_none(),
        "Old period must be removed."
    );
    let entry_3_1 = contract_stake.get(stake_era_3, period_2).unwrap();
    assert_eq!(entry_3_1.era, stake_era_3);
    assert_eq!(entry_3_1.period, period_2);
    assert_eq!(
        entry_3_1.total(),
        amount_3,
        "No carry over from previous entry since period has changed."
    );

    // 5th scenario - stake to the next era
    let era_4 = era_3 + 1;
    let stake_era_4 = era_4 + 1;
    let amount_4 = 5;
    contract_stake.stake(amount_4, period_info_2, era_4);
    let entry_4_1 = contract_stake.get(stake_era_3, period_2).unwrap();
    let entry_4_2 = contract_stake.get(stake_era_4, period_2).unwrap();
    assert_eq!(entry_4_1, entry_3_1, "Old entry must remain unchanged.");
    assert_eq!(entry_4_2.era, stake_era_4);
    assert_eq!(entry_4_2.period, period_2);
    assert_eq!(entry_4_2.total(), amount_3 + amount_4);
}

#[test]
fn contract_stake_info_unstake_is_ok() {
    let mut contract_stake = ContractStakeAmount::default();

    // Prep action - create a stake entry
    let era_1 = 2;
    let period = 3;
    let period_info = PeriodInfo {
        number: period,
        subperiod: Subperiod::Voting,
        subperiod_end_era: 20,
    };
    let stake_amount = 100;
    contract_stake.stake(stake_amount, period_info, era_1);

    // 1st scenario - unstake in the same era
    let amount_1 = 5;
    contract_stake.unstake(amount_1, period_info, era_1);
    assert_eq!(
        contract_stake.total_staked_amount(period),
        stake_amount - amount_1
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::Voting),
        stake_amount - amount_1
    );

    // 2nd scenario - unstake in the future era, entries should be aligned to the current era
    let period_info = PeriodInfo {
        number: period,
        subperiod: Subperiod::BuildAndEarn,
        subperiod_end_era: 40,
    };
    let era_2 = era_1 + 3;
    let amount_2 = 7;
    contract_stake.unstake(amount_2, period_info, era_2);
    assert_eq!(
        contract_stake.total_staked_amount(period),
        stake_amount - amount_1 - amount_2
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::Voting),
        stake_amount - amount_1 - amount_2
    );
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
    let era_reward_1 = EraReward {
        staker_reward_pool: 23,
        staked: 41,
        dapp_reward_pool: 17,
    };
    assert!(era_reward_span.push(era_1, era_reward_1).is_ok());
    assert_eq!(era_reward_span.len(), 1);
    assert_eq!(era_reward_span.first_era(), era_1);
    assert_eq!(era_reward_span.last_era(), era_1);

    // Insert another value and verify state change
    let era_2 = era_1 + 1;
    let era_reward_2 = EraReward {
        staker_reward_pool: 37,
        staked: 53,
        dapp_reward_pool: 19,
    };
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
    let era_reward = EraReward {
        staker_reward_pool: 23,
        staked: 41,
        dapp_reward_pool: 17,
    };
    assert!(era_reward_span.push(era_1, era_reward).is_ok());

    // Attempting to push incorrect era results in an error
    for wrong_era in &[era_1 - 1, era_1, era_1 + 2] {
        assert_eq!(
            era_reward_span.push(*wrong_era, era_reward),
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

#[test]
fn tier_slot_configuration_basic_tests() {
    // TODO: this should be expanded & improved later
    get_u32_type!(TiersNum, 4);
    let params = TierParameters::<TiersNum> {
        reward_portion: BoundedVec::try_from(vec![
            Permill::from_percent(40),
            Permill::from_percent(30),
            Permill::from_percent(20),
            Permill::from_percent(10),
        ])
        .unwrap(),
        slot_distribution: BoundedVec::try_from(vec![
            Permill::from_percent(10),
            Permill::from_percent(20),
            Permill::from_percent(30),
            Permill::from_percent(40),
        ])
        .unwrap(),
        tier_thresholds: BoundedVec::try_from(vec![
            TierThreshold::DynamicTvlAmount {
                amount: 1000,
                minimum_amount: 800,
            },
            TierThreshold::DynamicTvlAmount {
                amount: 500,
                minimum_amount: 350,
            },
            TierThreshold::DynamicTvlAmount {
                amount: 100,
                minimum_amount: 70,
            },
            TierThreshold::FixedTvlAmount { amount: 50 },
        ])
        .unwrap(),
    };
    assert!(params.is_valid(), "Example params must be valid!");

    // Create a configuration with some values
    let init_config = TiersConfiguration::<TiersNum> {
        number_of_slots: 100,
        slots_per_tier: BoundedVec::try_from(vec![10, 20, 30, 40]).unwrap(),
        reward_portion: params.reward_portion.clone(),
        tier_thresholds: params.tier_thresholds.clone(),
    };
    assert!(init_config.is_valid(), "Init config must be valid!");

    // Create a new config, based on a new price
    let new_price = FixedU64::from_rational(20, 100); // in production will be expressed in USD
    let new_config = init_config.calculate_new(new_price, &params);
    assert!(new_config.is_valid());

    // TODO: expand tests, add more sanity checks (e.g. tier 3 requirement should never be lower than tier 4, etc.)
}
