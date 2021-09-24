use super::{pallet::pallet::Error, Event, *};
use frame_support::{assert_noop, assert_ok, traits::Hooks};
use mock::{Balances, EraIndex, *};
use sp_core::H160;
use sp_runtime::{traits::Zero, Perbill};
use testing_utils::*;

#[test]
fn bond_and_stake_different_eras_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let staker_id = 1;
        let first_stake_value = 100;
        let contract_id = SmartContract::Evm(H160::repeat_byte(0x01));

        let current_era = 50;
        CurrentEra::<TestRuntime>::put(current_era);

        // Insert a contract under registered contracts.
        register_contract(20, &contract_id);

        // initially, storage values should be None
        assert!(!ContractLastClaimed::<TestRuntime>::get(&contract_id).is_some());
        assert!(!ContractLastStaked::<TestRuntime>::get(&contract_id).is_some());
        assert!(!ContractEraStake::<TestRuntime>::get(&contract_id, current_era).is_some());

        ///////////////////////////////////////////////////////////
        ////////////  FIRST BOND AND STAKE
        ///////////////////////////////////////////////////////////
        // Bond and stake on a single contract and ensure it went ok.
        assert_ok!(DappsStaking::bond_and_stake(
            Origin::signed(staker_id),
            contract_id.clone(),
            first_stake_value,
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::BondAndStake(
            staker_id,
            contract_id.clone(),
            first_stake_value,
        )));

        // Verify storage values to see if contract was successfully bonded and staked.
        verify_ledger(staker_id, first_stake_value);
        verify_era_staking_points(
            &contract_id,
            first_stake_value,
            current_era,
            vec![(staker_id, first_stake_value)],
        );
        verify_pallet_era_rewards(current_era, first_stake_value, Zero::zero());

        // Since this was first stake on contract, last claimed should be set to the current era
        assert_eq!(
            current_era,
            ContractLastClaimed::<TestRuntime>::get(&contract_id).unwrap()
        );
        assert_eq!(
            current_era,
            ContractLastStaked::<TestRuntime>::get(&contract_id).unwrap()
        );

        // Prepare new values and advance era.
        let second_stake_value = 300;
        let total_stake_value = first_stake_value + second_stake_value;
        let old_era = current_era;
        let current_era = old_era + 10;
        CurrentEra::<TestRuntime>::put(current_era);

        ///////////////////////////////////////////////////////////
        ////////////  SECOND BOND AND STAKE
        ///////////////////////////////////////////////////////////
        // Stake and bond again on the same contract but using a different amount.
        assert_ok!(DappsStaking::bond_and_stake(
            Origin::signed(staker_id),
            contract_id.clone(),
            second_stake_value,
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::BondAndStake(
            staker_id,
            contract_id.clone(),
            second_stake_value,
        )));

        // Verify that storage values are as expected
        verify_ledger(staker_id, total_stake_value);
        verify_era_staking_points(
            &contract_id,
            total_stake_value,
            current_era,
            vec![(staker_id, total_stake_value)],
        );
        verify_pallet_era_rewards(current_era, total_stake_value, Zero::zero());

        // Contract was staked second time without being claimed, value shouldn't be changed
        assert_eq!(
            old_era,
            ContractLastClaimed::<TestRuntime>::get(contract_id.clone()).unwrap()
        );
        // But the era of last staking should be changed to the current era.
        assert_eq!(
            current_era,
            ContractLastStaked::<TestRuntime>::get(contract_id.clone()).unwrap()
        );
    })
}

#[test]
fn bond_and_stake_two_different_contracts_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let staker_id = 1;
        let first_stake_value = 100;
        let second_stake_value = 300;
        let first_contract_id = SmartContract::Evm(H160::repeat_byte(0x01));
        let second_contract_id = SmartContract::Evm(H160::repeat_byte(0x02));
        let current_era = 50;
        CurrentEra::<TestRuntime>::put(current_era);

        // Insert contracts under registered contracts. Don't use the staker Id.
        register_contract(5, &first_contract_id);
        register_contract(6, &second_contract_id);

        // Stake on both contracts.
        assert_ok!(DappsStaking::bond_and_stake(
            Origin::signed(staker_id),
            first_contract_id.clone(),
            first_stake_value
        ));
        assert_ok!(DappsStaking::bond_and_stake(
            Origin::signed(staker_id),
            second_contract_id.clone(),
            second_stake_value
        ));
        let total_stake_value = first_stake_value + second_stake_value;

        // Verify storage values to see if funds were successfully bonded
        verify_ledger(staker_id, total_stake_value);
        verify_era_staking_points(
            &first_contract_id,
            first_stake_value,
            current_era,
            vec![(staker_id, first_stake_value)],
        );
        verify_era_staking_points(
            &second_contract_id,
            second_stake_value,
            current_era,
            vec![(staker_id, second_stake_value)],
        );
        verify_pallet_era_rewards(current_era, total_stake_value, Zero::zero());
    })
}

