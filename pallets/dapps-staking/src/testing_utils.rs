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

use super::{pallet::pallet::Event, *};
use frame_support::assert_ok;
use mock::{Balance, EraIndex, *};
use sp_runtime::{traits::AccountIdConversion, Perbill};

/// Helper struct used to store information relevant to era/contract/staker combination.
pub(crate) struct MemorySnapshot {
    era_info: EraInfo,
    dapp_info: DAppInfo<AccountId>,
    staker_info: StakerInfo,
    contract_info: ContractStakeInfo,
    free_balance: Balance,
    ledger: AccountLedger,
}

impl MemorySnapshot {
    /// Prepares a new `MemorySnapshot` struct based on the given arguments.
    pub(crate) fn all(
        era: EraIndex,
        contract_id: &MockSmartContract<AccountId>,
        account: AccountId,
    ) -> Self {
        Self {
            era_info: DappsStaking::general_era_info(era).unwrap(),
            dapp_info: RegisteredDapps::<TestRuntime>::get(contract_id).unwrap(),
            staker_info: GeneralStakerInfo::<TestRuntime>::get(&account, contract_id),
            contract_info: DappsStaking::contract_stake_info(contract_id, era).unwrap_or_default(),
            ledger: DappsStaking::ledger(&account),
            free_balance: <TestRuntime as Config>::Currency::free_balance(&account),
        }
    }

    /// Prepares a new `MemorySnapshot` struct but only with contract-related info
    /// (no info specific for individual staker).
    pub(crate) fn contract(era: EraIndex, contract_id: &MockSmartContract<AccountId>) -> Self {
        Self {
            era_info: DappsStaking::general_era_info(era).unwrap(),
            dapp_info: RegisteredDapps::<TestRuntime>::get(contract_id).unwrap(),
            staker_info: Default::default(),
            contract_info: DappsStaking::contract_stake_info(contract_id, era).unwrap_or_default(),
            ledger: Default::default(),
            free_balance: Default::default(),
        }
    }
}

/// Used to fetch the free balance of dapps staking account
pub(crate) fn free_balance_of_dapps_staking_account() -> Balance {
    <TestRuntime as Config>::Currency::free_balance(&account_id())
}

/// Used to fetch pallet account Id
pub(crate) fn account_id() -> AccountId {
    <TestRuntime as Config>::PalletId::get().into_account_truncating()
}

/// Used to get total dapps reward for an era.
pub(crate) fn get_total_reward_per_era() -> Balance {
    mock::joint_block_reward() * BLOCKS_PER_ERA as Balance
}

