//! Dapps staking migration utility module

use super::*;

pub mod v2 {

    use super::*;
    use codec::{Decode, Encode, FullCodec};
    use frame_support::{
        storage::{
            generator::{StorageDoubleMap, StorageMap},
            unhashed,
        },
        traits::Get,
        weights::Weight,
    };
    use sp_std::collections::btree_map::BTreeMap;
    use sp_std::fmt::Debug;

    // #[cfg(feature = "try-runtime")]
    use frame_support::log;
    #[cfg(feature = "try-runtime")]
    use frame_support::traits::OnRuntimeUpgradeHelpersExt;
    #[cfg(feature = "try-runtime")]
    use sp_runtime::traits::Zero;

    // The old value used to store locked amount
    type OldLedger<T> = pallet::pallet::BalanceOf<T>;

    // The old struct used to sotre staking points. Contains unused `formed_staked_era` value.
    #[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
    struct OldEraStakingPoints<AccountId: Ord, Balance: HasCompact> {
        total: Balance,
        stakers: BTreeMap<AccountId, Balance>,
        former_staked_era: EraIndex,
        claimed_rewards: Balance,
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    struct OldEraRewardAndStake<Balance> {
        rewards: Balance,
        staked: Balance,
    }

    /// Serves as migration state representation.
    /// E.g. we might be migrating `Ledger` but need to stop since we've reached the predefined weight limit.
    /// Therefore we use this enum to store migration state `MigrationState::Ledger(Some(last_processed_key))`.
    #[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
    pub enum MigrationState {
        NotStarted,
        /// In the middle of `Ledger` migration.
        Ledger(Option<Vec<u8>>),
        /// In the middle of `StakingInfo` migration.
        StakingInfo(Option<Vec<u8>>),
        /// In the middle of `RewardsAndStakes` migration.
        RewardsAndStakes(Option<Vec<u8>>),
        Finished,
    }

    impl Default for MigrationState {
        fn default() -> Self {
            MigrationState::NotStarted
        }
    }

    /// TODO: this should be part of `IterableStorageMap` and all other `Iterable` storage traits.
    /// Translates a value from format `O` into format `V`.
    /// If key is invalid, translation is ignored.
    /// If translation function `F` fails (returns None), entry is removed from the underlying map.
    fn translate<O: Decode + Debug, V: FullCodec + Debug, F: FnMut(O) -> Option<V>>(
        key: &[u8],
        mut f: F,
    ) {
        let value = match unhashed::get::<O>(key) {
            Some(value) => value,
            None => {
                return;
            }
        };

        match f(value) {
            Some(new) => {
                unhashed::put::<V>(key, &new);
            }
            None => unhashed::kill(key),
        }
    }

    #[cfg(feature = "try-runtime")]
    pub fn pre_migrate<T: Config, U: OnRuntimeUpgradeHelpersExt>() -> Result<(), &'static str> {
        assert_eq!(Version::V1_0_0, StorageVersion::<T>::get());

        let ledger_count = Ledger::<T>::iter_keys().count() as u64;
        U::set_temp_storage::<u64>(ledger_count, "ledger_count");

        let staking_info_count = ContractEraStake::<T>::iter_keys().count() as u64;
        U::set_temp_storage(staking_info_count, "staking_info_count");

        let rewards_and_stakes_count = EraRewardsAndStakes::<T>::iter_keys().count() as u64;
        U::set_temp_storage(rewards_and_stakes_count, "rewards_and_stakes_count");

        log::info!(
            ">>> PreMigrate: ledger count: {:?}, staking info count: {:?}, rewards&stakes count: {:?}",
            ledger_count,
            staking_info_count,
            rewards_and_stakes_count,
        );

        Ok(().into())
    }