#[test]
fn bond_and_stake_two_stakers_one_contract_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let first_staker_id = 1;
        let second_staker_id = 2;
        let first_stake_value = 50;
        let second_stake_value = 235;
        let contract_id = SmartContract::Evm(H160::repeat_byte(0x01));
        let current_era = 25;

        CurrentEra::<TestRuntime>::put(current_era);

        // Insert a contract under registered contracts.
        register_contract(10, &contract_id);

        // Both stakers stake on the same contract, expect a pass.
        assert_ok!(DappsStaking::bond_and_stake(
            Origin::signed(first_staker_id),
            contract_id.clone(),
            first_stake_value
        ));
        assert_ok!(DappsStaking::bond_and_stake(
            Origin::signed(second_staker_id),
            contract_id.clone(),
            second_stake_value
        ));
        let total_stake_value = first_stake_value + second_stake_value;

        // Verify storage values to see if funds were successfully bonded
        verify_ledger(first_staker_id, first_stake_value);
        verify_ledger(second_staker_id, second_stake_value);
        verify_era_staking_points(
            &contract_id,
            total_stake_value,
            current_era,
            vec![
                (first_staker_id, first_stake_value),
                (second_staker_id, second_stake_value),
            ],
        );
        verify_pallet_era_rewards(current_era, total_stake_value, Zero::zero());
    })
}

#[test]
fn bond_and_stake_different_value_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let staker_id = 1;
        let contract_id = SmartContract::Evm(H160::repeat_byte(0x01));

        // Insert a contract under registered contracts.
        register_contract(20, &contract_id);

        // Bond&stake almost the entire available balance of the staker.
        let staker_free_balance = Balances::free_balance(&staker_id);
        assert_ok!(DappsStaking::bond_and_stake(
            Origin::signed(staker_id),
            contract_id.clone(),
            staker_free_balance - 1
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::BondAndStake(
            staker_id,
            contract_id.clone(),
            staker_free_balance - 1,
        )));

        // Bond&stake again with less than existential deposit but this time expect a pass
        // since we're only increasing the already staked amount.
        assert_ok!(DappsStaking::bond_and_stake(
            Origin::signed(staker_id),
            contract_id,
            1
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::BondAndStake(
            staker_id,
            contract_id.clone(),
            1,
        )));

        // Bond&stake more than what's available in funds. Verify that only what's available is bonded&staked.
        let staker_id = 2;
        let staker_free_balance = Balances::free_balance(&staker_id);
        assert_ok!(DappsStaking::bond_and_stake(
            Origin::signed(staker_id),
            contract_id,
            staker_free_balance + 1
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::BondAndStake(
            staker_id,
            contract_id.clone(),
            staker_free_balance,
        )));

        // Bond&stake some amount, a bit less than free balance
        let staker_id = 3;
        let staker_free_balance = Balances::free_balance(&staker_id);
        assert_ok!(DappsStaking::bond_and_stake(
            Origin::signed(staker_id),
            contract_id,
            staker_free_balance - 200
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::BondAndStake(
            staker_id,
            contract_id.clone(),
            staker_free_balance - 200,
        )));
        // Try to bond&stake more than we have available (since we already locked most of the free balance).
        assert_ok!(DappsStaking::bond_and_stake(
            Origin::signed(staker_id),
            contract_id,
            500
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::BondAndStake(
            staker_id,
            contract_id.clone(),
            200,
        )));
    })
}

