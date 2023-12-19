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

//! ## Summary
//!
//! Purpose of this pallet is to provide multi-stage migration for moving
//! from the old _dapps_staking_v2_ over to the new _dapp_staking_v3_.
//!
//! ## Approach
//!
//! ### Multi-Stage Migration
//!
//! Since a lot of data has to be cleaned up & migrated, it is necessary to do this in multiple steps.
//! To reduce the risk of something going wrong, nothing is done in _mandatory hooks_, like `on_initialize` or `on_idle`.
//! Instead, a dedicated extrinsic call is introduced, which can be called to move the migration forward.
//! As long as this call moves the migration forward, its cost is refunded to the user.
//! Once migration finishes, the extrinsic call will no longer do anything but won't refund the call cost either.
//!
//! ### Migration Steps
//!
//! The general approach used when migrating is:
//! 1. Clean up old pallet's storage using custom code
//! 2. Use dedicated dApp staking v3 extrinsic calls for registering dApps & locking funds.
//!
//! The main benefits of this approach are that we don't duplicate logic that is already present in dApp staking v3,
//! and that we ensure proper events are emitted for each action which will make indexers happy. No special handling will
//! be required to migrate dApps or locked/staked funds over from the old pallet to the new one, from the indexers  perspective.
//!
//! ### Final Cleanup
//!
//! The pallet doesn't clean after itself, so when it's removed from the runtime,
//! the old storage should be cleaned up using `RemovePallet` type.
//!

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

use pallet_dapps_staking::{Ledger as OldLedger, RegisteredDapps as OldRegisteredDapps};

#[cfg(feature = "try-runtime")]
use astar_primitives::Balance;
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

