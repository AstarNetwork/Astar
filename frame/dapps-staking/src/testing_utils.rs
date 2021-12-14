use super::{Event, *};
use frame_support::assert_ok;
use mock::{EraIndex, *};
use sp_runtime::{traits::AccountIdConversion, Perbill};

/// Helper struct used to store information relevant to era/contract/staker combination.
pub(crate) struct MemorySnapshot {
    reward_and_stake: EraRewardAndStake<Balance>,
    dapp_info: DeveloperInfo<AccountId>,
    contract_info: EraStakingPoints<Balance>,
    staker_info: StakerInfo<Balance>,
    ledger: AccountLedger<MockSmartContract<AccountId>, Balance>,
    staker_free_balance: Balance,
}

impl MemorySnapshot {
    /// Prepares a new `MemorySnapshot` struct based on the given arguments.
    fn all(era: EraIndex, contract_id: &MockSmartContract<AccountId>, staker: AccountId) -> Self {
        Self {
            reward_and_stake: DappsStaking::era_reward_and_stake(era).unwrap(),
            dapp_info: RegisteredDapps::<TestRuntime>::get(contract_id).unwrap(),
            contract_info: DappsStaking::contract_staking_info(contract_id, era),
            staker_info: DappsStaking::staker_staking_info(&staker, contract_id, era),
            ledger: DappsStaking::ledger(&staker),
            staker_free_balance: <TestRuntime as Config>::Currency::free_balance(&staker),
        }
    }

    /// Prepares a new `MemorySnapshot` struct but only with contract-related info
    /// (no info specific for individual staker).
    fn contract(era: EraIndex, contract_id: &MockSmartContract<AccountId>) -> Self {
        Self {
            reward_and_stake: DappsStaking::era_reward_and_stake(era).unwrap(),
            dapp_info: RegisteredDapps::<TestRuntime>::get(contract_id).unwrap(),
            contract_info: DappsStaking::contract_staking_info(contract_id, era),
            staker_info: Default::default(),
            ledger: Default::default(),
            staker_free_balance: Default::default(),
        }
    }
}

/// Used to fetch the free balance of dapps staking account
pub(crate) fn free_balance_of_dapps_staking_account() -> Balance {
    <TestRuntime as Config>::Currency::free_balance(
        &<TestRuntime as Config>::PalletId::get().into_account(),
    )
}

/// Used to register contract for staking and assert success.
pub(crate) fn assert_register(developer: AccountId, contract_id: &MockSmartContract<AccountId>) {
    let init_reserved_balance = <TestRuntime as Config>::Currency::reserved_balance(&developer);

    // Contract shouldn't exist.
    assert!(!RegisteredDapps::<TestRuntime>::contains_key(contract_id));
    assert!(!RegisteredDevelopers::<TestRuntime>::contains_key(
        developer
    ));

    // Verify op is successfull
    assert_ok!(DappsStaking::enable_developer_pre_approval(
        Origin::root(),
        false
    ));
    assert_ok!(DappsStaking::register(
        Origin::signed(developer),
        contract_id.clone()
    ));

    let dapp_info = RegisteredDapps::<TestRuntime>::get(contract_id).unwrap();
    assert_eq!(dapp_info.state, DAppState::Registered);
    assert_eq!(dapp_info.developer, developer);
    assert_eq!(
        *contract_id,
        RegisteredDevelopers::<TestRuntime>::get(developer).unwrap()
    );

    let final_reserved_balance = <TestRuntime as Config>::Currency::reserved_balance(&developer);
    assert_eq!(
        final_reserved_balance,
        init_reserved_balance + <TestRuntime as Config>::RegisterDeposit::get()
    );
}

/// Used to get total dapps reward for an era.
pub(crate) fn get_total_reward_per_era() -> Balance {
    BLOCK_REWARD * BLOCKS_PER_ERA as Balance
}