#[test]
fn bond_and_stake_contract_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let staker_id = 1;
        let stake_value = 100;

        // Check not registered contract. Expect an error.
        let evm_contract = SmartContract::Evm(H160::repeat_byte(0x01));
        assert_noop!(
            DappsStaking::bond_and_stake(Origin::signed(staker_id), evm_contract, stake_value),
            Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn bond_and_stake_insufficient_value() {
    ExternalityBuilder::build().execute_with(|| {
        let staker_id = 1;
        let contract_id = SmartContract::Evm(H160::repeat_byte(0x01));

        // Insert a contract under registered contracts.
        register_contract(20, &contract_id);

        // If user tries to make an initial bond&stake with less than minimum amount, raise an error.
        assert_noop!(
            DappsStaking::bond_and_stake(
                Origin::signed(staker_id),
                contract_id.clone(),
                MINIMUM_STAKING_AMOUNT - 1
            ),
            Error::<TestRuntime>::InsufficientStakingValue
        );

        // Now bond&stake the entire stash so we lock all the available funds.
        let staker_free_balance = Balances::free_balance(&staker_id);
        assert_ok!(DappsStaking::bond_and_stake(
            Origin::signed(staker_id),
            contract_id,
            staker_free_balance
        ));

        // Now try to bond&stake some additional funds and expect an error since we cannot bond&stake 0.
        assert_noop!(
            DappsStaking::bond_and_stake(Origin::signed(staker_id), contract_id.clone(), 1),
            Error::<TestRuntime>::StakingWithNoValue
        );
    })
}

#[test]
fn bond_and_stake_too_many_stakers_per_contract() {
    ExternalityBuilder::build().execute_with(|| {
        let contract_id = SmartContract::Evm(H160::repeat_byte(0x01));
        // Insert a contract under registered contracts.
        register_contract(10, &contract_id);

        // Stake with MAX_NUMBER_OF_STAKERS on the same contract. It must work.
        for staker_id in 1..=MAX_NUMBER_OF_STAKERS {
            assert_ok!(DappsStaking::bond_and_stake(
                Origin::signed(staker_id.into()),
                contract_id.clone(),
                100
            ));
        }

        // Now try to stake with an additional staker and expect an error.
        assert_noop!(
            DappsStaking::bond_and_stake(
                Origin::signed((1 + MAX_NUMBER_OF_STAKERS).into()),
                contract_id.clone(),
                100
            ),
            Error::<TestRuntime>::MaxNumberOfStakersExceeded
        );
    })
}

#[test]
fn unbond_unstake_and_withdraw_multiple_time_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let staker_id = 1;
        let contract_id = SmartContract::Evm(H160::repeat_byte(0x01));
        let original_staked_value = 300 + MINIMUM_STAKING_AMOUNT;
        let old_era = 30;

        CurrentEra::<TestRuntime>::put(old_era);

        // Insert a contract under registered contracts, bond&stake it.
        register_contract(10, &contract_id);
        bond_and_stake_with_verification(staker_id, &contract_id, original_staked_value);
        let new_era = old_era + 10;
        CurrentEra::<TestRuntime>::put(new_era);

        // Unstake such an amount so there will remain staked funds on the contract
        let unstaked_value = 100;
        assert_ok!(DappsStaking::unbond_unstake_and_withdraw(
            Origin::signed(staker_id),
            contract_id.clone(),
            unstaked_value
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::UnbondUnstakeAndWithdraw(
            staker_id,
            contract_id.clone(),
            unstaked_value,
        )));

        let new_staked_value = original_staked_value - unstaked_value;

        // Verify that storage values for the current are as expected.
        verify_ledger(staker_id, new_staked_value);
        verify_era_staking_points(
            &contract_id,
            new_staked_value,
            new_era,
            vec![(staker_id, new_staked_value)],
        );
        verify_pallet_era_rewards(new_era, new_staked_value, Zero::zero());
        assert_eq!(
            new_era,
            ContractLastStaked::<TestRuntime>::get(&contract_id).unwrap()
        );

        // Also verify that the storage values for the old era haven't been changed due to unstaking
        verify_era_staking_points(
            &contract_id,
            original_staked_value,
            old_era,
            vec![(staker_id, original_staked_value)],
        );
        verify_pallet_era_rewards(old_era, original_staked_value, Zero::zero());

        // Unbond yet again, but don't advance era
        // Unstake such an amount so there will remain staked funds on the contract
        let unstaked_value = 50;
        assert_ok!(DappsStaking::unbond_unstake_and_withdraw(
            Origin::signed(staker_id),
            contract_id.clone(),
            unstaked_value
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::UnbondUnstakeAndWithdraw(
            staker_id,
            contract_id.clone(),
            unstaked_value,
        )));

        let new_staked_value = new_staked_value - unstaked_value;

        // Verify that storage values for the current are have been changed as expected.
        verify_ledger(staker_id, new_staked_value);
        verify_era_staking_points(
            &contract_id,
            new_staked_value,
            new_era,
            vec![(staker_id, new_staked_value)],
        );
        verify_pallet_era_rewards(new_era, new_staked_value, Zero::zero());
        assert_eq!(
            new_era,
            ContractLastStaked::<TestRuntime>::get(&contract_id).unwrap()
        );
    })
}

