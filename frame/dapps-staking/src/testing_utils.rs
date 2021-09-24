use super::*;
use frame_support::assert_ok;
use mock::{EraIndex, *};
use sp_runtime::traits::Zero;
use sp_runtime::Perbill;

/// Used to register contract for staking and assert success.
pub(crate) fn register_contract(developer: AccountId, contract: &SmartContract<AccountId>) {
    assert_ok!(DappsStaking::register(
        Origin::signed(developer),
        contract.clone()
    ));
}

/// Used to skip "for_era" eras, rewarding each era in the process.
pub(crate) fn advance_era_and_reward(
    for_era: EraIndex,
    rewards: BalanceOf<TestRuntime>,
    staked: BalanceOf<TestRuntime>,
) {
    // TODO advance era by incrementing block production, needed for block rewards
    let current: EraIndex = mock::DappsStaking::current_era();

    let era_reward = EraRewardAndStake { rewards, staked };
    for era in 0..for_era {
        <EraRewardsAndStakes<TestRuntime>>::insert(current + era, era_reward.clone());
    }
    <CurrentEra<TestRuntime>>::put(&current + for_era);
}

/// Used to perform bond_and_stake with success assertion.
pub(crate) fn bond_and_stake_with_verification(
    staker_id: AccountId,
    contract_id: &SmartContract<AccountId>,
    value: Balance,
) {
    assert_ok!(DappsStaking::bond_and_stake(
        Origin::signed(staker_id),
        contract_id.clone(),
        value,
    ));
}

/// Used to verify ledger content.
pub(crate) fn verify_ledger(staker_id: AccountId, staked_value: Balance) {
    // Verify that ledger storage values are as expected.
    let ledger = Ledger::<TestRuntime>::get(staker_id).unwrap();
    assert_eq!(staked_value, ledger.total);
    assert_eq!(staked_value, ledger.active);
}

/// Used to verify era staking points content.
pub(crate) fn verify_era_staking_points(
    contract_id: &SmartContract<AccountId>,
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

/// Used to verify pallet era reward values.
pub(crate) fn verify_pallet_era_rewards(
    era: crate::EraIndex,
    total_staked_value: Balance,
    total_reward_value: Balance,
) {
    // Verify that total staked amount in era is as expected
    let era_rewards = EraRewardsAndStakes::<TestRuntime>::get(era).unwrap();
    assert_eq!(total_staked_value, era_rewards.staked);
    assert_eq!(total_reward_value, era_rewards.rewards);
}

/// Used to verify storage content after claim() is successfuly executed.
pub(crate) fn verify_contract_history_is_cleared(
    contract: SmartContract<mock::AccountId>,
    from_era: EraIndex,
    to_era: EraIndex,
) {
    // check claim era is changed
    assert_eq!(
        mock::DappsStaking::contract_last_claimed(&contract).unwrap_or(Zero::zero()),
        to_era
    );

    // check last staked era changed
    assert_eq!(
        mock::DappsStaking::contract_last_staked(&contract).unwrap_or(Zero::zero()),
        to_era
    );

    // check new ContractEraStaked
    assert!(mock::DappsStaking::contract_era_stake(contract, to_era).is_some());

    // check history storage is cleared
    for era in from_era..to_era {
        assert!(mock::DappsStaking::contract_era_stake(contract, era).is_none());
    }
}

/// Used to perform claim with success assertion
pub(crate) fn claim(
    claimer: AccountId,
    contract: SmartContract<mock::AccountId>,
    start_era: EraIndex,
    claim_era: EraIndex,
) {
    assert_ok!(DappsStaking::claim(Origin::signed(claimer), contract));
    // check the event for claim
    System::assert_last_event(mock::Event::DappsStaking(crate::Event::ContractClaimed(
        contract, claimer, start_era, claim_era,
    )));
}

/// Used to calculate the expected reward for the staker
pub(crate) fn calc_expected_staker_reward(
    era_reward: mock::Balance,
    era_stake: mock::Balance,
    contract_stake: mock::Balance,
    staker_stake: mock::Balance,
) -> mock::Balance {
    let contract_reward = Perbill::from_rational(era_reward, era_stake) * contract_stake;
    let contract_staker_part =
        Perbill::from_percent(100 - DEVELOPER_REWARD_PERCENTAGE) * contract_reward;

    Perbill::from_rational(contract_staker_part, contract_stake) * staker_stake
}

/// Used to calculate the expected reward for the developer
pub(crate) fn calc_expected_developer_reward(
    era_reward: mock::Balance,
    era_stake: mock::Balance,
    contract_stake: mock::Balance,
) -> mock::Balance {
    let contract_reward = Perbill::from_rational(era_reward, era_stake) * contract_stake;

    Perbill::from_percent(DEVELOPER_REWARD_PERCENTAGE) * contract_reward
}
