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

    /// Migration V10 to V11 wrapped in a [`frame_support::migrations::VersionedMigration`], ensuring
    /// the migration is only performed when on-chain version is 10.
    pub type V10ToV11<T, TierParamsConfig> = frame_support::migrations::VersionedMigration<
        10,
        11,
        v11::VersionMigrateV10ToV11<T, TierParamsConfig>,
        Pallet<T>,
        <T as frame_system::Config>::DbWeight,
    >;
}

/// Configuration for V11 tier parameters
pub trait TierParamsV11Config {
    fn reward_portion() -> [Permill; 4];
    fn slot_distribution() -> [Permill; 4];
    fn tier_thresholds() -> [TierThreshold; 4];
    fn slot_number_args() -> (u64, u64);
    fn rank_points() -> [Vec<u8>; 4];
    fn base_reward_portion() -> Permill;
}

mod v11 {
    use super::*;
    use crate::migration::v10::DAppTierRewards as DAppTierRewardsV10;

    pub struct VersionMigrateV10ToV11<T, P>(PhantomData<(T, P)>);

    impl<T: Config, P: TierParamsV11Config> UncheckedOnRuntimeUpgrade for VersionMigrateV10ToV11<T, P> {
        fn on_runtime_upgrade() -> Weight {
            let mut reads = 1u64; // HistoryCleanupMarker
            let mut writes = 0u64;

            let cleanup_marker = HistoryCleanupMarker::<T>::get();
            let oldest_valid_era = cleanup_marker.oldest_valid_era;

            log::info!(
                target: LOG_TARGET,
                "Migration v11: oldest_valid_era = {}, will skip expired entries",
                oldest_valid_era
            );

            // 1. Migrate StaticTierParams
            let reward_portion = P::reward_portion();
            let slot_distribution = P::slot_distribution();
            let tier_thresholds = P::tier_thresholds();
            let rank_points_config = P::rank_points();

            let new_params = TierParameters::<T::NumberOfTiers> {
                reward_portion: BoundedVec::try_from(reward_portion.to_vec())
                    .expect("4 tiers configured"),
                slot_distribution: BoundedVec::try_from(slot_distribution.to_vec())
                    .expect("4 tiers configured"),
                tier_thresholds: BoundedVec::try_from(tier_thresholds.to_vec())
                    .expect("4 tiers configured"),
                slot_number_args: P::slot_number_args(),
                rank_points: BoundedVec::try_from(
                    rank_points_config
                        .into_iter()
                        .map(|points| BoundedVec::try_from(points).expect("rank points"))
                        .collect::<Vec<_>>(),
                )
                .expect("4 tiers"),
                base_reward_portion: P::base_reward_portion(),
            };

            if !new_params.is_valid() {
                log::error!(
                    target: LOG_TARGET,
                    "New TierParameters validation failed. Enabling maintenance mode."
                );
                ActiveProtocolState::<T>::mutate(|state| {
                    state.maintenance = true;
                });
                return T::DbWeight::get().reads_writes(reads, 1);
            }

            StaticTierParams::<T>::put(new_params);
            writes += 1;
            log::info!(target: LOG_TARGET, "StaticTierParams updated successfully");

            // 2. Migrate DAppTiers entries - only valid ones
            let mut migrated_count = 0u32;
            let mut deleted_count = 0u32;
            let mut migration_failed = false;

            let all_eras: Vec<EraNumber> = v10::DAppTiers::<T>::iter_keys().collect();
            reads += all_eras.len() as u64;

            for era in all_eras {
                // Delete expired entries
                if era < oldest_valid_era {
                    v10::DAppTiers::<T>::remove(era);
                    deleted_count += 1;
                    writes += 1;
                    continue;
                }

                reads += 1;
                let maybe_old: Option<
                    DAppTierRewardsV10<T::MaxNumberOfContracts, T::NumberOfTiers>,
                > = v10::DAppTiers::<T>::get(era);

                match maybe_old {
                    Some(old) => {
                        let new = DAppTierRewards {
                            dapps: old.dapps,
                            rewards: old.rewards,
                            period: old.period,
                            rank_rewards: old.rank_rewards,
                            rank_points: BoundedVec::default(), // Empty = legacy formula
                        };
                        DAppTiers::<T>::insert(era, new);
                        migrated_count += 1;
                        writes += 1;
                    }
                    None => {
                        log::error!(
                            target: LOG_TARGET,
                            "Failed to decode DAppTiers for valid era {}",
                            era
                        );
                        migration_failed = true;
                        break;
                    }
                }
            }

            if migration_failed {
                log::error!(
                    target: LOG_TARGET,
                    "DAppTiers migration failed. Enabling maintenance mode."
                );
                ActiveProtocolState::<T>::mutate(|state| {
                    state.maintenance = true;
                });
                return T::DbWeight::get().reads_writes(reads, writes + 1);
            }

            // 3. Update cleanup marker
            if deleted_count > 0 {
                HistoryCleanupMarker::<T>::mutate(|marker| {
                    marker.dapp_tiers_index = marker.dapp_tiers_index.max(oldest_valid_era);
                });
                writes += 1;
            }

            log::info!(
                target: LOG_TARGET,
                "Migration v11 complete: migrated={}, deleted_expired={}",
                migrated_count,
                deleted_count
            );

            T::DbWeight::get().reads_writes(reads, writes)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
            let cleanup_marker = HistoryCleanupMarker::<T>::get();
            let oldest_valid_era = cleanup_marker.oldest_valid_era;

            let valid_count = v10::DAppTiers::<T>::iter_keys()
                .filter(|era| *era >= oldest_valid_era)
                .count() as u32;

            log::info!(
                target: LOG_TARGET,
                "Pre-upgrade: {} valid DAppTiers entries (era >= {})",
                valid_count,
                oldest_valid_era
            );

            // Verify all valid entries can be decoded
            for era in v10::DAppTiers::<T>::iter_keys() {
                if era >= oldest_valid_era {
                    ensure!(
                        v10::DAppTiers::<T>::get(era).is_some(),
                        "Failed to decode DAppTiers for era {}",
                    );
                }
            }

            Ok((valid_count, oldest_valid_era).encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(data: Vec<u8>) -> Result<(), TryRuntimeError> {
            let (expected_count, oldest_valid_era): (u32, EraNumber) =
                Decode::decode(&mut &data[..])
                    .map_err(|_| TryRuntimeError::Other("Failed to decode pre-upgrade data"))?;

            ensure!(
                Pallet::<T>::on_chain_storage_version() == StorageVersion::new(11),
                "Version should be 11"
            );

            let new_params = StaticTierParams::<T>::get();
            ensure!(new_params.is_valid(), "New tier params invalid");

            let new_count = DAppTiers::<T>::iter().count() as u32;
            ensure!(new_count == expected_count, "DAppTiers count mismatch");

            for (era, rewards) in DAppTiers::<T>::iter() {
                ensure!(era >= oldest_valid_era, "Found expired entry");
                ensure!(
                    rewards.rank_points.is_empty(),
                    "Should have empty rank_points"
                );
            }

            ensure!(
                !ActiveProtocolState::<T>::get().maintenance,
                "Maintenance mode should not be enabled"
            );

            Ok(())
        }
    }
}

mod v10 {
    use super::*;
    use frame_support::storage_alias;

    /// v10 DAppTierRewards (without rank_points)
    #[derive(Encode, Decode, Clone)]
    pub struct DAppTierRewards<MD: Get<u32>, NT: Get<u32>> {
        pub dapps: BoundedBTreeMap<DAppId, RankedTier, MD>,
        pub rewards: BoundedVec<Balance, NT>,
        #[codec(compact)]
        pub period: PeriodNumber,
        pub rank_rewards: BoundedVec<Balance, NT>,
    }

    /// v10 storage alias for DAppTiers
    #[storage_alias]
    pub type DAppTiers<T: Config> = StorageMap<
        Pallet<T>,
        Twox64Concat,
        EraNumber,
        DAppTierRewards<<T as Config>::MaxNumberOfContracts, <T as Config>::NumberOfTiers>,
        OptionQuery,
    >;
}