#[test]
fn unbond_unstake_and_withdraw_value_below_staking_threshold() {
    ExternalityBuilder::build().execute_with(|| {
        let staker_id = 1;
        let contract_id = SmartContract::Evm(H160::repeat_byte(0x01));
        let first_value_to_unstake = 300;
        let staked_value = first_value_to_unstake + MINIMUM_STAKING_AMOUNT;

        let current_era = 200;
        CurrentEra::<TestRuntime>::put(current_era);

        // Insert a contract under registered contracts, bond&stake it.
        register_contract(10, &contract_id);
        bond_and_stake_with_verification(staker_id, &contract_id, staked_value);

        // Unstake such an amount that exactly minimum staking amount will remain staked.
        assert_ok!(DappsStaking::unbond_unstake_and_withdraw(
            Origin::signed(staker_id),
            contract_id.clone(),
            first_value_to_unstake
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::UnbondUnstakeAndWithdraw(
            staker_id,
            contract_id.clone(),
            first_value_to_unstake,
        )));

        // Unstake 1 token and expect that the entire staked amount will be unstaked.
        assert_ok!(DappsStaking::unbond_unstake_and_withdraw(
            Origin::signed(staker_id),
            contract_id.clone(),
            1
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::UnbondUnstakeAndWithdraw(
            staker_id,
            contract_id.clone(),
            MINIMUM_STAKING_AMOUNT,
        )));
        assert!(!Ledger::<TestRuntime>::contains_key(staker_id));

        verify_era_staking_points(&contract_id, Zero::zero(), current_era, vec![]);
        verify_pallet_era_rewards(current_era, Zero::zero(), Zero::zero());
    })
}

#[test]
fn unbond_unstake_and_withdraw_in_different_eras() {
    ExternalityBuilder::build().execute_with(|| {
        let first_staker_id = 1;
        let second_staker_id = 2;
        let contract_id = SmartContract::Evm(H160::repeat_byte(0x01));
        let staked_value = 500;

        let current_era = 200;
        CurrentEra::<TestRuntime>::put(current_era);

        // Insert a contract under registered contracts, bond&stake it with two different stakers.
        register_contract(10, &contract_id);
        bond_and_stake_with_verification(first_staker_id, &contract_id, staked_value);
        bond_and_stake_with_verification(second_staker_id, &contract_id, staked_value);
        let total_staked_value = 2 * staked_value;

        // Advance era, unbond&withdraw with first staker, verify that it was successful
        let current_era = current_era + 50;
        CurrentEra::<TestRuntime>::put(current_era);
        let first_unstake_value = 100;
        assert_ok!(DappsStaking::unbond_unstake_and_withdraw(
            Origin::signed(first_staker_id),
            contract_id.clone(),
            first_unstake_value
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::UnbondUnstakeAndWithdraw(
            first_staker_id,
            contract_id.clone(),
            first_unstake_value,
        )));

        // Verify that storage values are as expected for both stakers and total staked value
        let new_total_staked = total_staked_value - first_unstake_value;
        let first_staked_value = staked_value - first_unstake_value;
        verify_era_staking_points(
            &contract_id,
            new_total_staked,
            current_era,
            vec![
                (first_staker_id, first_staked_value),
                (second_staker_id, staked_value),
            ],
        );
        verify_pallet_era_rewards(current_era, new_total_staked, Zero::zero());
        assert_eq!(
            current_era,
            ContractLastStaked::<TestRuntime>::get(&contract_id).unwrap()
        );

        // Advance era, unbond with second staker and verify storage values are as expected
        let current_era = current_era + 50;
        CurrentEra::<TestRuntime>::put(current_era);
        let second_unstake_value = 333;
        assert_ok!(DappsStaking::unbond_unstake_and_withdraw(
            Origin::signed(second_staker_id),
            contract_id.clone(),
            second_unstake_value
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::UnbondUnstakeAndWithdraw(
            second_staker_id,
            contract_id.clone(),
            second_unstake_value,
        )));

        // Verify that storage values are as expected for both stakers and total staked value
        let new_total_staked = new_total_staked - second_unstake_value;
        let second_staked_value = staked_value - second_unstake_value;
        verify_era_staking_points(
            &contract_id,
            new_total_staked,
            current_era,
            vec![
                (first_staker_id, first_staked_value),
                (second_staker_id, second_staked_value),
            ],
        );
        verify_pallet_era_rewards(current_era, new_total_staked, Zero::zero());
        assert_eq!(
            current_era,
            ContractLastStaked::<TestRuntime>::get(&contract_id).unwrap()
        );
    })
}

