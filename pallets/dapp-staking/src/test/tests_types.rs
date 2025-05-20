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

use astar_primitives::{
    dapp_staking::{RankedTier, StandardTierSlots, STANDARD_TIER_SLOTS_ARGS},
    Balance,
};
use frame_support::{assert_ok, parameter_types};
use sp_arithmetic::fixed_point::FixedU128;
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

// Helper to generate custom `Get` types for testing the `BonusStatus` enum.
macro_rules! get_u8_type {
    ($struct_name:ident, $value:expr) => {
        #[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
        struct $struct_name;
        impl Get<u8> for $struct_name {
            fn get() -> u8 {
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

    // Voting subperiod checks
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
            "Cannot trigger 'true' in the Voting subperiod type."
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
        reward_beneficiary: None,
    };

    // Owner receives reward in case no beneficiary is set
    assert_eq!(*dapp_info.reward_beneficiary(), owner);

    // Beneficiary receives rewards in case it is set
    dapp_info.reward_beneficiary = Some(beneficiary);
    assert_eq!(*dapp_info.reward_beneficiary(), beneficiary);
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

    // Incorrect period should simply return 0
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
    acc_ledger.staked = StakeAmount::default();
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
fn account_ledger_add_stake_amount_basic_example_with_different_subperiods_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

    // Sanity check
    let period_number = 2;
    assert!(acc_ledger
        .add_stake_amount(
            StakeAmount {
                period: period_number,
                ..StakeAmount::default()
            },
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

    // 1st scenario - stake some amount in Voting subperiod, and ensure values are as expected.
    let era_1 = 1;
    let period_1 = 1;
    let period_info_1 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::Voting,
        next_subperiod_start_era: 100,
    };
    let lock_amount = 17;
    let voting_amount = 11;
    let stake_amount_1 = StakeAmount {
        voting: voting_amount,
        build_and_earn: 0,
        era: era_1,
        period: period_1,
    };
    acc_ledger.add_lock_amount(lock_amount);

    assert!(acc_ledger
        .add_stake_amount(stake_amount_1, era_1, period_info_1)
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
    assert_eq!(acc_ledger.staked_future.unwrap().era, era_1 + 1);
    assert_eq!(
        acc_ledger.staked_future.unwrap().voting,
        stake_amount_1.voting
    );
    assert!(acc_ledger.staked_future.unwrap().build_and_earn.is_zero());
    assert_eq!(acc_ledger.staked_amount(period_1), stake_amount_1.total());
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::Voting, period_1),
        stake_amount_1.voting
    );
    assert!(acc_ledger
        .staked_amount_for_type(Subperiod::BuildAndEarn, period_1)
        .is_zero());

    // Second scenario - stake some more, but to the next period type
    let snapshot = acc_ledger.staked_future.unwrap();
    let period_info_2 = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::BuildAndEarn,
        next_subperiod_start_era: 100,
    };
    let era_2 = era_1 + 1;
    let bep_amount = 1;
    let stake_amount_2 = StakeAmount {
        voting: 0,
        build_and_earn: bep_amount,
        era: era_2,
        period: period_1,
    };
    assert!(acc_ledger
        .add_stake_amount(stake_amount_2, era_2, period_info_2)
        .is_ok());
    assert_eq!(
        acc_ledger.staked_amount(period_1),
        stake_amount_1.total() + stake_amount_2.total()
    );
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::Voting, period_1),
        stake_amount_1.total()
    );
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::BuildAndEarn, period_1),
        stake_amount_2.total()
    );

    assert_eq!(acc_ledger.staked_future.unwrap().era, era_2 + 1);
    assert_eq!(
        acc_ledger.staked_future.unwrap().voting,
        stake_amount_1.voting
    );
    assert_eq!(
        acc_ledger.staked_future.unwrap().build_and_earn,
        stake_amount_2.build_and_earn
    );

    assert_eq!(acc_ledger.staked, snapshot);
}

