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

use astar_primitives::Balance;
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
    let next_subperiod_start_era = 5;
    let info = PeriodInfo {
        number: period_number,
        subperiod: Subperiod::Voting,
        next_subperiod_start_era: next_subperiod_start_era,
    };

    // Sanity checks
    assert_eq!(info.number, period_number);
    assert_eq!(info.subperiod, Subperiod::Voting);
    assert_eq!(info.next_subperiod_start_era, next_subperiod_start_era);

    // Voting period checks
    assert!(!info.is_next_period(next_subperiod_start_era - 1));
    assert!(!info.is_next_period(next_subperiod_start_era));
    assert!(!info.is_next_period(next_subperiod_start_era + 1));
    for era in vec![
        next_subperiod_start_era - 1,
        next_subperiod_start_era,
        next_subperiod_start_era + 1,
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
        next_subperiod_start_era: next_subperiod_start_era,
    };
    assert!(!info.is_next_period(next_subperiod_start_era - 1));
    assert!(info.is_next_period(next_subperiod_start_era));
    assert!(info.is_next_period(next_subperiod_start_era + 1));
}

#[test]
fn protocol_state_default() {
    let protocol_state = ProtocolState::default();

    assert_eq!(protocol_state.era, 0);
    assert_eq!(
        protocol_state.next_era_start, 1,
        "Era should start immediately on the first block"
    );
}

#[test]
fn protocol_state_basic_checks() {
    let mut protocol_state = ProtocolState::default();
    let period_number = 5;
    let next_subperiod_start_era = 11;
    let next_era_start = 31;
    protocol_state.period_info = PeriodInfo {
        number: period_number,
        subperiod: Subperiod::Voting,
        next_subperiod_start_era: next_subperiod_start_era,
    };
    protocol_state.next_era_start = next_era_start;

    assert_eq!(protocol_state.period_number(), period_number);
    assert_eq!(protocol_state.subperiod(), Subperiod::Voting);

    // New era check
    assert!(!protocol_state.is_new_era(next_era_start - 1));
    assert!(protocol_state.is_new_era(next_era_start));
    assert!(protocol_state.is_new_era(next_era_start + 1));

    // Toggle new period type check - 'Voting' to 'BuildAndEarn'
    let next_subperiod_start_era_1 = 23;
    let next_era_start_1 = 41;
    protocol_state.advance_to_next_subperiod(next_subperiod_start_era_1, next_era_start_1);
    assert_eq!(protocol_state.subperiod(), Subperiod::BuildAndEarn);
    assert_eq!(
        protocol_state.period_number(),
        period_number,
        "Switching from 'Voting' to 'BuildAndEarn' should not trigger period bump."
    );
    assert_eq!(
        protocol_state.next_subperiod_start_era(),
        next_subperiod_start_era_1
    );
    assert!(!protocol_state.is_new_era(next_era_start_1 - 1));
    assert!(protocol_state.is_new_era(next_era_start_1));

    // Toggle from 'BuildAndEarn' over to 'Voting'
    let next_subperiod_start_era_2 = 24;
    let next_era_start_2 = 91;
    protocol_state.advance_to_next_subperiod(next_subperiod_start_era_2, next_era_start_2);
    assert_eq!(protocol_state.subperiod(), Subperiod::Voting);
    assert_eq!(
        protocol_state.period_number(),
        period_number + 1,
        "Switching from 'BuildAndEarn' to 'Voting' must trigger period bump."
    );
    assert_eq!(
        protocol_state.next_subperiod_start_era(),
        next_subperiod_start_era_2
    );
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

    // Check if dApp is registered
    assert!(dapp_info.is_registered());

    dapp_info.state = DAppState::Unregistered(10);
    assert!(!dapp_info.is_registered());
}

#[test]
fn unlocking_chunk_basic_check() {
    // Sanity check
    let unlocking_chunk = UnlockingChunk::default();
    assert!(unlocking_chunk.amount.is_zero());
    assert!(unlocking_chunk.unlock_block.is_zero());
}

#[test]
fn account_ledger_default() {
    get_u32_type!(UnlockingDummy, 5);
    let acc_ledger = AccountLedger::<UnlockingDummy>::default();

    assert!(acc_ledger.is_empty());
    assert!(acc_ledger.active_locked_amount().is_zero());
}

#[test]
fn account_ledger_add_lock_amount_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

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
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

    // Sanity check scenario
    // Cannot reduce if there is nothing locked, should be a noop
    acc_ledger.subtract_lock_amount(0);
    acc_ledger.subtract_lock_amount(10);
    assert!(acc_ledger.is_empty());

    // First basic scenario
    // Add some lock amount, then reduce it
    let lock_amount_1 = 19;
    let unlock_amount = 7;
    acc_ledger.add_lock_amount(lock_amount_1);
    acc_ledger.subtract_lock_amount(unlock_amount);
    assert_eq!(
        acc_ledger.total_locked_amount(),
        lock_amount_1 - unlock_amount
    );
    assert_eq!(
        acc_ledger.active_locked_amount(),
        lock_amount_1 - unlock_amount
    );
    assert_eq!(acc_ledger.unlocking_amount(), 0);

    // Second basic scenario
    let lock_amount_1 = lock_amount_1 - unlock_amount;
    let lock_amount_2 = 31;
    acc_ledger.add_lock_amount(lock_amount_2 - lock_amount_1);
    assert_eq!(acc_ledger.active_locked_amount(), lock_amount_2);

    // Subtract from the first era and verify state is as expected
    acc_ledger.subtract_lock_amount(unlock_amount);
    assert_eq!(
        acc_ledger.active_locked_amount(),
        lock_amount_2 - unlock_amount
    );
}

#[test]
fn account_ledger_add_unlocking_chunk_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

    // Base sanity check
    let default_unlocking_chunk = UnlockingChunk::default();
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
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

    // Sanity check
    assert!(acc_ledger.staked_amount(0).is_zero());
    assert!(acc_ledger.staked_amount(1).is_zero());

    // Period matches
    let amount_1 = 29;
    let period = 5;
    acc_ledger.staked = StakeAmount {
        voting: amount_1,
        build_and_earn: 0,
        era: 1,
        period,
    };
    assert_eq!(acc_ledger.staked_amount(period), amount_1);

    // Period doesn't match
    assert!(acc_ledger.staked_amount(period - 1).is_zero());
    assert!(acc_ledger.staked_amount(period + 1).is_zero());

    // Add future entry
    let amount_2 = 17;
    acc_ledger.staked_future = Some(StakeAmount {
        voting: 0,
        build_and_earn: amount_2,
        era: 2,
        period,
    });
    assert_eq!(acc_ledger.staked_amount(period), amount_2);
    assert!(acc_ledger.staked_amount(period - 1).is_zero());
    assert!(acc_ledger.staked_amount(period + 1).is_zero());
}