#[test]
fn unbond_unstake_and_withdraw_contract_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let staker_id = 1;
        let unstake_value = 100;

        // Contract isn't registered, expect an error.
        let evm_contract = SmartContract::Evm(H160::repeat_byte(0x01));
        assert_noop!(
            DappsStaking::bond_and_stake(Origin::signed(staker_id), evm_contract, unstake_value),
            Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn unbond_unstake_and_withdraw_unstake_not_possible() {
    ExternalityBuilder::build().execute_with(|| {
        let first_staker_id = 1;
        let first_contract_id = SmartContract::Evm(H160::repeat_byte(0x01));
        let original_staked_value = 100 + MINIMUM_STAKING_AMOUNT;

        // Insert a contract under registered contracts, bond&stake it.
        register_contract(10, &first_contract_id);

        // Try to unstake with 0, expect an error.
        assert_noop!(
            DappsStaking::unbond_unstake_and_withdraw(
                Origin::signed(first_staker_id),
                first_contract_id.clone(),
                Zero::zero()
            ),
            Error::<TestRuntime>::UnstakingWithNoValue
        );

        // Try to unstake contract which hasn't been staked by anyone
        assert_noop!(
            DappsStaking::unbond_unstake_and_withdraw(
                Origin::signed(first_staker_id),
                first_contract_id.clone(),
                original_staked_value
            ),
            Error::<TestRuntime>::NotStakedContract
        );

        // Now we finally stake the contract
        bond_and_stake_with_verification(
            first_staker_id,
            &first_contract_id,
            original_staked_value,
        );

        // Try to unbond and withdraw using a different staker, one that hasn't staked on this one. Expect an error.
        let second_staker_id = 2;
        assert_noop!(
            DappsStaking::unbond_unstake_and_withdraw(
                Origin::signed(second_staker_id),
                first_contract_id.clone(),
                original_staked_value
            ),
            Error::<TestRuntime>::NotStakedContract
        );

        // Bond a second contract using the second staker. Ensure that second staker still cannot unbond&withdraw funds from the first contract
        let second_contract_id = SmartContract::Evm(H160::repeat_byte(0x02));
        register_contract(20, &second_contract_id);
        bond_and_stake_with_verification(
            second_staker_id,
            &second_contract_id,
            original_staked_value,
        );
        assert_noop!(
            DappsStaking::unbond_unstake_and_withdraw(
                Origin::signed(second_staker_id),
                first_contract_id.clone(),
                original_staked_value
            ),
            Error::<TestRuntime>::NotStakedContract
        );
    })
}

#[test]
fn register_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let developer = 1;
        let ok_contract = SmartContract::Evm(H160::repeat_byte(0x01));

        register_contract(developer, &ok_contract);
        System::assert_last_event(mock::Event::DappsStaking(Event::NewContract(
            developer,
            ok_contract,
        )));
    })
}

#[test]
fn register_twice_with_same_account_nok() {
    ExternalityBuilder::build().execute_with(|| {
        let developer = 1;
        let contract1 = SmartContract::Evm(H160::repeat_byte(0x01));
        let contract2 = SmartContract::Evm(H160::repeat_byte(0x02));

        register_contract(developer, &contract1);

        System::assert_last_event(mock::Event::DappsStaking(Event::NewContract(
            developer, contract1,
        )));

        // now register different contract with same account
        assert_noop!(
            DappsStaking::register(Origin::signed(developer), contract2),
            Error::<TestRuntime>::AlreadyUsedDeveloperAccount
        );
    })
}

#[test]
fn register_same_contract_twice_nok() {
    ExternalityBuilder::build().execute_with(|| {
        let developer1 = 1;
        let developer2 = 2;
        let contract = SmartContract::Evm(H160::repeat_byte(0x01));

        register_contract(developer1, &contract);

        System::assert_last_event(mock::Event::DappsStaking(Event::NewContract(
            developer1, contract,
        )));

        // now register same contract by different developer
        assert_noop!(
            DappsStaking::register(Origin::signed(developer2), contract),
            Error::<TestRuntime>::AlreadyRegisteredContract
        );
        assert_eq!(mock::DappsStaking::contract_last_claimed(contract), None);
        assert_eq!(mock::DappsStaking::contract_last_staked(contract), None);
    })
}

#[test]
fn new_era_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // set initial era index
        let starting_era = 42;
        <CurrentEra<TestRuntime>>::put(starting_era);

        // Increment block by setting it to the first block in era value
        let initial_block_number = BLOCKS_PER_ERA * 42 + 1;
        Pallet::<TestRuntime>::on_initialize(initial_block_number);
        let current = mock::DappsStaking::current_era();
        assert_eq!(starting_era, current);

        // verify that block reward is added to the block_reward_accumulator
        let block_reward = mock::DappsStaking::block_reward_accumulator();
        assert_eq!(BLOCK_REWARD, block_reward);

        // register and bond to verify storage item
        let staker = 2;
        let developer = 3;
        const STAKED_AMOUNT: Balance = 100;
        let contract = SmartContract::Evm(H160::repeat_byte(0x01));
        register_contract(developer, &contract);
        bond_and_stake_with_verification(staker, &contract, STAKED_AMOUNT);

        // increment (BLOCKS_PER_ERA - 1) more blocks to end the era
        // CurrentEra should be incremented
        // block_reward_accumulator should be reset to 0
        for block_increment in 1..=(BLOCKS_PER_ERA - 1) {
            Pallet::<TestRuntime>::on_initialize(initial_block_number + block_increment);
        }
        let current = mock::DappsStaking::current_era();
        assert_eq!(starting_era + 1, current);
        System::assert_last_event(mock::Event::DappsStaking(Event::NewDappStakingEra(
            starting_era + 1,
        )));

        // verify that block reward accumulator is reset to 0
        let block_reward = mock::DappsStaking::block_reward_accumulator();
        assert!(block_reward.is_zero());

        let expected_era_reward = Perbill::from_percent(DAPPS_REWARD_PERCENTAGE)
            * BLOCK_REWARD
            * BLOCKS_PER_ERA as Balance;
        // verify that .staked is copied and .reward is added
        verify_pallet_era_rewards(starting_era, STAKED_AMOUNT, expected_era_reward);
    })
}

