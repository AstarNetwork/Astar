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
use frame_support::{
    migrations::{MigrationId, SteppedMigration, SteppedMigrationError},
    storage_alias,
    traits::UncheckedOnRuntimeUpgrade,
    weights::WeightMeter,
};

#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

/// Exports for versioned migration `type`s for this pallet.
pub mod versioned_migrations {
    use super::*;

    /// Migration V6 to V7 wrapped in a [`frame_support::migrations::VersionedMigration`], ensuring
    /// the migration is only performed when on-chain version is 6.
    pub type V6ToV7<T> = frame_support::migrations::VersionedMigration<
        6,
        7,
        v7::VersionMigrateV6ToV7<T>,
        Pallet<T>,
        <T as frame_system::Config>::DbWeight,
    >;

    /// Migration V7 to V8 wrapped in a [`frame_support::migrations::VersionedMigration`], ensuring
    /// the migration is only performed when on-chain version is 7.
    pub type V7ToV8<T, TierThresholds, ThresholdVariationPercentage> =
        frame_support::migrations::VersionedMigration<
            7,
            8,
            v8::VersionMigrateV7ToV8<T, TierThresholds, ThresholdVariationPercentage>,
            Pallet<T>,
            <T as frame_system::Config>::DbWeight,
        >;
}

// TierThreshold as percentage of the total issuance
mod v8 {
    use super::*;
    use crate::migration::v7::TierParameters as TierParametersV7;
    use crate::migration::v7::TiersConfiguration as TiersConfigurationV7;

    pub struct VersionMigrateV7ToV8<T, TierThresholds, ThresholdVariationPercentage>(
        PhantomData<(T, TierThresholds, ThresholdVariationPercentage)>,
    );

