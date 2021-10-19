use super::*;
use frame_support::assert_ok;
use mock::{EraIndex, *};
use sp_runtime::{traits::AccountIdConversion, Perbill};

/// Used to fetch the free balance of dapps staking account
pub(crate) fn free_balance_of_dapps_staking_account() -> Balance {
    <TestRuntime as Config>::Currency::free_balance(
        &<TestRuntime as Config>::PalletId::get().into_account(),
    )
}

/// Used to register contract for staking and assert success.
pub(crate) fn register_contract(developer: AccountId, contract: &MockSmartContract<AccountId>) {
    assert_ok!(DappsStaking::enable_developer_pre_approval(
        Origin::root(),
        false
    ));
    assert_ok!(DappsStaking::register(
        Origin::signed(developer),
        contract.clone()
    ));
}

/// Used to get total dapps reward for an era.
pub(crate) fn get_total_reward_per_era() -> Balance {
    BLOCK_REWARD * BLOCKS_PER_ERA as Balance
}

/// Used to perform bond_and_stake with success assertion.
pub(crate) fn bond_and_stake_with_verification(
    staker_id: AccountId,
    contract_id: &MockSmartContract<AccountId>,
    value: Balance,
) {
    assert_ok!(DappsStaking::bond_and_stake(
        Origin::signed(staker_id),
        contract_id.clone(),
        value,
    ));
}

/// Used to perform unbond_unstake_and_withdraw with success assertion.
pub(crate) fn unbond_unstake_and_withdraw_with_verification(
    staker_id: AccountId,
    contract_id: &MockSmartContract<AccountId>,
    value: Balance,
) {
    assert_ok!(DappsStaking::unbond_unstake_and_withdraw(
        Origin::signed(staker_id),
        contract_id.clone(),
        value,
    ));
}

/// Used to verify ledger content.
pub(crate) fn verify_ledger(staker_id: AccountId, staked_value: Balance) {
    // Verify that ledger storage values are as expected.
    let ledger = Ledger::<TestRuntime>::get(staker_id);
    assert_eq!(staked_value, ledger);
}

/// Used to verify era staking points content. Note that this requires era staking points for the specified era to exist.
pub(crate) fn verify_era_staking_points(
    contract_id: &MockSmartContract<AccountId>,
    total_staked_value: Balance,
    era: crate::EraIndex,
    stakers: Vec<(AccountId, Balance)>,
) {
    // Verify that era staking points are as expected for the contract
    let era_staking_points = ContractEraStake::<TestRuntime>::get(&contract_id, era).unwrap();
    assert_eq!(total_staked_value, era_staking_points.total);
    assert_eq!(stakers.len(), era_staking_points.stakers.len());

    for (staker_id, staked_value) in stakers {
        assert_eq!(
            staked_value,
            *era_staking_points.stakers.get(&staker_id).unwrap()
        );
    }
}

/// Used to verify pallet era staked value.
pub(crate) fn verify_pallet_era_staked(era: crate::EraIndex, total_staked_value: Balance) {
    // Verify that total staked amount in era is as expected
    let era_rewards = EraRewardsAndStakes::<TestRuntime>::get(era).unwrap();
    assert_eq!(total_staked_value, era_rewards.staked);
}

/// Used to verify pallet era staked and reward values.
pub(crate) fn verify_pallet_era_staked_and_reward(
    era: crate::EraIndex,
    total_staked_value: Balance,
    total_reward_value: Balance,
) {
    // Verify that total staked amount in era is as expected
    let era_rewards = EraRewardsAndStakes::<TestRuntime>::get(era).unwrap();
    assert_eq!(total_staked_value, era_rewards.staked);
    assert_eq!(total_reward_value, era_rewards.rewards);
}

/// Used to perform claim with success assertion
pub(crate) fn claim_with_verification(
    claimer: AccountId,
    contract: MockSmartContract<AccountId>,
    claim_era: EraIndex,
) {
    assert_ok!(DappsStaking::claim(
        Origin::signed(claimer),
        contract,
        claim_era
    ));

    let rewards_and_stakes = DappsStaking::era_reward_and_stake(&claim_era).unwrap();
    let staking_points = DappsStaking::contract_era_stake(&contract, &claim_era).unwrap();

    let reward = Perbill::from_rational(staking_points.total, rewards_and_stakes.staked)
        * rewards_and_stakes.rewards
        * reward_scaling_factor(claim_era);

    System::assert_last_event(mock::Event::DappsStaking(crate::Event::ContractClaimed(
        contract, claim_era, reward,
    )));
}

// Get reward scaling factor for the given era
pub(crate) fn reward_scaling_factor(era: EraIndex) -> Balance {
    if era < BonusEraDuration::get() {
        pallet::REWARD_SCALING as Balance
    } else {
        1 as Balance
    }
}

/// Used to calculate the expected reward for the staker
pub(crate) fn calc_expected_staker_reward(
    claim_era: EraIndex,
    contract_stake: Balance,
    staker_stake: Balance,
) -> Balance {
    let rewards_and_stakes = DappsStaking::era_reward_and_stake(&claim_era).unwrap();
    let contract_reward = Perbill::from_rational(contract_stake, rewards_and_stakes.staked)
        * rewards_and_stakes.rewards
        * reward_scaling_factor(claim_era);
    let contract_reward_staker_part =
        Perbill::from_percent(100 - DEVELOPER_REWARD_PERCENTAGE) * contract_reward;

    Perbill::from_rational(staker_stake, contract_stake) * contract_reward_staker_part
}

/// Used to calculate the expected reward for the developer
pub(crate) fn calc_expected_developer_reward(
    claim_era: EraIndex,
    contract_stake: Balance,
) -> Balance {
    let rewards_and_stakes = DappsStaking::era_reward_and_stake(&claim_era).unwrap();
    let contract_reward = Perbill::from_rational(contract_stake, rewards_and_stakes.staked)
        * rewards_and_stakes.rewards
        * reward_scaling_factor(claim_era);
    Perbill::from_percent(DEVELOPER_REWARD_PERCENTAGE) * contract_reward
}

/// Check staker/dev Balance after reward distribution.
/// Check that claimed rewards for staker/dev are updated.
pub(crate) fn check_rewards_on_balance_and_storage(
    user: &AccountId,
    free_balance: Balance,
    expected_era_reward: Balance,
) {
    assert_eq!(
        <TestRuntime as Config>::Currency::free_balance(user),
        free_balance + expected_era_reward
    );
}

/// Check that claimed rewards on this contract are updated
pub(crate) fn check_paidout_rewards_for_contract(
    contract: &MockSmartContract<AccountId>,
    era: EraIndex,
    expected_rewards: Balance,
) {
    let contract_staking_info = DappsStaking::contract_era_stake(contract, era).unwrap_or_default();
    assert_eq!(contract_staking_info.claimed_rewards, expected_rewards,)
}

/// Used to verify that storage is cleared of all contract related values after unregistration.
pub(crate) fn verify_storage_after_unregister(
    developer: &AccountId,
    contract_id: &MockSmartContract<AccountId>,
    current_era: EraIndex,
) {
    assert!(!ContractEraStake::<TestRuntime>::contains_key(
        contract_id,
        &current_era
    ));
    assert!(RegisteredDapps::<TestRuntime>::contains_key(contract_id));
    assert!(!RegisteredDevelopers::<TestRuntime>::contains_key(
        developer
    ));
}