#[test]
fn new_era_forcing() {
    ExternalityBuilder::build().execute_with(|| {
        let block_number = BlockPerEra::get() / 2;
        let starting_era = 3;

        // set initial era index
        <CurrentEra<TestRuntime>>::put(starting_era);

        // call on_initilize. It is not last block in the era, but it should increment the era
        <ForceEra<TestRuntime>>::put(Forcing::ForceNew);
        Pallet::<TestRuntime>::on_initialize(block_number);

        // check that era is incremented
        let current = mock::DappsStaking::current_era();
        assert_eq!(starting_era + 1, current);

        // check that forcing is cleared
        assert_eq!(mock::DappsStaking::force_era(), Forcing::ForceNone);

        // check the event for the new era
        System::assert_last_event(mock::Event::DappsStaking(Event::NewDappStakingEra(
            starting_era + 1,
        )));
    })
}

#[test]
fn claim_contract_not_registered() {
    ExternalityBuilder::build().execute_with(|| {
        let claimer = 2;
        let contract = SmartContract::Evm(H160::repeat_byte(0x01));

        assert_noop!(
            DappsStaking::claim(Origin::signed(claimer), contract),
            Error::<TestRuntime>::ContractNotRegistered
        );
    })
}

#[test]
fn claim_nothing_to_claim() {
    ExternalityBuilder::build().execute_with(|| {
        let developer1 = 1;
        let claimer = 2;
        const ERA_REWARD: Balance = 1000;
        let contract = SmartContract::Evm(H160::repeat_byte(0x01));
        const START_ERA: EraIndex = 1;

        advance_era_and_reward(START_ERA, ERA_REWARD, 0);
        DappsStaking::current_era();
        register_contract(developer1, &contract);

        assert_noop!(
            DappsStaking::claim(Origin::signed(claimer), contract),
            Error::<TestRuntime>::NothingToClaim
        );
    })
}

#[test]
fn claim_twice_in_same_era() {
    ExternalityBuilder::build().execute_with(|| {
        let developer = 1;
        let claimer = 2;
        const ERA_REWARD: Balance = 1000;
        const STAKE_AMOUNT: Balance = 100;
        let contract = SmartContract::Evm(H160::repeat_byte(0x01));
        const START_ERA: EraIndex = 1;
        const SKIP_ERA: EraIndex = 3;

        advance_era_and_reward(START_ERA, ERA_REWARD, 0);
        let start_era = DappsStaking::current_era();

        register_contract(developer, &contract);
        bond_and_stake_with_verification(claimer, &contract, STAKE_AMOUNT);
        advance_era_and_reward(SKIP_ERA, ERA_REWARD, 0);

        let claim_era: EraIndex = DappsStaking::current_era();
        claim(claimer, contract, start_era, claim_era.clone());

        assert_noop!(
            DappsStaking::claim(Origin::signed(claimer), contract),
            Error::<TestRuntime>::AlreadyClaimedInThisEra
        );
    })
}

#[test]
fn claim_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let developer = 1;
        let claimer = 2;
        const ERA_REWARD: Balance = 1000;
        const STAKE_AMOUNT: Balance = 100;
        let contract = SmartContract::Evm(H160::repeat_byte(0x01));
        const START_ERA: EraIndex = 1;
        const SKIP_ERA: EraIndex = 3;

        advance_era_and_reward(START_ERA, ERA_REWARD, 0);
        let start_era = DappsStaking::current_era();
        register_contract(developer, &contract);
        bond_and_stake_with_verification(claimer, &contract, STAKE_AMOUNT);
        advance_era_and_reward(SKIP_ERA, ERA_REWARD, 0);
        let claim_era: EraIndex = DappsStaking::current_era();

        claim(claimer, contract, start_era, claim_era.clone());
        verify_contract_history_is_cleared(contract, START_ERA, claim_era);
    })
}