/// Used to register contract for staking and assert success.
pub(crate) fn assert_register(developer: AccountId, contract_id: &MockSmartContract<AccountId>) {
    let init_reserved_balance = <TestRuntime as Config>::Currency::reserved_balance(&developer);

    // Contract shouldn't exist.
    assert!(!RegisteredDapps::<TestRuntime>::contains_key(contract_id));
    assert!(!RegisteredDevelopers::<TestRuntime>::contains_key(
        developer
    ));

    // Verify op is successful
    assert_ok!(DappsStaking::register(
        RuntimeOrigin::root(),
        developer,
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

/// Perform `unregister` with all the accompanied checks including before/after storage comparison.
pub(crate) fn assert_unregister(developer: AccountId, contract_id: &MockSmartContract<AccountId>) {
    let current_era = DappsStaking::current_era();
    let init_state = MemorySnapshot::contract(current_era, contract_id);
    let init_reserved_balance = <TestRuntime as Config>::Currency::reserved_balance(&developer);

    // dApp should be registered prior to unregistering it
    assert_eq!(init_state.dapp_info.state, DAppState::Registered);

    // Ensure that contract can be unregistered
    assert_ok!(DappsStaking::unregister(
        RuntimeOrigin::root(),
        contract_id.clone()
    ));
    System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::ContractRemoved(
        developer,
        contract_id.clone(),
    )));

    let final_state = MemorySnapshot::contract(current_era, contract_id);
    let final_reserved_balance = <TestRuntime as Config>::Currency::reserved_balance(&developer);
    assert_eq!(
        final_reserved_balance,
        init_reserved_balance - <TestRuntime as Config>::RegisterDeposit::get()
    );

    assert_eq!(final_state.era_info.staked, init_state.era_info.staked);

    assert_eq!(
        final_state.contract_info.total,
        init_state.contract_info.total
    );
    assert_eq!(
        final_state.contract_info.number_of_stakers,
        init_state.contract_info.number_of_stakers
    );

    assert_eq!(
        final_state.dapp_info.state,
        DAppState::Unregistered(current_era)
    );
    assert_eq!(final_state.dapp_info.developer, developer);
}

/// Perform `withdraw_from_unregistered` with all the accompanied checks including before/after storage comparison.
pub(crate) fn assert_withdraw_from_unregistered(
    staker: AccountId,
    contract_id: &MockSmartContract<AccountId>,
) {
    let current_era = DappsStaking::current_era();
    let init_state = MemorySnapshot::all(current_era, contract_id, staker);

    // Initial checks
    if let DAppState::Unregistered(era) = init_state.dapp_info.state {
        assert!(era <= DappsStaking::current_era());
    } else {
        panic!("Contract should be unregistered.")
    };

    let staked_value = init_state.staker_info.latest_staked_value();
    assert!(staked_value > 0);

    // Op with verification
    assert_ok!(DappsStaking::withdraw_from_unregistered(
        RuntimeOrigin::signed(staker.clone()),
        contract_id.clone()
    ));
    System::assert_last_event(mock::RuntimeEvent::DappsStaking(
        Event::WithdrawFromUnregistered(staker, contract_id.clone(), staked_value),
    ));

    let final_state = MemorySnapshot::all(current_era, contract_id, staker);

    // Verify that all final states are as expected
    assert_eq!(
        init_state.era_info.staked,
        final_state.era_info.staked + staked_value
    );
    assert_eq!(
        init_state.era_info.locked,
        final_state.era_info.locked + staked_value
    );
    assert_eq!(init_state.dapp_info, final_state.dapp_info);
    assert_eq!(
        init_state.ledger.locked,
        final_state.ledger.locked + staked_value
    );
    assert_eq!(
        init_state.ledger.unbonding_info,
        final_state.ledger.unbonding_info
    );

    assert!(final_state.staker_info.latest_staked_value().is_zero());
    assert!(!GeneralStakerInfo::<TestRuntime>::contains_key(
        &staker,
        contract_id
    ));
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
    let available_for_staking = init_state.free_balance
        - init_state.ledger.locked
        - <TestRuntime as Config>::MinimumRemainingAmount::get();
    let staking_value = available_for_staking.min(value);

    // Perform op and verify everything is as expected
    assert_ok!(DappsStaking::bond_and_stake(
        RuntimeOrigin::signed(staker),
        contract_id.clone(),
        value,
    ));
    System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::BondAndStake(
        staker,
        contract_id.clone(),
        staking_value,
    )));

    let final_state = MemorySnapshot::all(current_era, &contract_id, staker);

    // In case staker hasn't been staking this contract until now
    if init_state.staker_info.latest_staked_value() == 0 {
        assert!(GeneralStakerInfo::<TestRuntime>::contains_key(
            &staker,
            contract_id
        ));
        assert_eq!(
            final_state.contract_info.number_of_stakers,
            init_state.contract_info.number_of_stakers + 1
        );
    }

    // Verify the remaining states
    assert_eq!(
        final_state.era_info.staked,
        init_state.era_info.staked + staking_value
    );
    assert_eq!(
        final_state.era_info.locked,
        init_state.era_info.locked + staking_value
    );
    assert_eq!(
        final_state.contract_info.total,
        init_state.contract_info.total + staking_value
    );
    assert_eq!(
        final_state.staker_info.latest_staked_value(),
        init_state.staker_info.latest_staked_value() + staking_value
    );
    assert_eq!(
        final_state.ledger.locked,
        init_state.ledger.locked + staking_value
    );
}

/// Used to perform start_unbonding with success and storage assertions.
pub(crate) fn assert_unbond_and_unstake(
    staker: AccountId,
    contract_id: &MockSmartContract<AccountId>,
    value: Balance,
) {
    // Get latest staking info
    let current_era = DappsStaking::current_era();
    let init_state = MemorySnapshot::all(current_era, &contract_id, staker);

    // Calculate the expected resulting unbonding amount
    let remaining_staked = init_state
        .staker_info
        .latest_staked_value()
        .saturating_sub(value);
    let expected_unbond_amount = if remaining_staked < MINIMUM_STAKING_AMOUNT {
        init_state.staker_info.latest_staked_value()
    } else {
        value
    };
    let remaining_staked = init_state.staker_info.latest_staked_value() - expected_unbond_amount;

    // Ensure op is successful and event is emitted
    assert_ok!(DappsStaking::unbond_and_unstake(
        RuntimeOrigin::signed(staker),
        contract_id.clone(),
        value
    ));
    System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::UnbondAndUnstake(
        staker,
        contract_id.clone(),
        expected_unbond_amount,
    )));

    // Fetch the latest unbonding info so we can compare it to initial unbonding info
    let final_state = MemorySnapshot::all(current_era, &contract_id, staker);
    let expected_unlock_era = current_era + UNBONDING_PERIOD;
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
        unlock_era: current_era + UNBONDING_PERIOD,
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
        init_state.staker_info.latest_staked_value() - expected_unbond_amount,
        final_state.staker_info.latest_staked_value()
    );

    // Ensure that the number of stakers is as expected
    let delta = if remaining_staked > 0 { 0 } else { 1 };
    assert_eq!(
        init_state.contract_info.number_of_stakers - delta,
        final_state.contract_info.number_of_stakers
    );

    // Ensure that total staked value has been decreased
    assert_eq!(
        init_state.era_info.staked - expected_unbond_amount,
        final_state.era_info.staked
    );
    // Ensure that locked amount is the same since this will only start the unbonding period
    assert_eq!(init_state.era_info.locked, final_state.era_info.locked);
}

