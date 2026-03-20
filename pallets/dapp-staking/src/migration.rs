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
    /// - Prune old `PeriodEnd` entries for periods 0..=6
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

    /// Maximum period number to prune (inclusive).
    const PRUNE_MAX_PERIOD: PeriodNumber = 6;

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

    #[frame_support::storage_alias]
    pub type OldStaticTierParams<T: Config> =
        StorageValue<Pallet<T>, OldTierParameters<<T as Config>::NumberOfTiers>, OptionQuery>;

    pub struct VersionMigrateV11ToV12<T>(PhantomData<T>);

    impl<T: Config> UncheckedOnRuntimeUpgrade for VersionMigrateV11ToV12<T> {
        fn on_runtime_upgrade() -> Weight {
            let mut reads: u64 = 0;
            let mut writes: u64 = 0;

            // 1. Prune old PeriodEnd entries for periods 0..=6
            for period in 0..=PRUNE_MAX_PERIOD {
                reads += 1;
                if PeriodEnd::<T>::take(period).is_some() {
                    writes += 1;
                    log::info!(
                        target: LOG_TARGET,
                        "Pruned PeriodEnd entry for period {}",
                        period
                    );
                }
            }

            // 2. Migrate StaticTierParams to new shape (remove slot_number_args, convert DynamicPercentage)
            reads += 1;
            if let Some(old_params) = OldStaticTierParams::<T>::get() {
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

                let new_params = TierParameters::<T::NumberOfTiers> {
                    reward_portion: old_params.reward_portion,
                    slot_distribution: old_params.slot_distribution,
                    tier_thresholds: BoundedVec::truncate_from(new_tier_thresholds),
                    tier_rank_multipliers: old_params.tier_rank_multipliers,
                };

                if new_params.is_valid() {
                    StaticTierParams::<T>::put(new_params);
                    writes += 1;
                    log::info!(target: LOG_TARGET, "StaticTierParams migrated to v12 successfully");
                } else {
                    log::error!(
                        target: LOG_TARGET,
                        "New TierParameters validation failed during v12 migration. Enabling maintenance mode."
                    );
                    ActiveProtocolState::<T>::mutate(|state| {
                        state.maintenance = true;
                    });
                    reads += 1;
                    writes += 1;
                }
            } else {
                log::warn!(
                    target: LOG_TARGET,
                    "No StaticTierParams found during v12 migration (raw storage empty or decode failed)."
                );
            }

            // 3. Clean up raw storage for old StaticTierParams key (same key, already overwritten above)
            // No additional action needed since StaticTierParams::put already overwrites.

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

            // Verify PeriodEnd entries for periods 0..=6 are removed
            for period in 0..=PRUNE_MAX_PERIOD {
                ensure!(
                    PeriodEnd::<T>::get(period).is_none(),
                    "PeriodEnd entry should be removed for pruned period"
                );
            }

            log::info!(target: LOG_TARGET, "V11ToV12 post-upgrade: all checks passed");
            Ok(())
        }
    }
}