#[test]
fn claim_one_contract() {
    ExternalityBuilder::build().execute_with(|| {
        let developer = 1;
        let staker1: mock::AccountId = 2;
        let staker2: mock::AccountId = 3;
        const ERA_REWARD: mock::Balance = 10000;
        const STAKE_AMOUNT1: mock::Balance = 400;
        const STAKE_AMOUNT2: mock::Balance = 600;
        const INITIAL_STAKE: mock::Balance = 1000;
        let contract = SmartContract::Evm(H160::repeat_byte(0x01));
        const START_ERA: EraIndex = 1;
        const SKIP_ERA: EraIndex = 2;

        let free_balance_staker1 = <mock::TestRuntime as Config>::Currency::free_balance(&staker1);
        let free_balance_staker2 = <mock::TestRuntime as Config>::Currency::free_balance(&staker2);
        let free_developer_balance =
            <mock::TestRuntime as Config>::Currency::free_balance(&developer);

        advance_era_and_reward(START_ERA, ERA_REWARD, INITIAL_STAKE);
        let start_era = DappsStaking::current_era();
        register_contract(developer, &contract);
        bond_and_stake_with_verification(staker1, &contract, STAKE_AMOUNT1);
        bond_and_stake_with_verification(staker2, &contract, STAKE_AMOUNT2);
        advance_era_and_reward(SKIP_ERA, ERA_REWARD, INITIAL_STAKE);
        let claim_era: EraIndex = DappsStaking::current_era();
        claim(staker1, contract, start_era, claim_era.clone());
        verify_contract_history_is_cleared(contract, START_ERA, claim_era);
        let num_eras: u128 = SKIP_ERA as u128; // number of rewarded eras

        // calculate reward per stakers
        let expected_staker1_reward =
            calc_expected_staker_reward(ERA_REWARD, INITIAL_STAKE, INITIAL_STAKE, STAKE_AMOUNT1);
        let expected_staker2_reward =
            calc_expected_staker_reward(ERA_REWARD, INITIAL_STAKE, INITIAL_STAKE, STAKE_AMOUNT2);
        // calculate reward per developer
        let expected_developer_reward =
            calc_expected_developer_reward(ERA_REWARD, INITIAL_STAKE, INITIAL_STAKE);

        assert_eq!(
            <mock::TestRuntime as Config>::Currency::free_balance(&staker1),
            free_balance_staker1 + num_eras * expected_staker1_reward
        );
        assert_eq!(
            <mock::TestRuntime as Config>::Currency::free_balance(&staker2),
            free_balance_staker2 + num_eras * expected_staker2_reward
        );
        assert_eq!(
            <mock::TestRuntime as Config>::Currency::free_balance(&developer),
            free_developer_balance + num_eras * expected_developer_reward as u128
        );
    })
}