/// Used to perform start_unbonding with success and storage assertions.
pub(crate) fn assert_withdraw_unbonded(staker: AccountId) {
    let current_era = DappsStaking::current_era();

    let init_era_info = GeneralEraInfo::<TestRuntime>::get(current_era).unwrap();
    let init_ledger = Ledger::<TestRuntime>::get(&staker);

    // Get the current unlocking chunks
    let (valid_info, remaining_info) = init_ledger.unbonding_info.partition(current_era);
    let expected_unbond_amount = valid_info.sum();

    // Ensure op is successful and event is emitted
    assert_ok!(DappsStaking::withdraw_unbonded(RuntimeOrigin::signed(
        staker
    ),));
    System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::Withdrawn(
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
    let final_rewards_and_stakes = GeneralEraInfo::<TestRuntime>::get(current_era).unwrap();
    assert_eq!(final_rewards_and_stakes.staked, init_era_info.staked);
    assert_eq!(
        final_rewards_and_stakes.locked,
        init_era_info.locked - expected_unbond_amount
    );
    assert_eq!(
        final_ledger.locked,
        init_ledger.locked - expected_unbond_amount
    );
}

/// Used to perform nomination transfer with success and storage assertions.
pub(crate) fn assert_nomination_transfer(
    staker: AccountId,
    origin_contract_id: &MockSmartContract<AccountId>,
    value: Balance,
    target_contract_id: &MockSmartContract<AccountId>,
) {
    // Get latest staking info
    let current_era = DappsStaking::current_era();
    let origin_init_state = MemorySnapshot::all(current_era, &origin_contract_id, staker);
    let target_init_state = MemorySnapshot::all(current_era, &target_contract_id, staker);

    // Calculate value which will actually be transfered
    let init_staked_value = origin_init_state.staker_info.latest_staked_value();
    let expected_transfer_amount = if init_staked_value - value >= MINIMUM_STAKING_AMOUNT {
        value
    } else {
        init_staked_value
    };

    // Ensure op is successful and event is emitted
    assert_ok!(DappsStaking::nomination_transfer(
        RuntimeOrigin::signed(staker),
        origin_contract_id.clone(),
        value,
        target_contract_id.clone()
    ));
    System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::NominationTransfer(
        staker,
        origin_contract_id.clone(),
        expected_transfer_amount,
        target_contract_id.clone(),
    )));

    let origin_final_state = MemorySnapshot::all(current_era, &origin_contract_id, staker);
    let target_final_state = MemorySnapshot::all(current_era, &target_contract_id, staker);

    // Ensure staker info has increased/decreased staked amount
    assert_eq!(
        origin_final_state.staker_info.latest_staked_value(),
        init_staked_value - expected_transfer_amount
    );
    assert_eq!(
        target_final_state.staker_info.latest_staked_value(),
        target_init_state.staker_info.latest_staked_value() + expected_transfer_amount
    );

    // Ensure total value staked on contracts has appropriately increased/decreased
    assert_eq!(
        origin_final_state.contract_info.total,
        origin_init_state.contract_info.total - expected_transfer_amount
    );
    assert_eq!(
        target_final_state.contract_info.total,
        target_init_state.contract_info.total + expected_transfer_amount
    );

    // Ensure number of contracts has been reduced on origin contract if it is fully unstaked
    let origin_contract_fully_unstaked = init_staked_value == expected_transfer_amount;
    if origin_contract_fully_unstaked {
        assert_eq!(
            origin_final_state.contract_info.number_of_stakers + 1,
            origin_init_state.contract_info.number_of_stakers
        );
    }

    // Ensure number of contracts has been increased on target contract it is first stake by the staker
    let no_init_stake_on_target_contract = target_init_state
        .staker_info
        .latest_staked_value()
        .is_zero();
    if no_init_stake_on_target_contract {
        assert_eq!(
            target_final_state.contract_info.number_of_stakers,
            target_init_state.contract_info.number_of_stakers + 1
        );
    }

    // Ensure DB entry has been removed if era stake vector is empty
    let fully_unstaked_and_nothing_to_claim =
        origin_contract_fully_unstaked && origin_final_state.staker_info.clone().claim() == (0, 0);
    if fully_unstaked_and_nothing_to_claim {
        assert!(!GeneralStakerInfo::<TestRuntime>::contains_key(
            &staker,
            &origin_contract_id
        ));
    }
}