#[test]
fn account_ledger_add_stake_amount_basic_example_with_same_subperiods_works() {
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger = AccountLedger::<UnlockingDummy>::default();

    // 1st scenario - stake some amount in first era of the `Build&Earn` subperiod, and ensure values are as expected.
    let era_1 = 2;
    let period_1 = 1;
    let period_info = PeriodInfo {
        number: period_1,
        subperiod: Subperiod::BuildAndEarn,
        next_subperiod_start_era: 100,
    };
    let lock_amount = 17;
    let bep_amount_1 = 11;
    let stake_amount_1 = StakeAmount {
        voting: 0,
        build_and_earn: bep_amount_1,
        era: era_1,
        period: period_1,
    };
    acc_ledger.add_lock_amount(lock_amount);

    assert!(acc_ledger
        .add_stake_amount(stake_amount_1, era_1, period_info)
        .is_ok());

    assert!(
        acc_ledger.staked.is_empty(),
        "Current era must remain unchanged."
    );
    assert_eq!(acc_ledger.staked_future.unwrap().period, period_1);
    assert_eq!(acc_ledger.staked_future.unwrap().era, era_1 + 1);
    assert_eq!(
        acc_ledger.staked_future.unwrap().build_and_earn,
        stake_amount_1.build_and_earn
    );
    assert!(acc_ledger.staked_future.unwrap().voting.is_zero());
    assert_eq!(acc_ledger.staked_amount(period_1), stake_amount_1.total());
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::BuildAndEarn, period_1),
        stake_amount_1.build_and_earn
    );
    assert!(acc_ledger
        .staked_amount_for_type(Subperiod::Voting, period_1)
        .is_zero());

    // 2nd scenario - stake again, in the same era
    let snapshot = acc_ledger.staked;
    let bep_amount_2 = 1;
    let stake_amount_2 = StakeAmount {
        voting: 0,
        build_and_earn: bep_amount_2,
        era: era_1,
        period: period_1,
    };

    assert!(acc_ledger
        .add_stake_amount(stake_amount_2, era_1, period_info)
        .is_ok());
    assert_eq!(acc_ledger.staked, snapshot);
    assert_eq!(
        acc_ledger.staked_amount(period_1),
        stake_amount_1.total() + stake_amount_2.total()
    );

    // 2nd scenario - advance an era, and stake some more
    let snapshot = acc_ledger.staked_future.unwrap();
    let era_2 = era_1 + 1;
    let bep_amount_3 = 1;
    let stake_amount_3 = StakeAmount {
        voting: 0,
        build_and_earn: bep_amount_3,
        era: era_2,
        period: period_1,
    };
    assert!(acc_ledger
        .add_stake_amount(stake_amount_3, era_2, period_info)
        .is_ok());

    assert_eq!(
        acc_ledger.staked_amount(period_1),
        stake_amount_1.total() + stake_amount_2.total() + stake_amount_3.total()
    );
    assert!(acc_ledger
        .staked_amount_for_type(Subperiod::Voting, period_1)
        .is_zero(),);
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::BuildAndEarn, period_1),
        stake_amount_1.build_and_earn
            + stake_amount_2.build_and_earn
            + stake_amount_3.build_and_earn
    );
    assert_eq!(acc_ledger.staked_future.unwrap().period, period_1);
    assert_eq!(acc_ledger.staked_future.unwrap().era, era_2 + 1);
    assert_eq!(
        acc_ledger.staked_future.unwrap().build_and_earn,
        stake_amount_1.build_and_earn
            + stake_amount_2.build_and_earn
            + stake_amount_3.build_and_earn
    );
    assert!(acc_ledger.staked_future.unwrap().voting.is_zero());

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
    let voting_amount_1 = 11;
    let stake_amount_1 = StakeAmount {
        voting: voting_amount_1,
        build_and_earn: 0,
        era: era_1,
        period: period_1,
    };
    acc_ledger.add_lock_amount(lock_amount);

    // We only have entry for the current era
    acc_ledger.staked = stake_amount_1;

    let voting_amount_2 = 2;
    let stake_amount_2 = StakeAmount {
        voting: voting_amount_2,
        build_and_earn: 0,
        era: era_1,
        period: period_1,
    };
    let acc_ledger_snapshot = acc_ledger.clone();
    assert!(acc_ledger
        .add_stake_amount(stake_amount_2, era_1, period_info_1)
        .is_ok());
    assert_eq!(
        acc_ledger.staked_amount(period_1),
        stake_amount_1.total() + stake_amount_2.total()
    );
    assert_eq!(
        acc_ledger.staked, acc_ledger_snapshot.staked,
        "This entry must remain unchanged."
    );
    assert_eq!(
        acc_ledger.staked_amount_for_type(Subperiod::Voting, period_1),
        stake_amount_1.voting + stake_amount_2.voting
    );
    assert_eq!(
        acc_ledger
            .staked_future
            .unwrap()
            .for_type(Subperiod::Voting),
        stake_amount_1.voting + stake_amount_2.voting
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
    let voting_amount_1 = 7;
    let stake_amount_1 = StakeAmount {
        voting: voting_amount_1,
        build_and_earn: 0,
        era: era_1,
        period: period_1,
    };
    acc_ledger.add_lock_amount(lock_amount);
    assert!(acc_ledger
        .add_stake_amount(stake_amount_1, era_1, period_info_1)
        .is_ok());

    // Try to add to era after next, it should fail.
    let stake_amount_2 = StakeAmount {
        voting: 1,
        build_and_earn: 0,
        era: era_1 + 2,
        period: period_1,
    };
    assert_eq!(
        acc_ledger.add_stake_amount(stake_amount_2, era_1 + 2, period_info_1),
        Err(AccountLedgerError::InvalidEra)
    );

    // Try to add to the next period, it should fail.
    let stake_amount_3 = StakeAmount {
        voting: 1,
        build_and_earn: 0,
        era: era_1,
        period: period_1 + 1,
    };
    assert_eq!(
        acc_ledger.add_stake_amount(
            stake_amount_3,
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
    let bep_amount_1 = 7;
    acc_ledger.staked = StakeAmount {
        voting: 0,
        build_and_earn: bep_amount_1,
        era: era_1,
        period: period_1,
    };
    acc_ledger.staked_future = None;

    let stake_amount_4 = StakeAmount {
        voting: 0,
        build_and_earn: 1,
        era: era_1 + 1,
        period: period_1,
    };
    assert_eq!(
        acc_ledger.add_stake_amount(stake_amount_4, era_1 + 1, period_info_1),
        Err(AccountLedgerError::InvalidEra)
    );

    let stake_amount_5 = StakeAmount {
        voting: 1,
        build_and_earn: 0,
        era: era_1,
        period: period_1 + 1,
    };
    assert_eq!(
        acc_ledger.add_stake_amount(
            stake_amount_5,
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
    let stake_amount = StakeAmount {
        voting: 10,
        build_and_earn: 0,
        era: 1,
        period: 1,
    };
    assert_eq!(
        acc_ledger.add_stake_amount(
            stake_amount,
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
    let stake_amount = StakeAmount {
        voting: lock_amount + 1,
        build_and_earn: 0,
        era: era_1,
        period: period_1,
    };
    assert_eq!(
        acc_ledger.add_stake_amount(stake_amount, era_1, period_info_1),
        Err(AccountLedgerError::UnavailableStakeFunds)
    );

    // Additional check - have some active stake, and then try to stake more than available
    let stake_amount = StakeAmount {
        voting: lock_amount - 2,
        build_and_earn: 0,
        era: era_1,
        period: period_1,
    };
    assert!(acc_ledger
        .add_stake_amount(stake_amount, era_1, period_info_1)
        .is_ok());

    let stake_amount = StakeAmount {
        voting: 3,
        build_and_earn: 0,
        era: era_1,
        period: period_1,
    };
    assert_eq!(
        acc_ledger.add_stake_amount(stake_amount, era_1, period_info_1),
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
    let stake_amount = StakeAmount {
        voting: 0,
        build_and_earn: amount_1,
        era: era_1,
        period: period_1,
    };
    assert!(acc_ledger
        .add_stake_amount(stake_amount, era_1, period_info_1)
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
    let stake_amount = StakeAmount {
        voting: 0,
        build_and_earn: amount_2,
        era: era_2,
        period: period_1,
    };
    assert!(acc_ledger
        .add_stake_amount(stake_amount, era_2, period_info_1)
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
    let stake_amount = StakeAmount {
        voting: 0,
        build_and_earn: amount_1,
        era: era_1,
        period: period_1,
    };
    assert!(acc_ledger
        .add_stake_amount(stake_amount, era_1, period_info_1)
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
    let stake_amount = StakeAmount {
        voting: 0,
        build_and_earn: amount_1,
        era: era_1,
        period: period_1,
    };
    assert!(acc_ledger
        .add_stake_amount(stake_amount, era_1, period_info_1)
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
    let stake_amount = StakeAmount {
        voting: stake_amount,
        build_and_earn: 0,
        era: lock_era,
        period: stake_period,
    };
    assert!(acc_ledger
        .add_stake_amount(stake_amount, lock_era, period_info)
        .is_ok());
    assert_eq!(
        acc_ledger.unlockable_amount(stake_period),
        lock_amount - stake_amount.total()
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

    // Add some voting subperiod stake
    let vp_stake_amount = 7;
    let stake_amount = StakeAmount {
        voting: vp_stake_amount,
        ..StakeAmount::default()
    };
    era_info.add_stake_amount(stake_amount);
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
    let stake_amount = StakeAmount {
        build_and_earn: bep_stake_amount,
        ..StakeAmount::default()
    };
    era_info.add_stake_amount(stake_amount);
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

    // === Scenario 1: Partial unstake without overflow ===
    let unstake_amount_1_current = StakeAmount {
        voting: 0,
        build_and_earn: bep_stake_amount_1,
        era,
        period: period_number,
    };
    let unstake_amount_1_next = StakeAmount {
        voting: 0,
        build_and_earn: bep_stake_amount_1,
        era: era + 1,
        period: period_number,
    };
    era_info.unstake_amount(vec![unstake_amount_1_current, unstake_amount_1_next]);
    let era_info_snapshot_1 = era_info;

    // Current era
    assert_eq!(
        era_info.total_staked_amount(),
        total_staked - unstake_amount_1_current.total()
    );
    assert_eq!(era_info.staked_amount(Subperiod::Voting), vp_stake_amount);
    assert!(era_info.staked_amount(Subperiod::BuildAndEarn).is_zero());

    // Next era
    assert_eq!(
        era_info.total_staked_amount_next_era(),
        total_staked_next_era - unstake_amount_1_next.total()
    );
    assert_eq!(
        era_info.staked_amount_next_era(Subperiod::Voting),
        vp_stake_amount
    );
    assert_eq!(
        era_info.staked_amount_next_era(Subperiod::BuildAndEarn),
        bep_stake_amount_2 - unstake_amount_1_next.build_and_earn
    );

    // === Scenario 2: Overflow as no effect on era stake ===
    let overflow = 2;
    let unstake_amount_2_current = StakeAmount {
        voting: 0,
        build_and_earn: overflow, // current era stake is already at 0 from scenario 1
        era,
        period: period_number,
    };
    era_info.unstake_amount(vec![unstake_amount_2_current]);

    // Era info remains unchanged, overflow tentative as no effect
    assert_eq!(era_info, era_info_snapshot_1);

    // === Scenario 3: Unstake per subperiod types works ===

    // Make a new dummy era info with stake amounts
    let mut era_info_2 = EraInfo::default();
    era_info_2.current_stake_amount = StakeAmount {
        voting: 10,
        build_and_earn: 10,
        era,
        period: period_number,
    };
    era_info_2.next_stake_amount = StakeAmount {
        voting: 10,
        build_and_earn: 20,
        era: era + 1,
        period: period_number,
    };
    let unstake_amount_current = StakeAmount {
        voting: 5,
        build_and_earn: 5,
        era,
        period: period_number,
    };
    let unstake_amount_next = StakeAmount {
        voting: 5,
        build_and_earn: 5,
        era: era + 1,
        period: period_number,
    };
    era_info_2.unstake_amount(vec![unstake_amount_current, unstake_amount_next]);

    let mut expected_era_info = EraInfo::default();
    expected_era_info.current_stake_amount = StakeAmount {
        voting: 5,
        build_and_earn: 5,
        era,
        period: period_number,
    };
    expected_era_info.next_stake_amount = StakeAmount {
        voting: 5,
        build_and_earn: 15,
        era: era + 1,
        period: period_number,
    };
    assert_eq!(era_info_2, expected_era_info);
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

    // Stake some amount in voting subperiod
    let vp_stake_1 = 11;
    stake_amount.add(vp_stake_1, Subperiod::Voting);
    assert_eq!(stake_amount.total(), vp_stake_1);
    assert_eq!(stake_amount.for_type(Subperiod::Voting), vp_stake_1);
    assert!(stake_amount.for_type(Subperiod::BuildAndEarn).is_zero());

    // Stake some amount in build&earn subperiod
    let bep_stake_1 = 13;
    stake_amount.add(bep_stake_1, Subperiod::BuildAndEarn);
    assert_eq!(stake_amount.total(), vp_stake_1 + bep_stake_1);
    assert_eq!(stake_amount.for_type(Subperiod::Voting), vp_stake_1);
    assert_eq!(stake_amount.for_type(Subperiod::BuildAndEarn), bep_stake_1);

    // Unstake some amount, expect build&earn subperiod to be reduced
    let unstake_1 = 5;
    stake_amount.subtract(5);
    assert_eq!(stake_amount.total(), vp_stake_1 + bep_stake_1 - unstake_1);
    assert_eq!(stake_amount.for_type(Subperiod::Voting), vp_stake_1);
    assert_eq!(
        stake_amount.for_type(Subperiod::BuildAndEarn),
        bep_stake_1 - unstake_1
    );

    // Unstake some amount, once again expect build&earn subperiod to be reduced
    let unstake_2 = 2;
    stake_amount.subtract(unstake_2);
    assert_eq!(
        stake_amount.total(),
        vp_stake_1 + bep_stake_1 - unstake_1 - unstake_2
    );
    assert_eq!(stake_amount.for_type(Subperiod::Voting), vp_stake_1);
    assert_eq!(
        stake_amount.for_type(Subperiod::BuildAndEarn),
        bep_stake_1 - unstake_1 - unstake_2
    );

    // Unstake even more, but this time expect voting subperiod amount to be reduced
    let total_stake = vp_stake_1 + bep_stake_1 - unstake_1 - unstake_2;
    let unstake_3 = bep_stake_1 - unstake_1 - unstake_2 + 1;
    stake_amount.subtract(unstake_3);
    assert_eq!(stake_amount.total(), total_stake - unstake_3);
    assert_eq!(stake_amount.for_type(Subperiod::Voting), vp_stake_1 - 1);
    assert!(stake_amount.for_type(Subperiod::BuildAndEarn).is_zero());
}

#[test]
fn singular_staking_info_basics_are_ok() {
    get_u8_type!(MaxMoves, 2);
    type TestBonusStatusWrapper = BonusStatusWrapper<MaxMoves>;

    let period_number = 3;
    let bonus_status = *TestBonusStatusWrapper::default();
    let mut staking_info = SingularStakingInfo::new(period_number, bonus_status);

    // Sanity checks
    assert_eq!(staking_info.period_number(), period_number);
    assert!(staking_info.is_bonus_eligible());
    assert!(staking_info.total_staked_amount().is_zero());
    assert!(staking_info.is_empty());
    assert!(staking_info.era().is_zero());

    // Add some staked amount during `Voting` period
    let era_1 = 7;
    let vote_stake_amount_1 = 11;
    let stake_amount_1 = StakeAmount {
        voting: vote_stake_amount_1,
        build_and_earn: 0,
        era: era_1,
        period: period_number,
    };
    staking_info.stake(stake_amount_1, era_1, 0);
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
    assert!(staking_info.previous_staked.is_empty());

    // Add some staked amount during `BuildAndEarn` period
    let era_2 = 9;
    let bep_stake_amount_1 = 23;
    let stake_amount_2 = StakeAmount {
        voting: 0,
        build_and_earn: bep_stake_amount_1,
        era: era_2,
        period: period_number,
    };

    staking_info.stake(stake_amount_2, era_2, 0);
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

    assert_eq!(staking_info.previous_staked.total(), vote_stake_amount_1);
    assert_eq!(
        staking_info.previous_staked.era, era_2,
        "Must be equal to the previous staked era."
    );
}

#[test]
fn singular_previous_stake_is_ok() {
    let period_number = 1;
    let bonus_status = 0;
    let mut staking_info = SingularStakingInfo::new(period_number, bonus_status);

    // Add some staked amount during `Build&Earn` period
    let era_1 = 7;
    let bep_stake_amount_1 = 10;
    let stake_amount_1 = StakeAmount {
        voting: 0,
        build_and_earn: bep_stake_amount_1,
        era: era_1,
        period: period_number,
    };
    staking_info.stake(stake_amount_1, era_1, 0);
    assert!(staking_info.previous_staked.is_empty());

    // Add more staked amount during same era
    let bep_stake_amount_2 = 20;
    let stake_amount_2 = StakeAmount {
        voting: 0,
        build_and_earn: bep_stake_amount_2,
        era: era_1,
        period: period_number,
    };
    staking_info.stake(stake_amount_2, era_1, 0);
    assert!(staking_info.previous_staked.is_empty());

    // Add more staked amount during a future era
    let era_2 = 17;
    let bep_stake_amount_3 = 30;
    let stake_amount_3 = StakeAmount {
        voting: 0,
        build_and_earn: bep_stake_amount_3,
        era: era_2,
        period: period_number,
    };
    staking_info.stake(stake_amount_3, era_2, 0);
    assert_eq!(
        staking_info.previous_staked.total(),
        stake_amount_1.total() + stake_amount_2.total()
    );
    assert_eq!(
        staking_info.staked.total(),
        stake_amount_1.total() + stake_amount_2.total() + stake_amount_3.total()
    );
}

#[test]
fn singular_staking_info_unstake_during_voting_is_ok() {
    get_u8_type!(MaxMoves, 1);
    type TestBonusStatusWrapper = BonusStatusWrapper<MaxMoves>;

    let period_number = 3;
    let bonus_status = *TestBonusStatusWrapper::default();
    let mut staking_info = SingularStakingInfo::new(period_number, bonus_status);

    // Prep actions
    let era_1 = 2;
    let vote_stake_amount_1 = 11;
    let stake_amount_1 = StakeAmount {
        voting: vote_stake_amount_1,
        build_and_earn: 0,
        era: era_1,
        period: period_number,
    };
    staking_info.stake(stake_amount_1, era_1, 0);

    // 1. Unstake some amount during `Voting` period, bonus should remain as expected.
    let unstake_amount_1 = 5;
    let expected_stake_amount = StakeAmount {
        voting: unstake_amount_1,
        build_and_earn: 0,
        era: era_1 + 1,
        period: period_number,
    };
    assert_eq!(
        staking_info.unstake(unstake_amount_1, era_1, Subperiod::Voting),
        (vec![expected_stake_amount], bonus_status)
    );
    assert_eq!(
        staking_info.total_staked_amount(),
        vote_stake_amount_1 - unstake_amount_1
    );
    assert!(staking_info.is_bonus_eligible());
    assert_eq!(
        staking_info.era(),
        era_1 + 1,
        "Stake era should remain valid."
    );

    assert!(staking_info.previous_staked.is_empty());
    assert!(staking_info.previous_staked.era.is_zero());

    // 2. Fully unstake, attempting to underflow
    let era_2 = era_1 + 2;
    let remaining_stake = staking_info.total_staked_amount();
    let expected_stake_amount_1 = StakeAmount {
        voting: remaining_stake,
        build_and_earn: 0,
        era: era_2,
        period: period_number,
    };
    let expected_stake_amount_2 = StakeAmount {
        voting: remaining_stake,
        build_and_earn: 0,
        era: era_2 + 1,
        period: period_number,
    };
    assert_eq!(
        staking_info.unstake(remaining_stake + 1, era_2, Subperiod::Voting),
        (
            vec![expected_stake_amount_1, expected_stake_amount_2],
            bonus_status
        ),
        "Also chipping away from the next era since the unstake is relevant to the ongoing era."
    );
    assert!(staking_info.total_staked_amount().is_zero());
    assert!(staking_info.era().is_zero());

    assert!(staking_info.previous_staked.is_empty());
    assert!(staking_info.previous_staked.era.is_zero());
}

#[test]
fn singular_staking_info_unstake_during_bep_is_ok() {
    get_u8_type!(MaxMoves, 1);
    type TestBonusStatusWrapper = BonusStatusWrapper<MaxMoves>;

    let period_number = 3;
    let bonus_status = *TestBonusStatusWrapper::default();
    let mut staking_info = SingularStakingInfo::new(period_number, bonus_status);

    // Sanity check
    assert_eq!(
        staking_info.bonus_status,
        MaxMoves::get() + 1,
        "Sanity check to cover all scenarios.",
    );

    // Prep actions
    let era_1 = 3;
    let vote_stake_amount_prep = 11;
    let bep_stake_amount_prep = 1;
    let stake_amount_1 = StakeAmount {
        voting: vote_stake_amount_prep,
        build_and_earn: bep_stake_amount_prep,
        era: era_1 - 1,
        period: period_number,
    };
    staking_info.stake(stake_amount_1, era_1 - 1, 0);

    let bep_stake_amount_1 = 23;
    let stake_amount_2 = StakeAmount {
        voting: 0,
        build_and_earn: bep_stake_amount_1,
        era: era_1,
        period: period_number,
    };
    staking_info.stake(stake_amount_2, era_1, 0);

    let expected_prep_amount = vote_stake_amount_prep + bep_stake_amount_prep;
    assert_eq!(staking_info.previous_staked.total(), expected_prep_amount);
    assert_eq!(staking_info.previous_staked.era, era_1);

    // 1st scenario - Unstake some of the amount staked during B&E period
    let unstake_1 = 5;
    let expected_stake_amount_1 = StakeAmount {
        voting: 0,
        build_and_earn: unstake_1,
        era: era_1 + 1,
        period: period_number,
    };
    assert_eq!(
        staking_info.unstake(unstake_1, era_1, Subperiod::BuildAndEarn),
        // We're unstaking from the `era_1 + 1` because stake was made for that era
        (vec![expected_stake_amount_1], bonus_status)
    );
    assert_eq!(
        staking_info.total_staked_amount(),
        expected_prep_amount + bep_stake_amount_1 - unstake_1
    );
    assert_eq!(
        staking_info.staked_amount(Subperiod::Voting),
        vote_stake_amount_prep
    );
    assert_eq!(
        staking_info.staked_amount(Subperiod::BuildAndEarn),
        bep_stake_amount_prep + bep_stake_amount_1 - unstake_1
    );
    assert!(staking_info.is_bonus_eligible());
    assert_eq!(
        staking_info.era(),
        era_1 + 1,
        "Stake era should remain valid."
    );

    // No changes to the previous staked amount
    assert_eq!(staking_info.previous_staked.total(), expected_prep_amount);
    assert_eq!(staking_info.previous_staked.era, era_1);

    // 2nd scenario - Ensure that staked amount is larger than the previous stake amount, and then
    // unstake enough to result in some overflow of the stake delta.
    let bep_stake_amount_2 = 13;
    let stake_amount = StakeAmount {
        voting: 0,
        build_and_earn: bep_stake_amount_2,
        era: era_1,
        period: period_number,
    };
    staking_info.stake(stake_amount, era_1, 0);
    // This must remain unchanged since we are still staking for era_1 (same as previous stake operation)
    assert_eq!(staking_info.previous_staked.total(), expected_prep_amount);
    assert_eq!(staking_info.previous_staked.era, era_1);

    let previous_total_stake = staking_info.previous_staked.total();
    let delta = staking_info.staked.total() - staking_info.previous_staked.total();
    let overflow = 1;
    let unstake_2 = delta + overflow;

    let expected_stake_amount_1 = StakeAmount {
        voting: 0,
        build_and_earn: overflow,
        era: era_1,
        period: period_number,
    };
    let expected_stake_amount_2 = StakeAmount {
        voting: 0,
        build_and_earn: unstake_2,
        era: era_1 + 1,
        period: period_number,
    };
    assert_eq!(
        staking_info.unstake(unstake_2, era_1, Subperiod::BuildAndEarn),
        (
            vec![expected_stake_amount_1, expected_stake_amount_2],
            bonus_status
        )
    );

    assert_eq!(
        staking_info.total_staked_amount(),
        expected_prep_amount + bep_stake_amount_1 + bep_stake_amount_2 - unstake_1 - unstake_2
    );
    assert_eq!(
        staking_info.staked_amount(Subperiod::Voting),
        vote_stake_amount_prep
    );
    assert_eq!(
        staking_info.staked_amount(Subperiod::BuildAndEarn),
        bep_stake_amount_prep + bep_stake_amount_1 + bep_stake_amount_2 - unstake_1 - unstake_2
    );

    assert_eq!(
        staking_info.previous_staked.total(),
        previous_total_stake - overflow
    );
    assert_eq!(staking_info.previous_staked.era, era_1);

    // 3rd scenario - unstake all of the amount staked during B&E subperiod, and then some more.
    // The point is to take a chunk from the voting subperiod stake too.
    let current_total_stake = staking_info.total_staked_amount();
    let current_bep_stake = staking_info.staked_amount(Subperiod::BuildAndEarn);
    let voting_stake_overflow = 2;
    let unstake_2 = current_bep_stake + voting_stake_overflow;
    let era_2 = era_1 + 3;

    let expected_stake_amount_1 = StakeAmount {
        voting: voting_stake_overflow,
        build_and_earn: current_bep_stake,
        era: era_2,
        period: period_number,
    };
    let expected_stake_amount_2 = StakeAmount {
        voting: voting_stake_overflow,
        build_and_earn: current_bep_stake,
        era: era_2 + 1,
        period: period_number,
    };
    assert_eq!(
        staking_info.unstake(unstake_2, era_2, Subperiod::BuildAndEarn),
        (
            vec![expected_stake_amount_1, expected_stake_amount_2],
            bonus_status - 1
        ),
        "Also chipping away from the next era since the unstake is relevant to the ongoing era."
    );
    assert_eq!(
        staking_info.total_staked_amount(),
        current_total_stake - unstake_2
    );
    assert_eq!(
        staking_info.staked_amount(Subperiod::Voting),
        vote_stake_amount_prep - voting_stake_overflow
    );
    assert!(staking_info
        .staked_amount(Subperiod::BuildAndEarn)
        .is_zero());
    assert_eq!(
        staking_info.bonus_status,
        bonus_status - 1,
        "Bonus status moves counter should have been decreased."
    );
    assert!(staking_info.is_bonus_eligible(), "Bonus should have been preserved since it is the first partial unstake from the 'voting subperiod' stake");
    assert_eq!(staking_info.era(), era_2);

    assert_eq!(staking_info.previous_staked.total(), current_total_stake);
    assert_eq!(staking_info.previous_staked.era, era_2 - 1);

    // 4th scenario - Bonus forfeited
    // Fully exhaust the bonus by performing another unstake during the B&E subperiod
    // Voting stake exists (not merged into b&e) but got chipped
    let era_3 = era_2 + 2;
    let unstake_3 = 5;

    let expected_stake_amount_1 = StakeAmount {
        voting: unstake_3,
        build_and_earn: 0,
        era: era_3,
        period: period_number,
    };
    let expected_stake_amount_2 = StakeAmount {
        voting: unstake_3,
        build_and_earn: 0,
        era: era_3 + 1,
        period: period_number,
    };
    assert_eq!(
        staking_info.unstake(unstake_3, era_3, Subperiod::BuildAndEarn),
        (vec![expected_stake_amount_1, expected_stake_amount_2], 0)
    );
    assert!(
        !staking_info.is_bonus_eligible(),
        "Bonus should no longer be active."
    );
    assert_eq!(staking_info.era(), era_3);
}

#[test]
fn singular_staking_info_unstake_stake_amount_entries_are_ok() {
    let period_number = 1;

    // 1. Unstake only reduces the amount from the future era
    {
        let era = 3;
        let bep_stake_amount = 13;
        let stake_amount = StakeAmount {
            voting: 0,
            build_and_earn: bep_stake_amount,
            era,
            period: period_number,
        };
        let unstake_amount = 3;
        let mut staking_info = SingularStakingInfo::new(period_number, 0);
        staking_info.stake(stake_amount, era, 0);

        let expected_stake_amount = StakeAmount {
            voting: 0,
            build_and_earn: unstake_amount,
            era: era + 1,
            period: period_number,
        };
        assert_eq!(
            staking_info.unstake(unstake_amount, era, Subperiod::BuildAndEarn),
            (vec![expected_stake_amount], 0)
        );
    }

    // 2. Unstake reduces the amount from the current & next era.
    {
        let era = 3;
        let bep_stake_amount = 17;
        let stake_amount = StakeAmount {
            voting: 0,
            build_and_earn: bep_stake_amount,
            era,
            period: period_number,
        };
        let unstake_amount = 5;
        let mut staking_info = SingularStakingInfo::new(period_number, 0);
        staking_info.stake(stake_amount, era, 0);

        let expected_stake_amount_1 = StakeAmount {
            voting: 0,
            build_and_earn: unstake_amount,
            era: era + 1,
            period: period_number,
        };
        let expected_stake_amount_2 = StakeAmount {
            voting: 0,
            build_and_earn: unstake_amount,
            era: era + 2,
            period: period_number,
        };
        assert_eq!(
            staking_info
                .clone()
                .unstake(unstake_amount, era + 1, Subperiod::BuildAndEarn),
            (vec![expected_stake_amount_1, expected_stake_amount_2], 0)
        );
    }

    // 3. Unstake reduces the amount from the current & next era.
    //    Unlike the previous example, entries are not aligned with the current era
    {
        let era = 3;
        let bep_stake_amount = 17;
        let unstake_amount = 5;
        let stake_amount = StakeAmount {
            voting: 0,
            build_and_earn: bep_stake_amount,
            era,
            period: period_number,
        };
        let mut staking_info = SingularStakingInfo::new(period_number, 0);
        staking_info.stake(stake_amount, era, 0);

        let expected_stake_amount_1 = StakeAmount {
            voting: 0,
            build_and_earn: unstake_amount,
            era: era + 2,
            period: period_number,
        };
        let expected_stake_amount_2 = StakeAmount {
            voting: 0,
            build_and_earn: unstake_amount,
            era: era + 3,
            period: period_number,
        };
        assert_eq!(
            staking_info
                .clone()
                .unstake(unstake_amount, era + 2, Subperiod::BuildAndEarn),
            (vec![expected_stake_amount_1, expected_stake_amount_2], 0)
        );
    }
}

#[test]
fn singular_staking_stake_with_bonus_status() {
    let voting_amount = 100;
    let bep_amount = 30;
    let prep_stake_amount = StakeAmount {
        era: 1,
        voting: 0,
        build_and_earn: bep_amount,
        period: 0,
    };

    // Prep - StakeAmount with forfeited bonus and voting stake
    let mut staking_info = SingularStakingInfo::new(0, 0);
    staking_info.stake(prep_stake_amount, 1, 0);
    assert_eq!(
        staking_info.bonus_status, 0,
        "Bonus status should be initialized to 0 before staking"
    );

    // Scenario 1 - Stake again but with incoming bonus status
    let incoming_bonus_status = 1;
    let stake_amount = StakeAmount {
        era: 1,
        voting: voting_amount,
        build_and_earn: bep_amount,
        period: 0,
    };
    staking_info.stake(stake_amount, 1, incoming_bonus_status);

    // Check if the bonus status is updated
    assert_eq!(
        staking_info.bonus_status, incoming_bonus_status,
        "Bonus status should be updated to incoming one"
    );
    // Ensure that the previous voting stake amount was moved to BuildAndEarn
    assert_eq!(
        staking_info.staked_amount(Subperiod::Voting),
        voting_amount,
        "Voting amount should increase correctly"
    );
    assert_eq!(
        staking_info.staked_amount(Subperiod::BuildAndEarn),
        bep_amount + bep_amount,
        "BuildAndEarn amount should increase correctly"
    );

    // Scenario 2 - bonus_status is not 0 anymore and new voting amount is staked on same staking_info
    let voting_amount_snapshot = staking_info.staked_amount(Subperiod::Voting);
    let bep_amount_snapshot = staking_info.staked_amount(Subperiod::BuildAndEarn);
    staking_info.stake(stake_amount, 1, 0);
    assert_eq!(
        staking_info.bonus_status, incoming_bonus_status,
        "Bonus status should be initialized to prev incoming_bonus_status"
    );

    let incoming_bonus_status_2 = 10;
    let expected_merged_bonus_status = (incoming_bonus_status_2 + staking_info.bonus_status) / 2; // (both bonus_status are not 0)
    staking_info.stake(stake_amount, 1, incoming_bonus_status_2);

    // Ensure that the StakeAmount is increased with bonus status merged
    assert_eq!(
        staking_info.bonus_status, expected_merged_bonus_status,
        "Bonus status should remain the same"
    );
    assert_eq!(
        staking_info.staked_amount(Subperiod::Voting),
        voting_amount_snapshot + voting_amount * 2,
        "Voting amount should increase correctly"
    );
    assert_eq!(
        staking_info.staked_amount(Subperiod::BuildAndEarn),
        bep_amount_snapshot + bep_amount * 2,
        "BuildAndEarn amount should increase correctly"
    );
}

#[test]
fn contract_stake_amount_basic_get_checks_work() {
    // Sanity checks for empty struct
    let contract_stake = ContractStakeAmount {
        staked: StakeAmount::default(),
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
    let voting_amount_1 = 31;
    let amount_1 = StakeAmount {
        voting: voting_amount_1,
        build_and_earn: 0,
        era: era_1,
        period: period_1,
    };
    contract_stake.stake(amount_1, era_1, period_1);
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
    assert_eq!(entry_1_1.total(), amount_1.total());
    assert_eq!(entry_1_1.for_type(Subperiod::Voting), voting_amount_1);
    assert!(entry_1_1.for_type(Subperiod::BuildAndEarn).is_zero());

    // 2nd scenario - stake some more to the same era but different period type, and verify state change.
    let bep_amount_2 = 31;
    let amount_2 = StakeAmount {
        voting: 0,
        build_and_earn: bep_amount_2,
        era: era_1,
        period: period_1,
    };
    contract_stake.stake(amount_2, era_1, period_1);
    let entry_1_2 = contract_stake.get(stake_era_1, period_1).unwrap();
    assert_eq!(entry_1_2.era, stake_era_1);
    assert_eq!(entry_1_2.total(), amount_1.total() + amount_2.total());
    assert_eq!(entry_1_2.for_type(Subperiod::Voting), amount_1.voting);
    assert_eq!(
        entry_1_2.for_type(Subperiod::BuildAndEarn),
        amount_2.build_and_earn
    );
    assert!(
        contract_stake.staked.is_empty(),
        "Only future entry should be modified."
    );
    assert!(contract_stake.staked_future.is_some());

    // 3rd scenario - stake more to the next era, while still in the same period.
    let era_2 = era_1 + 2;
    let stake_era_2 = era_2 + 1;
    let bep_amount_3 = 37;
    let amount_3 = StakeAmount {
        voting: 0,
        build_and_earn: bep_amount_3,
        era: era_2,
        period: period_1,
    };
    contract_stake.stake(amount_3, era_2, period_1);
    let entry_2_1 = contract_stake
        .get(era_2, period_1)
        .expect("Since stake will change next era, entries should be aligned.");
    let entry_2_2 = contract_stake.get(stake_era_2, period_1).unwrap();
    assert_eq!(
        entry_2_1.for_type(Subperiod::Voting),
        entry_1_2.for_type(Subperiod::Voting)
    );
    assert_eq!(
        entry_2_1.for_type(Subperiod::BuildAndEarn),
        entry_1_2.for_type(Subperiod::BuildAndEarn)
    );
    assert_eq!(entry_2_2.era, stake_era_2);
    assert_eq!(entry_2_2.period, period_1);
    assert_eq!(
        entry_2_2.total(),
        entry_2_1.total() + amount_3.total(),
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
    let bep_amount_4 = 41;
    let amount_4 = StakeAmount {
        voting: 0,
        build_and_earn: bep_amount_4,
        era: era_3,
        period: period_2,
    };

    contract_stake.stake(amount_4, era_3, period_2);
    assert!(
        contract_stake.get(era_2, period_1).is_none(),
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
        amount_4.total(),
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
    let bep_amount_5 = 41;
    let amount_5 = StakeAmount {
        voting: 0,
        build_and_earn: bep_amount_5,
        era: era_4,
        period: period_2,
    };
    contract_stake.stake(amount_5, era_4, period_2);
    let entry_4_1 = contract_stake.get(stake_era_3, period_2).unwrap();
    let entry_4_2 = contract_stake.get(stake_era_4, period_2).unwrap();
    assert_eq!(entry_4_1, entry_3_1, "Old entry must remain unchanged.");
    assert_eq!(entry_4_2.era, stake_era_4);
    assert_eq!(entry_4_2.period, period_2);
    assert_eq!(entry_4_2.total(), amount_4.total() + amount_5.total());
    assert!(
        !contract_stake.staked.is_empty(),
        "staked should keep the old future entry"
    );
    assert!(contract_stake.staked_future.is_some());
}

#[test]
fn contract_stake_amount_basic_unstake_is_ok() {
    let mut contract_stake = ContractStakeAmount::default();

    // Prep action - create a stake entry
    let era_1 = 2;
    let era_2 = era_1 + 1;
    let period = 3;
    let period_info = PeriodInfo {
        number: period,
        subperiod: Subperiod::Voting,
        next_subperiod_start_era: 20,
    };
    let vp_stake_amount = 47;
    let bep_stake_amount = 53;
    let stake_amount = StakeAmount {
        voting: vp_stake_amount,
        build_and_earn: bep_stake_amount,
        era: era_1,
        period,
    };
    contract_stake.stake(stake_amount, era_1, period);
    let total_stake_amount = stake_amount.total();

    // 1st scenario - unstake some amount from the next era, B&E subperiod
    let amount_1 = 5;
    let unstake_amount_1 = StakeAmount {
        voting: 0,
        build_and_earn: amount_1,
        era: era_1 + 1,
        period,
    };
    contract_stake.unstake(&vec![unstake_amount_1], period_info, era_1);
    assert_eq!(
        contract_stake.total_staked_amount(period),
        total_stake_amount - amount_1
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::Voting),
        vp_stake_amount
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::BuildAndEarn),
        bep_stake_amount - amount_1
    );
    assert!(contract_stake.staked.is_empty());
    assert!(contract_stake.staked_future.is_some());

    // 2nd scenario - unstake in the next era, expect entry alignment
    let period_info = PeriodInfo {
        number: period,
        subperiod: Subperiod::BuildAndEarn,
        next_subperiod_start_era: 40,
    };

    let unstake_amount_2 = StakeAmount {
        voting: 0,
        build_and_earn: amount_1,
        era: era_2,
        period,
    };
    contract_stake.unstake(&vec![unstake_amount_2], period_info, era_2);
    assert_eq!(
        contract_stake.total_staked_amount(period),
        total_stake_amount - amount_1 * 2
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::Voting),
        vp_stake_amount
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::BuildAndEarn),
        bep_stake_amount - amount_1 * 2,
    );
    assert!(
        !contract_stake.staked.is_empty(),
        "future entry should be moved over to the current entry"
    );
    assert!(
        contract_stake.staked_future.is_none(),
        "future entry should be cleaned up since it refers to the current era"
    );

    // 3rd scenario - unstake such amount we chip away from the Voting subperiod stake amount
    let voting_unstake_amount = 2;
    let bep_unstake_amount = contract_stake.staked_amount(period, Subperiod::BuildAndEarn);
    let unstake_amount_3 = StakeAmount {
        voting: voting_unstake_amount,
        build_and_earn: bep_unstake_amount,
        era: era_2,
        period,
    };
    contract_stake.unstake(&vec![unstake_amount_3], period_info, era_2);
    assert_eq!(
        contract_stake.total_staked_amount(period),
        total_stake_amount
            - (unstake_amount_1.total() + unstake_amount_2.total() + unstake_amount_3.total())
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::Voting),
        vp_stake_amount - voting_unstake_amount
    );
    assert!(contract_stake
        .staked_amount(period, Subperiod::BuildAndEarn)
        .is_zero(),);

    // 4th scenario - bump up unstake eras by more than 1, entries should be aligned to the current era
    let era_3 = era_2 + 3;
    let voting_unstake_amount_4 = 7;
    let unstake_amount_4 = StakeAmount {
        voting: voting_unstake_amount_4,
        build_and_earn: 0,
        era: era_3,
        period,
    };
    contract_stake.unstake(&vec![unstake_amount_4], period_info, era_3);
    assert_eq!(
        contract_stake.total_staked_amount(period),
        total_stake_amount
            - (unstake_amount_1.total()
                + unstake_amount_2.total()
                + unstake_amount_3.total()
                + unstake_amount_4.total())
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::Voting),
        vp_stake_amount
            - (unstake_amount_1.voting
                + unstake_amount_2.voting
                + unstake_amount_3.voting
                + unstake_amount_4.voting)
    );
    assert!(contract_stake
        .staked_amount(period, Subperiod::BuildAndEarn)
        .is_zero());
    assert_eq!(
        contract_stake.staked.era, era_3,
        "Should be aligned to the current era."
    );
    assert!(
        contract_stake.staked_future.is_none(),
        "future entry should remain 'None'"
    );

    // 5th scenario - do a full unstake, even with overflow, with existing future entry, expect a cleanup
    let unstake_amount_5 = StakeAmount {
        voting: contract_stake.total_staked_amount(period) + 1,
        build_and_earn: 0,
        era: era_3,
        period,
    };
    contract_stake.unstake(&vec![unstake_amount_5], period_info, era_3);
    assert!(contract_stake.staked.is_empty());
    assert!(contract_stake.staked_future.is_none());
}

#[test]
fn contract_stake_amount_unstake_from_subsperiod_type_is_ok() {
    let mut contract_stake = ContractStakeAmount::default();

    // Prep action - create a stake entry
    let era_1 = 2;
    let period = 3;
    let period_info = PeriodInfo {
        number: period,
        subperiod: Subperiod::Voting,
        next_subperiod_start_era: 20,
    };
    let vp_stake_amount = 47;
    let bep_stake_amount = 53;
    let stake_amount = StakeAmount {
        voting: vp_stake_amount,
        build_and_earn: bep_stake_amount,
        era: era_1,
        period,
    };
    contract_stake.stake(stake_amount, era_1, period);
    let total_stake_amount = stake_amount.total();

    // unstake some amount from Voting stake don't affect B&E stake
    let amount = 1;
    let unstake_amount = StakeAmount {
        voting: amount,
        build_and_earn: 0,
        era: era_1 + 1,
        period,
    };
    contract_stake.unstake(&vec![unstake_amount], period_info, era_1);
    assert_eq!(
        contract_stake.total_staked_amount(period),
        total_stake_amount - amount
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::Voting),
        vp_stake_amount - amount
    );
    assert_eq!(
        contract_stake.staked_amount(period, Subperiod::BuildAndEarn),
        bep_stake_amount
    );
}

#[test]
fn contract_stake_amount_advanced_unstake_is_ok() {
    let mut contract_stake = ContractStakeAmount::default();

    // Prep action - create staked & staked_future fields
    let era_1 = 3;
    let era_2 = era_1 + 1;
    let period = 1;
    let period_info = PeriodInfo {
        number: period,
        subperiod: Subperiod::Voting,
        next_subperiod_start_era: 20,
    };
    let vp_stake_amount = 31;
    let bep_stake_amount = 19;

    // Stake in two consecutive eras. Entries will be aligned.
    let stake_amount_1 = StakeAmount {
        voting: vp_stake_amount,
        build_and_earn: 0,
        era: era_1,
        period,
    };
    let stake_amount_2 = StakeAmount {
        voting: 0,
        build_and_earn: bep_stake_amount,
        era: era_2,
        period,
    };
    contract_stake.stake(stake_amount_1, era_1, period);
    contract_stake.stake(stake_amount_2, era_2, period);
    let total_stake_amount = stake_amount_1.total() + stake_amount_2.total();

    // Unstake some amount from both staked & staked_future fields
    let unstake_amount_1 = StakeAmount {
        voting: 2,
        build_and_earn: 0,
        era: era_2,
        period,
    };
    let unstake_amount_2 = StakeAmount {
        voting: 0,
        build_and_earn: 3,
        era: era_2 + 1,
        period,
    };
    contract_stake.unstake(
        &vec![unstake_amount_1, unstake_amount_2],
        period_info,
        era_2,
    );

    // Verify future era staked values
    assert_eq!(
        contract_stake.staked_future.expect("Must exist").total(),
        total_stake_amount - unstake_amount_2.total()
    );
    assert_eq!(
        contract_stake.staked_future.expect("Must exist").voting,
        stake_amount_1.voting
    );
    assert_eq!(
        contract_stake
            .staked_future
            .expect("Must exist")
            .build_and_earn,
        stake_amount_2.total() - unstake_amount_2.total()
    );

    // Verify current era stake values
    assert_eq!(
        contract_stake.staked.total(),
        stake_amount_1.total() - unstake_amount_1.total()
    );
    assert_eq!(
        contract_stake.staked.voting,
        stake_amount_1.voting - unstake_amount_1.voting
    );
    assert!(contract_stake.staked.build_and_earn.is_zero());
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
            TierThreshold::FixedPercentage {
                required_percentage: Perbill::from_percent(3),
            },
            TierThreshold::FixedPercentage {
                required_percentage: Perbill::from_percent(2),
            },
            TierThreshold::FixedPercentage {
                required_percentage: Perbill::from_percent(1),
            },
        ])
        .unwrap(),
        slot_number_args: STANDARD_TIER_SLOTS_ARGS,
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
    new_params.tier_thresholds = BoundedVec::try_from(vec![TierThreshold::FixedPercentage {
        required_percentage: Perbill::from_percent(1),
    }])
    .unwrap();
    assert!(!new_params.is_valid());

    // 5th scenario - DynamicPercentage with valid min/max (min <= max)
    let mut valid_dynamic_params = params.clone();
    valid_dynamic_params.tier_thresholds = BoundedVec::try_from(vec![
        TierThreshold::DynamicPercentage {
            percentage: Perbill::from_percent(2),
            minimum_required_percentage: Perbill::from_percent(1),
            maximum_possible_percentage: Perbill::from_percent(3),
        },
        TierThreshold::DynamicPercentage {
            percentage: Perbill::from_percent(2),
            minimum_required_percentage: Perbill::from_percent(2), // equal min and max is valid
            maximum_possible_percentage: Perbill::from_percent(2),
        },
        TierThreshold::FixedPercentage {
            required_percentage: Perbill::from_percent(1),
        },
    ])
    .unwrap();
    assert!(valid_dynamic_params.is_valid());

    // 6th scenario - DynamicPercentage with invalid min/max (min > max)
    let mut invalid_dynamic_params = params.clone();
    invalid_dynamic_params.tier_thresholds = BoundedVec::try_from(vec![
        TierThreshold::DynamicPercentage {
            percentage: Perbill::from_percent(2),
            minimum_required_percentage: Perbill::from_percent(4), // min > max is invalid
            maximum_possible_percentage: Perbill::from_percent(3),
        },
        TierThreshold::FixedPercentage {
            required_percentage: Perbill::from_percent(2),
        },
        TierThreshold::FixedPercentage {
            required_percentage: Perbill::from_percent(1),
        },
    ])
    .unwrap();
    assert!(!invalid_dynamic_params.is_valid());
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
            TierThreshold::DynamicPercentage {
                percentage: Perbill::from_percent(12),
                minimum_required_percentage: Perbill::from_percent(8),
                maximum_possible_percentage: Perbill::from_percent(100),
            },
            TierThreshold::DynamicPercentage {
                percentage: Perbill::from_percent(7),
                minimum_required_percentage: Perbill::from_percent(5),
                maximum_possible_percentage: Perbill::from_percent(100),
            },
            TierThreshold::DynamicPercentage {
                percentage: Perbill::from_percent(4),
                minimum_required_percentage: Perbill::from_percent(3),
                maximum_possible_percentage: Perbill::from_percent(100),
            },
            TierThreshold::FixedPercentage {
                required_percentage: Perbill::from_percent(3),
            },
        ])
        .unwrap(),
        slot_number_args: STANDARD_TIER_SLOTS_ARGS,
    };
    assert!(params.is_valid(), "Example params must be valid!");

    // Create a configuration with some values
    parameter_types! {
        pub const BaseNativeCurrencyPrice: FixedU128 = FixedU128::from_rational(5, 100);
    }
    let total_issuance: Balance = 9_000_000_000;
    let tier_thresholds = params
        .tier_thresholds
        .iter()
        .map(|t| t.threshold(total_issuance))
        .collect::<Vec<Balance>>()
        .try_into()
        .expect("Invalid number of tier thresholds provided.");

    let init_config = TiersConfiguration::<TiersNum, StandardTierSlots, BaseNativeCurrencyPrice> {
        slots_per_tier: BoundedVec::try_from(vec![10, 20, 30, 40]).unwrap(),
        reward_portion: params.reward_portion.clone(),
        tier_thresholds,
        _phantom: PhantomData::default(),
    };
    assert!(init_config.is_valid(), "Init config must be valid!");

    // Create a new config, based on a new price
    let high_price = FixedU128::from_rational(20, 100); // in production will be expressed in USD
    let new_config = init_config.calculate_new(&params, high_price, total_issuance);
    assert!(new_config.is_valid());

    let low_price = FixedU128::from_rational(1, 100); // in production will be expressed in USD
    let new_config = init_config.calculate_new(&params, low_price, total_issuance);
    assert!(new_config.is_valid());

    // TODO: expand tests, add more sanity checks (e.g. tier 3 requirement should never be lower than tier 4, etc.)
}

#[test]
fn dapp_tier_rewards_basic_tests() {
    get_u32_type!(NumberOfDApps, 8);
    get_u32_type!(NumberOfTiers, 3);

    // Example dApps & rewards
    let dapps = BTreeMap::<DAppId, RankedTier>::from([
        (1, RankedTier::new_saturated(0, 0)),
        (2, RankedTier::new_saturated(0, 0)),
        (3, RankedTier::new_saturated(1, 0)),
        (5, RankedTier::new_saturated(1, 0)),
        (6, RankedTier::new_saturated(2, 0)),
    ]);
    let tier_rewards = vec![300, 20, 1];
    let period = 2;

    let mut dapp_tier_rewards = DAppTierRewards::<NumberOfDApps, NumberOfTiers>::new(
        dapps.clone(),
        tier_rewards.clone(),
        period,
        vec![0, 0, 0],
    )
    .expect("Bounds are respected.");

    // 1st scenario - claim reward for a dApps
    let ranked_tier = dapps[&1];
    assert_eq!(
        dapp_tier_rewards.try_claim(1),
        Ok((tier_rewards[ranked_tier.tier() as usize], ranked_tier))
    );

    let ranked_tier = dapps[&5];
    assert_eq!(
        dapp_tier_rewards.try_claim(5),
        Ok((tier_rewards[ranked_tier.tier() as usize], ranked_tier))
    );

    // 2nd scenario - try to claim already claimed reward
    assert_eq!(
        dapp_tier_rewards.try_claim(1),
        Err(DAppTierError::NoDAppInTiers),
        "Cannot claim the same reward twice."
    );

    // 3rd scenario - claim for a dApp that is not in the list
    assert_eq!(
        dapp_tier_rewards.try_claim(4),
        Err(DAppTierError::NoDAppInTiers),
        "dApp doesn't exist in the list so no rewards can be claimed."
    );
}

#[test]
fn cleanup_marker_works() {
    let cleanup_marker = CleanupMarker::default();
    assert!(!cleanup_marker.has_pending_cleanups());

    let cleanup_marker = CleanupMarker {
        era_reward_index: 1,
        dapp_tiers_index: 2,
        oldest_valid_era: 3,
    };
    assert!(
        cleanup_marker.has_pending_cleanups(),
        "There are pending cleanups for both era rewards and dApp tiers."
    );

    let cleanup_marker = CleanupMarker {
        era_reward_index: 7,
        dapp_tiers_index: 6,
        oldest_valid_era: 7,
    };
    assert!(
        cleanup_marker.has_pending_cleanups(),
        "There are pending cleanups for dApp tiers."
    );

    let cleanup_marker = CleanupMarker {
        era_reward_index: 9,
        dapp_tiers_index: 11,
        oldest_valid_era: 11,
    };
    assert!(
        cleanup_marker.has_pending_cleanups(),
        "There are pending cleanups for era reward spans."
    );
}

#[test]
fn dapp_tier_rewards_with_rank() {
    get_u32_type!(NumberOfDApps, 8);
    get_u32_type!(NumberOfTiers, 3);

    // Example dApps & rewards
    let dapps = BTreeMap::<DAppId, RankedTier>::from([
        (1, RankedTier::new_saturated(0, 5)),
        (2, RankedTier::new_saturated(0, 0)),
        (3, RankedTier::new_saturated(1, 10)),
        (5, RankedTier::new_saturated(1, 5)),
        (6, RankedTier::new_saturated(2, 0)),
    ]);
    let tier_rewards = vec![300, 20, 1];
    let rank_rewards = vec![0, 2, 0];
    let period = 2;

    let mut dapp_tier_rewards = DAppTierRewards::<NumberOfDApps, NumberOfTiers>::new(
        dapps.clone(),
        tier_rewards.clone(),
        period,
        rank_rewards.clone(),
    )
    .expect("Bounds are respected.");

    // has rank but no reward per rank
    // receive only tier reward
    let ranked_tier = dapps[&1];
    assert_eq!(
        dapp_tier_rewards.try_claim(1),
        Ok((tier_rewards[ranked_tier.tier() as usize], ranked_tier))
    );

    // has no rank, receive only tier reward
    let ranked_tier = dapps[&2];
    assert_eq!(
        dapp_tier_rewards.try_claim(2),
        Ok((tier_rewards[ranked_tier.tier() as usize], ranked_tier))
    );

    // receives both tier and rank rewards
    let ranked_tier = dapps[&3];
    let (tier, rank) = ranked_tier.deconstruct();
    assert_eq!(
        dapp_tier_rewards.try_claim(3),
        Ok((
            tier_rewards[tier as usize] + rank_rewards[tier as usize] * rank as Balance,
            ranked_tier
        ))
    );
}

#[test]
fn tier_thresholds_conversion_test() {
    get_u32_type!(TiersNum, 2);
    let total_issuance: Balance = 1_000_000;

    let thresholds: BoundedVec<TierThreshold, TiersNum> = BoundedVec::try_from(vec![
        TierThreshold::FixedPercentage {
            required_percentage: Perbill::from_percent(10),
        },
        TierThreshold::DynamicPercentage {
            percentage: Perbill::from_percent(5),
            minimum_required_percentage: Perbill::from_percent(2),
            maximum_possible_percentage: Perbill::from_percent(100),
        },
    ])
    .unwrap();

    let tier_thresholds: BoundedVec<Balance, TiersNum> = thresholds
        .iter()
        .map(|t| t.threshold(total_issuance))
        .collect::<Vec<Balance>>()
        .try_into()
        .expect("Invalid number of tier thresholds provided.");

    assert_eq!(tier_thresholds[0], 100_000); // 10% of total issuance
    assert_eq!(tier_thresholds[1], 50_000); // 5% of total issuance
}

#[test]
fn tier_configuration_calculate_new_with_maximum_threshold() {
    get_u32_type!(TiersNum, 4);

    let slot_distribution = BoundedVec::<Permill, TiersNum>::try_from(vec![
        Permill::from_percent(10),
        Permill::from_percent(20),
        Permill::from_percent(30),
        Permill::from_percent(40),
    ])
    .unwrap();

    let reward_portion = BoundedVec::<Permill, TiersNum>::try_from(vec![
        Permill::from_percent(10),
        Permill::from_percent(20),
        Permill::from_percent(30),
        Permill::from_percent(40),
    ])
    .unwrap();

    // Create tier thresholds (legacy without maximum)
    let tier_thresholds_legacy = BoundedVec::<TierThreshold, TiersNum>::try_from(vec![
        TierThreshold::DynamicPercentage {
            percentage: Perbill::from_percent(4),
            minimum_required_percentage: Perbill::from_percent(3),
            maximum_possible_percentage: Perbill::from_percent(100),
        },
        TierThreshold::DynamicPercentage {
            percentage: Perbill::from_percent(3),
            minimum_required_percentage: Perbill::from_percent(2),
            maximum_possible_percentage: Perbill::from_percent(100),
        },
        TierThreshold::DynamicPercentage {
            percentage: Perbill::from_percent(2),
            minimum_required_percentage: Perbill::from_percent(1),
            maximum_possible_percentage: Perbill::from_percent(100),
        },
        TierThreshold::FixedPercentage {
            required_percentage: Perbill::from_parts(5_000_000), // 0.5%
        },
    ])
    .unwrap();

    // Create tier thresholds (with maximum)
    let tier_thresholds_with_max = BoundedVec::<TierThreshold, TiersNum>::try_from(vec![
        TierThreshold::DynamicPercentage {
            percentage: Perbill::from_percent(4),
            minimum_required_percentage: Perbill::from_percent(3),
            maximum_possible_percentage: Perbill::from_percent(5),
        },
        TierThreshold::DynamicPercentage {
            percentage: Perbill::from_percent(3),
            minimum_required_percentage: Perbill::from_percent(2),
            maximum_possible_percentage: Perbill::from_percent(4),
        },
        TierThreshold::DynamicPercentage {
            percentage: Perbill::from_percent(2),
            minimum_required_percentage: Perbill::from_percent(1),
            maximum_possible_percentage: Perbill::from_percent(3),
        },
        TierThreshold::FixedPercentage {
            required_percentage: Perbill::from_parts(5_000_000), // 0.5%
        },
    ])
    .unwrap();

    let params_legacy = TierParameters::<TiersNum> {
        slot_distribution: slot_distribution.clone(),
        tier_thresholds: tier_thresholds_legacy,
        reward_portion: reward_portion.clone(),
        slot_number_args: STANDARD_TIER_SLOTS_ARGS,
    };

    let params_with_max = TierParameters::<TiersNum> {
        slot_distribution,
        tier_thresholds: tier_thresholds_with_max,
        reward_portion: reward_portion.clone(),
        slot_number_args: STANDARD_TIER_SLOTS_ARGS,
    };

    // Create a starting configuration with some values
    parameter_types! {
        pub const BaseNativeCurrencyPrice: FixedU128 = FixedU128::from_rational(5, 100);
    }
    let total_issuance: Balance = 8_400_000_000;
    let tier_thresholds = params_legacy
        .tier_thresholds
        .iter()
        .map(|t| t.threshold(total_issuance))
        .collect::<Vec<Balance>>()
        .try_into()
        .expect("Invalid number of tier thresholds provided.");

    let init_config = TiersConfiguration::<TiersNum, StandardTierSlots, BaseNativeCurrencyPrice> {
        slots_per_tier: BoundedVec::try_from(vec![10, 20, 30, 40]).unwrap(),
        reward_portion: reward_portion.clone(),
        tier_thresholds,
        _phantom: PhantomData::default(),
    };
    assert!(init_config.is_valid(), "Init config must be valid!");

    // Test Case: When price decreases significantly, legacy thresholds might exceed the maximum
    let very_low_price = FixedU128::from_rational(1, 100); // 0.2x base price

    // For legacy parameters (no maximum)
    let new_config_legacy =
        init_config.calculate_new(&params_legacy, very_low_price, total_issuance);

    // For parameters with maximum
    let new_config_with_max =
        init_config.calculate_new(&params_with_max, very_low_price, total_issuance);

    // Legacy thresholds will be high
    assert!(new_config_legacy.tier_thresholds[0] > Perbill::from_percent(5) * total_issuance);
    assert!(new_config_legacy.tier_thresholds[1] > Perbill::from_percent(4) * total_issuance);
    assert!(new_config_legacy.tier_thresholds[2] > Perbill::from_percent(3) * total_issuance);
    assert_eq!(
        new_config_legacy.tier_thresholds[3],
        Perbill::from_parts(5_000_000) * total_issuance
    );

    // Maximum thresholds will be capped
    assert_eq!(
        new_config_with_max.tier_thresholds[0],
        Perbill::from_percent(5) * total_issuance
    );
    assert_eq!(
        new_config_with_max.tier_thresholds[1],
        Perbill::from_percent(4) * total_issuance
    );
    assert_eq!(
        new_config_with_max.tier_thresholds[2],
        Perbill::from_percent(3) * total_issuance
    );
    assert_eq!(
        new_config_with_max.tier_thresholds[3],
        Perbill::from_parts(5_000_000) * total_issuance
    );
}
