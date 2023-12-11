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

/// Purpose of this pallet is to provide multi-stage migration for moving
/// from the old _dapps_staking_v2_ over to the new _dapp_staking_v3_.
pub use pallet::*;

use frame_support::{
    log,
    pallet_prelude::*,
    traits::{fungible::MutateFreeze, Get, LockableCurrency},
};

use frame_system::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode};
use sp_io::{hashing::twox_128, storage::clear_prefix, KillStorageResult};
use sp_runtime::traits::TrailingZeroInput;

use pallet_dapp_staking_v3::{
    AccountLedger as NewAccountLedger, CurrentEraInfo as NewCurrentEraInfo, EraInfo as NewEraInfo,
    Ledger as NewLedger, PalletDisabled as OldPalletDisabled,
};
use pallet_dapps_staking::{
    CurrentEra as OldCurrentEra, GeneralEraInfo as OldGeneralEraInfo, Ledger as OldLedger,
    RegisteredDapps as OldRegisteredDapps,
};

pub use crate::pallet::CustomMigration;

const LOG_TARGET: &str = "dapp-staking-migration";

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_dapp_staking_v3::Config + pallet_dapps_staking::Config
    {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::storage]
    #[pallet::getter(fn migration_state)]
    pub type MigrationStateStorage<T: Config> = StorageValue<_, MigrationState, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Number of entries migrated from v2 over to v3
        EntriesMigrated(u32),
        /// Number of entries deleted from v2
        EntriesDeleted(u32),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
            // TODO
            Weight::zero()
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight({
            let max_allowed_call_weight = Pallet::<T>::max_call_weight();
            weight_limit
                .unwrap_or(max_allowed_call_weight)
                .min(max_allowed_call_weight)
        })]
        pub fn migrate(
            origin: OriginFor<T>,
            weight_limit: Option<Weight>,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            let consumed_weight = Self::do_migrate(weight_limit);

            Ok(Some(consumed_weight).into())
        }
    }

    impl<T: Config> Pallet<T> {
        fn do_migrate(requested_weight_limit: Option<Weight>) -> Weight {
            let version = <pallet_dapp_staking_v3::Pallet<T>>::on_chain_storage_version();
            let mut consumed_weight = T::DbWeight::get().reads(1);

            // TODO: perhaps this can be improved a bit
            if version != 0 {
                log::trace!(
                    target: LOG_TARGET,
                    "Version is {:?} so skipping migration procedures.",
                    version,
                );
                Self::deposit_event(Event::<T>::EntriesMigrated(0));
                return consumed_weight;
            }

            let max_allowed_call_weight = Self::max_call_weight();
            let weight_limit = requested_weight_limit
                .unwrap_or(max_allowed_call_weight)
                .min(max_allowed_call_weight);
            log::trace!(
                target: LOG_TARGET,
                "Migration weight limit will be {:?}.",
                weight_limit,
            );

            let mut migration_state = MigrationStateStorage::<T>::get();

            while consumed_weight.all_lt(weight_limit) {
                match migration_state {
                    MigrationState::NotInProgress | MigrationState::RegisteredDApps => {
                        migration_state = MigrationState::RegisteredDApps;

                        match Self::migrate_dapps() {
                            Ok(weight) => {
                                consumed_weight.saturating_accrue(weight);
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
                        }
                        Err(weight) => {
                            consumed_weight.saturating_accrue(weight);
                            migration_state = MigrationState::Cleanup;
                        }
                    },
                    MigrationState::Cleanup => match Self::cleanup_old_storage() {
                        Ok(weight) => {
                            consumed_weight.saturating_accrue(weight);
                        }
                        Err(weight) => {
                            consumed_weight.saturating_accrue(weight);
                            migration_state = MigrationState::Final;
                        }
                    },
                    MigrationState::Final => {
                        let weight = Self::final_migration_step();
                        consumed_weight.saturating_accrue(weight);
                    }
                }
            }

            consumed_weight
        }

        /// Used to migrate `RegisteredDapps` from the _old_ dApps staking v2 pallet over to the new `IntegratedDApps`.
        ///
        /// Steps:
        /// 1. Attempt to `drain` a single DB entry from the old storage. If it's unregistered, move on.
        /// 2. Re-decode old smart contract type into new one. Operation should be infalible in practice since the same underlying type is used.
        /// 3. `register` the old-new smart contract into dApp staking v3 pallet.
        ///
        /// Returns `Ok(_)` if an entry was migrated, `Err(_)` if there are no more entries to migrate.
        pub(crate) fn migrate_dapps() -> Result<Weight, Weight> {
            match OldRegisteredDapps::<T>::drain().next() {
                Some((smart_contract, old_dapp_info)) => {
                    // In case dApp was unregistered, nothing more to do here
                    if old_dapp_info.is_unregistered() {
                        // TODO - benchmark this
                        return Ok(T::DbWeight::get().reads(1));
                    }

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
                            return Ok(T::DbWeight::get().reads(1));
                        }
                    };

                    // TODO: alternative is that we don't use the extrinsic logic, but do the insert manually.
                    // Howeer, this approach has the benefit of the obvious code reuse + emitting of the event.
                    // Each dApp is getting a new unique dApp id and this will keep event data consistent - every dApp will have an event with its associated Id.
                    //
                    // TODO2: maybe also use the same approach for `lock` of the new `Ledger`?
                    match pallet_dapp_staking_v3::Pallet::<T>::register(
                        frame_system::RawOrigin::Root.into(),
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

                    // TODO - benchmark this
                    Ok(T::DbWeight::get().reads(1))
                }
                None => {
                    // TODO - benchmark this
                    // Nothing more to migrate here
                    Err(T::DbWeight::get().reads(1))
                }
            }
        }

        /// Used to migrate `Ledger` from the _old_ dApps staking v2 pallet over to the new `Ledger`.
        ///
        /// Steps:
        /// 1. Attempt to `drain` a single DB entry from the old storage.
        /// 2. Re-decode old ledger into the new one. Operation should be infalible in practice since the same underlying type is used.
        /// 3. `register` the old-new smart contract into dApp staking v3 pallet.
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

                    // TODO: emit event for claiming unbonded amount
                    // TODO2: check with team to understant what kind of additional events we want to emit in order for
                    // indexer logic to keep on working?

                    // TODO3: also need to copy over `staked` into `CurrentEraInfo` storage!

                    match <T as pallet_dapp_staking_v3::Config>::Currency::set_freeze(
                        &pallet_dapp_staking_v3::FreezeReason::DAppStaking.into(),
                        &staker,
                        locked,
                    ) {
                        Ok(_) => {}
                        Err(err) => {
                            // Shortly - this can never happen. If it does, it should be detected during test.
                            // However, fallback is to just log it and continue - stakers locks have been released,
                            // so worst case scenario, we will have some stakers with full unlock.
                            log::error!(
                                target: LOG_TARGET,
                                "Failed to set freeze for {:?} with error: {:?}.",
                                staker,
                                err,
                            );

                            #[cfg(feature = "try-runtime")]
                            panic!(
                                "Failed to set freeze for {:?} with error: {:?}.",
                                staker, err
                            );
                            #[cfg(not(feature = "try-runtime"))]
                            return Ok(T::DbWeight::get().reads(1));
                        }
                    }

                    NewLedger::<T>::insert(
                        &staker,
                        NewAccountLedger {
                            locked,
                            ..Default::default()
                        },
                    );

                    // TODO - benchmark this
                    Ok(T::DbWeight::get().reads(1))
                }
                None => {
                    // TODO - benchmark this
                    // Nothing more to migrate here
                    Err(T::DbWeight::get().reads(1))
                }
            }
        }

        /// Used to remove one entry from the old _dapps_staking_v2_ storage.
        ///
        /// If there are no more entries to remove, returns `Err(_)` with consumed weight. Otherwise returns Ok with consumed weight.
        pub(crate) fn cleanup_old_storage() -> Result<Weight, Weight> {
            let hashed_prefix = twox_128(pallet_dapps_staking::Pallet::<T>::name().as_bytes());
            let keys_removed = match clear_prefix(&hashed_prefix, Some(1)) {
                KillStorageResult::AllRemoved(value) => value,
                KillStorageResult::SomeRemaining(value) => value,
            } as u64;

            if keys_removed > 0 {
                Ok(T::DbWeight::get().writes(1))
            } else {
                Err(T::DbWeight::get().reads(1))
            }
        }

        /// Execute final migration step - copy over total locked amount.
        pub(crate) fn final_migration_step() -> Weight {
            let ongoing_era = OldCurrentEra::<T>::get();
            let general_era_info = OldGeneralEraInfo::<T>::get(&ongoing_era).unwrap_or_else(|| {
                log::error!(
                    target: LOG_TARGET,
                    "Failed to get general era info for era (old dApps staking): {:?}.",
                    ongoing_era,
                );

                // This should never happen, but if it does, we want to know about it.
                #[cfg(feature = "try-runtime")]
                panic!("Failed to get general era info for era: {:?}.", ongoing_era);
                #[cfg(not(feature = "try-runtime"))]
                Default::default()
            });

            // In the _old_ dapps staking, `staked` kept track of how much
            // was actively locked & staked in the ongoing era.
            NewCurrentEraInfo::<T>::put(NewEraInfo {
                total_locked: general_era_info.staked,
                ..Default::default()
            });

            // TODO - benchmark this
            T::DbWeight::get().reads(1)
        }

        /// Max allowed weight that migration should be allowed to consume
        fn max_call_weight() -> Weight {
            // 50% of block should be fine
            T::BlockWeights::get().max_block / 2
        }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
    pub enum MigrationState {
        /// No migration in progress
        NotInProgress,
        /// In the middle of `RegisteredDApps` migration.
        RegisteredDApps,
        /// In the middle of `Ledgers` migration.
        Ledgers,
        /// In the middle of old v2 storage cleanup
        Cleanup,
        /// Final migration step, single migration steps.
        Final,
    }

    impl Default for MigrationState {
        fn default() -> Self {
            MigrationState::NotInProgress
        }
    }

    pub struct CustomMigration<T: Config>(PhantomData<T>);
    impl<T: Config> frame_support::traits::OnRuntimeUpgrade for CustomMigration<T> {
        fn on_runtime_upgrade() -> Weight {
            // Ensures that first step only starts the migration with minimal changes in case of production build.
            // In case of `try-runtime`, we want predefined limit.
            let limit = if cfg!(feature = "try-runtime") {
                None
            } else {
                Some(Weight::zero())
            };
            Pallet::<T>::do_migrate(limit)
        }
    }
}