/// Perform `bond_and_stake` with all the accompanied checks including before/after storage comparison.
pub(crate) fn assert_bond_and_stake(
    staker: AccountId,
    contract_id: &MockSmartContract<AccountId>,
    value: Balance,
) {
    let current_era = DappsStaking::current_era();
    let init_state = MemorySnapshot::all(current_era, &contract_id, staker);

    // Calculate the expected value that will be staked.
    let available_for_staking = init_state.staker_free_balance
        - init_state.ledger.locked
        - <TestRuntime as Config>::MinimumRemainingAmount::get();
    let staking_value = available_for_staking.min(value);

    // Perform op and verify everything is as expected
    assert_ok!(DappsStaking::bond_and_stake(
        Origin::signed(staker),
        contract_id.clone(),
        value,
    ));
    System::assert_last_event(mock::Event::DappsStaking(Event::BondAndStake(
        staker,
        contract_id.clone(),
        staking_value,
    )));

    let final_state = MemorySnapshot::all(current_era, &contract_id, staker);

    // In case this is the staker's entry stake on this contract
    if init_state.staker_info.staked == 0 {
        assert!(!init_state.ledger.staked_contracts.contains_key(contract_id));
        assert_eq!(
            final_state.contract_info.number_of_stakers,
            init_state.contract_info.number_of_stakers + 1
        );
    }

    // Verify the remaining states
    assert_eq!(
        final_state.reward_and_stake.staked,
        init_state.reward_and_stake.staked + staking_value
    );
    assert_eq!(
        final_state.contract_info.total,
        init_state.contract_info.total + staking_value
    );
    assert_eq!(
        final_state.staker_info.staked,
        init_state.staker_info.staked + staking_value
    );
    assert_eq!(
        final_state.ledger.locked,
        init_state.ledger.locked + staking_value
    );
    assert_eq!(final_state.ledger.staked_contracts[&contract_id], None);
}

/// Perform `withdraw_from_unregistered` with all the accompanied checks including before/after storage comparison.
pub(crate) fn assert_withdraw_from_unregistered(
    staker: AccountId,
    contract_id: &MockSmartContract<AccountId>,
) {
    let current_era = DappsStaking::current_era();
    let init_state = MemorySnapshot::all(current_era, contract_id, staker);

    // Initial checks
    assert_eq!(init_state.dapp_info.state, DAppState::Unregistered);
    assert!(init_state.staker_info.staked > 0);

    // Op with verification
    assert_ok!(DappsStaking::withdraw_from_unregistered(
        Origin::signed(staker.clone()),
        contract_id.clone()
    ));
    System::assert_last_event(mock::Event::DappsStaking(Event::WithdrawFromUnregistered(
        staker,
        contract_id.clone(),
        init_state.staker_info.staked,
    )));

    let final_state = MemorySnapshot::all(current_era, contract_id, staker);

    // Verify that all final states are as expected
    assert_eq!(init_state.reward_and_stake, final_state.reward_and_stake);
    assert_eq!(init_state.dapp_info, final_state.dapp_info);
    assert_eq!(
        final_state.ledger.locked,
        init_state.ledger.locked - init_state.staker_info.staked
    );
    assert_eq!(final_state.staker_info.staked, 0);
}

