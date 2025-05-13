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

#![cfg(feature = "try-runtime")]

extern crate alloc;

use crate::{ReferendumIndex, ReferendumInfo, ReferendumInfoOf, Voting, VotingOf};
use alloc::collections::BTreeMap;
use frame_support::traits::Currency;
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_democracy::BoundedCallOf;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::{traits::Zero, SaturatedConversion, TryRuntimeError};
use sp_std::vec::Vec;

const LOG_TARGET: &str = "mbm::democracy";

type BalanceOf<T> = <<T as pallet_democracy::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;
type AccountId<T> = <T as frame_system::Config>::AccountId;
type BlockNumber<T> = <<<T as frame_system::Config>::Block as sp_runtime::traits::Block>::Header as sp_runtime::traits::Header>::Number;
type MaxVotes<T> = <T as pallet_democracy::Config>::MaxVotes;
type VotingType<T> = Voting<BalanceOf<T>, AccountId<T>, BlockNumber<T>, MaxVotes<T>>;

#[derive(Encode, Decode)]
struct DemocracyTryRuntimeState<T: pallet_democracy::Config> {
    referendum_infos: BTreeMap<
        ReferendumIndex,
        ReferendumInfo<BlockNumberFor<T>, BoundedCallOf<T>, BalanceOf<T>>,
    >,
    voting_of: BTreeMap<T::AccountId, VotingType<T>>,
    current_block_number: frame_system::pallet_prelude::BlockNumberFor<T>,
}

pub(crate) fn pre_upgrade_body<T: pallet_democracy::Config + frame_system::Config>(
) -> Result<Vec<u8>, TryRuntimeError> {
    log::info!(
        target: LOG_TARGET,
        "Running democracy-mbm pre-upgrade migration check"
    );

    // Collect referendum data
    let referendum_infos = ReferendumInfoOf::<T>::iter().collect::<BTreeMap<_, _>>();

    log::info!(
    target: LOG_TARGET,
    "Found {} referendum infos to migrate",
    referendum_infos.len()
    );

    // Collect voting data
    let voting_of = VotingOf::<T>::iter().collect::<BTreeMap<_, _>>();

    // Get current block number for validation later
    let current_block_number = frame_system::Pallet::<T>::block_number();

    log::info!(
    target: LOG_TARGET,
    "Found {} votingOf to migrate",
    voting_of.len()
    );

    // Create a state struct to hold all the data
    let state: DemocracyTryRuntimeState<T> = DemocracyTryRuntimeState {
        referendum_infos,
        voting_of,
        current_block_number,
    };

    Ok(state.encode())
}

