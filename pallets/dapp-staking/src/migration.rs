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

    /// Migration V8 to V9 wrapped in a [`frame_support::migrations::VersionedMigration`], ensuring
    /// the migration is only performed when on-chain version is 8.
    pub type V8ToV9<T, InitArgs> = frame_support::migrations::VersionedMigration<
        8,
        9,
        v9::VersionMigrateV8ToV9<T, InitArgs>,
        Pallet<T>,
        <T as frame_system::Config>::DbWeight,
    >;
}

mod v9 {
    use super::*;
    use frame_support::DefaultNoBound;

    #[derive(
        Encode,
        Decode,
        MaxEncodedLen,
        RuntimeDebugNoBound,
        PartialEqNoBound,
        DefaultNoBound,
        EqNoBound,
        CloneNoBound,
        TypeInfo,
    )]
    #[scale_info(skip_type_params(NT))]
    pub struct TierParametersV8<NT: Get<u32>> {
        /// Reward distribution per tier, in percentage.
        /// First entry refers to the first tier, and so on.
        /// The sum of all values must not exceed 100%.
        /// In case it is less, portion of rewards will never be distributed.
        pub(crate) reward_portion: BoundedVec<Permill, NT>,
        /// Distribution of number of slots per tier, in percentage.
        /// First entry refers to the first tier, and so on.
        /// The sum of all values must not exceed 100%.
        /// In case it is less, slot capacity will never be fully filled.
        pub(crate) slot_distribution: BoundedVec<Permill, NT>,
        /// Requirements for entry into each tier.
        /// First entry refers to the first tier, and so on.
        pub(crate) tier_thresholds: BoundedVec<TierThreshold, NT>,
    }

    // The loyal staker flag is updated to `u8` with the new MaxBonusSafeMovesPerPeriod from config
    // for all already existing StakerInfo.
    pub struct VersionMigrateV8ToV9<T, InitArgs>(PhantomData<(T, InitArgs)>);

    impl<T: Config, InitArgs: Get<(u64, u64)>> UncheckedOnRuntimeUpgrade
        for VersionMigrateV8ToV9<T, InitArgs>
    {
        fn on_runtime_upgrade() -> Weight {
            let result = StaticTierParams::<T>::translate::<TierParametersV8<T::NumberOfTiers>, _>(
                |maybe_old_params| match maybe_old_params {
                    Some(old_params) => Some(TierParameters {
                        slot_distribution: old_params.slot_distribution,
                        reward_portion: old_params.reward_portion,
                        tier_thresholds: old_params.tier_thresholds,
                        slot_number_args: InitArgs::get(),
                    }),
                    _ => None,
                },
            );

            if result.is_err() {
                log::error!("Failed to translate StaticTierParams from previous V8 type to current V9 type.");
                // Enable maintenance mode.
                ActiveProtocolState::<T>::mutate(|state| {
                    state.maintenance = true;
                });
                log::warn!("Maintenance mode enabled.");
                return T::DbWeight::get().reads_writes(2, 1);
            }

            T::DbWeight::get().reads_writes(1, 1)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
            assert!(
                !ActiveProtocolState::<T>::get().maintenance,
                "Maintenance mode must be disabled before the runtime upgrade."
            );
            Ok(Vec::new())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_: Vec<u8>) -> Result<(), TryRuntimeError> {
            assert_eq!(
                Pallet::<T>::on_chain_storage_version(),
                EXPECTED_PALLET_DAPP_STAKING_VERSION,
                "dapp-staking::migration::v9: wrong storage version"
            );

            assert!(
                !ActiveProtocolState::<T>::get().maintenance,
                "Maintenance mode must be disabled after the successful runtime upgrade."
            );

            let new_tier_params = StaticTierParams::<T>::get();
            assert!(
                new_tier_params.is_valid(),
                "New tier params are invalid, re-check the values!"
            );
            assert_eq!(new_tier_params.slot_number_args, InitArgs::get());

            Ok(())
        }
    }
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
