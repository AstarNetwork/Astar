//! Dapps staking migration utility module

use super::*;

pub mod v2 {

    use super::*;
    use frame_support::{traits::Get, weights::Weight};
    use sp_std::collections::btree_map::BTreeMap;

    #[cfg(feature = "try-runtime")]
    use frame_support::log;
    #[cfg(feature = "try-runtime")]
    use frame_support::traits::OnRuntimeUpgradeHelpersExt;
    #[cfg(feature = "try-runtime")]
    use sp_runtime::traits::Zero;

    // The old value used to store locked amount
    type OldLedger<T> = pallet::pallet::BalanceOf<T>;

    // The old struct used to sotre staking points. Contains unused `formed_staked_era` value.
    #[derive(Clone, PartialEq, Encode, Decode)]
    struct OldEraStakingPoints<AccountId: Ord, Balance: HasCompact> {
        total: Balance,
        stakers: BTreeMap<AccountId, Balance>,
        former_staked_era: EraIndex,
        claimed_rewards: Balance,
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode)]
    struct OldEraRewardAndStake<Balance> {
        rewards: Balance,
        staked: Balance,
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

    pub fn migrate<T: Config>() -> Weight {
        assert_eq!(Version::V1_0_0, StorageVersion::<T>::get());

        let ledger_size = Ledger::<T>::iter_keys().count() as u64;
        let staking_point_size = ContractEraStake::<T>::iter_keys().count() as u64;

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
            ledger_size + staking_point_size,
            ledger_size + staking_point_size + 1,
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
