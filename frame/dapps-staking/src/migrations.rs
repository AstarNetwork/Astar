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
    pub struct OldEraStakingPoints<AccountId: Ord, Balance: HasCompact> {
        total: Balance,
        stakers: BTreeMap<AccountId, Balance>,
        former_staked_era: EraIndex,
        claimed_rewards: Balance,
    }

    #[cfg(feature = "try-runtime")]
    pub fn pre_migrate<T: Config, U: OnRuntimeUpgradeHelpersExt>() -> Result<(), &'static str> {
        let ledger_count = Ledger::<T>::iter_keys().count() as u64;
        U::set_temp_storage::<u64>(ledger_count, "ledger_count");

        let staking_info_count = ContractEraStake::<T>::iter_keys().count() as u64;
        U::set_temp_storage(staking_info_count, "staking_info_count");

        log::info!(
            ">>> PreMigrate: ledger count: {:?}, staking info count: {:?}",
            ledger_count,
            staking_info_count
        );

        Ok(().into())
    }

    pub fn migrate<T: Config>() -> Weight {
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

        StorageVersion::<T>::put(Version::V2_0_0);

        T::DbWeight::get().reads_writes(
            ledger_size + staking_point_size,
            ledger_size + staking_point_size + 1,
        )
    }

    #[cfg(feature = "try-runtime")]
    pub fn post_migrate<T: Config, U: OnRuntimeUpgradeHelpersExt>() -> Result<(), &'static str> {
        let init_ledger_count = U::get_temp_storage::<u64>("ledger_count").unwrap();
        let init_staking_info_count = U::get_temp_storage::<u64>("staking_info_count").unwrap();

        let current_ledger_count = Ledger::<T>::iter_keys().count() as u64;
        let current_staking_info_count = U::get_temp_storage::<u64>("staking_info_count").unwrap();

        assert_eq!(init_ledger_count, current_ledger_count);
        assert_eq!(init_staking_info_count, current_staking_info_count);

        for acc_ledger in Ledger::<T>::iter_values() {
            assert!(acc_ledger.locked > Zero::zero());
            assert!(acc_ledger.unbonding_info.is_empty());
        }

        log::info!(
            ">>> PostMigrate: ledger count: {:?}, staking info count: {:?}",
            current_ledger_count,
            current_staking_info_count
        );

        Ok(())
    }
}