#[test]
fn account_ledger_staked_amount_for_type_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

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
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

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
    let era_1 = 1;
    let staked_amount = 7;
    acc_ledger.staked = StakeAmount {
        voting: 0,
        build_and_earn: staked_amount,
        era: era_1,
        period: period_1,
    };

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
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

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
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

    // Sanity check
    let period_number = 2;
    assert!(acc_ledger
        .add_stake_amount(
            0,
            0,
            PeriodInfo {
                number: period_number,
                subperiod: Subperiod::Voting,
                next_subperiod_start_era: 0
            }
        )
        .is_ok());
    assert!(acc_ledger.staked.is_empty());
    assert!(acc_ledger.staked_future.is_none());

    // 1st scenario - stake some amount in Voting period, and ensure values are as expected.
    let era_1 = 1;
    let period_1 = 1;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::Voting,
        next_subperiod_start_era: 100,
    };
    let lock_amount = 17;
    let stake_amount = 11;
    acc_ledger.add_lock_amount(lock_amount);

    assert!(acc_ledger
        .add_stake_amount(stake_amount, era_1, period_info_1)
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
        next_subperiod_start_era: 100,
    };
    let era_2 = era_1 + 1;
    assert!(acc_ledger.add_stake_amount(1, era_2, period_info_2).is_ok());
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
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

    // 1st scenario - stake some amount, and ensure values are as expected.
    let era_1 = 1;
    let period_1 = 1;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::Voting,
        next_subperiod_start_era: 100,
    };
    let lock_amount = 17;
    let stake_amount_1 = 11;
    acc_ledger.add_lock_amount(lock_amount);

    // We only have entry for the current era
    acc_ledger.staked = StakeAmount {
        voting: stake_amount_1,
        build_and_earn: 0,
        era: era_1,
        period: period_1,
    };

    let stake_amount_2 = 2;
    let acc_ledger_snapshot = acc_ledger.clone();
    assert!(acc_ledger
        .add_stake_amount(stake_amount_2, era_1, period_info_1)
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
    assert_eq!(acc_ledger.staked_future.unwrap().era, era_1 + 1);
}

#[test]
fn account_ledger_add_stake_amount_invalid_era_or_period_fails() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

    // Prep actions
    let era_1 = 5;
    let period_1 = 2;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::Voting,
        next_subperiod_start_era: 100,
    };
    let lock_amount = 13;
    let stake_amount = 7;
    acc_ledger.add_lock_amount(lock_amount);
    assert!(acc_ledger
        .add_stake_amount(stake_amount, era_1, period_info_1)
        .is_ok());

    // Try to add to era after next, it should fail.
    assert_eq!(
        acc_ledger.add_stake_amount(1, era_1 + 2, period_info_1),
        Err(AccountLedgerError::InvalidEra)
    );

    // Try to add to the next period, it should fail.
    assert_eq!(
        acc_ledger.add_stake_amount(
            1,
            era_1,
            PeriodInfo {
                number: period_1 + 1,
                subperiod: Subperiod::Voting,
                next_subperiod_start_era: 100
            }
        ),
        Err(AccountLedgerError::InvalidPeriod)
    );

    // Alternative situation - no future entry, only current era
    acc_ledger.staked = StakeAmount {
        voting: 0,
        build_and_earn: stake_amount,
        era: era_1,
        period: period_1,
    };
    acc_ledger.staked_future = None;

    assert_eq!(
        acc_ledger.add_stake_amount(1, era_1 + 1, period_info_1),
        Err(AccountLedgerError::InvalidEra)
    );
    assert_eq!(
        acc_ledger.add_stake_amount(
            1,
            era_1,
            PeriodInfo {
                number: period_1 + 1,
                subperiod: Subperiod::Voting,
                next_subperiod_start_era: 100
            }
        ),
        Err(AccountLedgerError::InvalidPeriod)
    );
}

#[test]
fn account_ledger_add_stake_amount_too_large_amount_fails() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

    // Sanity check
    assert_eq!(
        acc_ledger.add_stake_amount(
            10,
            1,
            PeriodInfo {
                number: 1,
                subperiod: Subperiod::Voting,
                next_subperiod_start_era: 100
            }
        ),
        Err(AccountLedgerError::UnavailableStakeFunds)
    );

    // Lock some amount, and try to stake more than that
    let era_1 = 5;
    let period_1 = 2;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::Voting,
        next_subperiod_start_era: 100,
    };
    let lock_amount = 13;
    acc_ledger.add_lock_amount(lock_amount);
    assert_eq!(
        acc_ledger.add_stake_amount(lock_amount + 1, era_1, period_info_1),
        Err(AccountLedgerError::UnavailableStakeFunds)
    );

    // Additional check - have some active stake, and then try to overstake
    assert!(acc_ledger
        .add_stake_amount(lock_amount - 2, era_1, period_info_1)
        .is_ok());
    assert_eq!(
        acc_ledger.add_stake_amount(3, era_1, period_info_1),
        Err(AccountLedgerError::UnavailableStakeFunds)
    );
}

#[test]
fn account_ledger_unstake_amount_basic_scenario_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

    // Prep actions
    let amount_1 = 19;
    let era_1 = 2;
    let period_1 = 1;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::BuildAndEarn,
        next_subperiod_start_era: 100,
    };
    acc_ledger.add_lock_amount(amount_1);

    let mut acc_ledger_2 = acc_ledger.clone();

    // 'Current' staked entry will remain empty.
    assert!(acc_ledger
        .add_stake_amount(amount_1, era_1, period_info_1)
        .is_ok());

    // Only 'current' entry has some values, future is set to None.
    acc_ledger_2.staked = StakeAmount {
        voting: 0,
        build_and_earn: amount_1,
        era: era_1,
        period: period_1,
    };
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
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

    // Prep actions
    let amount_1 = 19;
    let era_1 = 2;
    let period_1 = 1;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::BuildAndEarn,
        next_subperiod_start_era: 100,
    };
    acc_ledger.add_lock_amount(amount_1);

    // We have two entries at once
    acc_ledger.staked = StakeAmount {
        voting: amount_1 - 1,
        build_and_earn: 0,
        era: era_1,
        period: period_1,
    };
    acc_ledger.staked_future = Some(StakeAmount {
        voting: amount_1 - 1,
        build_and_earn: 1,
        era: era_1 + 1,
        period: period_1,
    });

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
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

    // Prep actions
    let amount_1 = 13;
    let era_1 = 2;
    let period_1 = 1;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::BuildAndEarn,
        next_subperiod_start_era: 100,
    };
    acc_ledger.add_lock_amount(amount_1);
    assert!(acc_ledger
        .add_stake_amount(amount_1, era_1, period_info_1)
        .is_ok());

    // Try to unstake from the current & next era, it should work.
    assert!(acc_ledger.unstake_amount(1, era_1, period_info_1).is_ok());
    assert!(acc_ledger
        .unstake_amount(1, era_1 + 1, period_info_1)
        .is_ok());

    // Try to unstake from the stake era + 2, it should fail since it would mean we have unclaimed rewards.
    assert_eq!(
        acc_ledger.unstake_amount(1, era_1 + 2, period_info_1),
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
                next_subperiod_start_era: 100
            }
        ),
        Err(AccountLedgerError::InvalidPeriod)
    );

    // Alternative situation - no future entry, only current era
    acc_ledger.staked = StakeAmount {
        voting: 0,
        build_and_earn: 1,
        era: era_1,
        period: period_1,
    };
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
                next_subperiod_start_era: 100
            }
        ),
        Err(AccountLedgerError::InvalidPeriod)
    );
}

