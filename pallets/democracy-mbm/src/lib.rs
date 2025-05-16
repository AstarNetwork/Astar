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

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::StorageVersion;
use frame_support::weights::Weight;
use frame_support::{
    migrations::{MigrationId, SteppedMigration, SteppedMigrationError},
    weights::WeightMeter,
};
pub use pallet::*;
use pallet_democracy::{ReferendumIndex, ReferendumInfo, ReferendumInfoOf, Voting, VotingOf};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use sp_arithmetic::traits::Zero;
use sp_arithmetic::traits::{SaturatedConversion, Saturating};

#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

#[cfg(feature = "try-runtime")]
mod try_runtime;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarks;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
const PALLET_MIGRATIONS_ID: &[u8; 20] = b"pallet-democracy-mbm";

const LOG_TARGET: &str = "mbm::democracy";

/// Exports for versioned migration `type`s for this pallet.
pub mod versioned_migrations {
    use super::*;

    /// Migration V1 to V2 wrapped in a [`frame_support::migrations::VersionedMigration`], ensuring
    /// the migration is only performed when on-chain version is 1.
    pub type V1ToV2<T, InitArgs> = frame_support::migrations::VersionedMigration<
        1,
        2,
        DemocracyMigrationV1ToV2<T, InitArgs>,
        Pallet<T>,
        <T as frame_system::Config>::DbWeight,
    >;
}

/// Progressive migration state to keep track of progress
#[derive(Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen)]
pub enum MigrationStep<T: pallet_democracy::Config> {
    /// Migrating referendum info
    ReferendumInfo(ReferendumIndex),
    /// Finished Migrating referendum info, starting migration VotingOf
    StartingVotingOf,
    /// Migrating VotingOf
    VotingOf(<T as frame_system::Config>::AccountId),
    /// Finished all migrations
    Finished,
}

/// Stores the migration step and the block number at which the migration started
#[derive(Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen)]
pub struct MigrationState<T: pallet_democracy::Config> {
    pub step: MigrationStep<T>,
    pub start_block: u32,
}

type StepResultOf<T> = (MigrationStep<T>, bool);

pub struct DemocracyMigrationV1ToV2<T, W: weights::WeightInfo>(core::marker::PhantomData<(T, W)>);

impl<T: pallet_democracy::Config, W: weights::WeightInfo> SteppedMigration
    for DemocracyMigrationV1ToV2<T, W>
{
    type Cursor = MigrationState<T>;
    // Without the explicit length here the construction of the ID would not be infallible.
    type Identifier = MigrationId<20>;

    /// The identifier of this migration. Which should be globally unique.
    fn id() -> Self::Identifier {
        MigrationId {
            pallet_id: *PALLET_MIGRATIONS_ID,
            version_from: 1,
            version_to: 2,
        }
    }

    fn step(
        mut cursor: Option<Self::Cursor>,
        meter: &mut WeightMeter,
    ) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
        // Check that we have enough weight for at least the next step. If we don't, then the
        // migration cannot be complete.
        let required = match &cursor {
            Some(state) => Self::required_weight(&state.step),
            // Worst case weight for `migration_voting_of`.
            None => W::migration_voting_of(),
        };
        if meter.remaining().any_lt(required) {
            return Err(SteppedMigrationError::InsufficientWeight { required });
        }

        let mut referendum_entries = 0u32;
        let mut referendum_migrated = 0u32;
        let mut voting_entries = 0u32;
        let mut voting_migrated = 0u32;

        // We loop here to do as much progress as possible per step.
        loop {
            // Check that we would have enough weight to perform this step in the worst case
            // scenario.
            let required_weight = match &cursor {
                Some(state) => Self::required_weight(&state.step),
                // Worst case weight for `migration_voting_of`.
                None => W::migration_voting_of(),
            };
            if !meter.can_consume(required_weight) {
                break;
            }

            let next = match &cursor {
                // At first, migrate referendums and get the current block number to set at start_block
                None => {
                    let block_number =
                        frame_system::Pallet::<T>::block_number().saturated_into::<u32>();
                    Self::process_migration_result(
                        Self::migrate_referendum_info(None, block_number),
                        &mut referendum_migrated,
                        &mut referendum_entries,
                        block_number,
                    )
                }
                // Migrate any remaining referendums
                Some(MigrationState {
                    step: MigrationStep::ReferendumInfo(maybe_last_referendum),
                    start_block,
                }) => Self::process_migration_result(
                    Self::migrate_referendum_info(Some(maybe_last_referendum), *start_block),
                    &mut referendum_migrated,
                    &mut referendum_entries,
                    *start_block,
                ),
                // After the last referendum was migrated, start migrating VotingOf
                Some(MigrationState {
                    step: MigrationStep::StartingVotingOf,
                    start_block,
                }) => Self::process_migration_result(
                    Self::migrate_voting_of(None, *start_block),
                    &mut voting_migrated,
                    &mut voting_entries,
                    *start_block,
                ),
                // Keep migrating VotingOf
                Some(MigrationState {
                    step: MigrationStep::VotingOf(maybe_last_vote),
                    start_block,
                }) => Self::process_migration_result(
                    Self::migrate_voting_of(Some(maybe_last_vote), *start_block),
                    &mut voting_migrated,
                    &mut voting_entries,
                    *start_block,
                ),
                Some(MigrationState {
                    step: MigrationStep::Finished,
                    ..
                }) => {
                    StorageVersion::new(Self::id().version_to as u16)
                        .put::<pallet_democracy::Pallet<T>>();
                    log::info!(target: LOG_TARGET, "Democracy MBM migration finished");
                    return Ok(None);
                }
            };

            cursor = Some(next);
            meter.consume(required_weight);
        }

        log::info!(target: LOG_TARGET, "Iterated over {referendum_entries} referendum entries, migrated {referendum_migrated} referendums. Iterated over {voting_entries} votes entries, migrated {voting_migrated} votes");
        Ok(cursor)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
        try_runtime::pre_upgrade_body::<T>()
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
        try_runtime::post_upgrade_body::<T>(state)
    }
}

