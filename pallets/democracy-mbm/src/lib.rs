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
use pallet_democracy::{ReferendumIndex, ReferendumInfo, ReferendumInfoOf};
use sp_arithmetic::traits::{SaturatedConversion, Saturating};

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarks;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;

const LOG_TARGET: &str = "mbm::democracy";
const PALLET_MIGRATIONS_ID: &[u8; 20] = b"pallet-democracy-mbm";

pub struct LazyMigration<T, W: weights::WeightInfo>(core::marker::PhantomData<(T, W)>);

impl<T: pallet_democracy::Config, W: weights::WeightInfo> SteppedMigration for LazyMigration<T, W> {
    type Cursor = ReferendumIndex;
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
        let required = W::step();
        // If there is not enough weight for a single step, return an error. This case can be
        // problematic if it is the first migration that ran in this block. But there is nothing
        // that we can do about it here.
        if meter.remaining().any_lt(required) {
            return Err(SteppedMigrationError::InsufficientWeight { required });
        }

        let mut count = 0u32;
        let current_block_number =
            frame_system::Pallet::<T>::block_number().saturated_into::<u32>();

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

                ReferendumInfoOf::<T>::iter_from(ReferendumInfoOf::<T>::hashed_key_for(last_key))
            } else {
                // If no cursor is provided, start iterating from the beginning.
                ReferendumInfoOf::<T>::iter()
            };

            if let Some((ref last_key, mut ref_info)) = iter.next() {
                match ref_info {
                    ReferendumInfo::Ongoing(ref mut status) => {
                        // Double the blocks of the delay period
                        status.delay = status
                            .delay
                            .saturated_into::<u32>()
                            .saturating_mul(2)
                            .into();
                        // To migrate end period:
                        // 1. Get the remaining blocks
                        // 2. Multiply it by 2
                        let remaining_blocks = status
                            .end
                            .saturated_into::<u32>()
                            .saturating_sub(current_block_number);
                        status.end = remaining_blocks.saturating_mul(2).into();
                    }
                    ReferendumInfo::Finished { .. } => {
                        // Referendum is finished, skip it.
                        cursor = Some(last_key.clone());
                        continue;
                    }
                }

                meter.consume(W::step());

                ReferendumInfoOf::<T>::insert(last_key, ref_info.clone());

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
    pub trait Config: frame_system::Config + pallet_democracy::Config {}
}
