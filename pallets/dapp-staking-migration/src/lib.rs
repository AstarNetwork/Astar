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

pub use pallet::*;

use frame_support::{
    dispatch::PostDispatchInfo,
    pallet_prelude::*,
    storage_alias,
    traits::{ConstU32, Get},
    WeakBoundedVec,
};

use frame_system::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::Saturating;

use pallet_dapp_staking_v3::{SingularStakingInfo, StakeAmount, StakerInfo};

#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

pub use crate::pallet::SingularStakingInfoTranslationUpgrade;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

const LOG_TARGET: &str = "dapp-staking-migration";

mod v5 {
    use super::*;

    #[derive(
        Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo, Default,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct SingularStakingInfo {
        /// Staked amount
        pub(crate) staked: StakeAmount,
        /// Indicates whether a staker is a loyal staker or not.
        pub(crate) loyal_staker: bool,
    }

    #[storage_alias]
    pub type StakerInfo<T: Config> = StorageDoubleMap<
        pallet_dapp_staking_v3::Pallet<T>,
        Blake2_128Concat,
        <T as frame_system::Config>::AccountId,
        Blake2_128Concat,
        <T as pallet_dapp_staking_v3::Config>::SmartContract,
        SingularStakingInfo,
        OptionQuery,
    >;
}