/// Used to perform start_unbonding with sucess and storage assertions.
pub(crate) fn assert_unbond_and_unstake(
    staker: AccountId,
    contract_id: &MockSmartContract<AccountId>,
    value: Balance,
) {
    // Get latest staking info
    let current_era = DappsStaking::current_era();
    let init_state = MemorySnapshot::all(current_era, &contract_id, staker);

    // Calculate the expected resulting unbonding amount
    let remaining_staked = init_state.staker_info.staked.saturating_sub(value);
    let expected_unbond_amount = if remaining_staked < MINIMUM_STAKING_AMOUNT {
        init_state.staker_info.staked
    } else {
        value
    };
    let remaining_staked = init_state.staker_info.staked - expected_unbond_amount;

    // Ensure op is successful and event is emitted
    assert_ok!(DappsStaking::unbond_and_unstake(
        Origin::signed(staker),
        contract_id.clone(),
        value
    ));
    System::assert_last_event(mock::Event::DappsStaking(Event::UnbondAndUnstake(
        staker,
        contract_id.clone(),
        expected_unbond_amount,
    )));

    // Fetch the latest unbonding info so we can compare it to initial unbonding info
    let final_state = MemorySnapshot::all(current_era, &contract_id, staker);
    let expected_unlock_era = current_era + 1 + UNBONDING_PERIOD;
    match init_state
        .ledger
        .unbonding_info
        .vec()
        .binary_search_by(|x| x.unlock_era.cmp(&expected_unlock_era))
    {
        Ok(_) => assert_eq!(
            init_state.ledger.unbonding_info.len(),
            final_state.ledger.unbonding_info.len()
        ),
        Err(_) => assert_eq!(
            init_state.ledger.unbonding_info.len() + 1,
            final_state.ledger.unbonding_info.len()
        ),
    }
    assert_eq!(
        init_state.ledger.unbonding_info.sum() + expected_unbond_amount,
        final_state.ledger.unbonding_info.sum()
    );

    // Push the unlocking chunk we expect to have at the end and compare two structs
    let mut unbonding_info = init_state.ledger.unbonding_info.clone();
    unbonding_info.add(UnlockingChunk {
        amount: expected_unbond_amount,
        unlock_era: current_era + 1 + UNBONDING_PERIOD,
    });
    assert_eq!(unbonding_info, final_state.ledger.unbonding_info);

    // Ensure that total locked value for staker hasn't been changed.
    assert_eq!(init_state.ledger.locked, final_state.ledger.locked);
    if final_state.ledger.is_empty() {
        assert!(!Ledger::<TestRuntime>::contains_key(&staker));
    }

    // Ensure that total staked amount has been decreased for contract and staking points are updated
    assert_eq!(
        init_state.contract_info.total - expected_unbond_amount,
        final_state.contract_info.total
    );
    assert_eq!(
        init_state.staker_info.staked - expected_unbond_amount,
        final_state.staker_info.staked
    );

    // Ensure that the number of stakers is as expected
    let delta = if remaining_staked > 0 { 0 } else { 1 };
    assert_eq!(
        init_state.contract_info.number_of_stakers - delta,
        final_state.contract_info.number_of_stakers
    );

    // Ensure that total staked value has been decreased
    assert_eq!(
        init_state.reward_and_stake.staked - expected_unbond_amount,
        final_state.reward_and_stake.staked
    );
}

/// Used to perform start_unbonding with sucess and storage assertions.
pub(crate) fn assert_withdraw_unbonded(staker: AccountId) {
    let current_era = DappsStaking::current_era();

    let init_rewards_and_stakes = EraRewardsAndStakes::<TestRuntime>::get(current_era).unwrap();
    let init_ledger = Ledger::<TestRuntime>::get(&staker);

    // Get the current unlocking chunks
    let (valid_info, remaining_info) = init_ledger.unbonding_info.partition(current_era);
    let expected_unbond_amount = valid_info.sum();

    // Ensure op is successful and event is emitted
    assert_ok!(DappsStaking::withdraw_unbonded(Origin::signed(staker),));
    System::assert_last_event(mock::Event::DappsStaking(Event::Withdrawn(
        staker,
        expected_unbond_amount,
    )));

    // Fetch the latest unbonding info so we can compare it to expected remainder
    let final_ledger = Ledger::<TestRuntime>::get(&staker);
    assert_eq!(remaining_info, final_ledger.unbonding_info);
    if final_ledger.unbonding_info.is_empty() && final_ledger.locked == 0 {
        assert!(!Ledger::<TestRuntime>::contains_key(&staker));
    }

    // Compare the ledger and total staked value
    let final_rewards_and_stakes = EraRewardsAndStakes::<TestRuntime>::get(current_era).unwrap();
    assert_eq!(final_rewards_and_stakes, init_rewards_and_stakes);
    assert_eq!(
        final_ledger.locked,
        init_ledger.locked - expected_unbond_amount
    );
}

