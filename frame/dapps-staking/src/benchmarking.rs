#![cfg(feature = "runtime-benchmarks")]

// NOTE: in claim() benchmark, number of payees can significantly exceed the max number of stakers.
// Since 'n' and 'm' aren't independent in this scenario
// E.g. if we only have 1 staking point, we cannot have more than max_number_of_stakers payees.
// TODO: maybe add another benchmarking for claim that covers the higher values scenario?

use super::*;
use crate::Pallet as DappsStaking;

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::traits::{Get, OnFinalize, OnInitialize, OnUnbalanced};
use frame_system::{Pallet as System, RawOrigin};
use sp_runtime::traits::{Bounded, One};

const SEED: u32 = 9000;
const BLOCK_REWARD: u32 = 1000u32;

// TODO: This needs to be defined in the pallet.
// Current implementation allows arbitrary amount of staking points to be read from the storage
// if contract wasn't claimed for a long time. This could cause some very heavy calls.
// In order to avoid this, we should put a hard limit on how much into the history can we go.
// This also means that some rewards will just be slashed, they won't be transferred to the treasury.
const MAX_NUMBER_OF_ERA_STAKING_POINTS: u32 = 60;

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
    RewardsClaimed::<T>::remove_all(None);
    ContractEraStake::<T>::remove_all(None);
    ContractLastClaimed::<T>::remove_all(None);
    ContractLastStaked::<T>::remove_all(None);
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

/// Used to prepare era staking points for benchmarking claim()
///
/// number_of_payees indicates the amount of accounts (developer + stakers) that will get paid after claim() is called.
/// Note that this number can exceed MAX_AMOUNT_OF_STAKERS since theoretically, each era could have an entirely different set of stakers.
///
/// number_of_era_staking_points indicates how many different era_staking_points we have when we call claim. Note that we could have only 1
/// era_staking_point that happened 30 eras ago but that information would still apply to all the following eras.
/// Note that we don't take into account situations where two consecutive era_staking_points have a multiple eras in between them. If that
/// is the case, additional calculation is needed but it doesn't incur any overhead on read/write operations.
fn prepare_claim<T: Config>(
    contract_id: &T::SmartContract,
    number_of_payees: u32,
    number_of_era_staking_points: u32,
) -> Result<(), &'static str> {
    // Developer will always be one of the payees so we need one less staker to fulfill the number_of_payees requirement
    let number_of_stakers = number_of_payees - 1;

    // Start of with max possible amount of stakers and advance an era
    let mut stakers_so_far = T::MaxNumberOfStakersPerContract::get().min(number_of_stakers);
    let mut seed = SEED;
    let mut stakers = prepare_bond_and_stake::<T>(stakers_so_far, &contract_id, seed)?;
    advance_to_era::<T>(CurrentEra::<T>::get() + 1u32);
    // At this point, 'stakers_so_far' rewards can be claimed for different stakers + 1 for the developer

    for _ in 1..number_of_era_staking_points {
        // Calculate number of remaining payees we need to get into storage but don't exceed max allowed stakers per era.
        let remaining_payees = number_of_stakers
            .saturating_sub(stakers_so_far)
            .min(T::MaxNumberOfStakersPerContract::get());

        // In case we have some remaining payees we need to get into the storage, unbond old ones and add some new ones.
        if remaining_payees > 0 {
            for idx in 0..remaining_payees {
                // Unbond some old stakers. Since era advanced, their rewards will remain.
                DappsStaking::<T>::unbond_unstake_and_withdraw(
                    RawOrigin::Signed(stakers[idx as usize].clone()).into(),
                    contract_id.clone(),
                    T::MinimumStakingAmount::get(),
                )?;
            }
            // This ensures we don't reuse old account Ids
            seed += 1;
            stakers = prepare_bond_and_stake::<T>(remaining_payees, &contract_id, seed)?;
            stakers_so_far += remaining_payees;
        }

        advance_to_era::<T>(CurrentEra::<T>::get() + 1u32);
    }

    Ok(())
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
            RewardsClaimed::<T>::insert(&contract_id, &claimer_id, balance);
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
        let n in 0 .. T::MaxNumberOfStakersPerContract::get();
        initialize::<T>();

        let (_, contract_id) = register_contract::<T>()?;
        prepare_bond_and_stake::<T>(n, &contract_id, SEED)?;

        let staker = whitelisted_caller();
        let _ = T::Currency::make_free_balance_be(&staker, BalanceOf::<T>::max_value());
        let amount = BalanceOf::<T>::max_value() / 2u32.into();

    }: _(RawOrigin::Signed(staker.clone()), contract_id.clone(), amount.clone())
    verify {
        assert_last_event::<T>(Event::<T>::BondAndStake(staker, contract_id, amount).into());
    }

    unbond_unstake_and_withdraw {
        let n in 0 .. T::MaxNumberOfStakersPerContract::get();
        initialize::<T>();

        let (_, contract_id) = register_contract::<T>()?;
        prepare_bond_and_stake::<T>(n, &contract_id, SEED)?;

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
        let m in 1 .. MAX_NUMBER_OF_ERA_STAKING_POINTS;

        initialize::<T>();
        let (developer_id, contract_id) = register_contract::<T>()?;

        prepare_claim::<T>(&contract_id, n, m)?;
        let current_era = DappsStaking::<T>::current_era();

        let claimer: T::AccountId = whitelisted_caller();
    }: _(RawOrigin::Signed(claimer.clone()), contract_id.clone())
    verify {
        assert_eq!(Some(current_era), ContractLastClaimed::<T>::get(&contract_id));
    }

    force_new_era {
    }: _(RawOrigin::Root)

}

impl_benchmark_test_suite!(
    DappsStaking,
    crate::tests::new_test_ext(),
    crate::tests::Test,
);