#[test]
fn account_ledger_unstake_too_much_fails() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

    // Prep actions
    let amount_1 = 23;
    let era_1 = 2;
    let period_1 = 1;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::BuildAndEarn,
        next_subperiod_start_era: 100,
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
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

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
        next_subperiod_start_era: 100,
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
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

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
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

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
fn account_ledger_expired_cleanup_works() {
    get_u32_type!(UnlockingDummy, 5);

    // 1st scenario - nothing is expired
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();
    acc_ledger.staked = StakeAmount {
        voting: 3,
        build_and_earn: 7,
        era: 100,
        period: 5,
    };
    acc_ledger.staked_future = Some(StakeAmount {
        voting: 3,
        build_and_earn: 13,
        era: 101,
        period: 5,
    });

    let acc_ledger_snapshot = acc_ledger.clone();

    assert!(!acc_ledger.maybe_cleanup_expired(acc_ledger.staked.period - 1));
    assert_eq!(
        acc_ledger, acc_ledger_snapshot,
        "No change must happen since period hasn't expired."
    );

    assert!(!acc_ledger.maybe_cleanup_expired(acc_ledger.staked.period));
    assert_eq!(
        acc_ledger, acc_ledger_snapshot,
        "No change must happen since period hasn't expired."
    );

    // 2nd scenario - stake has expired
    assert!(acc_ledger.maybe_cleanup_expired(acc_ledger.staked.period + 1));
    assert!(acc_ledger.staked.is_empty());
    assert!(acc_ledger.staked_future.is_none());
}

#[test]
fn account_ledger_claim_up_to_era_only_staked_without_cleanup_works() {
    get_u32_type!(UnlockingDummy, 5);
    let stake_era = 100;

    let acc_ledger_snapshot = {
        let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();
        acc_ledger.staked = StakeAmount {
            voting: 3,
            build_and_earn: 7,
            era: stake_era,
            period: 5,
        };
        acc_ledger
    };

    // 1st scenario - claim one era, period hasn't ended yet
    {
        let mut acc_ledger = acc_ledger_snapshot.clone();
        let mut result_iter = acc_ledger
            .claim_up_to_era(stake_era, None)
            .expect("Must provide iter with exactly one era.");

        // Iter values are correct
        assert_eq!(
            result_iter.next(),
            Some((stake_era, acc_ledger_snapshot.staked.total()))
        );
        assert!(result_iter.next().is_none());

        // Ledger values are correct
        let mut expected_stake_amount = acc_ledger_snapshot.staked;
        expected_stake_amount.era += 1;
        assert_eq!(
            acc_ledger.staked, expected_stake_amount,
            "Only era should be bumped by 1."
        );
        assert!(
            acc_ledger.staked_future.is_none(),
            "Was and should remain None."
        );
    }

    // 2nd scenario - claim multiple eras (5), period hasn't ended yet
    {
        let mut acc_ledger = acc_ledger_snapshot.clone();
        let mut result_iter = acc_ledger
            .claim_up_to_era(stake_era + 4, None) // staked era + 4 additional eras
            .expect("Must provide iter with 5 values.");

        // Iter values are correct
        for inc in 0..5 {
            assert_eq!(
                result_iter.next(),
                Some((stake_era + inc, acc_ledger_snapshot.staked.total()))
            );
        }
        assert!(result_iter.next().is_none());

        // Ledger values are correct
        let mut expected_stake_amount = acc_ledger_snapshot.staked;
        expected_stake_amount.era += 5;
        assert_eq!(
            acc_ledger.staked, expected_stake_amount,
            "Only era should be bumped by 5."
        );
        assert!(
            acc_ledger.staked_future.is_none(),
            "Was and should remain None."
        );
    }
}

#[test]
fn account_ledger_claim_up_to_era_only_staked_with_cleanup_works() {
    get_u32_type!(UnlockingDummy, 5);
    let stake_era = 100;

    let acc_ledger_snapshot = {
        let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();
        acc_ledger.staked = StakeAmount {
            voting: 3,
            build_and_earn: 7,
            era: stake_era,
            period: 5,
        };
        acc_ledger
    };

    // 1st scenario - claim one era, period has ended
    {
        let mut acc_ledger = acc_ledger_snapshot.clone();
        let mut result_iter = acc_ledger
            .claim_up_to_era(stake_era, Some(stake_era))
            .expect("Must provide iter with exactly one era.");

        // Iter values are correct
        assert_eq!(
            result_iter.next(),
            Some((stake_era, acc_ledger_snapshot.staked.total()))
        );
        assert!(result_iter.next().is_none());

        // Ledger values are cleaned up
        assert!(
            acc_ledger.staked.is_empty(),
            "Period has ended so stake entry should be cleaned up."
        );
        assert!(
            acc_ledger.staked_future.is_none(),
            "Was and should remain None."
        );
    }

    // 2nd scenario - claim multiple eras (5), period has ended
    {
        let mut acc_ledger = acc_ledger_snapshot.clone();
        let mut result_iter = acc_ledger
            .claim_up_to_era(stake_era + 4, Some(stake_era)) // staked era + 4 additional eras
            .expect("Must provide iter with 5 values.");

        for inc in 0..5 {
            assert_eq!(
                result_iter.next(),
                Some((stake_era + inc, acc_ledger_snapshot.staked.total()))
            );
        }
        assert!(result_iter.next().is_none());

        // Ledger values are cleaned up
        assert!(
            acc_ledger.staked.is_empty(),
            "Period has ended so stake entry should be cleaned up."
        );
        assert!(
            acc_ledger.staked_future.is_none(),
            "Was and should remain None."
        );
    }

    // 3rd scenario - claim one era, period has ended in some future era
    {
        let mut acc_ledger = acc_ledger_snapshot.clone();
        let mut result_iter = acc_ledger
            .claim_up_to_era(stake_era, Some(stake_era + 1))
            .expect("Must provide iter with exactly one era.");

        // Iter values are correct
        assert_eq!(
            result_iter.next(),
            Some((stake_era, acc_ledger_snapshot.staked.total()))
        );
        assert!(result_iter.next().is_none());

        // Ledger values are correctly updated
        let mut expected_stake_amount = acc_ledger_snapshot.staked;
        expected_stake_amount.era += 1;
        assert_eq!(
            acc_ledger.staked, expected_stake_amount,
            "Entry must exist since we still haven't reached the period end era."
        );
        assert!(
            acc_ledger.staked_future.is_none(),
            "Was and should remain None."
        );
    }
}

