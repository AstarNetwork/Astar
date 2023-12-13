// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
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

//! Purpose of this pallet is to provide multi-stage migration for moving
//! from the old _dapps_staking_v2_ over to the new _dapp_staking_v3_.
//!
//! Since a lof of data has to be cleaned up & migrated, it is necessary to do this in multiple steps.
//! To reduce the risk of something going wrong, nothing is done in _mandatory hooks_, like `on_initialize` or `on_idle`.
//! Instead, a dedicated extrinsic call is introdudec, which can be called to move the migration forward.
//! As long as this call moves the migration forward, its cost is refunded to the user.
//! Once migration finishes, the extrinsic call will no longer do anything but won't refund the call cost either.
//!
//! The pallet doesn't clean after itself, so when it's removed from the runtime,
//! the old storage should be cleaned up using `RemovePallet` type.

pub use pallet::*;

use frame_support::{
    dispatch::PostDispatchInfo,
    log,
    pallet_prelude::*,
    traits::{Get, LockableCurrency, ReservableCurrency},
};

use frame_system::{pallet_prelude::*, RawOrigin};
use parity_scale_codec::{Decode, Encode};
use sp_io::{hashing::twox_128, storage::clear_prefix, KillStorageResult};
use sp_runtime::{
    traits::{TrailingZeroInput, UniqueSaturatedInto},
    Saturating,
};

use pallet_dapps_staking::{
    CurrentEra as OldCurrentEra, GeneralEraInfo as OldGeneralEraInfo, Ledger as OldLedger,
    RegisteredDapps as OldRegisteredDapps,
};

pub use crate::pallet::CustomMigration;

#[cfg(test)]
mod mock;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod weights;
use weights::{SubstrateWeight, WeightInfo};