pub use crate::pallet::DappStakingMigrationHandler;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

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

        /// Weight info for various calls & operations in the pallet.
        type WeightInfo: WeightInfo;
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
        /// Depending on the number of entries migrated and/or deleted, appropriate events are emited.
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

            // Ensure we can call dApp staking v3 extrinsics within this call.
            consumed_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 2));
            pallet_dapp_staking_v3::ActiveProtocolState::<T>::mutate(|state| {
                state.maintenance = false;
            });

            let mut migration_state = init_migration_state;
            let (mut entries_migrated, mut entries_deleted) = (0_u32, 0_u32);

            // Execute migration steps only if we have enough weight to do so.
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
                            &<T as Config>::WeightInfo::cleanup_old_storage_success(),
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
                            Ok((weight, count)) => {
                                consumed_weight.saturating_accrue(weight);
                                entries_deleted.saturating_accrue(count);
                            }
                            Err((weight, count)) => {
                                consumed_weight.saturating_accrue(weight);
                                entries_deleted.saturating_accrue(count);
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
                // Already charged in pessimistic manner at the beginning of the function.
                MigrationStateStorage::<T>::put(migration_state);
            }

            Ok(consumed_weight)
        }

        /// Used to migrate `RegisteredDapps` from the _old_ dApps staking v2 pallet over to the new `IntegratedDApps`.
        ///
        /// Steps:
        /// 1. Attempt to `drain` a single DB entry from the old storage. If it's unregistered, move on.
        /// 2. Unreserve the old `RegisterDeposit` amount from the developer account.
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
                        return Ok(<T as Config>::WeightInfo::migrate_dapps_success());
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
                            return Ok(<T as Config>::WeightInfo::migrate_dapps_success());
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

                    Ok(<T as Config>::WeightInfo::migrate_dapps_success())
                }
                None => {
                    // Nothing more to migrate here
                    Err(<T as Config>::WeightInfo::migrate_dapps_noop())
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

                    // No point in attempting to lock the old amount into dApp staking v3 if amount is insufficient.
                    if locked >= <T as pallet_dapp_staking_v3::Config>::MinimumLockedAmount::get() {
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
                    }

                    // In case no lock action, it will be imprecise but it's fine since this
                    // isn't expected to happen, and even if it does, it's not a big deal.
                    Ok(<T as Config>::WeightInfo::migrate_ledger_success())
                }
                None => {
                    // Nothing more to migrate here
                    Err(<T as Config>::WeightInfo::migrate_ledger_noop())
                }
            }
        }

        /// Used to remove one entry from the old _dapps_staking_v2_ storage.
        ///
        /// If there are no more entries to remove, returns `Err(_)` with consumed weight and number of deleted entries.
        /// Otherwise returns `Ok(_)` with consumed weight and number of consumed enries.
        pub(crate) fn cleanup_old_storage(limit: u32) -> Result<(Weight, u32), (Weight, u32)> {
            let hashed_prefix = twox_128(pallet_dapps_staking::Pallet::<T>::name().as_bytes());

            // Repeated calls in the same block don't work, so we set the limit to `Unlimited` in case of `try-runtime` testing.
            let inner_limit = if cfg!(feature = "try-runtime") {
                None
            } else {
                Some(limit)
            };

            let (keys_removed, done) = match clear_prefix(&hashed_prefix, inner_limit) {
                KillStorageResult::AllRemoved(value) => (value, true),
                KillStorageResult::SomeRemaining(value) => (value, false),
            };

            log::trace!(
                target: LOG_TARGET,
                "Removed {} keys from storage.",
                keys_removed
            );

            if !done {
                Ok((
                    <T as Config>::WeightInfo::cleanup_old_storage_success()
                        .saturating_mul(keys_removed.into()),
                    keys_removed as u32,
                ))
            } else {
                log::trace!(target: LOG_TARGET, "All keys have been removed.",);
                Err((
                    <T as Config>::WeightInfo::cleanup_old_storage_noop(),
                    keys_removed as u32,
                ))
            }
        }

        /// Max allowed weight that migration should be allowed to consume.
        pub(crate) fn max_call_weight() -> Weight {
            // 50% of block should be fine
            T::BlockWeights::get().max_block / 2
        }

        /// Min allowed weight that migration should be allowed to consume.
        ///
        /// This serves as a safety marging, to prevent accidental overspending, due to
        /// inprecision in implementation or benchmarks, when small weight limit is specified.
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
            <T as Config>::WeightInfo::migrate_dapps_success()
                .max(<T as Config>::WeightInfo::migrate_ledger_success())
                .max(<T as Config>::WeightInfo::cleanup_old_storage_success())
                // and add the weight of updating migration status
                .saturating_add(T::DbWeight::get().writes(1))
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

    pub struct DappStakingMigrationHandler<T: Config>(PhantomData<T>);
    impl<T: Config> frame_support::traits::OnRuntimeUpgrade for DappStakingMigrationHandler<T> {
        fn on_runtime_upgrade() -> Weight {
            // When upgrade happens, we need to put dApp staking v3 into maintenance mode immediately.
            // For the old pallet, since the storage cleanup is going to happen, maintenance mode must be ensured
            // by the runtime config itself.
            let mut consumed_weight = T::DbWeight::get().reads_writes(1, 2);
            pallet_dapp_staking_v3::ActiveProtocolState::<T>::mutate(|state| {
                state.maintenance = true;
            });

            // Set the correct init storage version
            pallet_dapp_staking_v3::STORAGE_VERSION.put::<pallet_dapp_staking_v3::Pallet<T>>();

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
            // Get dev accounts with registered dapps and their total reserved balance
            let developers: Vec<_> = pallet_dapps_staking::RegisteredDapps::<T>::iter()
                .filter_map(|(smart_contract, info)| {
                    if info.state == pallet_dapps_staking::DAppState::Registered {
                        let reserved =
                            <T as pallet_dapps_staking::Config>::Currency::reserved_balance(
                                &info.developer,
                            );
                        Some((info.developer, smart_contract, reserved))
                    } else {
                        None
                    }
                })
                .collect();

            // Get the stakers and their active locked (staked) amount.

            let min_lock_amount: Balance =
                <T as pallet_dapp_staking_v3::Config>::MinimumLockedAmount::get();
            let stakers: Vec<_> = pallet_dapps_staking::Ledger::<T>::iter()
                .filter_map(|(staker, ledger)| {
                    if ledger.locked >= min_lock_amount {
                        Some((staker, ledger.locked))
                    } else {
                        None
                    }
                })
                .collect();

            log::info!(
                target: LOG_TARGET,
                "Out of {} stakers, {} have sufficient amount to lock.",
                pallet_dapps_staking::Ledger::<T>::iter().count(),
                stakers.len(),
            );

            let helper = Helper::<T> {
                developers,
                stakers,
            };

            Ok(helper.encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
            use sp_runtime::traits::Zero;

            let helper = Helper::<T>::decode(&mut TrailingZeroInput::new(state.as_ref()))
                .map_err(|_| "Cannot decode data from pre_upgrade")?;

            // 1. Ensure that all entries have been unregistered/removed and all dev accounts have been refunded.
            //    Also check that dApps have been registered in the new pallet.
            assert!(pallet_dapps_staking::RegisteredDapps::<T>::iter()
                .count()
                .is_zero());
            assert_eq!(
                pallet_dapp_staking_v3::IntegratedDApps::<T>::iter().count(),
                helper.developers.len()
            );

            let register_deposit = <T as pallet_dapps_staking::Config>::RegisterDeposit::get();
            for (dev_account, smart_contract, old_reserved) in helper.developers {
                let new_reserved =
                    <T as pallet_dapps_staking::Config>::Currency::reserved_balance(&dev_account);
                assert_eq!(old_reserved, new_reserved + register_deposit);

                let new_smart_contract: <T as pallet_dapp_staking_v3::Config>::SmartContract =
                    Decode::decode(&mut TrailingZeroInput::new(
                        smart_contract.encode().as_ref(),
                    ))
                    .expect("Must succeed since we're using the same underlying type.");

                let dapp_info =
                    pallet_dapp_staking_v3::IntegratedDApps::<T>::get(&new_smart_contract)
                        .expect("Must exist!");
                assert_eq!(dapp_info.owner, dev_account);
            }

            // 2. Ensure that all ledger entries have been migrated over to the new pallet.
            //    Total locked amount in the new pallet must equal the sum of all old locked amounts.
            assert!(pallet_dapps_staking::Ledger::<T>::iter().count().is_zero());
            assert_eq!(
                pallet_dapp_staking_v3::Ledger::<T>::iter().count(),
                helper.stakers.len()
            );

            for (staker, old_locked) in &helper.stakers {
                let new_locked = pallet_dapp_staking_v3::Ledger::<T>::get(&staker).locked;
                assert_eq!(*old_locked, new_locked);
            }

            let total_locked = helper
                .stakers
                .iter()
                .map(|(_, locked)| locked)
                .sum::<Balance>();
            assert_eq!(
                pallet_dapp_staking_v3::CurrentEraInfo::<T>::get().total_locked,
                total_locked
            );

            log::info!(
                target: LOG_TARGET,
                "Total locked amount in the new pallet: {:?}.",
                total_locked,
            );

            // 3. Check that rest of the storage has been cleaned up.
            assert!(!pallet_dapps_staking::PalletDisabled::<T>::exists());
            assert!(!pallet_dapps_staking::CurrentEra::<T>::exists());
            assert!(!pallet_dapps_staking::BlockRewardAccumulator::<T>::exists());
            assert!(!pallet_dapps_staking::ForceEra::<T>::exists());
            assert!(!pallet_dapps_staking::NextEraStartingBlock::<T>::exists());
            assert!(!pallet_dapps_staking::StorageVersion::<T>::exists());

            assert!(pallet_dapps_staking::RegisteredDevelopers::<T>::iter()
                .count()
                .is_zero());
            assert!(pallet_dapps_staking::GeneralEraInfo::<T>::iter()
                .count()
                .is_zero());
            assert!(pallet_dapps_staking::ContractEraStake::<T>::iter()
                .count()
                .is_zero());
            assert!(pallet_dapps_staking::GeneralStakerInfo::<T>::iter()
                .count()
                .is_zero());

            Ok(())
        }
    }
}

#[cfg(feature = "try-runtime")]
/// Used to help with `try-runtime` testing.
#[derive(Encode, Decode)]
struct Helper<T: Config> {
    /// Vec of devs, with their associated smart contract & total reserved balance
    developers: Vec<(
        T::AccountId,
        <T as pallet_dapps_staking::Config>::SmartContract,
        Balance,
    )>,
    /// Stakers with their total active locked amount (not undergoing the unbonding process)
    stakers: Vec<(T::AccountId, Balance)>,
}