pub(crate) fn post_upgrade_body<T: pallet_democracy::Config + frame_system::Config>(
    state: Vec<u8>,
) -> Result<(), sp_runtime::TryRuntimeError> {
    log::info!(
        target: LOG_TARGET,
        "Running democracy-mbm post-upgrade migration check"
    );

    let prev_state: DemocracyTryRuntimeState<T> =
        DemocracyTryRuntimeState::<T>::decode(&mut &state[..])
            .expect("Failed to decode the previous storage state");

    // Verify referendum info migration
    log::info!(
        target: LOG_TARGET,
        "Verifying referendum infos"
    );

    for (index, ref_info) in ReferendumInfoOf::<T>::iter() {
        let prev_ref_info = prev_state
            .referendum_infos
            .get(&index)
            .expect("Referendum should exist in previous state");

        match (ref_info.clone(), prev_ref_info.clone()) {
            (ReferendumInfo::Ongoing(status), ReferendumInfo::Ongoing(prev_status)) => {
                // Verify delay has been doubled
                let expected_delay = prev_status
                    .delay
                    .saturated_into::<u32>()
                    .saturating_mul(2)
                    .into();

                log::info!(
                target: LOG_TARGET,
                "assert_eq  ref_info.index:{:?}, delay_before:{:?}, delay after:{:?}",
                index, status.delay, expected_delay
                );

                assert_eq!(status.delay, expected_delay, "Delay should be doubled");

                // Calculate and verify the new end time
                let prev_remaining_blocks = prev_status
                    .end
                    .saturated_into::<u32>()
                    .saturating_sub(prev_state.current_block_number.saturated_into::<u32>());
                let expected_doubled_remaining = prev_remaining_blocks.saturating_mul(2);
                let expected_end = prev_state
                    .current_block_number
                    .saturated_into::<u32>()
                    .saturating_add(expected_doubled_remaining)
                    .into();

                log::info!(
                target: LOG_TARGET,
                "assert_eq  ref_info.index:{:?}, end_before:{:?}, end_after:{:?}",
                index, status.end, expected_end
                );

                assert_eq!(
                    status.end, expected_end,
                    "End time should be correctly adjusted"
                );
            }
            (
                ReferendumInfo::Finished {
                    end: current_end, ..
                },
                ReferendumInfo::Finished { end: prev_end, .. },
            ) => {
                // Verify that end time of finished referendums remains unchanged
                assert_eq!(
                    current_end, prev_end,
                    "End time of finished referendum should not change"
                );
            }

            _ => {
                return Err(sp_runtime::TryRuntimeError::Other(
                    "Referendum state type mismatch between pre and post upgrade",
                ));
            }
        }
    }

    // Verify voting_of migration
    for (account, voting) in VotingOf::<T>::iter() {
        let prev_voting = prev_state
            .voting_of
            .get(&account)
            .expect("Account should have voting state in previous state");

        match (&voting, prev_voting) {
            (
                Voting::Direct { prior, .. },
                Voting::Direct {
                    prior: prev_prior, ..
                },
            ) => {
                if !prev_prior.locked().is_zero() {
                    // Verify lock time has been extended
                    let encoded = prev_prior.encode();
                    let prev_unlock_block =
                        u32::from_le_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
                    let prev_remaining_blocks = prev_unlock_block
                        .saturating_sub(prev_state.current_block_number.saturated_into::<u32>());
                    let expected_remaining = prev_remaining_blocks.saturating_mul(2);
                    let expected_unlock = prev_state
                        .current_block_number
                        .saturated_into::<u32>()
                        .saturating_add(expected_remaining);

                    let new_encoded = prior.encode();
                    let new_unlock_block = u32::from_le_bytes([
                        new_encoded[0],
                        new_encoded[1],
                        new_encoded[2],
                        new_encoded[3],
                    ]);

                    log::info!(
                            target: LOG_TARGET,
                            "assert_eq - Direct: Lock expiry time should be at least doubled  vote.account:{:?}, new_unlock_block:{:?}, expected_unlock:{:?}",
                            account, new_unlock_block, expected_unlock
                    );

                    log::info!(
                            target: LOG_TARGET,
                            "assert_eq - Direct:  amount should be unchanged vote.account:{:?}, ew_unlock_block:{:?}, expected_unlock:{:?}",
                            account, prior.locked(), prev_prior.locked()
                    );

                    assert!(
                        new_unlock_block >= expected_unlock,
                        "Lock expiry time should be at least doubled"
                    );
                    assert_eq!(
                        prior.locked(),
                        prev_prior.locked(),
                        "Lock amount should remain unchanged"
                    );
                }
            }
            (
                Voting::Delegating { prior, .. },
                Voting::Delegating {
                    prior: prev_prior, ..
                },
            ) => {
                if !prev_prior.locked().is_zero() {
                    // Verify lock time has been extended
                    let encoded = prev_prior.encode();
                    let prev_unlock_block =
                        u32::from_le_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
                    let prev_remaining_blocks = prev_unlock_block
                        .saturating_sub(prev_state.current_block_number.saturated_into::<u32>());
                    let expected_remaining = prev_remaining_blocks.saturating_mul(2);
                    let expected_unlock = prev_state
                        .current_block_number
                        .saturated_into::<u32>()
                        .saturating_add(expected_remaining);

                    let new_encoded = prior.encode();
                    let new_unlock_block = u32::from_le_bytes([
                        new_encoded[0],
                        new_encoded[1],
                        new_encoded[2],
                        new_encoded[3],
                    ]);

                    log::info!(
                            target: LOG_TARGET,
                            "assert_eq - Delegate: lock expiry time should be at least doubled  vote.account:{:?}, new_unlock_block:{:?}, expected_unlock:{:?}",
                            account, new_unlock_block, expected_unlock
                    );

                    log::info!(
                            target: LOG_TARGET,
                            "assert_eq - Delegate: amount should be unchanged vote.account:{:?}, ew_unlock_block:{:?}, expected_unlock:{:?}",
                            account, prior.locked(), prev_prior.locked()
                    );

                    assert!(
                        new_unlock_block >= expected_unlock,
                        "Lock expiry time should be at least doubled"
                    );
                    assert_eq!(
                        prior.locked(),
                        prev_prior.locked(),
                        "Lock amount should remain unchanged"
                    );
                }
            }
            _ => {
                return Err(sp_runtime::TryRuntimeError::Other(
                    "Voting state type mismatch between pre and post upgrade",
                ));
            }
        }
    }

    log::info!(
        target: LOG_TARGET,
        "Democracy-mbm migration successfully verified"
    );

    Ok(())
}
