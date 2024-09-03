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

use frame_support::{
    migrations::{MigrationId, SteppedMigration, SteppedMigrationError},
    weights::WeightMeter,
};
use pallet_vesting::{Vesting, VestingInfo};
use sp_arithmetic::traits::{SaturatedConversion, Saturating};
use sp_runtime::{traits::BlockNumberProvider, Percent};

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarks;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;

const LOG_TARGET: &str = "mbm::vesting";
const PALLET_MIGRATIONS_ID: &[u8; 18] = b"pallet-vesting-mbm";

pub struct LazyMigration<T, W: weights::WeightInfo>(core::marker::PhantomData<(T, W)>);

impl<T: pallet_vesting::Config, W: weights::WeightInfo> SteppedMigration for LazyMigration<T, W> {
    type Cursor = <T as frame_system::Config>::AccountId;
    // Without the explicit length here the construction of the ID would not be infallible.
    type Identifier = MigrationId<18>;

    /// The identifier of this migration. Which should be globally unique.
    fn id() -> Self::Identifier {
        MigrationId {
            pallet_id: *PALLET_MIGRATIONS_ID,
            version_from: 0,
            version_to: 1,
        }
    }

    fn step(
        mut cursor: Option<Self::Cursor>,
        meter: &mut WeightMeter,
    ) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
        let required = W::step(T::MAX_VESTING_SCHEDULES);
        // If there is not enough weight for a single step, return an error. This case can be
        // problematic if it is the first migration that ran in this block. But there is nothing
        // that we can do about it here.
        if meter.remaining().any_lt(required) {
            return Err(SteppedMigrationError::InsufficientWeight { required });
        }

        let mut count = 0u32;
        let para_block_number = frame_system::Pallet::<T>::block_number();
        let current_block_number = T::BlockNumberProvider::current_block_number();

        // We loop here to do as much progress as possible per step.
        loop {
            // stop when remaining weight is lower than step max weight
            if meter.remaining().any_lt(required) {
                break;
            }

            let mut iter = if let Some(last_key) = cursor {
                // If a cursor is provided, start iterating from the stored value
                // corresponding to the last key processed in the previous step.
                // Note that this only works if the old and the new map use the same way to hash
                // storage keys.
                Vesting::<T>::iter_from(Vesting::<T>::hashed_key_for(last_key))
            } else {
                // If no cursor is provided, start iterating from the beginning.
                Vesting::<T>::iter()
            };

            // If there's a next item in the iterator, perform the migration.
            if let Some((ref last_key, mut schedules)) = iter.next() {
                for schedule in schedules.iter_mut() {
                    // remaining locked balance
                    let locked = schedule.locked_at::<T::BlockNumberToBalance>(para_block_number);
                    // reduce unlock `per_block` into half
                    let per_block = Percent::from_percent(50) * schedule.per_block();
                    // remaining blocks to start vesting if vesting hasn't started yet
                    // remaining blocks will be doubled
                    let remaining_blocks = schedule
                        .starting_block()
                        .saturating_sub(para_block_number)
                        .saturating_mul(2u32.into());
                    let start_block = current_block_number.saturating_add(remaining_blocks);

                    *schedule = VestingInfo::new(locked, per_block, start_block);
                }

                // consume the exact weight
                meter.consume(W::step(schedules.len().saturated_into()));

                // Override vesting schedules
                Vesting::<T>::insert(last_key, schedules);

                // inc counter
                count.saturating_inc();

                // Return the processed key as the new cursor.
                cursor = Some(last_key.clone())
            } else {
                // Signal that the migration is complete (no more items to process).
                cursor = None;
                break;
            }
        }
        log::debug!(target: LOG_TARGET, "migrated {count:?} entries");
        Ok(cursor)
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_vesting::Config {}
}
