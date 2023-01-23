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

/// Purpose of this pallet is to provide multi-stage migration features for pallet-contracts v9 migration.
/// Once it's finished for both `Shibuya` and `Shiden`, it should be deleted.
pub use pallet::*;

use frame_support::{
    log,
    pallet_prelude::*,
    storage::{generator::StorageMap, unhashed},
    storage_alias,
    traits::Get,
    WeakBoundedVec,
};

use codec::{Decode, Encode, FullCodec};
use frame_system::pallet_prelude::*;
use pallet_contracts::Determinism;
use sp_runtime::Saturating;
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

pub use crate::pallet::CustomMigration;

const LOG_TARGET: &str = "pallet-contracts-migration";

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_contracts::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::storage]
    #[pallet::getter(fn migration_state)]
    pub type MigrationStateStorage<T: Config> = StorageValue<_, MigrationState, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Number of contracts that were migrated in the migration call
        ContractsMigrated(u32),
    }

    // The following structs & types were taken from `pallet-contracts` since they aren't exposed outside of the `pallet-contracts` crate.

    #[storage_alias]
    type CodeStorage<T: pallet_contracts::Config> =
        StorageMap<pallet_contracts::Pallet<T>, Identity, CodeHash<T>, PrefabWasmModule<T>>;

    type CodeHash<T> = <T as frame_system::Config>::Hash;
    type RelaxedCodeVec<T> = WeakBoundedVec<u8, <T as pallet_contracts::Config>::MaxCodeLen>;

    #[derive(Encode, Decode, RuntimeDebug, MaxEncodedLen)]
    pub struct OldPrefabWasmModule<T: pallet_contracts::Config> {
        #[codec(compact)]
        pub instruction_weights_version: u32,
        #[codec(compact)]
        pub initial: u32,
        #[codec(compact)]
        pub maximum: u32,
        pub code: RelaxedCodeVec<T>,
    }

    #[derive(Encode, Decode, RuntimeDebug, MaxEncodedLen)]
    pub struct PrefabWasmModule<T: pallet_contracts::Config> {
        #[codec(compact)]
        pub instruction_weights_version: u32,
        #[codec(compact)]
        pub initial: u32,
        #[codec(compact)]
        pub maximum: u32,
        pub code: RelaxedCodeVec<T>,
        pub determinism: Determinism,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
            // This is done in order to account for the read in call filter
            <pallet_contracts::Pallet<T>>::on_chain_storage_version();
            T::DbWeight::get().reads(1)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
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
            let version = <pallet_contracts::Pallet<T>>::on_chain_storage_version();
            let mut consumed_weight = T::DbWeight::get().reads(1);

            if version != 8 {
                log::trace!(
                    target: LOG_TARGET,
                    "Version is {:?} so skipping migration procedures.",
                    version,
                );
                Self::deposit_event(Event::<T>::ContractsMigrated(0));
                return consumed_weight;
            }

            let max_allowed_call_weight = Self::max_call_weight();
            let weight_limit = requested_weight_limit
                .unwrap_or(max_allowed_call_weight)
                .min(max_allowed_call_weight);
            log::trace!(
                target: LOG_TARGET,
                "CodeStorage migration weight limit will be {:?}.",
                weight_limit,
            );

            let migration_state = MigrationStateStorage::<T>::get().for_iteration();

            if let MigrationState::CodeStorage(last_processed_key) = migration_state {
                // First, get correct iterator.
                let key_iter = if let Some(previous_key) = last_processed_key {
                    CodeStorage::<T>::iter_keys_from(previous_key.into_inner())
                } else {
                    CodeStorage::<T>::iter_keys()
                };

                let mut counter = 0_u32;

                for key in key_iter {
                    let key_as_vec = CodeStorage::<T>::storage_map_final_key(key);
                    let used_weight =
                        Self::translate(&key_as_vec, |old: OldPrefabWasmModule<T>| {
                            Some(PrefabWasmModule::<T> {
                                instruction_weights_version: old.instruction_weights_version,
                                initial: old.initial,
                                maximum: old.maximum,
                                code: old.code,
                                determinism: Determinism::Deterministic,
                            })
                        });

                    // Increment total consumed weight.
                    consumed_weight.saturating_accrue(used_weight);
                    counter += 1;

                    // Check if we've consumed enough weight already.
                    if consumed_weight.any_gt(weight_limit) {
                        log::trace!(
                            target: LOG_TARGET,
                            "CodeStorage migration stopped after consuming {:?} weight and after processing {:?} DB entries.",
                            consumed_weight, counter,
                        );
                        MigrationStateStorage::<T>::put(MigrationState::CodeStorage(Some(
                            WeakBoundedVec::force_from(key_as_vec, None),
                        )));
                        consumed_weight.saturating_accrue(T::DbWeight::get().writes(1));

                        Self::deposit_event(Event::<T>::ContractsMigrated(counter));

                        // we want try-runtime to execute the entire migration
                        if cfg!(feature = "try-runtime") {
                            return Self::do_migrate(Some(weight_limit))
                                .saturating_add(consumed_weight);
                        } else {
                            return consumed_weight;
                        }
                    }
                }

                log::trace!(target: LOG_TARGET, "CodeStorage migration finished.",);
                Self::deposit_event(Event::<T>::ContractsMigrated(counter));

                // Clean up storage value so we can safely remove the pallet later
                MigrationStateStorage::<T>::kill();
                StorageVersion::new(9).put::<pallet_contracts::Pallet<T>>();
                consumed_weight.saturating_accrue(T::DbWeight::get().writes(2));
            }

            consumed_weight
        }

        /// Max allowed weight that migration should be allowed to consume
        fn max_call_weight() -> Weight {
            // 50% of block should be fine
            T::BlockWeights::get().max_block / 2
        }

        /// Used to translate a single value in the DB
        /// Returns conservative weight estimate of the operation
        fn translate<O: FullCodec, V: FullCodec, F: FnMut(O) -> Option<V>>(
            key: &[u8],
            mut f: F,
        ) -> Weight {
            let value = match unhashed::get::<O>(key) {
                Some(value) => value,
                None => {
                    return Weight::from_parts(
                        T::DbWeight::get().reads(1).ref_time(),
                        OldPrefabWasmModule::<T>::max_encoded_len() as u64,
                    );
                }
            };

            let mut proof_size = value.using_encoded(|o| o.len() as u64);

            match f(value) {
                Some(new) => {
                    proof_size.saturating_accrue(new.using_encoded(|n| n.len() as u64));
                    unhashed::put::<V>(key, &new);
                }
                // Cannot happen in this file
                None => unhashed::kill(key),
            }

            Weight::from_parts(T::DbWeight::get().reads_writes(1, 1).ref_time(), proof_size)
        }
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
    pub enum MigrationState {
        /// No migration in progress
        NotInProgress,
        /// In the middle of `CodeStorage` migration. The const for max size is an overestimate but that's fine.
        CodeStorage(Option<WeakBoundedVec<u8, ConstU32<1000>>>),
    }

    impl MigrationState {
        /// Convert `self` into value applicable for iteration
        fn for_iteration(self) -> Self {
            if self == Self::NotInProgress {
                Self::CodeStorage(None)
            } else {
                self
            }
        }
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

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
            for value in CodeStorage::<T>::iter_values() {
                ensure!(
                    value.determinism == Determinism::Deterministic,
                    "All pre-existing codes need to be deterministic."
                );
            }

            ensure!(
                !MigrationStateStorage::<T>::exists(),
                "MigrationStateStorage has to be killed at the end of migration."
            );

            ensure!(
                <pallet_contracts::Pallet<T>>::on_chain_storage_version() == 9,
                "pallet-contracts storage version must be 9 at the end of migration"
            );

            Ok(())
        }
    }
}