const LOG_TARGET: &str = "dapp-staking-migration";

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config:
        // Tight coupling, but it's fine since pallet is supposed to be just temporary and will be removed after migration.
        frame_system::Config + pallet_dapp_staking_v3::Config + pallet_dapps_staking::Config
    {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    /// Used to store the current migration state.
    #[pallet::storage]
    pub type MigrationStateStorage<T: Config> = StorageValue<_, MigrationState, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Number of entries migrated from v2 over to v3
        EntriesMigrated(u32),
        /// Number of entries deleted from v2
        EntriesDeleted(u32),
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Attempt to execute migration steps, consuming up to specified amount of weight.
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

    impl<T: Config> Pallet<T> {
        /// Execute migrations steps until the specified weight limit has been consumed.
        ///
        /// Depending on the number of entries migrated and/or deleted, appropriate events are emited.
        ///
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

            // Ensure we can call dApp staking v3 extrinsics within this call.
            consumed_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
            pallet_dapp_staking_v3::ActiveProtocolState::<T>::mutate(|state| {
                state.maintenance = false;
            });

            let mut migration_state = init_migration_state;
            let (mut entries_migrated, mut entries_deleted) = (0_u32, 0_u32);

            // Execute migration steps.
            //
            // 1. Migrate registered dApps
            // 2. Migrate ledgers
            // 3. Cleanup
            while weight_limit
                .saturating_sub(consumed_weight)
                .all_gte(Self::migration_weight_margin())
            {
                match migration_state {
                    MigrationState::NotInProgress | MigrationState::RegisteredDApps => {
                        migration_state = MigrationState::RegisteredDApps;

                        match Self::migrate_dapps() {
                            Ok(weight) => {
                                consumed_weight.saturating_accrue(weight);
                                entries_migrated.saturating_inc();
                            }
                            Err(weight) => {
                                consumed_weight.saturating_accrue(weight);
                                migration_state = MigrationState::Ledgers;
                            }
                        }
                    }
                    MigrationState::Ledgers => match Self::migrate_ledger() {
                        Ok(weight) => {
                            consumed_weight.saturating_accrue(weight);
                            entries_migrated.saturating_inc();
                        }
                        Err(weight) => {
                            consumed_weight.saturating_accrue(weight);
                            migration_state = MigrationState::Cleanup;
                        }
                    },
                    MigrationState::Cleanup => {
                        // Ensure we don't attempt to delete too much at once.
                        const SAFETY_MARGIN: u32 = 1000;
                        let remaining_weight = weight_limit.saturating_sub(consumed_weight);
                        let capacity = match remaining_weight.checked_div_per_component(
                            &SubstrateWeight::<T>::cleanup_old_storage_success(),
                        ) {
                            Some(entries_to_delete) => {
                                SAFETY_MARGIN.min(entries_to_delete.unique_saturated_into())
                            }
                            None => {
                                // Not enough weight to delete even a single entry
                                break;
                            }
                        };

                        match Self::cleanup_old_storage(capacity) {
                            Ok(weight) => {
                                consumed_weight.saturating_accrue(weight);
                                entries_deleted.saturating_inc();
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
                Self::deposit_event(Event::<T>::EntriesMigrated(entries_migrated));
            }
            if entries_deleted > 0 {
                Self::deposit_event(Event::<T>::EntriesDeleted(entries_deleted));
            }

            // Put the pallet back into maintenance mode.
            pallet_dapp_staking_v3::ActiveProtocolState::<T>::mutate(|state| {
                state.maintenance = true;
            });

            if migration_state != init_migration_state {
                MigrationStateStorage::<T>::put(migration_state);
                consumed_weight.saturating_accrue(T::DbWeight::get().writes(1));
            }

            Ok(consumed_weight)
        }

        /// Used to migrate `RegisteredDapps` from the _old_ dApps staking v2 pallet over to the new `IntegratedDApps`.
        ///
        /// Steps:
        /// 1. Attempt to `drain` a single DB entry from the old storage. If it's unregistered, move on.
        /// 2. Unregister the old `RegisterDeposit` from the developer account.
        /// 2. Re-decode old smart contract type into new one. Operation should be infalible in practice since the same underlying type is used.
        /// 3. `register` the old-new smart contract into dApp staking v3 pallet.
        ///
        /// Returns `Ok(_)` if an entry was migrated, `Err(_)` if there are no more entries to migrate.
        pub(crate) fn migrate_dapps() -> Result<Weight, Weight> {
            match OldRegisteredDapps::<T>::drain().next() {
                Some((smart_contract, old_dapp_info)) => {
                    // In case dApp was unregistered, nothing more to do here
                    if old_dapp_info.is_unregistered() {
                        // Not precise, but happens rarely
                        return Ok(SubstrateWeight::<T>::migrate_dapps_success());
                    }

                    // Release reserved funds from the old dApps staking
                    <T as pallet_dapps_staking::Config>::Currency::unreserve(
                        &old_dapp_info.developer,
                        <T as pallet_dapps_staking::Config>::RegisterDeposit::get(),
                    );

                    // Trick to get around different associated types which are essentially the same underlying struct.
                    let new_smart_contract = match Decode::decode(&mut TrailingZeroInput::new(
                        smart_contract.encode().as_ref(),
                    )) {
                        Ok(new_smart_contract) => new_smart_contract,
                        Err(_) => {
                            log::error!(
                                target: LOG_TARGET,
                                "Failed to decode smart contract: {:?}.",
                                smart_contract,
                            );

                            // This should never happen, but if it does, we want to know about it.
                            #[cfg(feature = "try-runtime")]
                            panic!("Failed to decode smart contract: {:?}", smart_contract);
                            #[cfg(not(feature = "try-runtime"))]
                            // Not precise, but must never happen in production
                            return Ok(SubstrateWeight::<T>::migrate_dapps_success());
                        }
                    };

                    match pallet_dapp_staking_v3::Pallet::<T>::register(
                        RawOrigin::Root.into(),
                        old_dapp_info.developer.clone(),
                        new_smart_contract,
                    ) {
                        Ok(_) => {}
                        Err(error) => {
                            log::error!(
                                target: LOG_TARGET,
                                "Failed to register smart contract: {:?} with error: {:?}.",
                                smart_contract,
                                error,
                            );

                            // This should never happen, but if it does, we want to know about it.
                            #[cfg(feature = "try-runtime")]
                            panic!(
                                "Failed to register smart contract: {:?} with error: {:?}.",
                                smart_contract, error
                            );
                        }
                    }

                    Ok(SubstrateWeight::<T>::migrate_dapps_success())
                }
                None => {
                    // Nothing more to migrate here
                    Err(SubstrateWeight::<T>::migrate_dapps_noop())
                }
            }
        }

        /// Used to migrate `Ledger` from the _old_ dApps staking v2 pallet over to the new `Ledger`.
        ///
        /// Steps:
        /// 1. Attempt to `drain` a single DB entry from the old storage.
        /// 2. Release the old lock from the staker account, in full.
        /// 3. Lock (or freeze) the old _staked_ amount into the new dApp staking v3 pallet.
        ///
        /// **NOTE:** the amount that was undergoing the unbonding process is not migrated but is immediately fully released.
        ///
        /// Returns `Ok(_)` if an entry was migrated, `Err(_)` if there are no more entries to migrate.
        pub(crate) fn migrate_ledger() -> Result<Weight, Weight> {
            match OldLedger::<T>::drain().next() {
                Some((staker, old_account_ledger)) => {
                    let locked = old_account_ledger.locked;

                    // Old unbonding amount can just be released, to keep things simple.
                    // Alternative is to re-calculat this into unlocking chunks.
                    let _total_unbonding = old_account_ledger.unbonding_info.sum();

                    <T as pallet_dapps_staking::Config>::Currency::remove_lock(
                        pallet_dapps_staking::pallet::STAKING_ID,
                        &staker,
                    );

                    match pallet_dapp_staking_v3::Pallet::<T>::lock(
                        RawOrigin::Signed(staker.clone()).into(),
                        locked,
                    ) {
                        Ok(_) => {}
                        Err(error) => {
                            log::error!(
                                target: LOG_TARGET,
                                "Failed to lock for staker {:?} with error: {:?}.",
                                staker,
                                error,
                            );

                            // This should never happen, but if it does, we want to know about it.
                            #[cfg(feature = "try-runtime")]
                            panic!(
                                "Failed to lock for staker {:?} with error: {:?}.",
                                staker, error,
                            );
                        }
                    }

                    Ok(SubstrateWeight::<T>::migrate_ledger_success())
                }
                None => {
                    // Nothing more to migrate here
                    Err(SubstrateWeight::<T>::migrate_ledger_noop())
                }
            }
        }

        /// Used to remove one entry from the old _dapps_staking_v2_ storage.
        ///
        /// If there are no more entries to remove, returns `Err(_)` with consumed weight. Otherwise returns Ok with consumed weight.
        pub(crate) fn cleanup_old_storage(limit: u32) -> Result<Weight, Weight> {
            let hashed_prefix = twox_128(pallet_dapps_staking::Pallet::<T>::name().as_bytes());
            let keys_removed = match clear_prefix(&hashed_prefix, Some(limit)) {
                KillStorageResult::AllRemoved(value) => value,
                KillStorageResult::SomeRemaining(value) => value,
            };

            if keys_removed > 0 {
                Ok(
                    SubstrateWeight::<T>::cleanup_old_storage_success()
                        .saturating_mul(limit.into()),
                )
            } else {
                Err(SubstrateWeight::<T>::cleanup_old_storage_noop())
            }
        }

        /// Max allowed weight that migration should be allowed to consume.
        fn max_call_weight() -> Weight {
            // 50% of block should be fine
            T::BlockWeights::get().max_block / 2
        }

        /// Min allowed weight that migration should be allowed to consume.
        ///
        /// This serves as a safety marging, to prevent accidental underspending due to
        /// inprecision in implementation or benchmarks.
        fn min_call_weight() -> Weight {
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
            SubstrateWeight::<T>::migrate_dapps_success()
                .max(SubstrateWeight::<T>::migrate_ledger_success())
                .max(SubstrateWeight::<T>::cleanup_old_storage_success())
                // and add the weight of updating migration status
                .saturating_add(T::DbWeight::get().reads_writes(1, 2))
        }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, Copy, TypeInfo, RuntimeDebug, MaxEncodedLen)]
    pub enum MigrationState {
        /// No migration in progress
        NotInProgress,
        /// In the middle of `RegisteredDApps` migration.
        RegisteredDApps,
        /// In the middle of `Ledgers` migration.
        Ledgers,
        /// In the middle of old v2 storage cleanup
        Cleanup,
        /// All migrations have been finished
        Finished,
    }

    impl Default for MigrationState {
        fn default() -> Self {
            MigrationState::NotInProgress
        }
    }

    pub struct CustomMigration<T: Config>(PhantomData<T>);
    impl<T: Config> frame_support::traits::OnRuntimeUpgrade for CustomMigration<T> {
        fn on_runtime_upgrade() -> Weight {
            // When upgrade happens, we need to put dApp staking v3 into maintenance mode immediately.
            // For the old pallet, since the storage cleanup is going to happen, maintenance mode must be ensured
            // by the runtime config itself.
            let mut consumed_weight = T::DbWeight::get().reads_writes(1, 1);
            pallet_dapp_staking_v3::ActiveProtocolState::<T>::mutate(|state| {
                state.maintenance = true;
            });

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

                log::info!(
                    target: LOG_TARGET,
                    "dApp Staking migration finished after {} steps with total weight of {}.",
                    steps,
                    consumed_weight,
                );

                consumed_weight
            } else {
                Weight::zero()
            }
        }
    }
}
