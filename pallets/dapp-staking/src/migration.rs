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
use frame_support::{
    migration::clear_storage_prefix,
    migrations::{MigrationId, SteppedMigration, SteppedMigrationError},
    traits::{GetStorageVersion, OnRuntimeUpgrade, UncheckedOnRuntimeUpgrade},
    weights::WeightMeter,
};

#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

/// Exports for versioned migration `type`s for this pallet.
pub mod versioned_migrations {
    use super::*;

    /// Migration V9 to V10 wrapped in a [`frame_support::migrations::VersionedMigration`], ensuring
    /// the migration is only performed when on-chain version is 9.
    pub type V9ToV10<T, MaxPercentages> = frame_support::migrations::VersionedMigration<
        9,
        10,
        v10::VersionMigrateV9ToV10<T, MaxPercentages>,
        Pallet<T>,
        <T as frame_system::Config>::DbWeight,
    >;
}

mod v10 {
    use super::*;
    use crate::migration::v9::{
        TierParameters as TierParametersV9, TierThreshold as TierThresholdV9,
    };

    pub struct VersionMigrateV9ToV10<T, MaxPercentages>(PhantomData<(T, MaxPercentages)>);

    impl<T: Config, MaxPercentages: Get<[Option<Perbill>; 4]>> UncheckedOnRuntimeUpgrade
        for VersionMigrateV9ToV10<T, MaxPercentages>
    {
        fn on_runtime_upgrade() -> Weight {
            let max_percentages = MaxPercentages::get();

            // Update static tier parameters with new max thresholds from the runtime configurable param TierThresholds
            let result = StaticTierParams::<T>::translate::<TierParametersV9<T::NumberOfTiers>, _>(
                |maybe_old_params| match maybe_old_params {
                    Some(old_params) => {
                        let new_tier_thresholds: Vec<TierThreshold> = old_params
                            .tier_thresholds
                            .iter()
                            .enumerate()
                            .map(|(idx, old_threshold)| {
                                let maximum_percentage = if idx < max_percentages.len() {
                                    max_percentages[idx]
                                } else {
                                    None
                                };
                                map_threshold(old_threshold, maximum_percentage)
                            })
                            .collect();

                        let tier_thresholds =
                            BoundedVec::<TierThreshold, T::NumberOfTiers>::try_from(
                                new_tier_thresholds,
                            );

                        match tier_thresholds {
                            Ok(tier_thresholds) => Some(TierParameters {
                                slot_distribution: old_params.slot_distribution,
                                reward_portion: old_params.reward_portion,
                                tier_thresholds,
                                slot_number_args: old_params.slot_number_args,
                            }),
                            Err(err) => {
                                log::error!(
                                    "Failed to convert TierThresholds parameters: {:?}",
                                    err
                                );
                                None
                            }
                        }
                    }
                    _ => None,
                },
            );

            if result.is_err() {
                log::error!("Failed to translate StaticTierParams from previous V9 type to current V10 type. Check TierParametersV9 decoding.");
                // Enable maintenance mode.
                ActiveProtocolState::<T>::mutate(|state| {
                    state.maintenance = true;
                });
                log::warn!("Maintenance mode enabled.");
                return T::DbWeight::get().reads_writes(1, 0);
            }

            T::DbWeight::get().reads_writes(1, 1)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
            let old_params = v9::StaticTierParams::<T>::get().ok_or_else(|| {
                TryRuntimeError::Other(
                    "dapp-staking-v3::migration::v10: No old params found for StaticTierParams",
                )
            })?;
            Ok(old_params.encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(data: Vec<u8>) -> Result<(), TryRuntimeError> {
            // Decode the old values
            let old_params: TierParametersV9<T::NumberOfTiers> = Decode::decode(&mut &data[..])
                .map_err(|_| {
                    TryRuntimeError::Other(
                        "dapp-staking-v3::migration::v10: Failed to decode old values",
                    )
                })?;

            // Get the new values
            let new_config = TierConfig::<T>::get();
            let new_params = StaticTierParams::<T>::get();

            // Verify that new params and new config are valid
            assert!(new_params.is_valid());
            assert!(new_config.is_valid());

            // Verify parameters remain unchanged
            assert_eq!(
                old_params.slot_distribution, new_params.slot_distribution,
                "dapp-staking-v3::migration::v10: Slot distribution has changed"
            );
            assert_eq!(
                old_params.reward_portion, new_params.reward_portion,
                "dapp-staking-v3::migration::v10: Reward portion has changed"
            );
            assert_eq!(
                old_params.tier_thresholds.len(),
                new_params.tier_thresholds.len(),
                "dapp-staking-v3::migration::v10: Number of tier thresholds has changed"
            );

            for (_, (old_threshold, new_threshold)) in old_params
                .tier_thresholds
                .iter()
                .zip(new_params.tier_thresholds.iter())
                .enumerate()
            {
                match (old_threshold, new_threshold) {
                    (
                        TierThresholdV9::FixedPercentage {
                            required_percentage: old_req,
                        },
                        TierThreshold::FixedPercentage {
                            required_percentage: new_req,
                        },
                    ) => {
                        assert_eq!(
                            old_req, new_req,
                            "dapp-staking-v3::migration::v10: Fixed percentage changed",
                        );
                    }
                    (
                        TierThresholdV9::DynamicPercentage {
                            percentage: old_percentage,
                            minimum_required_percentage: old_min,
                        },
                        TierThreshold::DynamicPercentage {
                            percentage: new_percentage,
                            minimum_required_percentage: new_min,
                            maximum_possible_percentage: _, // We don't verify this as it's new
                        },
                    ) => {
                        assert_eq!(
                            old_percentage, new_percentage,
                            "dapp-staking-v3::migration::v10: Percentage changed"
                        );
                        assert_eq!(
                            old_min, new_min,
                            "dapp-staking-v3::migration::v10: Minimum percentage changed"
                        );
                    }
                    _ => {
                        return Err(TryRuntimeError::Other(
                            "dapp-staking-v3::migration::v10: Tier threshold type mismatch",
                        ));
                    }
                }
            }

            let expected_max_percentages = MaxPercentages::get();
            for (idx, tier_threshold) in new_params.tier_thresholds.iter().enumerate() {
                if let TierThreshold::DynamicPercentage {
                    maximum_possible_percentage,
                    ..
                } = tier_threshold
                {
                    let expected_maximum_percentage = if idx < expected_max_percentages.len() {
                        expected_max_percentages[idx]
                    } else {
                        None
                    }
                    .unwrap_or(Perbill::from_percent(100));
                    assert_eq!(
                        *maximum_possible_percentage, expected_maximum_percentage,
                        "dapp-staking-v3::migration::v10: Max percentage differs from expected",
                    );
                }
            }

            // Verify storage version has been updated
            ensure!(
                Pallet::<T>::on_chain_storage_version() >= 10,
                "dapp-staking-v3::migration::v10: Wrong storage version."
            );

            Ok(())
        }
    }

    pub fn map_threshold(old: &TierThresholdV9, max_percentage: Option<Perbill>) -> TierThreshold {
        match old {
            TierThresholdV9::FixedPercentage {
                required_percentage,
            } => TierThreshold::FixedPercentage {
                required_percentage: *required_percentage,
            },
            TierThresholdV9::DynamicPercentage {
                percentage,
                minimum_required_percentage,
            } => TierThreshold::DynamicPercentage {
                percentage: *percentage,
                minimum_required_percentage: *minimum_required_percentage,
                maximum_possible_percentage: max_percentage.unwrap_or(Perbill::from_percent(100)), // Default to 100% if not specified,
            },
        }
    }
}

mod v9 {
    use super::*;
    use frame_support::storage_alias;

    #[derive(Encode, Decode)]
    pub struct TierParameters<NT: Get<u32>> {
        pub reward_portion: BoundedVec<Permill, NT>,
        pub slot_distribution: BoundedVec<Permill, NT>,
        pub tier_thresholds: BoundedVec<TierThreshold, NT>,
        pub slot_number_args: (u64, u64),
    }

    #[derive(Encode, Decode)]
    pub enum TierThreshold {
        FixedPercentage {
            required_percentage: Perbill,
        },
        DynamicPercentage {
            percentage: Perbill,
            minimum_required_percentage: Perbill,
        },
    }

    /// v9 type for [`crate::StaticTierParams`]
    #[storage_alias]
    pub type StaticTierParams<T: Config> =
        StorageValue<Pallet<T>, TierParameters<<T as Config>::NumberOfTiers>>;
}

const PALLET_MIGRATIONS_ID: &[u8; 16] = b"dapp-staking-mbm";

pub struct LazyMigration<T, W: WeightInfo>(PhantomData<(T, W)>);

impl<T: Config, W: WeightInfo> SteppedMigration for LazyMigration<T, W> {
    type Cursor = <T as frame_system::Config>::AccountId;
    // Without the explicit length here the construction of the ID would not be infallible.
    type Identifier = MigrationId<16>;

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
        let required = W::step();
        // If there is not enough weight for a single step, return an error. This case can be
        // problematic if it is the first migration that ran in this block. But there is nothing
        // that we can do about it here.
        if meter.remaining().any_lt(required) {
            return Err(SteppedMigrationError::InsufficientWeight { required });
        }

        let mut count = 0u32;
        let mut migrated = 0u32;
        let current_block_number =
            frame_system::Pallet::<T>::block_number().saturated_into::<u32>();

        // We loop here to do as much progress as possible per step.
        loop {
            if meter.try_consume(required).is_err() {
                break;
            }

            let mut iter = if let Some(last_key) = cursor {
                // If a cursor is provided, start iterating from the stored value
                // corresponding to the last key processed in the previous step.
                // Note that this only works if the old and the new map use the same way to hash
                // storage keys.
                Ledger::<T>::iter_from(Ledger::<T>::hashed_key_for(last_key))
            } else {
                // If no cursor is provided, start iterating from the beginning.
                Ledger::<T>::iter()
            };

            // If there's a next item in the iterator, perform the migration.
            if let Some((ref last_key, mut ledger)) = iter.next() {
                // inc count
                count.saturating_inc();

                if ledger.unlocking.is_empty() {
                    // no unlocking for this account, nothing to update
                    // Return the processed key as the new cursor.
                    cursor = Some(last_key.clone());
                    continue;
                }
                for chunk in ledger.unlocking.iter_mut() {
                    if current_block_number >= chunk.unlock_block {
                        continue; // chunk already unlocked
                    }
                    let remaining_blocks = chunk.unlock_block.saturating_sub(current_block_number);
                    chunk.unlock_block.saturating_accrue(remaining_blocks);
                }

                // Override ledger
                Ledger::<T>::insert(last_key, ledger);

                // inc migrated
                migrated.saturating_inc();

                // Return the processed key as the new cursor.
                cursor = Some(last_key.clone())
            } else {
                // Signal that the migration is complete (no more items to process).
                cursor = None;
                break;
            }
        }
        log::info!(target: LOG_TARGET, "🚚 iterated {count} entries, migrated {migrated}");
        Ok(cursor)
    }
}

/// Double the remaining block for next era start
pub struct AdjustEraMigration<T>(PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for AdjustEraMigration<T> {
    fn on_runtime_upgrade() -> Weight {
        log::info!("🚚 migrated to async backing, adjust next era start");
        ActiveProtocolState::<T>::mutate_exists(|maybe| {
            if let Some(state) = maybe {
                let current_block_number =
                    frame_system::Pallet::<T>::block_number().saturated_into::<u32>();
                let remaining = state.next_era_start.saturating_sub(current_block_number);
                state.next_era_start.saturating_accrue(remaining);
            }
        });
        T::DbWeight::get().reads_writes(1, 1)
    }
}

pub const EXPECTED_PALLET_DAPP_STAKING_VERSION: u16 = 9;

pub struct DappStakingCleanupMigration<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for DappStakingCleanupMigration<T> {
    fn on_runtime_upgrade() -> Weight {
        let dapp_staking_storage_version =
            <Pallet<T> as GetStorageVersion>::on_chain_storage_version();
        if dapp_staking_storage_version != EXPECTED_PALLET_DAPP_STAKING_VERSION {
            log::info!("Aborting migration due to unexpected on-chain storage versions for pallet-dapp-staking: {:?}. Expectation was: {:?}.", dapp_staking_storage_version, EXPECTED_PALLET_DAPP_STAKING_VERSION);
            return T::DbWeight::get().reads(1);
        }

        let pallet_prefix: &[u8] = b"DappStaking";
        let result =
            clear_storage_prefix(pallet_prefix, b"ActiveBonusUpdateState", &[], None, None);
        log::info!(
            "cleanup dAppStaking migration result: {:?}",
            result.deconstruct()
        );

        T::DbWeight::get().reads_writes(1, 1)
    }
}