impl<T: pallet_democracy::Config + frame_system::Config, W: weights::WeightInfo>
    DemocracyMigrationV1ToV2<T, W>
{
    fn required_weight(state: &MigrationStep<T>) -> Weight {
        match state {
            MigrationStep::ReferendumInfo(_) => W::migration_referendum_info(),
            MigrationStep::StartingVotingOf | MigrationStep::VotingOf(_) => {
                W::migration_voting_of()
            }
            MigrationStep::Finished => Weight::zero(),
        }
    }

    fn migrate_referendum_info(
        maybe_last_key: Option<&ReferendumIndex>,
        block_number: u32,
    ) -> StepResultOf<T> {
        let mut iter = if let Some(last_key) = maybe_last_key {
            ReferendumInfoOf::<T>::iter_from_key(last_key)
        } else {
            ReferendumInfoOf::<T>::iter()
        };

        if let Some((last_key, mut ref_info)) = iter.next() {
            match ref_info {
                ReferendumInfo::Ongoing(ref mut status) => {
                    // Double the blocks of the delay period
                    status.delay = status
                        .delay
                        .saturated_into::<u32>()
                        .saturating_mul(2)
                        .into();

                    // For the end time:
                    // 1. Calculate remaining blocks until the original end
                    let remaining_blocks = status
                        .end
                        .saturated_into::<u32>()
                        .saturating_sub(block_number);

                    // 2. Double the remaining blocks
                    let doubled_remaining = remaining_blocks.saturating_mul(2);

                    // 3. Add it to the current block number to get the new end
                    status.end = block_number.saturating_add(doubled_remaining).into();

                    ReferendumInfoOf::<T>::insert(&last_key, ref_info);

                    // entry has been migrated
                    return (MigrationStep::ReferendumInfo(last_key), true);
                }
                ReferendumInfo::Finished { .. } => {
                    // continue;
                }
            }

            (MigrationStep::ReferendumInfo(last_key), false)
        } else {
            (MigrationStep::StartingVotingOf, false)
        }
    }

    fn migrate_voting_of(
        maybe_last_key: Option<&T::AccountId>,
        block_number: u32,
    ) -> StepResultOf<T> {
        let mut iter = if let Some(last_key) = maybe_last_key {
            VotingOf::<T>::iter_from_key(last_key)
        } else {
            VotingOf::<T>::iter()
        };

        if let Some((last_key, mut voting)) = iter.next() {
            match &mut voting {
                Voting::Direct { prior, .. } | Voting::Delegating { prior, .. } => {
                    let lock_amount = prior.locked();

                    if !lock_amount.is_zero() {
                        // 1. Calculate the remaining blocks
                        // as the field block_number is private in PriorLock enum
                        // it encodes the enum and decodes the 4 bytes (as it's an u32)
                        let encoded = prior.encode();
                        let unlock_block_number =
                            u32::from_le_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);

                        // Some past lock still remains if they haven't been unlocked
                        // only migrate when they are in the future
                        if unlock_block_number > block_number {
                            let remaining_blocks = unlock_block_number
                                .saturating_sub(block_number)
                                .saturating_mul(2);
                            let extended_time =
                                block_number.saturating_add(remaining_blocks).into();

                            // 2. Clean the lock by setting block number and balance to 0
                            prior.rejig(u32::MAX.into());

                            // 3. Save the lock with migrated values
                            prior.accumulate(extended_time, lock_amount);

                            VotingOf::<T>::insert(&last_key, voting);

                            // entry has been migrated
                            return (MigrationStep::VotingOf(last_key), true);
                        }
                    }
                }
            }

            (MigrationStep::VotingOf(last_key), false)
        } else {
            (MigrationStep::Finished, false)
        }
    }

    fn process_migration_result(
        result: StepResultOf<T>,
        migrate_counter: &mut u32,
        entries_counter: &mut u32,
        start_block: u32,
    ) -> MigrationState<T> {
        let (step, was_updated) = result;
        if was_updated {
            migrate_counter.saturating_inc();
        }
        entries_counter.saturating_inc();
        MigrationState { step, start_block }
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_democracy::Config {}
}
