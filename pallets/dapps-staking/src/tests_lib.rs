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

use super::*;
use frame_support::assert_ok;
use mock::Balance;

#[test]
fn unbonding_info_test() {
    let mut unbonding_info = UnbondingInfo::default();

    // assert basic ops on empty info
    assert!(unbonding_info.is_empty());
    assert!(unbonding_info.len().is_zero());
    let (first_info, second_info) = unbonding_info.clone().partition(2);
    assert!(first_info.is_empty());
    assert!(second_info.is_empty());

    // Prepare unlocking chunks.
    let count = 5;
    let base_amount: Balance = 100;
    let base_unlock_era = 4 * count;
    let mut chunks = vec![];
    for x in 1_u32..=count as u32 {
        chunks.push(UnlockingChunk {
            amount: base_amount * x as Balance,
            unlock_era: base_unlock_era - 3 * x,
        });
    }

    // Add one unlocking chunk and verify basic ops.
    unbonding_info.add(chunks[0 as usize]);

    assert!(!unbonding_info.is_empty());
    assert_eq!(1, unbonding_info.len());
    assert_eq!(chunks[0 as usize].amount, unbonding_info.sum());

    let (first_info, second_info) = unbonding_info.clone().partition(base_unlock_era);
    assert_eq!(1, first_info.len());
    assert_eq!(chunks[0 as usize].amount, first_info.sum());
    assert!(second_info.is_empty());

    // Add remainder and verify basic ops
    for x in unbonding_info.len() as usize..chunks.len() {
        unbonding_info.add(chunks[x]);
        // Ensure internal vec is sorted
        assert!(unbonding_info
            .vec()
            .windows(2)
            .all(|w| w[0].unlock_era <= w[1].unlock_era));
    }
    assert_eq!(chunks.len(), unbonding_info.len() as usize);
    let total: Balance = chunks.iter().map(|c| c.amount).sum();
    assert_eq!(total, unbonding_info.sum());

    let partition_era = chunks[2].unlock_era + 1;
    let (first_info, second_info) = unbonding_info.clone().partition(partition_era);
    assert_eq!(3, first_info.len());
    assert_eq!(2, second_info.len());
    assert_eq!(unbonding_info.sum(), first_info.sum() + second_info.sum());
}

#[test]
fn staker_info_basic() {
    let staker_info = StakerInfo::default();

    assert!(staker_info.is_empty());
    assert_eq!(staker_info.len(), 0);
    assert_eq!(staker_info.latest_staked_value(), 0);
}

#[test]
fn staker_info_stake_ops() {
    let mut staker_info = StakerInfo::default();

    // Do first stake and verify it
    let first_era = 1;
    let first_stake = 100;
    assert_ok!(staker_info.stake(first_era, first_stake));
    assert!(!staker_info.is_empty());
    assert_eq!(staker_info.len(), 1);
    assert_eq!(staker_info.latest_staked_value(), first_stake);

    // Do second stake and verify it
    let second_era = first_era + 1;
    let second_stake = 200;
    assert_ok!(staker_info.stake(second_era, second_stake));
    assert_eq!(staker_info.len(), 2);
    assert_eq!(
        staker_info.latest_staked_value(),
        first_stake + second_stake
    );

    // Do third stake and verify it
    let third_era = second_era + 2; // must be greater than 1 so a `hole` is present
    let third_stake = 333;
    assert_ok!(staker_info.stake(third_era, third_stake));
    assert_eq!(
        staker_info.latest_staked_value(),
        first_stake + second_stake + third_stake
    );
    assert_eq!(staker_info.len(), 3);

    // Do fourth stake and verify it
    let fourth_era = third_era; // ensure that multi-stake in same era works
    let fourth_stake = 444;
    assert_ok!(staker_info.stake(fourth_era, fourth_stake));
    assert_eq!(staker_info.len(), 3);
    assert_eq!(
        staker_info.latest_staked_value(),
        first_stake + second_stake + third_stake + fourth_stake
    );
}

#[test]
fn staker_info_stake_error() {
    let mut staker_info = StakerInfo::default();
    assert_ok!(staker_info.stake(5, 100));
    if let Err(_) = staker_info.stake(4, 100) {
    } else {
        panic!("Mustn't be able to stake with past era.");
    }
}

#[test]
fn staker_info_unstake_ops() {
    let mut staker_info = StakerInfo::default();

    // Unstake on empty staker_info
    assert!(staker_info.is_empty());
    assert_ok!(staker_info.unstake(1, 100));
    assert!(staker_info.is_empty());

    // Prepare some stakes
    let (first_era, second_era) = (1, 3);
    let (first_stake, second_stake) = (110, 222);
    let total_staked = first_stake + second_stake;
    assert_ok!(staker_info.stake(first_era, first_stake));
    assert_ok!(staker_info.stake(second_era, second_stake));

    // Unstake an existing EraStake
    let first_unstake_era = second_era;
    let first_unstake = 55;
    assert_ok!(staker_info.unstake(first_unstake_era, first_unstake));
    assert_eq!(staker_info.len(), 2);
    assert_eq!(
        staker_info.latest_staked_value(),
        total_staked - first_unstake
    );
    let total_staked = total_staked - first_unstake;

    // Unstake an non-existing EraStake
    let second_unstake_era = first_unstake_era + 2;
    let second_unstake = 37;
    assert_ok!(staker_info.unstake(second_unstake_era, second_unstake));
    assert_eq!(staker_info.len(), 3);
    assert_eq!(
        staker_info.latest_staked_value(),
        total_staked - second_unstake
    );
    let total_staked = total_staked - second_unstake;

    // Save this for later
    let temp_staker_info = staker_info.clone();

    // Fully unstake existing EraStake
    assert_ok!(staker_info.unstake(second_unstake_era, total_staked));
    assert_eq!(staker_info.len(), 3);
    assert_eq!(staker_info.latest_staked_value(), 0);

    // Fully unstake non-existing EraStake
    let mut staker_info = temp_staker_info; // restore
    assert_ok!(staker_info.unstake(second_unstake_era + 1, total_staked));
    assert_eq!(staker_info.len(), 4);
    assert_eq!(staker_info.latest_staked_value(), 0);
}

