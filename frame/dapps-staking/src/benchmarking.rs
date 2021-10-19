#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as DappsStaking;

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::traits::{Get, OnFinalize, OnInitialize, OnUnbalanced};
use frame_system::{Pallet as System, RawOrigin};
use sp_runtime::traits::{Bounded, One};

const SEED: u32 = 9000;
const BLOCK_REWARD: u32 = 1000u32;

/// Used to prepare Dapps staking for testing.
/// Resets all existing storage ensuring a clean run for the code that follows.
///
/// Also initializes the first block which should start a new era.
fn initialize<T: Config>() {
    // Remove everything from storage.
    Ledger::<T>::remove_all(None);
    RegisteredDevelopers::<T>::remove_all(None);
    RegisteredDapps::<T>::remove_all(None);
    EraRewardsAndStakes::<T>::remove_all(None);
    ContractEraStake::<T>::remove_all(None);
    CurrentEra::<T>::kill();
    BlockRewardAccumulator::<T>::kill();
    PreApprovalIsEnabled::<T>::kill();

    // Initialize the first block.
    DappsStaking::<T>::on_unbalanced(T::Currency::issue(BLOCK_REWARD.into()));
    DappsStaking::<T>::on_initialize(1u32.into());
}

/// Assert that the last event equals the provided one.
fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
    frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

/// Advance to the specified era, block by block.
fn advance_to_era<T: Config>(n: EraIndex) {
    while DappsStaking::<T>::current_era() < n {
        DappsStaking::<T>::on_finalize(System::<T>::block_number());
        System::<T>::set_block_number(System::<T>::block_number() + One::one());
        // This is performed outside of dapps staking but we expect it before on_initialize
        DappsStaking::<T>::on_unbalanced(T::Currency::issue(BLOCK_REWARD.into()));
        DappsStaking::<T>::on_initialize(System::<T>::block_number());
    }
}

/// Used to register a contract by a developer account.
///
/// Registered contract is returned.
fn register_contract<T: Config>() -> Result<(T::AccountId, T::SmartContract), &'static str> {
    let developer: T::AccountId = account("developer", 10000, SEED);
    T::Currency::make_free_balance_be(&developer, BalanceOf::<T>::max_value());
    let contract_id = T::SmartContract::default();
    DappsStaking::<T>::register(
        RawOrigin::Signed(developer.clone()).into(),
        contract_id.clone(),
    )?;

    Ok((developer, contract_id))
}

/// Used to bond_and_stake the given contract with the specified amount of stakers.
/// Method will create new staker accounts using the provided seed.
///
/// Returns all created staker accounts in a vector.
fn prepare_bond_and_stake<T: Config>(
    number_of_stakers: u32,
    contract_id: &T::SmartContract,
    seed: u32,
) -> Result<Vec<T::AccountId>, &'static str> {
    let stake_balance = T::MinimumStakingAmount::get(); // maybe make this an argument?
    let mut stakers = Vec::new();

    for id in 0..number_of_stakers {
        let staker_acc: T::AccountId = account("pre_staker", id, seed);
        stakers.push(staker_acc.clone());
        T::Currency::make_free_balance_be(&staker_acc, BalanceOf::<T>::max_value());

        DappsStaking::<T>::bond_and_stake(
            RawOrigin::Signed(staker_acc).into(),
            contract_id.clone(),
            stake_balance.clone(),
        )?;
    }

    Ok(stakers)
}

benchmarks! {

    register {
        initialize::<T>();
        let developer_id = whitelisted_caller();
        let contract_id = T::SmartContract::default();
        T::Currency::make_free_balance_be(&developer_id, BalanceOf::<T>::max_value());
    }: _(RawOrigin::Signed(developer_id.clone()), contract_id.clone())
    verify {
        assert_last_event::<T>(Event::<T>::NewContract(developer_id, contract_id).into());
    }

    unregister {
        let n in 0 .. T::MaxNumberOfStakersPerContract::get();
        initialize::<T>();
        let (developer_id, contract_id) = register_contract::<T>()?;
        prepare_bond_and_stake::<T>(n, &contract_id, SEED)?;
        for id in 0..n {
            let claimer_id: T::AccountId = account("claimer", id, SEED);
            let balance: BalanceOf<T> = 100000u32.into();
        }

    }: _(RawOrigin::Signed(developer_id.clone()), contract_id.clone())
    verify {
        assert_last_event::<T>(Event::<T>::ContractRemoved(developer_id, contract_id).into());
    }

    enable_developer_pre_approval {
        let pre_approval_enabled = true;
    }: _(RawOrigin::Root, pre_approval_enabled)
    verify {
        assert!(PreApprovalIsEnabled::<T>::get());
    }

    developer_pre_approval {
        let pre_approved_id: T::AccountId = account("pre_approved", 100, SEED);
    }: _(RawOrigin::Root, pre_approved_id.clone())
    verify {
        assert!(PreApprovedDevelopers::<T>::contains_key(&pre_approved_id));
    }

    bond_and_stake {
        initialize::<T>();

        let (_, contract_id) = register_contract::<T>()?;
        prepare_bond_and_stake::<T>(T::MaxNumberOfStakersPerContract::get() - 1, &contract_id, SEED)?;

        let staker = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&staker, BalanceOf::<T>::max_value());
        let amount = BalanceOf::<T>::max_value() / 2u32.into();

    }: _(RawOrigin::Signed(staker.clone()), contract_id.clone(), amount.clone())
    verify {
        assert_last_event::<T>(Event::<T>::BondAndStake(staker, contract_id, amount).into());
    }

    unbond_unstake_and_withdraw {
        initialize::<T>();

        let (_, contract_id) = register_contract::<T>()?;
        prepare_bond_and_stake::<T>(T::MaxNumberOfStakersPerContract::get() - 1, &contract_id, SEED)?;

        let staker = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&staker, BalanceOf::<T>::max_value());
        let amount = BalanceOf::<T>::max_value() / 2u32.into();

        DappsStaking::<T>::bond_and_stake(RawOrigin::Signed(staker.clone()).into(), contract_id.clone(), amount.clone())?;
        advance_to_era::<T>(2);

    }: _(RawOrigin::Signed(staker.clone()), contract_id.clone(), amount.clone())
    verify {
        assert_last_event::<T>(Event::<T>::UnbondUnstakeAndWithdraw(staker, contract_id, amount).into());
    }

    claim {
        let n in 2 .. T::MaxNumberOfStakersPerContract::get();

        initialize::<T>();
        let (developer_id, contract_id) = register_contract::<T>()?;

        let number_of_stakers = n - 1;
        let claim_era = DappsStaking::<T>::current_era();
        prepare_bond_and_stake::<T>(number_of_stakers, &contract_id, SEED)?;

        advance_to_era::<T>(claim_era + 1u32);

        let reward = DappsStaking::<T>::era_reward_and_stake(&claim_era).unwrap().rewards;

        let claimer: T::AccountId = whitelisted_caller();
    }: _(RawOrigin::Signed(claimer.clone()), contract_id.clone(), claim_era)
    verify {
        assert_last_event::<T>(Event::<T>::ContractClaimed(contract_id, claim_era, reward).into());
    }

    force_new_era {
    }: _(RawOrigin::Root)

}

impl_benchmark_test_suite!(
    DappsStaking,
    crate::tests::new_test_ext(),
    crate::tests::Test,
);
