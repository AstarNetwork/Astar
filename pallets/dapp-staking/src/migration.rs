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

use super::*;
use core::marker::PhantomData;
use frame_support::traits::UncheckedOnRuntimeUpgrade;

#[cfg(feature = "try-runtime")]
mod try_runtime_imports {
    pub use sp_runtime::TryRuntimeError;
}

#[cfg(feature = "try-runtime")]
use try_runtime_imports::*;

/// Exports for versioned migration `type`s for this pallet.
pub mod versioned_migrations {
    use super::*;

    /// Migration V11 to V12:
    /// - Migrate `StaticTierParams` to remove `slot_number_args` and `DynamicPercentage`
    pub type V11ToV12<T> = frame_support::migrations::VersionedMigration<
        11,
        12,
        v12::VersionMigrateV11ToV12<T>,
        Pallet<T>,
        <T as frame_system::Config>::DbWeight,
    >;
}

mod v12 {
    use super::*;

    /// Old `TierThreshold` enum that includes the removed `DynamicPercentage` variant.
    #[derive(Encode, Decode, Clone)]
    pub enum OldTierThreshold {
        FixedPercentage {
            required_percentage: Perbill,
        },
        DynamicPercentage {
            percentage: Perbill,
            minimum_required_percentage: Perbill,
            maximum_possible_percentage: Perbill,
        },
    }

    /// Old `TierParameters` shape (with `slot_number_args` field).
    #[derive(Encode, Decode, Clone)]
    pub struct OldTierParameters<NT: Get<u32>> {
        pub reward_portion: BoundedVec<Permill, NT>,
        pub slot_distribution: BoundedVec<Permill, NT>,
        pub tier_thresholds: BoundedVec<OldTierThreshold, NT>,
        pub slot_number_args: (u64, u64),
        pub tier_rank_multipliers: BoundedVec<u32, NT>,
    }

    pub struct VersionMigrateV11ToV12<T>(PhantomData<T>);

    impl<T: Config> UncheckedOnRuntimeUpgrade for VersionMigrateV11ToV12<T> {
        fn on_runtime_upgrade() -> Weight {
            let mut reads: u64 = 0;
            let mut writes: u64 = 0;

            // Migrate StaticTierParams to new shape (remove slot_number_args, convert DynamicPercentage)
            reads += 1;
            let result = StaticTierParams::<T>::translate::<OldTierParameters<T::NumberOfTiers>, _>(
                |maybe_old_params| match maybe_old_params {
                    Some(old_params) => {
                        let new_tier_thresholds: Vec<TierThreshold> = old_params
                            .tier_thresholds
                            .iter()
                            .map(|t| match t {
                                OldTierThreshold::FixedPercentage {
                                    required_percentage,
                                } => TierThreshold::FixedPercentage {
                                    required_percentage: *required_percentage,
                                },
                                OldTierThreshold::DynamicPercentage { percentage, .. } => {
                                    TierThreshold::FixedPercentage {
                                        required_percentage: *percentage,
                                    }
                                }
                            })
                            .collect();

                        Some(TierParameters {
                            reward_portion: old_params.reward_portion,
                            slot_distribution: old_params.slot_distribution,
                            tier_thresholds: BoundedVec::truncate_from(new_tier_thresholds),
                            tier_rank_multipliers: old_params.tier_rank_multipliers,
                        })
                    }
                    None => None,
                },
            );

            if result.is_err() {
                log::error!(
                    target: LOG_TARGET,
                    "Failed to translate StaticTierParams from old type to new v12 type. \
                     Enabling maintenance mode."
                );
                ActiveProtocolState::<T>::mutate(|state| {
                    state.maintenance = true;
                });
                reads += 1;
                writes += 1;
            } else {
                writes += 1;
                log::info!(
                    target: LOG_TARGET,
                    "StaticTierParams migrated to v12 successfully"
                );
            }

            T::DbWeight::get().reads_writes(reads, writes)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
            log::info!(target: LOG_TARGET, "V11ToV12 pre-upgrade: checking state");
            Ok(Vec::new())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_data: Vec<u8>) -> Result<(), TryRuntimeError> {
            // Verify storage version
            ensure!(
                Pallet::<T>::on_chain_storage_version() == StorageVersion::new(12),
                "Storage version should be 12"
            );

            // Verify StaticTierParams can be decoded with new type
            let params = StaticTierParams::<T>::get();
            ensure!(
                params.is_valid(),
                "StaticTierParams invalid after migration"
            );

            Ok(())
        }
    }
}