#[test]
fn account_ledger_claim_up_to_era_only_staked_future_without_cleanup_works() {
    get_u32_type!(UnlockingDummy, 5);
    let stake_era = 50;

    let acc_ledger_snapshot = {
        let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();
        acc_ledger.staked_future = Some(StakeAmount {
            voting: 5,
            build_and_earn: 11,
            era: stake_era,
            period: 4,
        });
        acc_ledger
    };

    // 1st scenario - claim one era, period hasn't ended yet
    {
        let mut acc_ledger = acc_ledger_snapshot.clone();
        let mut result_iter = acc_ledger
            .claim_up_to_era(stake_era, None)
            .expect("Must provide iter with exactly one era.");

        // Iter values are correct
        assert_eq!(
            result_iter.next(),
            Some((
                stake_era,
                acc_ledger_snapshot.staked_future.unwrap().total()
            ))
        );
        assert!(result_iter.next().is_none());

        // Ledger values are correct
        let mut expected_stake_amount = acc_ledger_snapshot.staked_future.unwrap();
        expected_stake_amount.era += 1;
        assert_eq!(
            acc_ledger.staked, expected_stake_amount,
            "Era must be bumped by 1, and entry must switch from staked_future over to staked."
        );
        assert!(
            acc_ledger.staked_future.is_none(),
            "staked_future must be cleaned up after the claim."
        );
    }

    // 2nd scenario - claim multiple eras (5), period hasn't ended yet
    {
        let mut acc_ledger = acc_ledger_snapshot.clone();
        let mut result_iter = acc_ledger
            .claim_up_to_era(stake_era + 4, None) // staked era + 4 additional eras
            .expect("Must provide iter with 5 entries.");

        // Iter values are correct
        for inc in 0..5 {
            assert_eq!(
                result_iter.next(),
                Some((
                    stake_era + inc,
                    acc_ledger_snapshot.staked_future.unwrap().total()
                ))
            );
        }
        assert!(result_iter.next().is_none());

        // Ledger values are correct
        let mut expected_stake_amount = acc_ledger_snapshot.staked_future.unwrap();
        expected_stake_amount.era += 5;
        assert_eq!(
            acc_ledger.staked, expected_stake_amount,
            "Era must be bumped by 5, and entry must switch from staked_future over to staked."
        );
        assert!(
            acc_ledger.staked_future.is_none(),
            "staked_future must be cleaned up after the claim."
        );
    }
}

#[test]
fn account_ledger_claim_up_to_era_only_staked_future_with_cleanup_works() {
    get_u32_type!(UnlockingDummy, 5);
    let stake_era = 50;

    let acc_ledger_snapshot = {
        let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();
        acc_ledger.staked_future = Some(StakeAmount {
            voting: 2,
            build_and_earn: 17,
            era: stake_era,
            period: 3,
        });
        acc_ledger
    };

    // 1st scenario - claim one era, period has ended
    {
        let mut acc_ledger = acc_ledger_snapshot.clone();
        let mut result_iter = acc_ledger
            .claim_up_to_era(stake_era, Some(stake_era))
            .expect("Must provide iter with exactly one era.");

        // Iter values are correct
        assert_eq!(
            result_iter.next(),
            Some((
                stake_era,
                acc_ledger_snapshot.staked_future.unwrap().total()
            ))
        );
        assert!(result_iter.next().is_none());

        // Ledger values are cleaned up
        assert!(
            acc_ledger.staked.is_empty(),
            "Period has ended so stake entry should be cleaned up."
        );
        assert!(
            acc_ledger.staked_future.is_none(),
            "Was and should remain None."
        );
    }

    // 2nd scenario - claim multiple eras (5), period has ended
    {
        let mut acc_ledger = acc_ledger_snapshot.clone();
        let mut result_iter = acc_ledger
            .claim_up_to_era(stake_era + 4, Some(stake_era)) // staked era + 4 additional eras
            .expect("Must provide iter with 5 entries.");

        for inc in 0..5 {
            assert_eq!(
                result_iter.next(),
                Some((
                    stake_era + inc,
                    acc_ledger_snapshot.staked_future.unwrap().total()
                ))
            );
        }
        assert!(result_iter.next().is_none());

        // Ledger values are cleaned up
        assert!(
            acc_ledger.staked.is_empty(),
            "Period has ended so stake entry should be cleaned up."
        );
        assert!(
            acc_ledger.staked_future.is_none(),
            "Was and should remain None."
        );
    }

    // 3rd scenario - claim one era, period has ended in some future era
    {
        let mut acc_ledger = acc_ledger_snapshot.clone();
        let mut result_iter = acc_ledger
            .claim_up_to_era(stake_era, Some(stake_era + 1))
            .expect("Must provide iter with exactly one era.");

        // Iter values are correct
        assert_eq!(
            result_iter.next(),
            Some((
                stake_era,
                acc_ledger_snapshot.staked_future.unwrap().total()
            ))
        );
        assert!(result_iter.next().is_none());

        // Ledger values are correctly updated
        let mut expected_stake_amount = acc_ledger_snapshot.staked_future.unwrap();
        expected_stake_amount.era += 1;
        assert_eq!(
            acc_ledger.staked, expected_stake_amount,
            "Entry must exist since we still haven't reached the period end era."
        );
        assert!(
            acc_ledger.staked_future.is_none(),
            "Was and should remain None."
        );
    }
}

