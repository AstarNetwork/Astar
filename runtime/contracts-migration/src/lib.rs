#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
    log,
    pallet_prelude::*,
    storage::{
        generator::{StorageDoubleMap, StorageMap},
        unhashed,
    },
    storage_alias,
    traits::{Currency, Get, Imbalance, OnTimestampSet},
    WeakBoundedVec,
};

use codec::{Decode, Encode, FullCodec};
use frame_system::{limits::BlockWeights, pallet_prelude::*};
use pallet_contracts::Determinism;
use sp_runtime::{
    traits::{CheckedAdd, Zero},
    Perbill, Saturating,
};
use sp_std::{fmt::Debug, vec};

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
        SomeEvent,
    }

    #[pallet::error]
    pub enum Error<T> {
        SomeError,
    }

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

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(1_000_000)]
        pub fn migrate(
            origin: OriginFor<T>,
            weight_limit: Option<Weight>,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            let version = <pallet_contracts::Pallet<T>>::on_chain_storage_version();
            let mut consumed_weight = T::DbWeight::get().reads(1);

            if version != 8 {
                log::trace!(
                    target: LOG_TARGET,
                    "Version is {:?} so skipping migration procedures.",
                    version,
                );
                return Ok(Some(consumed_weight).into());
            }

            let max_allowed_call_weight = Self::max_call_weight();
            let weight_limit = weight_limit
                .unwrap_or(max_allowed_call_weight)
                .min(max_allowed_call_weight);

            let migration_state = MigrationStateStorage::<T>::get().for_iteration();

            if let MigrationState::CodeStorage(last_processed_key) = migration_state.clone() {
                // First, get correct iterator.
                let key_iter = if let Some(previous_key) = last_processed_key {
                    CodeStorage::<T>::iter_keys_from(previous_key.into_inner())
                } else {
                    CodeStorage::<T>::iter_keys()
                };

                for key in key_iter {
                    // TODO: need function from map that will only translate ONE value!
                    let key_as_vec = CodeStorage::<T>::storage_map_final_key(key);
                    let mut proof_size = 0_u64;
                    Self::translate(&key_as_vec, |old: OldPrefabWasmModule<T>| {
                        proof_size = old.using_encoded(|o| o.len() as u64);
                        Some(PrefabWasmModule::<T> {
                            instruction_weights_version: old.instruction_weights_version,
                            initial: old.initial,
                            maximum: old.maximum,
                            code: old.code,
                            determinism: Determinism::Deterministic,
                        })
                    });

                    // Increment total consumed weight.
                    consumed_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
                    // consumed_weight.saturating_accrue(Weight::from_proof_size())

                    // // Check if we've consumed enough weight already.
                    // if consumed_weight >= weight_limit {
                    //     log::info!(
                    //         ">>> Ledger migration stopped after consuming {:?} weight.",
                    //         consumed_weight
                    //     );
                    //     MigrationStateV2::<T>::put(MigrationState::Ledger(Some(key_as_vec)));
                    //     consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

                    //     // we want try-runtime to execute the entire migration
                    //     if cfg!(feature = "try-runtime") {
                    //         return stateful_migrate::<T>(weight_limit);
                    //     } else {
                    //         return consumed_weight;
                    //     }
                    // }
                }

                // log::info!(">>> Ledger migration finished.");
                // // This means we're finished with migration of the Ledger. Hooray!
                // // Next step of the migration should be configured.
                // migration_state = MigrationState::StakingInfo(None);
            }

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Max allowed weight that migration shoudl be allowed to consume
        fn max_call_weight() -> Weight {
            T::BlockWeights::get().max_block / 5 * 3
        }

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
        NotInProgress,
        /// In the middle of `CodeStorage` migration.
        CodeStorage(Option<WeakBoundedVec<u8, ConstU32<1000>>>),
    }

    impl MigrationState {
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
}