    impl<
            T: Config,
            TierThresholds: Get<[TierThreshold; 4]>,
            ThresholdVariationPercentage: Get<u32>,
        > UncheckedOnRuntimeUpgrade
        for VersionMigrateV7ToV8<T, TierThresholds, ThresholdVariationPercentage>
    {
        fn on_runtime_upgrade() -> Weight {
            // 1. Update static tier parameters with new thresholds from the runtime configurable param TierThresholds
            let result = StaticTierParams::<T>::translate::<TierParametersV7<T::NumberOfTiers>, _>(
                |maybe_old_params| match maybe_old_params {
                    Some(old_params) => {
                        let tier_thresholds: Result<
                            BoundedVec<TierThreshold, T::NumberOfTiers>,
                            _,
                        > = BoundedVec::try_from(TierThresholds::get().to_vec());

                        match tier_thresholds {
                            Ok(tier_thresholds) => Some(TierParameters {
                                slot_distribution: old_params.slot_distribution,
                                reward_portion: old_params.reward_portion,
                                tier_thresholds,
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
                log::error!("Failed to translate StaticTierParams from previous V7 type to current V8 type. Check TierParametersV7 decoding.");
                // Enable maintenance mode.
                ActiveProtocolState::<T>::mutate(|state| {
                    state.maintenance = true;
                });
                log::warn!("Maintenance mode enabled.");
                return T::DbWeight::get().reads_writes(1, 0);
            }

            // 2. Translate tier thresholds from V7 TierThresholds to Balance
            let result = TierConfig::<T>::translate::<
                TiersConfigurationV7<T::NumberOfTiers, T::TierSlots, T::BaseNativeCurrencyPrice>,
                _,
            >(|maybe_old_config| match maybe_old_config {
                Some(old_config) => {
                    let new_tier_thresholds: Result<BoundedVec<Balance, T::NumberOfTiers>, _> =
                        old_config
                            .tier_thresholds
                            .iter()
                            .map(|t| match t {
                                v7::TierThreshold::DynamicTvlAmount { amount, .. } => *amount,
                                v7::TierThreshold::FixedTvlAmount { amount } => *amount,
                            })
                            .collect::<Vec<Balance>>()
                            .try_into();

                    match new_tier_thresholds {
                        Ok(new_tier_thresholds) => Some(TiersConfiguration {
                            slots_per_tier: old_config.slots_per_tier,
                            reward_portion: old_config.reward_portion,
                            tier_thresholds: new_tier_thresholds,
                            _phantom: Default::default(),
                        }),
                        Err(err) => {
                            log::error!("Failed to convert tier thresholds to balances: {:?}", err);
                            None
                        }
                    }
                }
                _ => None,
            });

            if result.is_err() {
                log::error!("Failed to translate TierConfig from previous V7 type to current V8 type. Check TiersConfigurationV7 decoding.");
                // Enable maintenance mode.
                ActiveProtocolState::<T>::mutate(|state| {
                    state.maintenance = true;
                });
                log::warn!("Maintenance mode enabled.");
                return T::DbWeight::get().reads_writes(2, 1);
            }

            T::DbWeight::get().reads_writes(2, 2)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
            let tier_thresholds: Result<BoundedVec<TierThreshold, T::NumberOfTiers>, _> =
                BoundedVec::try_from(TierThresholds::get().to_vec());
            assert!(tier_thresholds.is_ok());

            let old_config = v7::TierConfig::<T>::get().ok_or_else(|| {
                TryRuntimeError::Other(
                    "dapp-staking::migration::v8: No old configuration found for TierConfig",
                )
            })?;
            Ok((old_config.number_of_slots, old_config.tier_thresholds).encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(data: Vec<u8>) -> Result<(), TryRuntimeError> {
            let (old_number_of_slots, old_tier_thresholds): (
                u16,
                BoundedVec<v7::TierThreshold, T::NumberOfTiers>,
            ) = Decode::decode(&mut &data[..]).map_err(|_| {
                TryRuntimeError::Other(
                    "dapp-staking::migration::v8: Failed to decode old v7 version of tier config",
                )
            })?;

            // 0. Prerequisites
            let actual_config = TierConfig::<T>::get();
            assert!(actual_config.is_valid());

            ensure!(
                Pallet::<T>::on_chain_storage_version() >= 8,
                "dapp-staking::migration::v8: Wrong storage version."
            );

            // 1. Ensure the number of slots is preserved
            let actual_number_of_slots = actual_config.total_number_of_slots();
            let within_tolerance =
                (old_number_of_slots.saturating_sub(1))..=old_number_of_slots.saturating_add(1);

            assert!(
                within_tolerance.contains(&actual_number_of_slots),
                "dapp-staking::migration::v8: New TiersConfiguration format not set correctly, number of slots has diverged. Old: {}. Actual: {}.",
                old_number_of_slots,
                actual_number_of_slots
            );

            // 2. Ensure the provided static tier params are applied
            let actual_tier_params = StaticTierParams::<T>::get();
            assert!(actual_tier_params.is_valid());

            let expected_tier_thresholds: Result<BoundedVec<TierThreshold, T::NumberOfTiers>, _> =
                BoundedVec::try_from(TierThresholds::get().to_vec());
            ensure!(
                expected_tier_thresholds.is_ok(),
                "dapp-staking::migration::v8: Failed to convert expected tier thresholds."
            );
            let actual_tier_thresholds = actual_tier_params.clone().tier_thresholds;
            assert_eq!(expected_tier_thresholds.unwrap(), actual_tier_thresholds);

            // 3. Double check new threshold amounts allowing
            let variation_percentage = ThresholdVariationPercentage::get();
            let total_issuance = T::Currency::total_issuance();
            let average_price = T::NativePriceProvider::average_price();

            let old_threshold_amounts: Result<BoundedVec<Balance, T::NumberOfTiers>, _> =
                old_tier_thresholds
                    .iter()
                    .map(|t| t.threshold())
                    .collect::<Vec<Balance>>()
                    .try_into();

            ensure!(
                old_threshold_amounts.is_ok(),
                "dapp-staking::migration::v8: Failed to convert old v7 version tier thresholds to balance amounts."
            );
            let old_threshold_amounts = old_threshold_amounts.unwrap();
            let expected_new_threshold_amounts = actual_config
                .calculate_new(&actual_tier_params, average_price, total_issuance)
                .tier_thresholds;

            for (old_amount, actual_amount) in old_threshold_amounts
                .iter()
                .zip(expected_new_threshold_amounts)
            {
                let lower_bound = old_amount
                    .saturating_mul(100u32.saturating_sub(variation_percentage).into())
                    .saturating_div(100u32.into());
                let upper_bound = old_amount
                    .saturating_mul(100u32.saturating_add(variation_percentage).into())
                    .saturating_div(100u32.into());

                assert!(
                    (lower_bound..=upper_bound).contains(&actual_amount),
                    "dapp-staking::migration::v8: New tier threshold amounts diverged to much from old values, consider adjusting static tier parameters. Old: {}. Actual: {}.",
                    old_amount,
                    actual_amount
                );
            }

            Ok(())
        }
    }
}
/// Translate DAppTiers to include rank rewards.
mod v7 {
    use super::*;
    use crate::migration::v6::DAppTierRewards as DAppTierRewardsV6;
    use astar_primitives::dapp_staking::TierSlots as TierSlotsFunc;

    /// Description of tier entry requirement.
    #[derive(Encode, Decode)]
    pub enum TierThreshold {
        FixedTvlAmount {
            amount: Balance,
        },
        DynamicTvlAmount {
            amount: Balance,
            minimum_amount: Balance,
        },
    }

    #[cfg(feature = "try-runtime")]
    impl TierThreshold {
        /// Return threshold for the tier.
        pub fn threshold(&self) -> Balance {
            match self {
                Self::FixedTvlAmount { amount } => *amount,
                Self::DynamicTvlAmount { amount, .. } => *amount,
            }
        }
    }

    /// Top level description of tier slot parameters used to calculate tier configuration.
    #[derive(Encode, Decode)]
    pub struct TierParameters<NT: Get<u32>> {
        /// Reward distribution per tier, in percentage.
        /// First entry refers to the first tier, and so on.
        /// The sum of all values must not exceed 100%.
        /// In case it is less, portion of rewards will never be distributed.
        pub reward_portion: BoundedVec<Permill, NT>,
        /// Distribution of number of slots per tier, in percentage.
        /// First entry refers to the first tier, and so on.
        /// The sum of all values must not exceed 100%.
        /// In case it is less, slot capacity will never be fully filled.
        pub slot_distribution: BoundedVec<Permill, NT>,
        /// Requirements for entry into each tier.
        /// First entry refers to the first tier, and so on.
        pub tier_thresholds: BoundedVec<v7::TierThreshold, NT>,
    }

    /// v7 type for configuration of dApp tiers.
    #[derive(Encode, Decode)]
    pub struct TiersConfiguration<NT: Get<u32>, T: TierSlotsFunc, P: Get<FixedU128>> {
        /// Total number of slots.
        #[codec(compact)]
        pub number_of_slots: u16,
        /// Number of slots per tier.
        /// First entry refers to the first tier, and so on.
        pub slots_per_tier: BoundedVec<u16, NT>,
        /// Reward distribution per tier, in percentage.
        /// First entry refers to the first tier, and so on.
        /// The sum of all values must be exactly equal to 1.
        pub reward_portion: BoundedVec<Permill, NT>,
        /// Requirements for entry into each tier.
        /// First entry refers to the first tier, and so on.
        pub tier_thresholds: BoundedVec<v7::TierThreshold, NT>,
        /// Phantom data to keep track of the tier slots function.
        #[codec(skip)]
        pub(crate) _phantom: PhantomData<(T, P)>,
    }

    /// v7 type for [`crate::StaticTierParams`]
    #[storage_alias]
    pub type StaticTierParams<T: Config> =
        StorageValue<Pallet<T>, TierParameters<<T as Config>::NumberOfTiers>, ValueQuery>;

    /// v7 type for [`crate::TierConfig`]
    #[storage_alias]
    pub type TierConfig<T: Config> = StorageValue<
        Pallet<T>,
        TiersConfiguration<
            <T as Config>::NumberOfTiers,
            <T as Config>::TierSlots,
            <T as Config>::BaseNativeCurrencyPrice,
        >,
        OptionQuery,
    >;

    pub struct VersionMigrateV6ToV7<T>(PhantomData<T>);

    impl<T: Config> UncheckedOnRuntimeUpgrade for VersionMigrateV6ToV7<T> {
        fn on_runtime_upgrade() -> Weight {
            let current = Pallet::<T>::in_code_storage_version();

            let mut translated = 0usize;
            DAppTiers::<T>::translate::<
                DAppTierRewardsV6<T::MaxNumberOfContracts, T::NumberOfTiers>,
                _,
            >(|_key, old_value| {
                translated.saturating_inc();

                // fill rank_rewards with zero
                let mut rank_rewards = Vec::new();
                rank_rewards.resize_with(old_value.rewards.len(), || Balance::zero());

                Some(DAppTierRewards {
                    dapps: old_value.dapps,
                    rewards: old_value.rewards,
                    period: old_value.period,
                    rank_rewards: BoundedVec::<Balance, T::NumberOfTiers>::try_from(rank_rewards)
                        .unwrap_or_default(),
                })
            });

            current.put::<Pallet<T>>();

            log::info!("Upgraded {translated} dAppTiers to {current:?}");

            T::DbWeight::get().reads_writes(1 + translated as u64, 1 + translated as u64)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
            Ok(Vec::new())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_data: Vec<u8>) -> Result<(), TryRuntimeError> {
            ensure!(
                Pallet::<T>::on_chain_storage_version() >= 7,
                "dapp-staking::migration::v7: wrong storage version"
            );
            Ok(())
        }
    }
}

pub mod v6 {
    use astar_primitives::{
        dapp_staking::{DAppId, PeriodNumber, RankedTier},
        Balance,
    };
    use frame_support::{
        pallet_prelude::{Decode, Get},
        BoundedBTreeMap, BoundedVec,
    };

    /// Information about all of the dApps that got into tiers, and tier rewards
    #[derive(Decode)]
    pub struct DAppTierRewards<MD: Get<u32>, NT: Get<u32>> {
        /// DApps and their corresponding tiers (or `None` if they have been claimed in the meantime)
        pub dapps: BoundedBTreeMap<DAppId, RankedTier, MD>,
        /// Rewards for each tier. First entry refers to the first tier, and so on.
        pub rewards: BoundedVec<Balance, NT>,
        /// Period during which this struct was created.
        #[codec(compact)]
        pub period: PeriodNumber,
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
                if ledger.unlocking.is_empty() {
                    // no unlocking for this account, nothing to update
                    cursor = Some(last_key.clone()); // Return the processed key as the new cursor.
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