#[test]
fn account_ledger_claim_up_to_era_staked_and_staked_future_works() {
    get_u32_type!(UnlockingDummy, 5);
    let stake_era_1 = 100;
    let stake_era_2 = stake_era_1 + 1;

    let acc_ledger_snapshot = {
        let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();
        acc_ledger.staked = StakeAmount {
            voting: 3,
            build_and_earn: 7,
            era: stake_era_1,
            period: 5,
        };
        acc_ledger.staked_future = Some(StakeAmount {
            voting: 3,
            build_and_earn: 11,
            era: stake_era_2,
            period: 5,
        });
        acc_ledger
    };

    // 1st scenario - claim only one era
    {
        let mut acc_ledger = acc_ledger_snapshot.clone();
        let mut result_iter = acc_ledger
            .claim_up_to_era(stake_era_1, None)
            .expect("Must provide iter with exactly one era.");

        // Iter values are correct
        assert_eq!(
            result_iter.next(),
            Some((stake_era_1, acc_ledger_snapshot.staked.total()))
        );
        assert!(result_iter.next().is_none());

        // Ledger values are correct
        let mut expected_stake_amount = acc_ledger_snapshot.staked;
        expected_stake_amount.era += 1;
        assert_eq!(
            acc_ledger.staked,
            acc_ledger_snapshot.staked_future.unwrap(),
            "staked_future entry must be moved over to staked."
        );
        assert!(
            acc_ledger.staked_future.is_none(),
            "staked_future is cleaned up since it's been moved over to staked entry."
        );
    }

    // 2nd scenario - claim multiple eras (3), period hasn't ended yet, do the cleanup
    {
        let mut acc_ledger = acc_ledger_snapshot.clone();
        let mut result_iter = acc_ledger
            .claim_up_to_era(stake_era_2 + 1, None) // staked era + 2 additional eras
            .expect("Must provide iter with exactly two entries.");

        // Iter values are correct
        assert_eq!(
            result_iter.next(),
            Some((stake_era_1, acc_ledger_snapshot.staked.total()))
        );
        assert_eq!(
            result_iter.next(),
            Some((
                stake_era_2,
                acc_ledger_snapshot.staked_future.unwrap().total()
            ))
        );
        assert_eq!(
            result_iter.next(),
            Some((
                stake_era_2 + 1,
                acc_ledger_snapshot.staked_future.unwrap().total()
            ))
        );
        assert!(result_iter.next().is_none());

        // Ledger values are correct
        let mut expected_stake_amount = acc_ledger_snapshot.staked_future.unwrap();
        expected_stake_amount.era += 2;
        assert_eq!(
            acc_ledger.staked, expected_stake_amount,
            "staked_future must move over to staked, and era must be incremented by 2."
        );
        assert!(
            acc_ledger.staked_future.is_none(),
            "staked_future is cleaned up since it's been moved over to staked entry."
        );
    }
}

#[test]
fn account_ledger_claim_up_to_era_fails_for_historic_eras() {
    get_u32_type!(UnlockingDummy, 5);
    let stake_era = 50;

    // Only staked entry
    {
        let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();
        acc_ledger.staked = StakeAmount {
            voting: 2,
            build_and_earn: 17,
            era: stake_era,
            period: 3,
        };
        assert_eq!(
            acc_ledger.claim_up_to_era(stake_era - 1, None),
            Err(AccountLedgerError::NothingToClaim)
        );
    }

    // Only staked-future entry
    {
        let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();
        acc_ledger.staked_future = Some(StakeAmount {
            voting: 2,
            build_and_earn: 17,
            era: stake_era,
            period: 3,
        });
        assert_eq!(
            acc_ledger.claim_up_to_era(stake_era - 1, None),
            Err(AccountLedgerError::NothingToClaim)
        );
    }

    // Both staked and staked-future entries
    {
        let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();
        acc_ledger.staked = StakeAmount {
            voting: 2,
            build_and_earn: 17,
            era: stake_era,
            period: 3,
        };
        acc_ledger.staked_future = Some(StakeAmount {
            voting: 2,
            build_and_earn: 19,
            era: stake_era + 1,
            period: 3,
        });
        assert_eq!(
            acc_ledger.claim_up_to_era(stake_era - 1, None),
            Err(AccountLedgerError::NothingToClaim)
        );
    }
}

#[test]
fn era_stake_pair_iter_works() {
    // 1st scenario - only span is given
    let (era_1, last_era, amount) = (2, 5, 11);
    let mut iter_1 = EraStakePairIter::new((era_1, last_era, amount), None).unwrap();
    for era in era_1..=last_era {
        assert_eq!(iter_1.next(), Some((era, amount)));
    }
    assert!(iter_1.next().is_none());

    // 2nd scenario - first value & span are given
    let (maybe_era_1, maybe_first_amount) = (1, 7);
    let maybe_first = Some((maybe_era_1, maybe_first_amount));
    let mut iter_2 = EraStakePairIter::new((era_1, last_era, amount), maybe_first).unwrap();

    assert_eq!(iter_2.next(), Some((maybe_era_1, maybe_first_amount)));
    for era in era_1..=last_era {
        assert_eq!(iter_2.next(), Some((era, amount)));
    }
}

#[test]
fn era_stake_pair_iter_returns_error_for_illegal_data() {
    // 1st scenario - spans are reversed; first era comes AFTER the last era
    let (era_1, last_era, amount) = (2, 5, 11);
    assert!(EraStakePairIter::new((last_era, era_1, amount), None).is_err());

    // 2nd scenario - maybe_first covers the same era as the span
    assert!(EraStakePairIter::new((era_1, last_era, amount), Some((era_1, 10))).is_err());

    // 3rd scenario - maybe_first is before the span, but not exactly 1 era before the first era in the span
    assert!(EraStakePairIter::new((era_1, last_era, amount), Some((era_1 - 2, 10))).is_err());

    assert!(
        EraStakePairIter::new((era_1, last_era, amount), Some((era_1 - 1, 10))).is_ok(),
        "Sanity check."
    );
}

