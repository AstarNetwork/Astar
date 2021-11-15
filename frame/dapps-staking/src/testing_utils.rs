use super::{Event, *};
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

/// Used to perform start_unbonding with sucess and storage assertions.
pub(crate) fn start_unbonding_with_verification(
    staker: AccountId,
    contract_id: &MockSmartContract<AccountId>,
    value: Balance,
) {
    // Get latest staking info
    let current_era = DappsStaking::current_era();
    let era_staking_points = DappsStaking::staking_info(contract_id, current_era);
    let staked_value = era_staking_points.stakers[&staker];

    // Get the current unlocking chunks
    let pre_unbonding_info = UnbondingInfoStorage::<TestRuntime>::get(&staker, contract_id);
    let pre_unbonding_amount = pre_unbonding_info.sum();

    // Calculate the expected resulting unbonding amount
    let expected_unbond_amount =
        if staked_value - pre_unbonding_amount - value < MINIMUM_STAKING_AMOUNT {
            staked_value - pre_unbonding_amount
        } else {
            value
        };

    // Ensure op is successful and event is emitted
    assert_ok!(DappsStaking::start_unbonding(
        Origin::signed(staker),
        contract_id.clone(),
        value
    ));
    System::assert_last_event(mock::Event::DappsStaking(Event::UnbondingStarted(
        staker,
        contract_id.clone(),
        expected_unbond_amount,
        current_era,
    )));

    // Fetch the latest unbonding info so we can compare it to initial unbonding info
    let post_unbonding_info = UnbondingInfoStorage::<TestRuntime>::get(&staker, contract_id);
    assert_eq!(pre_unbonding_info.len() + 1, post_unbonding_info.len());
    assert_eq!(
        pre_unbonding_amount + expected_unbond_amount,
        post_unbonding_info.sum()
    );

    // Push the unlocking chunk we expect to have at the end and compare two structs
    let mut pre_unbonding_info = pre_unbonding_info;
    pre_unbonding_info.push(UnlockingChunk {
        amount: expected_unbond_amount,
        unlock_era: current_era + UNBONDING_PERIOD,
    });
    assert_eq!(pre_unbonding_info, post_unbonding_info);
}

/// Used to perform start_unbonding with sucess and storage assertions.
pub(crate) fn unstake_and_withdraw_with_verification(
    staker: AccountId,
    contract_id: &MockSmartContract<AccountId>,
) {
    // Get latest staking info
    let current_era = DappsStaking::current_era();
    let init_era_staking_points = DappsStaking::staking_info(contract_id, current_era);
    let staked_value = init_era_staking_points.stakers[&staker];

    // Get the staking amount for the contract in this era and locked amount for the staker
    let init_rewards_and_stakes = EraRewardsAndStakes::<TestRuntime>::get(current_era).unwrap();
    let init_ledger = Ledger::<TestRuntime>::get(&staker);

    // Get the current unlocking chunks
    let pre_unbonding_info = UnbondingInfoStorage::<TestRuntime>::get(&staker, contract_id);
    let (valid_info, remaining_info) = pre_unbonding_info.partition(current_era);
    let expected_unbond_amount = valid_info.sum();
    let remainder_staked_value = staked_value - expected_unbond_amount;

    // Ensure op is successful and event is emitted
    assert_ok!(DappsStaking::unstake_and_withdraw(
        Origin::signed(staker),
        contract_id.clone(),
    ));
    System::assert_last_event(mock::Event::DappsStaking(Event::UnstakeAndWithdraw(
        staker,
        contract_id.clone(),
        expected_unbond_amount,
    )));

    // Fetch the latest unbonding info so we can compare it to expected remainder
    let post_withdraw_info = UnbondingInfoStorage::<TestRuntime>::get(&staker, contract_id);
    assert_eq!(remaining_info, post_withdraw_info);
    if post_withdraw_info.is_empty() {
        assert!(!UnbondingInfoStorage::<TestRuntime>::contains_key(
            &staker,
            contract_id
        ));
    }

    // Compare the staking info with the initial one
    let post_era_staking_points = DappsStaking::staking_info(contract_id, current_era);
    assert_eq!(
        init_era_staking_points.total - expected_unbond_amount,
        post_era_staking_points.total
    );
    if remainder_staked_value == 0 {
        assert!(!post_era_staking_points.stakers.contains_key(&staker));
    } else {
        assert_eq!(
            remainder_staked_value,
            post_era_staking_points.stakers[&staker]
        );
    }

    // Compare the ledger and total staked value
    let post_rewards_and_stakes = EraRewardsAndStakes::<TestRuntime>::get(current_era).unwrap();
    let post_ledger = Ledger::<TestRuntime>::get(&staker);
    assert_eq!(
        post_rewards_and_stakes.staked,
        init_rewards_and_stakes.staked - expected_unbond_amount
    );
    assert_eq!(post_ledger, init_ledger - expected_unbond_amount);
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
    // Clear all events so we can check all the emitted events from claim
    clear_all_events();

    assert_ok!(DappsStaking::claim(
        Origin::signed(claimer),
        contract,
        claim_era
    ));

    // Calculated expected reward that will be distributed for the contract.
    let rewards_and_stakes = DappsStaking::era_reward_and_stake(&claim_era).unwrap();
    let staking_points = DappsStaking::contract_era_stake(&contract, &claim_era).unwrap();
    let calculated_reward = Perbill::from_rational(staking_points.total, rewards_and_stakes.staked)
        * rewards_and_stakes.rewards
        * reward_scaling_factor(claim_era);

    // Collect all Reward events and sum up all the rewards.
    let emitted_rewards: Balance = dapps_staking_events()
        .iter()
        .filter_map(|e| {
            if let crate::Event::Reward(_, _, _, single_reward) = e {
                Some(*single_reward as Balance)
            } else {
                None
            }
        })
        .sum();

    assert_eq!(calculated_reward, emitted_rewards);
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
) {
    assert!(RegisteredDapps::<TestRuntime>::contains_key(contract_id));
    assert!(!RegisteredDevelopers::<TestRuntime>::contains_key(
        developer
    ));
}
