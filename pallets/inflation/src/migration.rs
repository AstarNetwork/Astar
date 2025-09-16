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
use frame_support::pallet_prelude::Weight;
use frame_support::traits::UncheckedOnRuntimeUpgrade;

#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

/// Exports for versioned migration `type`s for this pallet.
pub mod versioned_migrations {
    use super::*;

    /// Migration V1 to V2 wrapped in a [`frame_support::migrations::VersionedMigration`], ensuring
    /// the migration is only performed when on-chain version is 1.
    pub type V1ToV2<T, DecayRate, DecayFactor> = frame_support::migrations::VersionedMigration<
        1,
        2,
        v2::VersionMigrateV1ToV2<T, DecayRate, DecayFactor>,
        Pallet<T>,
        <T as frame_system::Config>::DbWeight,
    >;
}

mod v2 {
    use super::*;
    use crate::migration::v1::{
        InflationConfiguration as InflationConfigurationV1,
        InflationParameters as InflationParametersV1,
    };

    pub struct VersionMigrateV1ToV2<T, DecayRate, DecayFactor>(
        PhantomData<(T, DecayRate, DecayFactor)>,
    );

    impl<T: Config, DecayRate: Get<Perquintill>, DecayFactor: Get<Perquintill>>
        UncheckedOnRuntimeUpgrade for VersionMigrateV1ToV2<T, DecayRate, DecayFactor>
    {
        fn on_runtime_upgrade() -> Weight {
            let decay_rate = DecayRate::get();
            let decay_factor = DecayFactor::get();

            // Add the _decay_rate_ to the inflation params
            let result =
                InflationParams::<T>::translate::<InflationParametersV1, _>(|maybe_old_params| {
                    match maybe_old_params {
                        Some(old_params) => Some(InflationParameters {
                            max_inflation_rate: old_params.max_inflation_rate,
                            treasury_part: old_params.treasury_part,
                            collators_part: old_params.collators_part,
                            dapps_part: old_params.dapps_part,
                            base_stakers_part: old_params.base_stakers_part,
                            adjustable_stakers_part: old_params.adjustable_stakers_part,
                            bonus_part: old_params.bonus_part,
                            ideal_staking_rate: old_params.ideal_staking_rate,
                            decay_rate,
                        }),
                        _ => None,
                    }
                });

            if result.is_err() {
                log::error!("Failed to translate InflationParams from previous V1 type to current V2 type. Check InflationParametersV1 decoding.");
                return T::DbWeight::get().reads_writes(1, 0);
            }

            // Add the _decay_rate_ and _decay_factor_ to the active config
            let result = ActiveInflationConfig::<T>::translate::<InflationConfigurationV1, _>(
                |maybe_old_config| match maybe_old_config {
                    Some(old_config) => Some(InflationConfiguration {
                        recalculation_era: old_config.recalculation_era,
                        issuance_safety_cap: old_config.issuance_safety_cap,
                        collator_reward_per_block: old_config.collator_reward_per_block,
                        treasury_reward_per_block: old_config.treasury_reward_per_block,
                        dapp_reward_pool_per_era: old_config.dapp_reward_pool_per_era,
                        base_staker_reward_pool_per_era: old_config.base_staker_reward_pool_per_era,
                        adjustable_staker_reward_pool_per_era: old_config
                            .adjustable_staker_reward_pool_per_era,
                        bonus_reward_pool_per_period: old_config.bonus_reward_pool_per_period,
                        ideal_staking_rate: old_config.ideal_staking_rate,
                        decay_rate,
                        decay_factor,
                    }),
                    _ => None,
                },
            );

            if result.is_err() {
                log::error!("Failed to translate InflationConfiguration from previous V1 type to current V2 type. Check InflationConfigurationV1 decoding.");
                return T::DbWeight::get().reads_writes(2, 1);
            }

            T::DbWeight::get().reads_writes(2, 2)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
            let old_config = v1::ActiveInflationConfig::<T>::get().ok_or_else(|| {
                TryRuntimeError::Other(
                    "pallet-inflation::migration::v2: No old config found for ActiveInflationConfig",
                )
            })?;

            let old_params = v1::InflationParams::<T>::get().ok_or_else(|| {
                TryRuntimeError::Other(
                    "pallet-inflation::migration::v2: No old params found for InflationParams",
                )
            })?;
            Ok((old_config, old_params).encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(data: Vec<u8>) -> Result<(), TryRuntimeError> {
            // Decode the old values
            let (old_config, old_params): (InflationConfigurationV1, InflationParametersV1) =
                Decode::decode(&mut &data[..]).map_err(|_| {
                    TryRuntimeError::Other(
                        "pallet-inflation::migration::v2: Failed to decode old values",
                    )
                })?;

            // Get the new values
            let new_config = ActiveInflationConfig::<T>::get();
            let new_params = InflationParams::<T>::get();

            // Verify that new params and new config are valid
            assert!(new_params.is_valid());
            new_config.sanity_check();

            // Verify active config remain unchanged
            assert_eq!(
                old_config.recalculation_era, new_config.recalculation_era,
                "pallet-inflation::migration::v2: Recalculation Era has changed"
            );
            assert_eq!(
                old_config.issuance_safety_cap, new_config.issuance_safety_cap,
                "pallet-inflation::migration::v2: Issuance Safety Cap has changed"
            );
            assert_eq!(
                old_config.collator_reward_per_block, new_config.collator_reward_per_block,
                "pallet-inflation::migration::v2: Collator Reward Per Block has changed"
            );
            assert_eq!(
                old_config.treasury_reward_per_block, new_config.treasury_reward_per_block,
                "pallet-inflation::migration::v2: Treasury Reward Per Block has changed"
            );
            assert_eq!(
                old_config.dapp_reward_pool_per_era, new_config.dapp_reward_pool_per_era,
                "pallet-inflation::migration::v2: Dapp Reward Per Era has changed"
            );
            assert_eq!(
                old_config.base_staker_reward_pool_per_era,
                new_config.base_staker_reward_pool_per_era,
                "pallet-inflation::migration::v2: Staker Reward Pool Per Era has changed"
            );
            assert_eq!(
                old_config.adjustable_staker_reward_pool_per_era, new_config.adjustable_staker_reward_pool_per_era,
                "pallet-inflation::migration::v2: Adjustable Staker Reward Pool Per Era has changed"
            );
            assert_eq!(
                old_config.bonus_reward_pool_per_period, new_config.bonus_reward_pool_per_period,
                "pallet-inflation::migration::v2: Bonus Reward Pool Per Period has changed"
            );
            assert_eq!(
                old_config.ideal_staking_rate, new_config.ideal_staking_rate,
                "pallet-inflation::migration::v2: Ideal staking rate has changed in config"
            );

            // Verify parameters remain unchanged
            assert_eq!(
                old_params.max_inflation_rate, new_params.max_inflation_rate,
                "pallet-inflation::migration::v2: Max inflation rate has changed"
            );
            assert_eq!(
                old_params.treasury_part, new_params.treasury_part,
                "pallet-inflation::migration::v2: Treasury part has changed"
            );
            assert_eq!(
                old_params.collators_part, new_params.collators_part,
                "pallet-inflation::migration::v2: Collator part has changed"
            );
            assert_eq!(
                old_params.dapps_part, new_params.dapps_part,
                "pallet-inflation::migration::v2: Dapps part has changed"
            );
            assert_eq!(
                old_params.base_stakers_part, new_params.base_stakers_part,
                "pallet-inflation::migration::v2: Base staker part has changed"
            );
            assert_eq!(
                old_params.adjustable_stakers_part, new_params.adjustable_stakers_part,
                "pallet-inflation::migration::v2: Adjustable staker part has changed"
            );
            assert_eq!(
                old_params.bonus_part, new_params.bonus_part,
                "pallet-inflation::migration::v2: Bonus staker part has changed"
            );
            assert_eq!(
                old_params.ideal_staking_rate, new_params.ideal_staking_rate,
                "pallet-inflation::migration::v2: Ideal staking rate has changed in params"
            );

            // Verify correct decay rate is initialized
            let expected_decay_rate = DecayRate::get();
            assert_eq!(
                expected_decay_rate, new_params.decay_rate,
                "pallet-inflation::migration::v2: No correct decay rate in params"
            );
            assert_eq!(
                expected_decay_rate, new_config.decay_rate,
                "pallet-inflation::migration::v2: No correct decay rate in config"
            );

            // Verify correct decay factor is initialized
            let expected_decay_factor = DecayFactor::get();
            assert_eq!(
                expected_decay_factor, new_config.decay_factor,
                "pallet-inflation::migration::v2: No correct decay factor in config"
            );

            // Verify storage version has been updated
            ensure!(
                Pallet::<T>::on_chain_storage_version() >= 2,
                "pallet-inflation::migration::v2: Wrong storage version."
            );

            Ok(())
        }
    }
}

mod v1 {
    use super::*;
    use frame_support::storage_alias;

    #[derive(Encode, Decode)]
    pub struct InflationConfiguration {
        #[codec(compact)]
        pub recalculation_era: EraNumber,
        #[codec(compact)]
        pub issuance_safety_cap: Balance,
        #[codec(compact)]
        pub collator_reward_per_block: Balance,
        #[codec(compact)]
        pub treasury_reward_per_block: Balance,
        #[codec(compact)]
        pub dapp_reward_pool_per_era: Balance,
        #[codec(compact)]
        pub base_staker_reward_pool_per_era: Balance,
        #[codec(compact)]
        pub adjustable_staker_reward_pool_per_era: Balance,
        #[codec(compact)]
        pub bonus_reward_pool_per_period: Balance,
        #[codec(compact)]
        pub ideal_staking_rate: Perquintill,
    }

    #[derive(Encode, Decode)]
    pub struct InflationParameters {
        #[codec(compact)]
        pub max_inflation_rate: Perquintill,
        #[codec(compact)]
        pub treasury_part: Perquintill,
        #[codec(compact)]
        pub collators_part: Perquintill,
        #[codec(compact)]
        pub dapps_part: Perquintill,
        #[codec(compact)]
        pub base_stakers_part: Perquintill,
        #[codec(compact)]
        pub adjustable_stakers_part: Perquintill,
        #[codec(compact)]
        pub bonus_part: Perquintill,
        #[codec(compact)]
        pub ideal_staking_rate: Perquintill,
    }

    /// v1 type for [`crate::ActiveInflationConfig`]
    #[storage_alias]
    pub type ActiveInflationConfig<T: Config> = StorageValue<Pallet<T>, InflationConfiguration>;

    /// v1 type for [`crate::InflationParams`]
    #[storage_alias]
    pub type InflationParams<T: Config> = StorageValue<Pallet<T>, InflationParameters>;
}