#[test]
fn era_info_lock_unlock_works() {
    let mut era_info = EraInfo::default();

    // Sanity check
    assert!(era_info.total_locked.is_zero());
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
    let era_info_snapshot = era_info;

    // First unlock & checks
    era_info.unlocking_started(unlock_amount);
    assert_eq!(
        era_info.total_locked,
        era_info_snapshot.total_locked - unlock_amount
    );
    assert_eq!(era_info.unlocking, unlock_amount);

    // Second unlock and checks
    era_info.unlocking_started(unlock_amount);
    assert_eq!(
        era_info.total_locked,
        era_info_snapshot.total_locked - unlock_amount * 2
    );
    assert_eq!(era_info.unlocking, unlock_amount * 2);

    // Claim unlocked chunks
    let old_era_info = era_info.clone();
    era_info.unlocking_removed(1);
    assert_eq!(era_info.unlocking, old_era_info.unlocking - 1);
    assert_eq!(era_info.total_locked, old_era_info.total_locked);
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
    era_info.current_stake_amount = StakeAmount {
        voting: vp_stake_amount,
        build_and_earn: bep_stake_amount_1,
        era,
        period: period_number,
    };
    era_info.next_stake_amount = StakeAmount {
        voting: vp_stake_amount,
        build_and_earn: bep_stake_amount_2,
        era: era + 1,
        period: period_number,
    };
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
fn era_info_migrate_to_next_era_works() {
    // Make dummy era info with stake amounts
    let era_info_snapshot = EraInfo {
        total_locked: 456,
        unlocking: 13,
        current_stake_amount: StakeAmount {
            voting: 13,
            build_and_earn: 29,
            era: 2,
            period: 1,
        },
        next_stake_amount: StakeAmount {
            voting: 13,
            build_and_earn: 41,
            era: 3,
            period: 1,
        },
    };

    // 1st scenario - rollover to next era, no subperiod change
    {
        let mut era_info = era_info_snapshot;
        era_info.migrate_to_next_era(None);

        assert_eq!(era_info.total_locked, era_info_snapshot.total_locked);
        assert_eq!(era_info.unlocking, era_info_snapshot.unlocking);
        assert_eq!(
            era_info.current_stake_amount,
            era_info_snapshot.next_stake_amount
        );

        let mut new_next_stake_amount = era_info_snapshot.next_stake_amount;
        new_next_stake_amount.era += 1;
        assert_eq!(era_info.next_stake_amount, new_next_stake_amount);
    }

    // 2nd scenario - rollover to next era, change from Voting into Build&Earn subperiod
    {
        let mut era_info = era_info_snapshot;
        era_info.migrate_to_next_era(Some(Subperiod::BuildAndEarn));

        assert_eq!(era_info.total_locked, era_info_snapshot.total_locked);
        assert_eq!(era_info.unlocking, era_info_snapshot.unlocking);
        assert_eq!(
            era_info.current_stake_amount,
            era_info_snapshot.next_stake_amount
        );

        let mut new_next_stake_amount = era_info_snapshot.next_stake_amount;
        new_next_stake_amount.era += 1;
        assert_eq!(era_info.next_stake_amount, new_next_stake_amount);
    }

    // 3rd scenario - rollover to next era, change from Build&Earn to Voting subperiod
    {
        let mut era_info = era_info_snapshot;
        era_info.migrate_to_next_era(Some(Subperiod::Voting));

        assert_eq!(era_info.total_locked, era_info_snapshot.total_locked);
        assert_eq!(era_info.unlocking, era_info_snapshot.unlocking);
        assert_eq!(
            era_info.current_stake_amount,
            StakeAmount {
                voting: Zero::zero(),
                build_and_earn: Zero::zero(),
                era: era_info_snapshot.current_stake_amount.era + 1,
                period: era_info_snapshot.current_stake_amount.period + 1,
            }
        );
        assert_eq!(
            era_info.next_stake_amount,
            StakeAmount {
                voting: Zero::zero(),
                build_and_earn: Zero::zero(),
                era: era_info_snapshot.current_stake_amount.era + 2,
                period: era_info_snapshot.current_stake_amount.period + 1,
            }
        );
    }
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
    assert!(staking_info.is_empty());
    assert!(staking_info.era().is_zero());
    assert!(!SingularStakingInfo::new(period_number, Subperiod::BuildAndEarn).is_loyal());

    // Add some staked amount during `Voting` period
    let era_1 = 7;
    let vote_stake_amount_1 = 11;
    staking_info.stake(vote_stake_amount_1, era_1, Subperiod::Voting);
    assert_eq!(staking_info.total_staked_amount(), vote_stake_amount_1);
    assert_eq!(
        staking_info.staked_amount(Subperiod::Voting),
        vote_stake_amount_1
    );
    assert!(staking_info
        .staked_amount(Subperiod::BuildAndEarn)
        .is_zero());
    assert_eq!(
        staking_info.era(),
        era_1 + 1,
        "Stake era should remain valid."
    );

    // Add some staked amount during `BuildAndEarn` period
    let era_2 = 9;
    let bep_stake_amount_1 = 23;
    staking_info.stake(bep_stake_amount_1, era_2, Subperiod::BuildAndEarn);
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
    assert_eq!(staking_info.era(), era_2 + 1);
}

#[test]
fn singular_staking_info_unstake_during_voting_is_ok() {
    let period_number = 3;
    let subperiod = Subperiod::Voting;
    let mut staking_info = SingularStakingInfo::new(period_number, subperiod);

    // Prep actions
    let era_1 = 2;
    let vote_stake_amount_1 = 11;
    staking_info.stake(vote_stake_amount_1, era_1, Subperiod::Voting);

    // Unstake some amount during `Voting` period, loyalty should remain as expected.
    let unstake_amount_1 = 5;
    assert_eq!(
        staking_info.unstake(unstake_amount_1, era_1, Subperiod::Voting),
        (unstake_amount_1, Balance::zero())
    );
    assert_eq!(
        staking_info.total_staked_amount(),
        vote_stake_amount_1 - unstake_amount_1
    );
    assert!(staking_info.is_loyal());
    assert_eq!(
        staking_info.era(),
        era_1 + 1,
        "Stake era should remain valid."
    );

    // Fully unstake, attempting to undersaturate, and ensure loyalty flag is still true.
    let era_2 = era_1 + 2;
    let remaining_stake = staking_info.total_staked_amount();
    assert_eq!(
        staking_info.unstake(remaining_stake + 1, era_2, Subperiod::Voting),
        (remaining_stake, Balance::zero())
    );
    assert!(staking_info.total_staked_amount().is_zero());
    assert!(staking_info.is_loyal());
    assert_eq!(staking_info.era(), era_2);
}

#[test]
fn singular_staking_info_unstake_during_bep_is_ok() {
    let period_number = 3;
    let subperiod = Subperiod::Voting;
    let mut staking_info = SingularStakingInfo::new(period_number, subperiod);

    // Prep actions
    let era_1 = 3;
    let vote_stake_amount_1 = 11;
    staking_info.stake(vote_stake_amount_1, era_1 - 1, Subperiod::Voting);
    let bep_stake_amount_1 = 23;
    staking_info.stake(bep_stake_amount_1, era_1, Subperiod::BuildAndEarn);

    // 1st scenario - Unstake some of the amount staked during B&E period
    let unstake_1 = 5;
    assert_eq!(
        staking_info.unstake(5, era_1, Subperiod::BuildAndEarn),
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
    assert_eq!(
        staking_info.era(),
        era_1 + 1,
        "Stake era should remain valid."
    );

    // 2nd scenario - unstake all of the amount staked during B&E period, and then some more.
    // The point is to take a chunk from the voting period stake too.
    let current_total_stake = staking_info.total_staked_amount();
    let current_bep_stake = staking_info.staked_amount(Subperiod::BuildAndEarn);
    let voting_stake_overflow = 2;
    let unstake_2 = current_bep_stake + voting_stake_overflow;
    let era_2 = era_1 + 3;

    assert_eq!(
        staking_info.unstake(unstake_2, era_2, Subperiod::BuildAndEarn),
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
    assert_eq!(staking_info.era(), era_2);
}

#[test]
fn contract_stake_amount_basic_get_checks_work() {
    // Sanity checks for empty struct
    let contract_stake = ContractStakeAmount {
        staked: Default::default(),
        staked_future: None,
    };
    assert!(contract_stake.is_empty());
    assert!(contract_stake.latest_stake_period().is_none());
    assert!(contract_stake.latest_stake_era().is_none());
    assert!(contract_stake.total_staked_amount(0).is_zero());
    assert!(contract_stake.staked_amount(0, Subperiod::Voting).is_zero());
    assert!(contract_stake
        .staked_amount(0, Subperiod::BuildAndEarn)
        .is_zero());

    let era = 3;
    let period = 2;
    let amount = StakeAmount {
        voting: 11,
        build_and_earn: 17,
        era,
        period,
    };
    let contract_stake = ContractStakeAmount {
        staked: amount,
        staked_future: None,
    };
    assert!(!contract_stake.is_empty());

    // Checks for illegal periods
    for illegal_period in [period - 1, period + 1] {
        assert!(contract_stake.total_staked_amount(illegal_period).is_zero());
        assert!(contract_stake
            .staked_amount(illegal_period, Subperiod::Voting)
            .is_zero());
        assert!(contract_stake
            .staked_amount(illegal_period, Subperiod::BuildAndEarn)
            .is_zero());
    }

    // Check for the valid period
    assert_eq!(contract_stake.latest_stake_period(), Some(period));
    assert_eq!(contract_stake.latest_stake_era(), Some(era));
    assert_eq!(contract_stake.total_staked_amount(period), amount.total());
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::Voting),
        amount.voting
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::BuildAndEarn),
        amount.build_and_earn
    );
}