    pub fn stateful_migrate<T: Config>(weight_limit: Weight) -> Weight {
        // Ensure this is a valid migration for this version
        if StorageVersion::<T>::get() != Version::V1_0_0 {
            return T::DbWeight::get().reads(1);
        }

        log::info!("Executing a step of stateful storage migration.");

        let mut migration_state = MigrationStateV2::<T>::get();
        let mut consumed_weight = T::DbWeight::get().reads(2);

        // The first storage we process is `Ledger` so we set the starting state if needed
        if migration_state == MigrationState::NotStarted {
            migration_state = MigrationState::Ledger(None);
            PalletDisabled::<T>::put(true);
            consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

            // If normal run, just exit here to avoid the risk of clogging the upgrade block.
            if !cfg!(feature = "try-runtime") {
                MigrationStateV2::<T>::put(migration_state);
                return consumed_weight;
            }
        }

        // Process ledger
        if let MigrationState::Ledger(last_processed_key) = migration_state.clone() {
            // First, get correct iterator.
            let key_iter = if let Some(previous_key) = last_processed_key {
                Ledger::<T>::iter_keys_from(previous_key)
            } else {
                Ledger::<T>::iter_keys()
            };

            for key in key_iter {
                // TODO: need function from map that will only translate ONE value!
                let key_as_vec = Ledger::<T>::storage_map_final_key(key);
                translate(&key_as_vec, |value: OldLedger<T>| {
                    Some(AccountLedger {
                        locked: value,
                        unbonding_info: Default::default(),
                    })
                });

                // Increment total consumed weight.
                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

                // Check if we've consumed enough weight already.
                if consumed_weight >= weight_limit {
                    log::info!(
                        ">>> Ledger migration stopped after consuming {:?} weight.",
                        consumed_weight
                    );
                    MigrationStateV2::<T>::put(MigrationState::Ledger(Some(key_as_vec)));
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

                    // we want try-runtime to execute the entire migration
                    if cfg!(feature = "try-runtime") {
                        return stateful_migrate::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }

            log::info!(">>> Ledger migration finished.");
            // This means we're finished with migration of the Ledger. Hooray!
            // Next step of the migration should be configured.
            migration_state = MigrationState::StakingInfo(None);
        }

        if let MigrationState::StakingInfo(last_processed_key) = migration_state.clone() {
            let key_iter = if let Some(previous_key) = last_processed_key {
                ContractEraStake::<T>::iter_keys_from(previous_key)
            } else {
                ContractEraStake::<T>::iter_keys()
            };

            for (key1, key2) in key_iter {
                let key_as_vec = ContractEraStake::<T>::storage_double_map_final_key(key1, key2);
                translate(
                    &key_as_vec,
                    |value: OldEraStakingPoints<T::AccountId, BalanceOf<T>>| {
                        Some(EraStakingPoints {
                            total: value.total,
                            stakers: value.stakers,
                            claimed_rewards: value.claimed_rewards,
                        })
                    },
                );

                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

                if consumed_weight >= weight_limit {
                    log::info!(
                        ">>> EraStakingPoints migration stopped after consuming {:?} weight.",
                        consumed_weight
                    );
                    MigrationStateV2::<T>::put(MigrationState::StakingInfo(Some(key_as_vec)));
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

                    if cfg!(feature = "try-runtime") {
                        return stateful_migrate::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }

            log::info!(">>> EraStakingPoints migration finished.");

            migration_state = MigrationState::RewardsAndStakes(None);
        }

        if let MigrationState::RewardsAndStakes(last_processed_key) = migration_state.clone() {
            let key_iter = if let Some(previous_key) = last_processed_key {
                EraRewardsAndStakes::<T>::iter_keys_from(previous_key)
            } else {
                EraRewardsAndStakes::<T>::iter_keys()
            };

            for key in key_iter {
                let key_as_vec = EraRewardsAndStakes::<T>::storage_map_final_key(key);
                translate(&key_as_vec, |value: OldEraRewardAndStake<BalanceOf<T>>| {
                    Some(EraRewardAndStake {
                        rewards: value.rewards,
                        staked: value.staked,
                    })
                });

                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

                if consumed_weight >= weight_limit {
                    log::info!(
                        ">>> EraRewardsAndStakes migration stopped after consuming {:?} weight.",
                        consumed_weight
                    );
                    MigrationStateV2::<T>::put(MigrationState::RewardsAndStakes(Some(key_as_vec)));
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

                    if cfg!(feature = "try-runtime") {
                        return stateful_migrate::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }

            log::info!(">>> EraRewardsAndStakes migration finished.");
        }

        MigrationStateV2::<T>::put(MigrationState::Finished);
        consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));
        log::info!(">>> Migration finalized.");

        StorageVersion::<T>::put(Version::V2_0_0);
        consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

        PalletDisabled::<T>::put(false);
        consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

        consumed_weight
    }

    pub fn migrate<T: Config>() -> Weight {
        if StorageVersion::<T>::get() != Version::V1_0_0 {
            return T::DbWeight::get().reads(1);
        }

        let ledger_size = Ledger::<T>::iter_keys().count() as u64;
        let staking_point_size = ContractEraStake::<T>::iter_keys().count() as u64;
        let rewards_and_stakes_size = EraRewardsAndStakes::<T>::iter_keys().count() as u64;

        Ledger::<T>::translate(|_, value: OldLedger<T>| {
            Some(AccountLedger {
                locked: value,
                unbonding_info: Default::default(),
            })
        });

        ContractEraStake::<T>::translate(
            |_, _, old_staking_info: OldEraStakingPoints<T::AccountId, BalanceOf<T>>| {
                Some(EraStakingPoints {
                    total: old_staking_info.total,
                    stakers: old_staking_info.stakers,
                    claimed_rewards: old_staking_info.claimed_rewards,
                })
            },
        );

        EraRewardsAndStakes::<T>::translate(
            |_, old_rewards_and_stakes: OldEraRewardAndStake<BalanceOf<T>>| {
                Some(EraRewardAndStake {
                    rewards: old_rewards_and_stakes.rewards,
                    staked: old_rewards_and_stakes.staked,
                })
            },
        );

        StorageVersion::<T>::put(Version::V2_0_0);

        T::DbWeight::get().reads_writes(
            ledger_size + staking_point_size + rewards_and_stakes_size,
            ledger_size + staking_point_size + rewards_and_stakes_size + 1,
        )
    }

    #[cfg(feature = "try-runtime")]
    pub fn post_migrate<T: Config, U: OnRuntimeUpgradeHelpersExt>() -> Result<(), &'static str> {
        assert_eq!(Version::V2_0_0, StorageVersion::<T>::get());

        let init_ledger_count = U::get_temp_storage::<u64>("ledger_count").unwrap();
        let init_staking_info_count = U::get_temp_storage::<u64>("staking_info_count").unwrap();
        let init_reward_and_stakes_count =
            U::get_temp_storage::<u64>("rewards_and_stakes_count").unwrap();

        let current_ledger_count = Ledger::<T>::iter_keys().count() as u64;
        let current_staking_info_count = ContractEraStake::<T>::iter_keys().count() as u64;
        let current_rewards_and_stakes_count = EraRewardsAndStakes::<T>::iter_keys().count() as u64;

        assert_eq!(init_ledger_count, current_ledger_count);
        assert_eq!(init_staking_info_count, current_staking_info_count);
        assert_eq!(
            init_reward_and_stakes_count,
            current_rewards_and_stakes_count
        );

        for acc_ledger in Ledger::<T>::iter_values() {
            assert!(acc_ledger.locked > Zero::zero());
            assert!(acc_ledger.unbonding_info.is_empty());
        }

        log::info!(
            ">>> PostMigrate: ledger count: {:?}, staking info count: {:?}, rewards&stakes count: {:?}",
            current_ledger_count,
            current_staking_info_count,
            current_rewards_and_stakes_count,
        );

        Ok(())
    }
}