const MAX_KEY_SIZE: u32 = 1024;
type StakingInfoKey = WeakBoundedVec<u8, ConstU32<MAX_KEY_SIZE>>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config:
        // Tight coupling, but it's fine since pallet is supposed to be just temporary and will be removed after migration.
        frame_system::Config + pallet_dapp_staking_v3::Config
    {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight info for various calls & operations in the pallet.
        type WeightInfo: WeightInfo;
    }

    /// Used to store the current migration state.
    #[pallet::storage]
    pub type MigrationStateStorage<T: Config> = StorageValue<_, MigrationState, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Number of staking info entries translated
        SingularStakingInfoTranslated(u32),
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Attempt to execute migration steps, consuming up to the specified amount of weight.
        /// If no weight is specified, max allowed weight is used.
        ///
        /// Regardless of the specified weight limit, it will be clamped between the minimum & maximum allowed values.
        /// This means that even if user specifies `Weight::zero()` as the limit,
        /// the call will be charged & executed using the minimum allowed weight.
        #[pallet::call_index(0)]
        #[pallet::weight({
            Pallet::<T>::clamp_call_weight(*weight_limit)
        })]
        pub fn migrate(
            origin: OriginFor<T>,
            weight_limit: Option<Weight>,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            let weight_to_use = Self::clamp_call_weight(weight_limit);
            let consumed_weight = Self::do_migrate(weight_to_use);

            // Refund the user in case migration call was needed.
            match consumed_weight {
                Ok(weight) => Ok(PostDispatchInfo {
                    actual_weight: Some(weight),
                    pays_fee: Pays::No,
                }),
                // No refunds or adjustments!
                Err(_) => Ok(().into()),
            }
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn integrity_test() {
            assert!(Pallet::<T>::max_call_weight().all_gte(Pallet::<T>::min_call_weight()));

            assert!(Pallet::<T>::max_call_weight()
                .all_lte(<T as frame_system::Config>::BlockWeights::get().max_block));

            assert!(Pallet::<T>::migration_weight_margin().all_lte(Pallet::<T>::min_call_weight()));
        }
    }

    impl<T: Config> Pallet<T> {
        /// Execute migrations steps until the specified weight limit has been consumed.
        ///
        /// Depending on the number of entries migrated and/or deleted, appropriate events are emitted.
        ///
        /// In case at least some progress is made, `Ok(_)` is returned.
        /// If no progress is made, `Err(_)` is returned.
        fn do_migrate(weight_limit: Weight) -> Result<Weight, Weight> {
            // Find out if migration is still in progress
            let init_migration_state = MigrationStateStorage::<T>::get();
            let mut consumed_weight = T::DbWeight::get().reads(1);

            if init_migration_state == MigrationState::Finished {
                log::trace!(
                    target: LOG_TARGET,
                    "Migration has been finished, skipping any action."
                );
                return Err(consumed_weight);
            }

            consumed_weight.saturating_accrue(T::DbWeight::get().writes(1));

            let mut migration_state = init_migration_state;
            let mut entries_migrated = 0_u32;

            while weight_limit
                .saturating_sub(consumed_weight)
                .all_gte(Self::migration_weight_margin())
            {
                match migration_state.clone() {
                    MigrationState::NotInProgress => match Self::translate_staking_info(None) {
                        Ok((last_key, weight)) => {
                            consumed_weight.saturating_accrue(weight);
                            entries_migrated.saturating_inc();

                            migration_state = MigrationState::SingularStakingInfo(last_key);
                        }
                        Err(weight) => {
                            consumed_weight.saturating_accrue(weight);
                            migration_state = MigrationState::Finished;
                        }
                    },
                    MigrationState::SingularStakingInfo(last_key) => {
                        match Self::translate_staking_info(Some(last_key)) {
                            Ok((last_key, weight)) => {
                                consumed_weight.saturating_accrue(weight);
                                entries_migrated.saturating_inc();

                                migration_state = MigrationState::SingularStakingInfo(last_key);
                            }
                            Err(weight) => {
                                consumed_weight.saturating_accrue(weight);
                                migration_state = MigrationState::Finished;
                            }
                        }
                    }
                    MigrationState::Finished => {
                        // Nothing more to do here
                        break;
                    }
                }
            }

            // Deposit events if needed
            if entries_migrated > 0 {
                Self::deposit_event(Event::<T>::SingularStakingInfoTranslated(entries_migrated));
            }

            // Update the migration status
            MigrationStateStorage::<T>::put(migration_state.clone());

            // Once migration has been finished, disable the maintenance mode and set correct storage version.
            if migration_state == MigrationState::Finished {
                log::trace!(target: LOG_TARGET, "Migration has been finished.");

                pallet_dapp_staking_v3::ActiveProtocolState::<T>::mutate(|state| {
                    state.maintenance = false;
                });
                StorageVersion::new(6).put::<pallet_dapp_staking_v3::Pallet<T>>();
                consumed_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 2));
            }

            Ok(consumed_weight)
        }

        pub(crate) fn translate_staking_info(
            last_key: Option<StakingInfoKey>,
        ) -> Result<(StakingInfoKey, Weight), Weight> {
            // Create an iterator to be used for reading a single entry
            let mut iter = if let Some(last_key) = last_key {
                v5::StakerInfo::<T>::iter_from(last_key.into_inner())
            } else {
                v5::StakerInfo::<T>::iter()
            };

            // Try to read the next entry
            if let Some((account_id, smart_contract_id, old)) = iter.next() {
                // Entry exists so it needs to be translated into the new format
                let new_staking_info = SingularStakingInfo::new_migration(
                    StakeAmount::default(),
                    old.staked,
                    old.loyal_staker,
                );
                StakerInfo::<T>::insert(&account_id, &smart_contract_id, new_staking_info);

                let hashed_key = StakerInfo::<T>::hashed_key_for(&account_id, &smart_contract_id);

                if cfg!(feature = "try-runtime") {
                    assert!(
                        hashed_key.len() < MAX_KEY_SIZE as usize,
                        "Key size exceeded max limit!"
                    );
                }

                Ok((
                    WeakBoundedVec::force_from(hashed_key, None),
                    <T as Config>::WeightInfo::translate_staking_info_success(),
                ))
            } else {
                Err(<T as Config>::WeightInfo::translate_staking_info_success_noop())
            }
        }

        /// Max allowed weight that migration should be allowed to consume.
        pub(crate) fn max_call_weight() -> Weight {
            // 50% of block should be fine
            T::BlockWeights::get().max_block / 2
        }

        /// Min allowed weight that migration should be allowed to consume.
        ///
        /// This serves as a safety margin, to prevent accidental overspending, due to
        /// imprecision in implementation or benchmarks, when small weight limit is specified.
        pub(crate) fn min_call_weight() -> Weight {
            // 5% of block should be fine
            T::BlockWeights::get().max_block / 10
        }

        /// Calculate call weight to use.
        ///
        /// In case of `None`, use the max allowed call weight.
        /// Otherwise clamp the specified weight between the allowed min & max values.
        fn clamp_call_weight(weight: Option<Weight>) -> Weight {
            weight
                .unwrap_or(Self::max_call_weight())
                .min(Self::max_call_weight())
                .max(Self::min_call_weight())
        }

        /// Returns the least amount of weight which should be remaining for migration in order to attempt another step.
        ///
        /// This is used to ensure we don't go over the limit.
        fn migration_weight_margin() -> Weight {
            // Consider the weight of all steps
            <T as Config>::WeightInfo::translate_staking_info_success()
                // and add the weight of updating migration status
                .saturating_add(T::DbWeight::get().writes(1))
        }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
    pub enum MigrationState {
        /// No migration in progress
        NotInProgress,
        /// In the middle of `SingularStakingInfo` migration/translation.
        SingularStakingInfo(StakingInfoKey),
        /// All migrations have been finished
        Finished,
    }

    impl Default for MigrationState {
        fn default() -> Self {
            MigrationState::NotInProgress
        }
    }

    pub struct SingularStakingInfoTranslationUpgrade<T: Config>(PhantomData<T>);
    impl<T: Config> frame_support::traits::OnRuntimeUpgrade
        for SingularStakingInfoTranslationUpgrade<T>
    {
        fn on_runtime_upgrade() -> Weight {
            let mut consumed_weight = T::DbWeight::get().reads_writes(1, 2);

            // Enable maintenance mode.
            pallet_dapp_staking_v3::ActiveProtocolState::<T>::mutate(|state| {
                state.maintenance = true;
            });

            // In case of try-runtime, we want to execute the whole logic, to ensure it works
            // with on-chain data.
            if cfg!(feature = "try-runtime") {
                let mut steps = 0_u32;
                while MigrationStateStorage::<T>::get() != MigrationState::Finished {
                    match Pallet::<T>::do_migrate(crate::Pallet::<T>::max_call_weight()) {
                        Ok(weight) => {
                            consumed_weight.saturating_accrue(weight);
                            steps.saturating_inc();
                        }
                        Err(_) => {
                            panic!("Must never happen since we check whether state is `Finished` before calling `do_migrate`.");
                        }
                    }
                }

                log::trace!(
                    target: LOG_TARGET,
                    "dApp Staking migration finished after {} steps with total weight of {}.",
                    steps,
                    consumed_weight,
                );

                consumed_weight
            } else {
                consumed_weight
            }
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
            // Get all staker info entries to be used later for verification
            let staker_info: Vec<_> = v5::StakerInfo::<T>::iter()
                .map(|(account_id, smart_contract, staking_info)| {
                    (
                        account_id,
                        smart_contract,
                        staking_info.staked,
                        staking_info.loyal_staker,
                    )
                })
                .collect();

            let helper = Helper::<T> { staker_info };

            Ok(helper.encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
            use sp_runtime::traits::TrailingZeroInput;

            // 0. Verify that migration state is `Finished`
            if MigrationStateStorage::<T>::get() != MigrationState::Finished {
                return Err("Migration state is not `Finished`".into());
            }

            let helper = Helper::<T>::decode(&mut TrailingZeroInput::new(state.as_ref()))
                .map_err(|_| "Cannot decode data from pre_upgrade")?;

            // 1. Verify that staker info is essentially same as before
            for (account_id, smart_contract, staked, loyal_staker) in helper.staker_info {
                let staking_info = StakerInfo::<T>::get(&account_id, &smart_contract)
                    .ok_or("Staking info not found but it must exist!")?;

                let expected_staking_info = SingularStakingInfo::new_migration(
                    StakeAmount::default(),
                    staked,
                    loyal_staker,
                );

                if staking_info != expected_staking_info {
                    log::error!(target: LOG_TARGET,
                        "Staking info mismatch for account {:?} and smart contract {:?}. Expected: {:?}, got: {:?}",
                        account_id, smart_contract, expected_staking_info, staking_info
                    );

                    return Err("Failed to verify staking info".into());
                }
            }

            // 2. Verify pallet is no longer in maintenance mode
            if pallet_dapp_staking_v3::ActiveProtocolState::<T>::get().maintenance {
                return Err("Pallet is still in maintenance mode".into());
            }

            // 3. Verify on-chain storage version is correct
            if StorageVersion::get::<pallet_dapp_staking_v3::Pallet<T>>() != 6 {
                return Err("Storage version is not correct".into());
            }

            log::trace!(target: LOG_TARGET, "Post-upgrade checks successful.");

            Ok(())
        }
    }
}

#[cfg(feature = "try-runtime")]
/// Used to help with `try-runtime` testing.
#[derive(Encode, Decode)]
struct Helper<T: Config> {
    staker_info: Vec<(T::AccountId, T::SmartContract, StakeAmount, bool)>,
}