#[test]
fn contract_stake_amount_advanced_get_checks_work() {
    let (era_1, era_2) = (4, 7);
    let period = 2;
    let amount_1 = StakeAmount {
        voting: 11,
        build_and_earn: 0,
        era: era_1,
        period,
    };
    let amount_2 = StakeAmount {
        voting: 11,
        build_and_earn: 13,
        era: era_2,
        period,
    };

    let contract_stake = ContractStakeAmount {
        staked: amount_1,
        staked_future: Some(amount_2),
    };

    // Sanity checks - all values from the 'future' entry should be relevant
    assert!(!contract_stake.is_empty());
    assert_eq!(contract_stake.latest_stake_period(), Some(period));
    assert_eq!(contract_stake.latest_stake_era(), Some(era_2));
    assert_eq!(contract_stake.total_staked_amount(period), amount_2.total());
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::Voting),
        amount_2.voting
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::BuildAndEarn),
        amount_2.build_and_earn
    );

    // 1st scenario - get existing entries
    assert_eq!(contract_stake.get(era_1, period), Some(amount_1));
    assert_eq!(contract_stake.get(era_2, period), Some(amount_2));

    // 2nd scenario - get non-existing entries for covered eras
    let era_3 = era_2 - 1;
    let entry_1 = contract_stake.get(era_3, 2).expect("Has to be Some");
    assert_eq!(entry_1.total(), amount_1.total());
    assert_eq!(entry_1.era, era_3);
    assert_eq!(entry_1.period, period);

    let era_4 = era_2 + 1;
    let entry_1 = contract_stake.get(era_4, period).expect("Has to be Some");
    assert_eq!(entry_1.total(), amount_2.total());
    assert_eq!(entry_1.era, era_4);
    assert_eq!(entry_1.period, period);

    // 3rd scenario - get non-existing entries for covered eras but mismatching period
    assert!(contract_stake.get(8, period + 1).is_none());

    // 4th scenario - get non-existing entries for non-covered eras
    assert!(contract_stake.get(3, period).is_none());
}

#[test]
fn contract_stake_amount_stake_is_ok() {
    let mut contract_stake = ContractStakeAmount::default();

    // 1st scenario - stake some amount and verify state change
    let era_1 = 3;
    let stake_era_1 = era_1 + 1;
    let period_1 = 5;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::Voting,
        next_subperiod_start_era: 20,
    };
    let amount_1 = 31;
    contract_stake.stake(amount_1, period_info_1, era_1);
    assert!(!contract_stake.is_empty());
    assert!(
        contract_stake.staked.is_empty(),
        "Only future entry should be modified."
    );
    assert!(contract_stake.staked_future.is_some());

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
    assert_eq!(entry_1_1.for_type(Subperiod::Voting), amount_1);
    assert!(entry_1_1.for_type(Subperiod::BuildAndEarn).is_zero());

    // 2nd scenario - stake some more to the same era but different period type, and verify state change.
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::BuildAndEarn,
        next_subperiod_start_era: 20,
    };
    contract_stake.stake(amount_1, period_info_1, era_1);
    let entry_1_2 = contract_stake.get(stake_era_1, period_1).unwrap();
    assert_eq!(entry_1_2.era, stake_era_1);
    assert_eq!(entry_1_2.total(), amount_1 * 2);
    assert_eq!(entry_1_2.for_type(Subperiod::Voting), amount_1);
    assert_eq!(entry_1_2.for_type(Subperiod::BuildAndEarn), amount_1);
    assert!(
        contract_stake.staked.is_empty(),
        "Only future entry should be modified."
    );
    assert!(contract_stake.staked_future.is_some());

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
    assert!(
        !contract_stake.staked.is_empty(),
        "staked should keep the old future entry"
    );
    assert!(contract_stake.staked_future.is_some());

    // 4th scenario - stake some more to the next era, but this time also bump the period.
    let era_3 = era_2 + 3;
    let stake_era_3 = era_3 + 1;
    let period_2 = period_1 + 1;
    let period_info_2 = PeriodInfo {
        number: period_2,
        subperiod: Subperiod::BuildAndEarn,
        next_subperiod_start_era: 20,
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
    assert!(
        contract_stake.staked.is_empty(),
        "New period, all stakes should be reset so 'staked' should be empty."
    );
    assert!(contract_stake.staked_future.is_some());

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
    assert!(
        !contract_stake.staked.is_empty(),
        "staked should keep the old future entry"
    );
    assert!(contract_stake.staked_future.is_some());
}