#[test]
fn claim_two_contracts() {
    ExternalityBuilder::build().execute_with(|| {
        let developer1 = 1;
        let developer2 = 10;
        let staker1: mock::AccountId = 2;
        let staker2: mock::AccountId = 3; // will stake on 2 contracts
        let staker3: mock::AccountId = 4;
        const ERA_REWARD: mock::Balance = 100;
        const STAKER1_AMOUNT: mock::Balance = 400;
        const STAKER2_AMOUNT1: mock::Balance = 600;
        const STAKER2_AMOUNT2: mock::Balance = 100;
        const STAKER3_AMOUNT: mock::Balance = 400;
        const CONTRACT1_STAKE: mock::Balance = 1000;
        const CONTRACT2_STAKE: mock::Balance = 500;
        const ERA_STAKED1: mock::Balance = 1000; // 400 + 600
        const ERA_STAKED2: mock::Balance = 1500; // 1000 + 100 + 400
        let contract1 = SmartContract::Evm(H160::repeat_byte(0x01));
        let contract2 = SmartContract::Evm(H160::repeat_byte(0x02));
        const START_ERA: EraIndex = 1;
        const SKIP_ERA: EraIndex = 3;

        // Store initial free balaces of developers and stakers
        let free_balance_staker1 = <mock::TestRuntime as Config>::Currency::free_balance(&staker1);
        let free_balance_staker2 = <mock::TestRuntime as Config>::Currency::free_balance(&staker2);
        let free_balance_staker3 = <mock::TestRuntime as Config>::Currency::free_balance(&staker3);
        let free_balance_developer1 =
            <mock::TestRuntime as Config>::Currency::free_balance(&developer1);
        let free_balance_developer2 =
            <mock::TestRuntime as Config>::Currency::free_balance(&developer2);

        advance_era_and_reward(START_ERA, ERA_REWARD, ERA_STAKED1);
        let start_era = DappsStaking::current_era();

        // Register contracts, bond&stake them with two stakers on first contract.
        register_contract(developer1, &contract1);
        register_contract(developer2, &contract2);
        bond_and_stake_with_verification(staker1, &contract1, STAKER1_AMOUNT);
        bond_and_stake_with_verification(staker2, &contract1, STAKER2_AMOUNT1);

        // Advance eras and then bond&stake with two stakers on second contract.
        advance_era_and_reward(SKIP_ERA, ERA_REWARD, ERA_STAKED1);
        bond_and_stake_with_verification(staker2, &contract2, STAKER2_AMOUNT2);
        bond_and_stake_with_verification(staker3, &contract2, STAKER3_AMOUNT);

        // Advance era again, by one so rewards can be claimed for previous eras.
        advance_era_and_reward(1, ERA_REWARD, ERA_STAKED2);

        // Claim rewards for first contract and verify storage content is as expected.
        let claim_era: EraIndex = DappsStaking::current_era();
        claim(staker1, contract1.clone(), start_era, claim_era.clone());
        verify_contract_history_is_cleared(contract1, START_ERA, claim_era);

        // calculate reward per stakers in contract1
        let expected_c1_staker1_e1_reward =
            calc_expected_staker_reward(ERA_REWARD, ERA_STAKED1, CONTRACT1_STAKE, STAKER1_AMOUNT);
        let expected_c1_staker1_e2_reward =
            calc_expected_staker_reward(ERA_REWARD, ERA_STAKED2, CONTRACT1_STAKE, STAKER1_AMOUNT);
        let expected_c1_staker2_e1_reward =
            calc_expected_staker_reward(ERA_REWARD, ERA_STAKED1, CONTRACT1_STAKE, STAKER2_AMOUNT1);
        let expected_c1_staker2_e2_reward =
            calc_expected_staker_reward(ERA_REWARD, ERA_STAKED2, CONTRACT1_STAKE, STAKER2_AMOUNT1);
        // calculate reward per developer contract 1
        let expected_c1_dev1_e1_reward =
            calc_expected_developer_reward(ERA_REWARD, ERA_STAKED1, CONTRACT1_STAKE);
        let expected_c1_dev1_e2_reward =
            calc_expected_developer_reward(ERA_REWARD, ERA_STAKED2, CONTRACT1_STAKE);

        let num_eras1 = 3; // number of rewarded eras
        assert_eq!(
            <mock::TestRuntime as Config>::Currency::free_balance(&staker1),
            free_balance_staker1
                + num_eras1 * expected_c1_staker1_e1_reward
                + 1 * expected_c1_staker1_e2_reward
        );

        // staker2 staked on both contracts. remember reward for contract2
        let expected_c1_staker2_reward =
            num_eras1 * expected_c1_staker2_e1_reward + 1 * expected_c1_staker2_e2_reward;
        assert_eq!(
            <mock::TestRuntime as Config>::Currency::free_balance(&staker2),
            free_balance_staker2 + expected_c1_staker2_reward
        );
        assert_eq!(
            <mock::TestRuntime as Config>::Currency::free_balance(&developer1),
            free_balance_developer1
                + num_eras1 * expected_c1_dev1_e1_reward as u128
                + expected_c1_dev1_e2_reward as u128
        );

        // claim rewards for contract2 one 4 eras later
        let num_eras2 = 5; // 1 era already passed since staking + another 4 eras
        advance_era_and_reward(4, ERA_REWARD, ERA_STAKED2);
        let claim_era: EraIndex = DappsStaking::current_era();
        claim(staker2, contract2.clone(), 4, claim_era.clone());

        // calculate reward per stakers in contract2
        let expected_c2_staker2_e2_reward =
            calc_expected_staker_reward(ERA_REWARD, ERA_STAKED2, CONTRACT2_STAKE, STAKER2_AMOUNT2);
        let expected_c2_staker3_e2_reward =
            calc_expected_staker_reward(ERA_REWARD, ERA_STAKED2, CONTRACT2_STAKE, STAKER3_AMOUNT);
        // calculate reward per developer
        let expected_c2_dev2_e2_reward =
            calc_expected_developer_reward(ERA_REWARD, ERA_STAKED2, CONTRACT2_STAKE);

        assert_eq!(
            <mock::TestRuntime as Config>::Currency::free_balance(&staker2)
                - expected_c1_staker2_reward,
            free_balance_staker2 + num_eras2 * expected_c2_staker2_e2_reward
        );
        assert_eq!(
            <mock::TestRuntime as Config>::Currency::free_balance(&staker3),
            free_balance_staker3 + num_eras2 * expected_c2_staker3_e2_reward
        );
        assert_eq!(
            <mock::TestRuntime as Config>::Currency::free_balance(&developer2),
            free_balance_developer2 + num_eras2 * expected_c2_dev2_e2_reward as u128
        );
    })
}