#[test]
fn stake_after_full_unstake() {
    let mut staker_info = StakerInfo::default();

    // Stake some amount
    let first_era = 1;
    let first_stake = 100;
    assert_ok!(staker_info.stake(first_era, first_stake));
    assert_eq!(staker_info.latest_staked_value(), first_stake);

    // Unstake all in next era
    let unstake_era = first_era + 1;
    assert_ok!(staker_info.unstake(unstake_era, first_stake));
    assert!(staker_info.latest_staked_value().is_zero());
    assert_eq!(staker_info.len(), 2);

    // Stake again in the next era
    let restake_era = unstake_era + 2;
    let restake_value = 57;
    assert_ok!(staker_info.stake(restake_era, restake_value));
    assert_eq!(staker_info.latest_staked_value(), restake_value);
    assert_eq!(staker_info.len(), 3);
}

#[test]
fn staker_info_unstake_error() {
    let mut staker_info = StakerInfo::default();
    assert_ok!(staker_info.stake(5, 100));
    if let Err(_) = staker_info.unstake(4, 100) {
    } else {
        panic!("Mustn't be able to unstake with past era.");
    }
}

#[test]
fn staker_info_claim_ops_basic() {
    let mut staker_info = StakerInfo::default();

    // Empty staker info
    assert!(staker_info.is_empty());
    assert_eq!(staker_info.claim(), (0, 0));
    assert!(staker_info.is_empty());

    // Only one unstaked exists
    assert_ok!(staker_info.stake(1, 100));
    assert_ok!(staker_info.unstake(1, 100));
    assert!(staker_info.is_empty());
    assert_eq!(staker_info.claim(), (0, 0));
    assert!(staker_info.is_empty());

    // Only one staked exists
    staker_info = StakerInfo::default();
    let stake_era = 1;
    let stake_value = 123;
    assert_ok!(staker_info.stake(stake_era, stake_value));
    assert_eq!(staker_info.len(), 1);
    assert_eq!(staker_info.claim(), (stake_era, stake_value));
    assert_eq!(staker_info.len(), 1);
}

#[test]
fn staker_info_claim_ops_advanced() {
    let mut staker_info = StakerInfo::default();

    // Two consecutive eras staked, third era contains a gap with the second one
    let (first_stake_era, second_stake_era, third_stake_era) = (1, 2, 4);
    let (first_stake_value, second_stake_value, third_stake_value) = (123, 456, 789);

    assert_ok!(staker_info.stake(first_stake_era, first_stake_value));
    assert_ok!(staker_info.stake(second_stake_era, second_stake_value));
    assert_ok!(staker_info.stake(third_stake_era, third_stake_value));

    // First claim
    assert_eq!(staker_info.len(), 3);
    assert_eq!(staker_info.claim(), (first_stake_era, first_stake_value));
    assert_eq!(staker_info.len(), 2);

    // Second claim
    assert_eq!(
        staker_info.claim(),
        (second_stake_era, first_stake_value + second_stake_value)
    );
    assert_eq!(staker_info.len(), 2);

    // Third claim, expect that 3rd era stake is the same as second
    assert_eq!(
        staker_info.claim(),
        (3, first_stake_value + second_stake_value)
    );
    assert_eq!(staker_info.len(), 1);

    // Fully unstake 5th era
    let total_staked = first_stake_value + second_stake_value + third_stake_value;
    assert_ok!(staker_info.unstake(5, total_staked));
    assert_eq!(staker_info.len(), 2);

    // Stake 7th era (so after it was unstaked)
    let fourth_era = 7;
    let fourth_stake_value = 147;
    assert_ok!(staker_info.stake(fourth_era, fourth_stake_value));
    assert_eq!(staker_info.len(), 3);

    // Claim 4th era
    assert_eq!(staker_info.claim(), (third_stake_era, total_staked));
    assert_eq!(staker_info.len(), 1);

    // Claim 7th era
    assert_eq!(staker_info.claim(), (fourth_era, fourth_stake_value));
    assert_eq!(staker_info.len(), 1);
    assert_eq!(staker_info.latest_staked_value(), fourth_stake_value);

    // Claim future eras
    for x in 1..10 {
        assert_eq!(staker_info.claim(), (fourth_era + x, fourth_stake_value));
        assert_eq!(staker_info.len(), 1);
        assert_eq!(staker_info.latest_staked_value(), fourth_stake_value);
    }
}