#[test]
fn contract_stake_amount_unstake_is_ok() {
    let mut contract_stake = ContractStakeAmount::default();

    // Prep action - create a stake entry
    let era_1 = 2;
    let period = 3;
    let period_info = PeriodInfo {
        number: period,
        subperiod: Subperiod::Voting,
        next_subperiod_start_era: 20,
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
    assert!(contract_stake.staked.is_empty());
    assert!(contract_stake.staked_future.is_some());

    // 2nd scenario - unstake in the next era
    let period_info = PeriodInfo {
        number: period,
        subperiod: Subperiod::BuildAndEarn,
        next_subperiod_start_era: 40,
    };
    let era_2 = era_1 + 1;

    contract_stake.unstake(amount_1, period_info, era_2);
    assert_eq!(
        contract_stake.total_staked_amount(period),
        stake_amount - amount_1 * 2
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::Voting),
        stake_amount - amount_1 * 2
    );
    assert!(
        !contract_stake.staked.is_empty(),
        "future entry should be moved over to the current entry"
    );
    assert!(
        contract_stake.staked_future.is_none(),
        "future entry should be cleaned up since it refers to the current era"
    );

    // 3rd scenario - bump up unstake eras by more than 1, entries should be aligned to the current era
    let era_3 = era_2 + 3;
    let amount_2 = 7;
    contract_stake.unstake(amount_2, period_info, era_3);
    assert_eq!(
        contract_stake.total_staked_amount(period),
        stake_amount - amount_1 * 2 - amount_2
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::Voting),
        stake_amount - amount_1 * 2 - amount_2
    );
    assert_eq!(
        contract_stake.staked.era, era_3,
        "Should be aligned to the current era."
    );
    assert!(
        contract_stake.staked_future.is_none(),
        "future enry should remain 'None'"
    );

    // 4th scenario - do a full unstake with existing future entry, expect a cleanup
    contract_stake.stake(stake_amount, period_info, era_3);
    contract_stake.unstake(
        contract_stake.total_staked_amount(period),
        period_info,
        era_3,
    );
    assert!(contract_stake.staked.is_empty());
    assert!(contract_stake.staked_future.is_none());
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

    // Try and get the values outside of the span
    assert!(era_reward_span
        .get(era_reward_span.first_era() - 1)
        .is_none());
    assert!(era_reward_span
        .get(era_reward_span.last_era() + 1)
        .is_none());
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
fn tier_threshold_is_ok() {
    let amount = 100;

    // Fixed TVL
    let fixed_threshold = TierThreshold::FixedTvlAmount { amount };
    assert!(fixed_threshold.is_satisfied(amount));
    assert!(fixed_threshold.is_satisfied(amount + 1));
    assert!(!fixed_threshold.is_satisfied(amount - 1));

    // Dynamic TVL
    let dynamic_threshold = TierThreshold::DynamicTvlAmount {
        amount,
        minimum_amount: amount / 2, // not important
    };
    assert!(dynamic_threshold.is_satisfied(amount));
    assert!(dynamic_threshold.is_satisfied(amount + 1));
    assert!(!dynamic_threshold.is_satisfied(amount - 1));
}

#[test]
fn tier_params_check_is_ok() {
    // Prepare valid params
    get_u32_type!(TiersNum, 3);
    let params = TierParameters::<TiersNum> {
        reward_portion: BoundedVec::try_from(vec![
            Permill::from_percent(60),
            Permill::from_percent(30),
            Permill::from_percent(10),
        ])
        .unwrap(),
        slot_distribution: BoundedVec::try_from(vec![
            Permill::from_percent(10),
            Permill::from_percent(20),
            Permill::from_percent(70),
        ])
        .unwrap(),
        tier_thresholds: BoundedVec::try_from(vec![
            TierThreshold::DynamicTvlAmount {
                amount: 1000,
                minimum_amount: 100,
            },
            TierThreshold::DynamicTvlAmount {
                amount: 100,
                minimum_amount: 10,
            },
            TierThreshold::FixedTvlAmount { amount: 10 },
        ])
        .unwrap(),
    };
    assert!(params.is_valid());

    // 1st scenario - sums are below 100%, and that is ok
    let mut new_params = params.clone();
    new_params.reward_portion = BoundedVec::try_from(vec![
        Permill::from_percent(59),
        Permill::from_percent(30),
        Permill::from_percent(10),
    ])
    .unwrap();
    new_params.slot_distribution = BoundedVec::try_from(vec![
        Permill::from_percent(10),
        Permill::from_percent(19),
        Permill::from_percent(70),
    ])
    .unwrap();
    assert!(params.is_valid());

    // 2nd scenario - reward portion is too much
    let mut new_params = params.clone();
    new_params.reward_portion = BoundedVec::try_from(vec![
        Permill::from_percent(61),
        Permill::from_percent(30),
        Permill::from_percent(10),
    ])
    .unwrap();
    assert!(!new_params.is_valid());

    // 3rd scenario - tier distribution is too much
    let mut new_params = params.clone();
    new_params.slot_distribution = BoundedVec::try_from(vec![
        Permill::from_percent(10),
        Permill::from_percent(20),
        Permill::from_percent(71),
    ])
    .unwrap();
    assert!(!new_params.is_valid());

    // 4th scenario - incorrect vector length
    let mut new_params = params.clone();
    new_params.tier_thresholds =
        BoundedVec::try_from(vec![TierThreshold::FixedTvlAmount { amount: 10 }]).unwrap();
    assert!(!new_params.is_valid());
}

#[test]
fn tier_configuration_basic_tests() {
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
    let high_price = FixedU64::from_rational(20, 100); // in production will be expressed in USD
    let new_config = init_config.calculate_new(high_price, &params);
    assert!(new_config.is_valid());

    let low_price = FixedU64::from_rational(1, 100); // in production will be expressed in USD
    let new_config = init_config.calculate_new(low_price, &params);
    assert!(new_config.is_valid());

    // TODO: expand tests, add more sanity checks (e.g. tier 3 requirement should never be lower than tier 4, etc.)
}

#[test]
fn dapp_tier_rewards_basic_tests() {
    get_u32_type!(NumberOfDApps, 8);
    get_u32_type!(NumberOfTiers, 3);

    // Example dApps & rewards
    let dapps = vec![
        DAppTier {
            dapp_id: 1,
            tier_id: Some(0),
        },
        DAppTier {
            dapp_id: 2,
            tier_id: Some(0),
        },
        DAppTier {
            dapp_id: 3,
            tier_id: Some(1),
        },
        DAppTier {
            dapp_id: 5,
            tier_id: Some(1),
        },
        DAppTier {
            dapp_id: 6,
            tier_id: Some(2),
        },
    ];
    let tier_rewards = vec![300, 20, 1];
    let period = 2;

    let mut dapp_tier_rewards = DAppTierRewards::<NumberOfDApps, NumberOfTiers>::new(
        dapps.clone(),
        tier_rewards.clone(),
        period,
    )
    .expect("Bounds are respected.");

    // 1st scenario - claim reward for a dApps
    let tier_id = dapps[0].tier_id.unwrap();
    assert_eq!(
        dapp_tier_rewards.try_claim(dapps[0].dapp_id),
        Ok((tier_rewards[tier_id as usize], tier_id))
    );

    let tier_id = dapps[3].tier_id.unwrap();
    assert_eq!(
        dapp_tier_rewards.try_claim(dapps[3].dapp_id),
        Ok((tier_rewards[tier_id as usize], tier_id))
    );

    // 2nd scenario - try to claim already claimed reward
    assert_eq!(
        dapp_tier_rewards.try_claim(dapps[0].dapp_id),
        Err(DAppTierError::RewardAlreadyClaimed),
        "Cannot claim the same reward twice."
    );

    // 3rd scenario - claim for a dApp that is not in the list
    assert_eq!(
        dapp_tier_rewards.try_claim(4),
        Err(DAppTierError::NoDAppInTiers),
        "dApp doesn't exist in the list so no rewards can be claimed."
    );
}