/// Used to verify era staking points content. Note that this requires era staking points for the specified era to exist.
pub(crate) fn verify_contract_staking_info(
    contract_id: &MockSmartContract<AccountId>,
    total_staked_value: Balance,
    era: crate::EraIndex,
    stakers: Vec<(AccountId, Balance)>,
) {
    // Verify that era staking points are as expected for the contract
    let era_staking_points = ContractEraStake::<TestRuntime>::get(&contract_id, era).unwrap();
    assert_eq!(total_staked_value, era_staking_points.total);
    assert_eq!(stakers.len(), era_staking_points.number_of_stakers as usize);

    for (staker_id, staked_value) in stakers {
        assert_eq!(
            staked_value,
            DappsStaking::staker_staking_info(&staker_id, contract_id, era).staked
        );
    }
}

/// Perform `claim` with all the accompanied checks including before/after storage comparison.
pub(crate) fn assert_claim(
    claimer: AccountId,
    contract_id: MockSmartContract<AccountId>,
    claim_era: EraIndex,
) {
    // Clear all events so we can check all the emitted events from claim
    // TODO: this might not be needed if we don't need to verify more than 1 event
    clear_all_events();

    let init_state = MemorySnapshot::all(claim_era, &contract_id, claimer);
    let claimer_is_dev = init_state.dapp_info.developer == claimer;

    // Read in structs from storage
    if !claimer_is_dev {
        assert!(init_state.staker_info.staked > 0);
    }
    assert_eq!(init_state.staker_info.claimed_rewards, 0);

    // Calculate contract portion of the reward
    let (developer_reward_part, stakers_joint_reward) = DappsStaking::dev_stakers_split(
        &init_state.contract_info,
        &init_state.reward_and_stake,
        &Perbill::from_percent(DEVELOPER_REWARD_PERCENTAGE),
    );

    let staker_reward_part = Perbill::from_rational(
        init_state.staker_info.staked,
        init_state.contract_info.total,
    ) * stakers_joint_reward;

    let calculated_reward = if claimer_is_dev {
        developer_reward_part + staker_reward_part
    } else {
        staker_reward_part
    };

    assert_ok!(DappsStaking::claim(
        Origin::signed(claimer),
        contract_id,
        claim_era
    ));
    System::assert_last_event(mock::Event::DappsStaking(Event::Reward(
        claimer,
        contract_id.clone(),
        claim_era,
        calculated_reward,
    )));

    let final_state = MemorySnapshot::all(claim_era, &contract_id, claimer);
    assert_eq!(
        init_state.staker_free_balance + calculated_reward,
        final_state.staker_free_balance
    );
    assert_eq!(final_state.staker_info.claimed_rewards, calculated_reward);
}

/// Perform `unregister` with all the accompanied checks including before/after storage comparison.
pub(crate) fn assert_unregister(developer: AccountId, contract_id: &MockSmartContract<AccountId>) {
    let current_era = DappsStaking::current_era();
    let init_state = MemorySnapshot::contract(current_era, contract_id);
    let init_reserved_balance = <TestRuntime as Config>::Currency::reserved_balance(&developer);

    // dApp should be registered prior to unregistering it
    assert_eq!(init_state.dapp_info.state, DAppState::Registered);

    // Ensure that contract can be unregistered
    assert_ok!(DappsStaking::unregister(
        Origin::signed(developer),
        contract_id.clone()
    ));
    System::assert_last_event(mock::Event::DappsStaking(Event::ContractRemoved(
        developer,
        contract_id.clone(),
    )));

    let final_state = MemorySnapshot::contract(current_era, contract_id);
    let final_reserved_balance = <TestRuntime as Config>::Currency::reserved_balance(&developer);
    assert_eq!(
        final_reserved_balance,
        init_reserved_balance - <TestRuntime as Config>::RegisterDeposit::get()
    );

    assert_eq!(
        final_state.reward_and_stake.staked,
        init_state.reward_and_stake.staked - init_state.contract_info.total
    );

    assert_eq!(final_state.contract_info.total, 0);
    assert_eq!(final_state.contract_info.number_of_stakers, 0);

    assert_eq!(final_state.dapp_info.state, DAppState::Unregistered);
    assert_eq!(final_state.dapp_info.developer, developer);
    assert!(RegisteredDevelopers::<TestRuntime>::contains_key(developer));
}