/// Used to perform claim for stakers with success assertion
pub(crate) fn assert_claim_staker(claimer: AccountId, contract_id: &MockSmartContract<AccountId>) {
    let (claim_era, _) = DappsStaking::staker_info(&claimer, contract_id).claim();
    let current_era = DappsStaking::current_era();

    //clean up possible leftover events
    System::reset_events();

    let init_state_claim_era = MemorySnapshot::all(claim_era, contract_id, claimer);
    let init_state_current_era = MemorySnapshot::all(current_era, contract_id, claimer);

    // Calculate contract portion of the reward
    let (_, stakers_joint_reward) = DappsStaking::dev_stakers_split(
        &init_state_claim_era.contract_info,
        &init_state_claim_era.era_info,
    );

    let (claim_era, staked) = init_state_claim_era.staker_info.clone().claim();
    assert!(claim_era > 0); // Sanity check - if this fails, method is being used incorrectly

    // Cannot claim rewards post unregister era, this indicates a bug!
    if let DAppState::Unregistered(unregistered_era) = init_state_claim_era.dapp_info.state {
        assert!(unregistered_era > claim_era);
    }

    let calculated_reward =
        Perbill::from_rational(staked, init_state_claim_era.contract_info.total)
            * stakers_joint_reward;
    let issuance_before_claim = <TestRuntime as Config>::Currency::total_issuance();

    assert_ok!(DappsStaking::claim_staker(
        RuntimeOrigin::signed(claimer),
        contract_id.clone(),
    ));

    let final_state_current_era = MemorySnapshot::all(current_era, contract_id, claimer);

    // assert staked and free balances depending on restake check,
    assert_restake_reward(
        &init_state_current_era,
        &final_state_current_era,
        calculated_reward,
    );

    // check for stake event if restaking is performed
    if DappsStaking::should_restake_reward(
        init_state_current_era.ledger.reward_destination,
        init_state_current_era.dapp_info.state,
        init_state_current_era.staker_info.latest_staked_value(),
    ) {
        // There should be at least 2 events, Reward and BondAndStake.
        // if there's less, panic is acceptable
        let events = dapps_staking_events();
        let second_last_event = &events[events.len() - 2];
        assert_eq!(
            second_last_event.clone(),
            Event::<TestRuntime>::BondAndStake(claimer, contract_id.clone(), calculated_reward)
        );
    }

    // last event should be Reward, regardless of restaking
    System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::Reward(
        claimer,
        contract_id.clone(),
        claim_era,
        calculated_reward,
    )));

    let (new_era, _) = final_state_current_era.staker_info.clone().claim();
    if final_state_current_era.staker_info.is_empty() {
        assert!(new_era.is_zero());
        assert!(!GeneralStakerInfo::<TestRuntime>::contains_key(
            &claimer,
            contract_id
        ));
    } else {
        assert!(new_era > claim_era);
    }
    assert!(new_era.is_zero() || new_era > claim_era);

    // Claim shouldn't mint new tokens, instead it should just transfer from the dapps staking pallet account
    let issuance_after_claim = <TestRuntime as Config>::Currency::total_issuance();
    assert_eq!(issuance_before_claim, issuance_after_claim);

    // Old `claim_era` contract info should never be changed
    let final_state_claim_era = MemorySnapshot::all(claim_era, contract_id, claimer);
    assert_eq!(
        init_state_claim_era.contract_info,
        final_state_claim_era.contract_info
    );
}

// assert staked and locked states depending on should_restake_reward
// returns should_restake_reward result so further checks can be made
fn assert_restake_reward(
    init_state_current_era: &MemorySnapshot,
    final_state_current_era: &MemorySnapshot,
    reward: Balance,
) {
    if DappsStaking::should_restake_reward(
        init_state_current_era.ledger.reward_destination,
        init_state_current_era.dapp_info.state,
        init_state_current_era.staker_info.latest_staked_value(),
    ) {
        // staked values should increase
        assert_eq!(
            init_state_current_era.staker_info.latest_staked_value() + reward,
            final_state_current_era.staker_info.latest_staked_value()
        );
        assert_eq!(
            init_state_current_era.era_info.staked + reward,
            final_state_current_era.era_info.staked
        );
        assert_eq!(
            init_state_current_era.era_info.locked + reward,
            final_state_current_era.era_info.locked
        );
        assert_eq!(
            init_state_current_era.contract_info.total + reward,
            final_state_current_era.contract_info.total
        );
    } else {
        // staked values should remain the same, and free balance increase
        assert_eq!(
            init_state_current_era.free_balance + reward,
            final_state_current_era.free_balance
        );
        assert_eq!(
            init_state_current_era.era_info.staked,
            final_state_current_era.era_info.staked
        );
        assert_eq!(
            init_state_current_era.era_info.locked,
            final_state_current_era.era_info.locked
        );
        assert_eq!(
            init_state_current_era.contract_info,
            final_state_current_era.contract_info
        );
    }
}

/// Used to perform claim for dApp reward with success assertion
pub(crate) fn assert_claim_dapp(contract_id: &MockSmartContract<AccountId>, claim_era: EraIndex) {
    let developer = DappsStaking::dapp_info(contract_id).unwrap().developer;
    let init_state = MemorySnapshot::all(claim_era, contract_id, developer);
    assert!(!init_state.contract_info.contract_reward_claimed);

    // Cannot claim rewards post unregister era
    if let DAppState::Unregistered(unregistered_era) = init_state.dapp_info.state {
        assert!(unregistered_era > claim_era);
    }

    // Calculate contract portion of the reward
    let (calculated_reward, _) =
        DappsStaking::dev_stakers_split(&init_state.contract_info, &init_state.era_info);

    assert_ok!(DappsStaking::claim_dapp(
        RuntimeOrigin::signed(developer),
        contract_id.clone(),
        claim_era,
    ));
    System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::Reward(
        developer,
        contract_id.clone(),
        claim_era,
        calculated_reward,
    )));

    let final_state = MemorySnapshot::all(claim_era, &contract_id, developer);
    assert_eq!(
        init_state.free_balance + calculated_reward,
        final_state.free_balance
    );

    assert!(final_state.contract_info.contract_reward_claimed);

    // Just in case dev is also a staker - this shouldn't cause any change in StakerInfo or Ledger
    assert_eq!(init_state.staker_info, final_state.staker_info);
    assert_eq!(init_state.ledger, final_state.ledger);
}

// change reward destination and verify the update
pub(crate) fn assert_set_reward_destination(
    account_id: AccountId,
    reward_destination: RewardDestination,
) {
    assert_ok!(DappsStaking::set_reward_destination(
        RuntimeOrigin::signed(account_id),
        reward_destination
    ));

    System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::RewardDestination(
        account_id,
        reward_destination,
    )));

    let ledger = Ledger::<TestRuntime>::get(&account_id);

    assert_eq!(ledger.reward_destination, reward_destination);
}

/// Used to burn stale rewards with success assertions
pub(crate) fn assert_burn_stale_reward(
    contract_id: &MockSmartContract<AccountId>,
    claim_era: EraIndex,
) {
    let developer = DappsStaking::dapp_info(contract_id).unwrap().developer;
    let init_state = MemorySnapshot::all(claim_era, contract_id, developer);
    let issuance_before_claim = <TestRuntime as Config>::Currency::total_issuance();

    assert!(!init_state.contract_info.contract_reward_claimed);

    // Calculate contract portion of the reward
    let (calculated_reward, _) =
        DappsStaking::dev_stakers_split(&init_state.contract_info, &init_state.era_info);

    assert_ok!(DappsStaking::burn_stale_reward(
        RuntimeOrigin::root(),
        contract_id.clone(),
        claim_era,
    ));
    System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::StaleRewardBurned(
        developer,
        contract_id.clone(),
        claim_era,
        calculated_reward,
    )));

    let final_state = MemorySnapshot::all(claim_era, &contract_id, developer);
    let issuance_after_claim = <TestRuntime as Config>::Currency::total_issuance();
    assert_eq!(init_state.free_balance, final_state.free_balance);
    assert!(final_state.contract_info.contract_reward_claimed);
    assert_eq!(
        issuance_before_claim - calculated_reward,
        issuance_after_claim
    );
}
