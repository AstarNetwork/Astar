use crate::mock::EraIndex;

use super::{Event, *};
use frame_support::{assert_err, assert_noop, assert_ok, assert_storage_noop, traits::Hooks};
use mock::{Balances, *};
use sp_core::H160;
use sp_runtime::Perbill;
use sp_std::convert::{From, TryInto};
use std::str::FromStr;
use testing_utils::*;

#[test]
fn bond_and_stake_different_eras_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let staker_id = 1;
        let first_stake_value = 100;
        let contract_id =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
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
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::BondAndStake(
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
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::BondAndStake(
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
        let first_contract_id =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
        let second_contract_id =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000008").unwrap());
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
        let contract_id =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
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
        let contract_id =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());

        // Insert a contract under registered contracts.
        register_contract(20, &contract_id);

        // Bond&stake almost the entire available balance of the staker.
        let staker_free_balance = Balances::free_balance(&staker_id);
        assert_ok!(DappsStaking::bond_and_stake(
            Origin::signed(staker_id),
            contract_id.clone(),
            staker_free_balance - 1
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::BondAndStake(
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
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::BondAndStake(
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
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::BondAndStake(
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
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::BondAndStake(
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
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::BondAndStake(
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

        // Wasm contracts aren't supported yet. // TODO: Why do we even have them in enum then?
        let wasm_contract = SmartContract::Wasm(10);
        assert_noop!(
            DappsStaking::bond_and_stake(Origin::signed(staker_id), wasm_contract, stake_value),
            crate::pallet::pallet::Error::<TestRuntime>::ContractIsNotValid
        );

        // Check a supported bot not registered contract. Expect an error.
        let evm_contract = SmartContract::<AccountId>::Evm(
            H160::from_str("1000000000000000000000000000000000000007").unwrap(),
        );
        assert_noop!(
            DappsStaking::bond_and_stake(Origin::signed(staker_id), evm_contract, stake_value),
            crate::pallet::pallet::Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn bond_and_stake_insufficient_value() {
    ExternalityBuilder::build().execute_with(|| {
        let staker_id = 1;
        let contract_id =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());

        // Insert a contract under registered contracts.
        register_contract(20, &contract_id);

        // If user tries to make an initial bond&stake with less than minimum amount, raise an error.
        assert_noop!(
            DappsStaking::bond_and_stake(
                Origin::signed(staker_id),
                contract_id.clone(),
                MINIMUM_STAKING_AMOUNT - 1
            ),
            crate::pallet::pallet::Error::<TestRuntime>::InsufficientStakingValue
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
            crate::pallet::pallet::Error::<TestRuntime>::StakingWithNoValue
        );
    })
}

#[test]
fn bond_and_stake_too_many_stakers_per_contract() {
    ExternalityBuilder::build().execute_with(|| {
        let contract_id =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
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
            DappsStaking::bond_and_stake(Origin::signed(5), contract_id.clone(), 100),
            crate::pallet::pallet::Error::<TestRuntime>::MaxNumberOfStakersExceeded
        );
    })
}

#[test]
fn unbond_unstake_and_withdraw_multiple_time_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let staker_id = 1;
        let contract_id =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
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
        System::assert_last_event(mock::Event::DappsStaking(
            crate::Event::UnbondUnstakeAndWithdraw(staker_id, contract_id.clone(), unstaked_value),
        ));

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
        System::assert_last_event(mock::Event::DappsStaking(
            crate::Event::UnbondUnstakeAndWithdraw(staker_id, contract_id.clone(), unstaked_value),
        ));

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
        let contract_id =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
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
        System::assert_last_event(mock::Event::DappsStaking(
            crate::Event::UnbondUnstakeAndWithdraw(
                staker_id,
                contract_id.clone(),
                first_value_to_unstake,
            ),
        ));

        // Unstake 1 token and expect that the entire staked amount will be unstaked.
        assert_ok!(DappsStaking::unbond_unstake_and_withdraw(
            Origin::signed(staker_id),
            contract_id.clone(),
            1
        ));
        System::assert_last_event(mock::Event::DappsStaking(
            crate::Event::UnbondUnstakeAndWithdraw(
                staker_id,
                contract_id.clone(),
                MINIMUM_STAKING_AMOUNT,
            ),
        ));
        assert!(!Ledger::<TestRuntime>::contains_key(staker_id));
        // TODO: Should I also delete such empty structs from storage? THey will get deleted eventually but why not do it beforehand?
        verify_era_staking_points(&contract_id, Zero::zero(), current_era, vec![]);
        verify_pallet_era_rewards(current_era, Zero::zero(), Zero::zero());
    })
}

#[test]
fn unbond_unstake_and_withdraw_in_different_eras() {
    ExternalityBuilder::build().execute_with(|| {
        let first_staker_id = 1;
        let second_staker_id = 2;
        let contract_id =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
        let staked_value = 500;

        let current_era = 200;
        CurrentEra::<TestRuntime>::put(current_era);

        // Insert a contract under registered contracts, bond&stake it.
        register_contract(10, &contract_id);
        bond_and_stake_with_verification(first_staker_id, &contract_id, staked_value);
        bond_and_stake_with_verification(second_staker_id, &contract_id, staked_value);
        let total_staked_value = 2 * staked_value;

        // Advance era, unbond&withdraw, verify that it was successful
        let current_era = current_era + 50;
        CurrentEra::<TestRuntime>::put(current_era);
        let first_unstake_value = 100;
        assert_ok!(DappsStaking::unbond_unstake_and_withdraw(
            Origin::signed(first_staker_id),
            contract_id.clone(),
            first_unstake_value
        ));
        System::assert_last_event(mock::Event::DappsStaking(
            crate::Event::UnbondUnstakeAndWithdraw(
                first_staker_id,
                contract_id.clone(),
                first_unstake_value,
            ),
        ));

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
        System::assert_last_event(mock::Event::DappsStaking(
            crate::Event::UnbondUnstakeAndWithdraw(
                second_staker_id,
                contract_id.clone(),
                second_unstake_value,
            ),
        ));

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

        // Wasm contracts aren't supported yet.
        let wasm_contract = SmartContract::Wasm(10);
        assert_noop!(
            DappsStaking::unbond_unstake_and_withdraw(
                Origin::signed(staker_id),
                wasm_contract,
                unstake_value
            ),
            crate::pallet::pallet::Error::<TestRuntime>::ContractIsNotValid
        );

        // Check a supported bot not registered contract. Expect an error.
        let evm_contract = SmartContract::<AccountId>::Evm(
            H160::from_str("1000000000000000000000000000000000000007").unwrap(),
        );
        assert_noop!(
            DappsStaking::bond_and_stake(Origin::signed(staker_id), evm_contract, unstake_value),
            crate::pallet::pallet::Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn unbond_unstake_and_withdraw_unstake_not_possible() {
    ExternalityBuilder::build().execute_with(|| {
        let first_staker_id = 1;
        let first_contract_id =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
        let original_staked_value = 100 + MINIMUM_STAKING_AMOUNT;

        // Insert a contract under registered contracts, bond&stake it.
        register_contract(10, &first_contract_id);

        // Try to unstake with 0, expect an error
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

        // Try to unbond and withdraw using a different staker, one that hasn't staked on this one.
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
        let second_contract_id =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000077").unwrap());
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
fn bonding_existential_deposit_amount_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // test that bonding works with amount that is equal to existential deposit
        let stash2_id = 2;
        let stash2_signed_id = Origin::signed(stash2_id);
        let controller2_id = 4u64;
        let staking2_amount = EXISTENTIAL_DEPOSIT;
        assert_ok!(DappsStaking::bond(
            stash2_signed_id,
            controller2_id,
            staking2_amount,
            crate::RewardDestination::Stash
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash2_id,
            staking2_amount,
        )));
    })
}

#[test]
fn bonding_entire_stash_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // test that bonding works with the amount that equals the entire stash
        let stash3_id = 540;
        let stash3_signed_id = Origin::signed(stash3_id);
        let controller3_id = 6u64;
        let stash3_free_amount = Balances::free_balance(&stash3_id);
        assert_ok!(DappsStaking::bond(
            stash3_signed_id,
            controller3_id,
            stash3_free_amount,
            crate::RewardDestination::Stash
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash3_id,
            stash3_free_amount,
        )));
    })
}

#[test]
fn bonding_more_than_in_stash_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // test that bonding works when more is staked than available in stash
        let stash4_id = 3;
        let controller4_id = 8u64;
        let stash4_free_amount = Balances::free_balance(&stash4_id);
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash4_id),
            controller4_id,
            stash4_free_amount + 1,
            crate::RewardDestination::Stash
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash4_id,
            stash4_free_amount,
        )));
    })
}

#[test]
fn bonding_less_than_exist_deposit_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // try to stake less than minimum amount, expect error InsufficientValue
        let stash2_id = 2;
        let controller2_id = 20u64;
        assert_noop!(
            DappsStaking::bond(
                Origin::signed(2),
                controller2_id,
                EXISTENTIAL_DEPOSIT - 1,
                crate::RewardDestination::Staked
            ),
            crate::pallet::pallet::Error::<TestRuntime>::InsufficientBondValue
        );
    })
}

#[test]
fn bonding_with_same_stash_or_controller_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash_id = 1;
        let controller_id = 3u64;

        assert_ok!(DappsStaking::bond(
            Origin::signed(stash_id),
            controller_id,
            50,
            crate::RewardDestination::Staked
        ));

        // repeat bonding with same stash account, expect error AlreadyBonded
        assert_noop!(
            DappsStaking::bond(
                Origin::signed(stash_id),
                controller_id,
                50,
                crate::RewardDestination::Staked
            ),
            crate::pallet::pallet::Error::<TestRuntime>::AlreadyBonded
        );

        // use already paired controller with a new stash, expect error AlreadyPaired
        let stash2_id = 2;
        assert_noop!(
            DappsStaking::bond(
                Origin::signed(stash2_id),
                controller_id,
                50,
                crate::RewardDestination::Staked
            ),
            crate::pallet::pallet::Error::<TestRuntime>::AlreadyPaired
        );
    })
}

#[test]
fn bonding_extra_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id: u64 = 1;
        let controller1_id = 20u64;
        let stash1_amount = Balances::free_balance(&stash1_id);

        assert_ok!(DappsStaking::bond(
            Origin::signed(stash1_id),
            controller1_id,
            stash1_amount - 1000,
            crate::RewardDestination::Staked
        ));

        // bond extra funds and expect a pass
        let first_extra_amount: mock::Balance = 900;
        assert_ok!(DappsStaking::bond_extra(
            Origin::signed(stash1_id),
            first_extra_amount
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash1_id,
            first_extra_amount,
        )));

        // bond remaining funds and expect a pass
        let second_extra_amount: mock::Balance = 100;
        assert_ok!(DappsStaking::bond_extra(
            Origin::signed(stash1_id),
            second_extra_amount
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash1_id,
            second_extra_amount,
        )));

        // TODO: if we bond additional funds, it will 'pass' but nothing will happen, no events will be deposited.
        // Is that correct??? Do we need a new error for this?
        // let third_extra_amount: mock::Balance = 10;
        // assert_noop!(
        //     DappsStaking::bond_extra(stash1_id_signed,
        //     third_extra_amount),
        //     <some error???>
        // );
    })
}

#[test]
fn bonding_extra_with_controller_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id = 1;
        let controller1_id = 20u64;

        assert_noop!(
            DappsStaking::bond_extra(Origin::signed(stash1_id), 10),
            crate::pallet::pallet::Error::<TestRuntime>::NotStash
        );
    })
}

#[test]
fn set_controller_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id = 1;
        let controller1_id = 20u64;

        // add stash and controller by bonding
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash1_id),
            controller1_id,
            50,
            crate::RewardDestination::Staked
        ));
        // set a new controller, different from the old one
        let new_controller1_id = 30u64;
        assert_ok!(DappsStaking::set_controller(
            Origin::signed(stash1_id),
            new_controller1_id
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::ControllerChanged(
            stash1_id,
            new_controller1_id,
        )));
    })
}

#[test]
fn set_controller_for_non_existing_stash_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id = 1;
        let controller1_id = 20u64;

        // stash doesn't exist yet, expect error NotStash
        assert_noop!(
            DappsStaking::set_controller(Origin::signed(stash1_id), controller1_id),
            crate::pallet::pallet::Error::<TestRuntime>::NotStash
        );
    })
}

#[test]
fn set_controller_twice_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id = 1;
        let controller1_id = 20u64;

        // add stash and controller by bonding
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash1_id),
            controller1_id,
            50,
            crate::RewardDestination::Staked
        ));

        // try to set the old controller, expect error AlreadyPaired
        assert_noop!(
            DappsStaking::set_controller(Origin::signed(stash1_id), controller1_id),
            crate::pallet::pallet::Error::<TestRuntime>::AlreadyPaired
        );
    })
}

#[test]
fn unbond_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // prepare stash-controller pair with some bonded funds
        let stash_id = 1;
        let controller_id = 100;
        let bond_amount = 50 + EXISTENTIAL_DEPOSIT;
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash_id),
            controller_id,
            bond_amount,
            crate::RewardDestination::Staked
        ));

        // unbond a valid amout
        assert_ok!(DappsStaking::unbond(Origin::signed(controller_id), 50));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Unbonded(
            stash_id, 50,
        )));

        // unbond 1 value and expect to unbond everything remaining since we come under the existintial limit
        assert_ok!(DappsStaking::unbond(Origin::signed(controller_id), 1));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Unbonded(
            stash_id,
            EXISTENTIAL_DEPOSIT,
        )));

        // at this point there's nothing more to unbond but we can still call unbond
        assert_ok!(DappsStaking::unbond(Origin::signed(controller_id), 1));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Unbonded(
            stash_id,
            Zero::zero(),
        )));
    })
}

#[test]
fn unbond_with_non_existing_controller_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // try to unbond using non-existing controller, expect error NotController
        let controller_id = 10;
        assert_noop!(
            DappsStaking::unbond(Origin::signed(controller_id), 100),
            crate::pallet::pallet::Error::<TestRuntime>::NotController
        );
    })
}

#[test]
fn unbond_with_stash_id_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash_id = 10;
        let controller_id = 100u64;
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash_id),
            controller_id,
            100,
            crate::RewardDestination::Staked
        ));

        // try to unbond using stash id, expect error NotController
        assert_noop!(
            DappsStaking::unbond(Origin::signed(stash_id), 100),
            crate::pallet::pallet::Error::<TestRuntime>::NotController
        );
    })
}

#[test]
fn unbond_too_many_chunks_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash_id = 10;
        let controller_id = 100u64;
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash_id),
            controller_id,
            100,
            crate::RewardDestination::Staked
        ));

        // remove values up to MAX_UNLOCKING_CHUNKS and expect everything to work
        for chunk in 1..=MAX_UNLOCKING_CHUNKS {
            assert_ok!(DappsStaking::unbond(Origin::signed(controller_id), 1));
        }
        assert_noop!(
            DappsStaking::unbond(Origin::signed(controller_id), 1),
            crate::pallet::pallet::Error::<TestRuntime>::NoMoreChunks
        );
    })
}

#[test]
fn withdraw_unbonded_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash_id = 1;
        let controller_id = 10;
        let bond_amount: Balance = 100;

        // create a bond
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash_id),
            controller_id,
            bond_amount,
            crate::RewardDestination::Staked
        ));

        // unbond some amount, the remainder bond should remain above existential deposit. Repeat twice to get two chunks.
        let first_unbond_amount = (bond_amount - 2 * EXISTENTIAL_DEPOSIT) / 2;
        for _ in 1..=2 {
            assert_ok!(DappsStaking::unbond(
                Origin::signed(controller_id),
                first_unbond_amount
            ));
        }

        // verify that withdraw works even if no chunks are available (era has not advanced enough)
        let current_era = <CurrentEra<TestRuntime>>::get().unwrap_or(Zero::zero());
        <CurrentEra<TestRuntime>>::put(current_era + UNBONDING_DURATION - 1);
        assert_storage_noop!(DappsStaking::withdraw_unbonded(Origin::signed(
            controller_id
        )));
        // no withdraw event should have happened, the old unbond event should still be the last
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Unbonded(
            stash_id,
            first_unbond_amount,
        )));

        // advance the era by 1 so we satisfy the bonding duration for chunks
        let current_era = <CurrentEra<TestRuntime>>::get().unwrap();
        <CurrentEra<TestRuntime>>::put(current_era + 1);

        // verify that we withdraw both chunks that were unbonded
        assert_ok!(DappsStaking::withdraw_unbonded(Origin::signed(
            controller_id
        )));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Withdrawn(
            stash_id,
            2 * first_unbond_amount,
        )));

        // At this point, we have bonded 2 * EXISTENTIAL_DEPOSIT
        // Unbond just enough to go below existential deposit and verify that entire bond is released
        assert_ok!(DappsStaking::unbond(
            Origin::signed(controller_id),
            EXISTENTIAL_DEPOSIT + 1
        ));
        let current_era = <CurrentEra<TestRuntime>>::get().unwrap_or(Zero::zero());
        <CurrentEra<TestRuntime>>::put(current_era + UNBONDING_DURATION + 1);
        assert_ok!(DappsStaking::withdraw_unbonded(Origin::signed(
            controller_id
        )));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Withdrawn(
            stash_id,
            2 * EXISTENTIAL_DEPOSIT,
        )));
    })
}

#[test]
fn withdraw_unbonded_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let controller_id = 10;

        assert_noop!(
            DappsStaking::withdraw_unbonded(Origin::signed(controller_id)),
            crate::pallet::pallet::Error::<TestRuntime>::NotController
        );
    })
}

#[test]
fn register_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // prepare stash-controller pair with some bonded funds
        let developer = 1;
        let ok_contract =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());

        register_contract(developer, &ok_contract);
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::NewContract(
            developer,
            ok_contract,
        )));
    })
}

#[test]
fn register_twice_same_account_nok() {
    ExternalityBuilder::build().execute_with(|| {
        // prepare stash-controller pair with some bonded funds
        let developer = 1;
        let contract1 =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
        let contract2 =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000008").unwrap());

        register_contract(developer, &contract1);

        System::assert_last_event(mock::Event::DappsStaking(crate::Event::NewContract(
            developer, contract1,
        )));

        // now register different contract with same account
        assert_noop!(
            DappsStaking::register(Origin::signed(developer), contract2),
            crate::pallet::pallet::Error::<TestRuntime>::AlreadyUsedDeveloperAccount
        );
    })
}

#[test]
fn register_same_contract_twice_nok() {
    ExternalityBuilder::build().execute_with(|| {
        // prepare stash-controller pair with some bonded funds
        let developer1 = 1;
        let developer2 = 2;
        let contract =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());

        register_contract(developer1, &contract);

        System::assert_last_event(mock::Event::DappsStaking(crate::Event::NewContract(
            developer1, contract,
        )));

        // now register same contract by different developer
        assert_noop!(
            DappsStaking::register(Origin::signed(developer2), contract),
            crate::pallet::pallet::Error::<TestRuntime>::AlreadyRegisteredContract
        );
        assert_eq!(mock::DappsStaking::contract_last_claimed(contract), None);
        assert_eq!(mock::DappsStaking::contract_last_staked(contract), None);
    })
}

#[test]
fn new_era_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let block_number = BlockPerEra::get() - 1;
        const CURRENT_ERA: crate::EraIndex = 3;

        // set initial era index
        <CurrentEra<TestRuntime>>::put(CURRENT_ERA);

        // increment the block, but it is still not last block in the era
        // and the CurrentEra should not change
        crate::pallet::pallet::Pallet::<TestRuntime>::on_initialize(block_number);
        let mut current = mock::DappsStaking::current_era();
        assert_eq!(CURRENT_ERA, current.unwrap_or(Zero::zero()));

        // increment the block, this time it should be last block in the era
        // and CurrentEra should be incremented
        crate::pallet::pallet::Pallet::<TestRuntime>::on_initialize(block_number + 1);
        current = mock::DappsStaking::current_era();
        assert_eq!(CURRENT_ERA + 1, current.unwrap_or(Zero::zero()));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::NewDappStakingEra(
            CURRENT_ERA + 1,
        )));
    })
}

#[test]
fn new_era_forcing() {
    ExternalityBuilder::build().execute_with(|| {
        let block_number = BlockPerEra::get() / 2;
        const CURRENT_ERA: crate::EraIndex = 3;

        // set initial era index
        <CurrentEra<TestRuntime>>::put(CURRENT_ERA);

        // call on_initilize. It is not last block in the era, but it should increment the era
        <ForceEra<TestRuntime>>::put(Forcing::ForceNew);
        crate::pallet::pallet::Pallet::<TestRuntime>::on_initialize(block_number);

        // check that era is incremented
        let current = mock::DappsStaking::current_era();
        assert_eq!(CURRENT_ERA + 1, current.unwrap_or(Zero::zero()));

        // check that forcing is cleared
        assert_eq!(mock::DappsStaking::force_era(), Forcing::ForceNone);

        // check the event for the new era
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::NewDappStakingEra(
            CURRENT_ERA + 1,
        )));
    })
}

#[test]
fn claim_contract_not_registered() {
    ExternalityBuilder::build().execute_with(|| {
        let claimer = 2;
        let contract =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());

        assert_noop!(
            DappsStaking::claim(Origin::signed(claimer), contract),
            crate::pallet::pallet::Error::<TestRuntime>::ContractNotRegistered
        );
    })
}

#[test]
fn claim_nothing_to_claim() {
    ExternalityBuilder::build().execute_with(|| {
        let developer1 = 1;
        let claimer = 2;
        const ERA_REWARD: Balance = 1000;
        let contract =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
        const FROM_ERA: EraIndex = 1;

        advance_era_and_reward(FROM_ERA, ERA_REWARD, 0);
        let start_era = DappsStaking::current_era().unwrap_or(Zero::zero());
        register(developer1, contract.clone());

        assert_noop!(
            DappsStaking::claim(Origin::signed(claimer), contract),
            crate::pallet::pallet::Error::<TestRuntime>::NothingToClaim
        );
    })
}

#[test]
fn claim_twice_in_same_era() {
    ExternalityBuilder::build().execute_with(|| {
        let developer1 = 1;
        let claimer = 2;
        const ERA_REWARD: Balance = 1000;
        const STAKE_AMOUNT: Balance = 100;
        let contract =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
        const FROM_ERA: EraIndex = 1;
        const SKIP_ERA: EraIndex = 3;

        advance_era_and_reward(FROM_ERA, ERA_REWARD, 0);
        let start_era = DappsStaking::current_era().unwrap_or(Zero::zero());
        register(developer1, contract.clone());
        bond_and_stake(claimer, contract, STAKE_AMOUNT);
        advance_era_and_reward(SKIP_ERA, ERA_REWARD, 0);
        let claim_era: EraIndex = DappsStaking::current_era().unwrap_or(Zero::zero());
        claim(claimer, contract, start_era, claim_era.clone());

        assert_noop!(
            DappsStaking::claim(Origin::signed(claimer), contract),
            crate::pallet::pallet::Error::<TestRuntime>::AlreadyClaimedInThisEra
        );
    })
}

#[test]
fn claim_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let developer1 = 1;
        let claimer = 2;
        const ERA_REWARD: Balance = 1000;
        const STAKE_AMOUNT: Balance = 100;
        let contract =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
        const FROM_ERA: EraIndex = 1;
        const SKIP_ERA: EraIndex = 3;

        advance_era_and_reward(FROM_ERA, ERA_REWARD, 0);
        let start_era = DappsStaking::current_era().unwrap_or(Zero::zero());
        register(developer1, contract.clone());
        bond_and_stake(claimer, contract, STAKE_AMOUNT);
        advance_era_and_reward(SKIP_ERA, ERA_REWARD, 0);
        let claim_era: EraIndex = DappsStaking::current_era().unwrap_or(Zero::zero());
        claim(claimer, contract, start_era, claim_era.clone());

        cleared_contract_history(contract, FROM_ERA, claim_era);
    })
}

#[test]
fn claim_one_contract_exists() {
    ExternalityBuilder::build().execute_with(|| {
        let developer1 = 1;
        let staker1: mock::AccountId = 2;
        let staker2: mock::AccountId = 3;
        const ERA_REWARD: mock::Balance = 100;
        const STAKE_AMOUNT1: mock::Balance = 100;
        const STAKE_AMOUNT2: mock::Balance = 900;
        const INITIAL_STAKE: mock::Balance = 1000;
        const NUM_OF_CONTRACTS: u64 = 1;
        let contract =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
        const FROM_ERA: EraIndex = 1;
        const SKIP_ERA: EraIndex = 2;

        let free_staker1 = <mock::TestRuntime as Config>::Currency::free_balance(&staker1);
        let free_staker2 = <mock::TestRuntime as Config>::Currency::free_balance(&staker2);
        let free_developer = <mock::TestRuntime as Config>::Currency::free_balance(&developer1);
        print!("free_developer {:?}\n", free_developer);

        advance_era_and_reward(FROM_ERA, ERA_REWARD, INITIAL_STAKE);
        let start_era = DappsStaking::current_era().unwrap_or(Zero::zero());
        register(developer1, contract.clone());
        bond_and_stake(staker1, contract, STAKE_AMOUNT1);
        bond_and_stake(staker2, contract, STAKE_AMOUNT2);
        advance_era_and_reward(SKIP_ERA, ERA_REWARD, INITIAL_STAKE);
        let claim_era: EraIndex = DappsStaking::current_era().unwrap_or(Zero::zero());
        claim(staker1, contract, start_era, claim_era.clone());
        cleared_contract_history(contract, FROM_ERA, claim_era);
        let num_eras = 2; // number of rewarded eras

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
            free_staker1 + num_eras * expected_staker1_reward
        );
        assert_eq!(
            <mock::TestRuntime as Config>::Currency::free_balance(&staker2),
            free_staker2 + num_eras * expected_staker2_reward
        );
        assert_eq!(
            <mock::TestRuntime as Config>::Currency::free_balance(&developer1),
            free_developer + num_eras * expected_developer_reward as u128
        );
    })
}

pub fn balance_to_u64(input: BalanceOf<TestRuntime>) -> Option<u64> {
    TryInto::<u64>::try_into(input).ok()
}

fn calc_expected_staker_reward(
    era_reward: mock::Balance,
    era_stake: mock::Balance,
    contract_stake: mock::Balance,
    staker_stake: mock::Balance,
) -> mock::Balance {
    print!(
        "calc_expected_staker_reward era_reward:{:?} era_stake:{:?} contract_stake:{:?} staker_stake:{:?} \n",
        era_reward, era_stake, contract_stake, staker_stake
    );
    let contract_reward = Perbill::from_rational(
        balance_to_u64(era_reward).unwrap_or(0),
        balance_to_u64(era_stake).unwrap_or(0),
    ) * contract_stake;
    print!("contract_reward {:?}\n", contract_reward);

    let contract_staker_part: u64 =
        Perbill::from_percent(20) * balance_to_u64(contract_reward).unwrap_or(0);
    print!("contract_staker_part {:?}\n", contract_staker_part);
    let expected_staker_reward = Perbill::from_rational(
        contract_staker_part,
        balance_to_u64(contract_stake).unwrap_or(0),
    ) * staker_stake;
    print!("expected_staker_reward {:?}\n", expected_staker_reward);

    expected_staker_reward
}

fn calc_expected_developer_reward(
    era_reward: mock::Balance,
    era_stake: mock::Balance,
    contract_stake: mock::Balance,
) -> mock::Balance {
    print!(
        "calc_expected_developer_reward era_reward:{:?} era_stake{:?} contract_stake{:?} \n",
        era_reward, era_stake, contract_stake
    );
    let contract_reward = Perbill::from_rational(
        balance_to_u64(era_reward).unwrap_or(0),
        balance_to_u64(era_stake).unwrap_or(0),
    ) * contract_stake;
    print!("contract_reward {:?}\n", contract_reward);

    let expected_developer_reward = Perbill::from_percent(80) * contract_reward;
    print!("contract_developer_part {:?}\n", expected_developer_reward);

    expected_developer_reward
}

#[test]
fn claim_with_more_staking_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let developer1 = 1;
        let claimer = 2;
        const ERA_REWARD: Balance = 1000;
        const STAKE_AMOUNT: Balance = 100;
        const INITIAL_STAKE: mock::Balance = 100;
        let contract =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000007").unwrap());
        const FROM_ERA: EraIndex = 1;
        const SKIP_ERA: EraIndex = 3;

        advance_era_and_reward(FROM_ERA, ERA_REWARD, INITIAL_STAKE);
        let start_era = DappsStaking::current_era().unwrap_or(Zero::zero());
        register(developer1, contract.clone());
        bond_and_stake(claimer, contract, STAKE_AMOUNT);
        advance_era_and_reward(SKIP_ERA, ERA_REWARD, INITIAL_STAKE);
        bond_and_stake(claimer, contract, STAKE_AMOUNT);
        advance_era_and_reward(1, ERA_REWARD, INITIAL_STAKE);
        bond_and_stake(claimer, contract, STAKE_AMOUNT);
        advance_era_and_reward(SKIP_ERA, ERA_REWARD, INITIAL_STAKE);
        let claim_era: EraIndex = DappsStaking::current_era().unwrap_or(Zero::zero());
        claim(claimer, contract, start_era, claim_era.clone());

        cleared_contract_history(contract, FROM_ERA, claim_era);
    })
}

//
// HELPER FUNCTIONS
//

// helper fn to make  register() one liner for readability
fn register(developer: u64, contract: SmartContract<AccountId>) {
    assert_ok!(DappsStaking::register(
        Origin::signed(developer),
        contract.clone()
    ));
}
// helper fn to skip "for_era" eras, but reward each era
fn advance_era_and_reward(
    for_era: EraIndex,
    rewards: BalanceOf<TestRuntime>,
    staked: BalanceOf<TestRuntime>,
) {
    // TODO advance era by incrementing block production, needed for block rewards
    let current: EraIndex = mock::DappsStaking::current_era().unwrap_or(Zero::zero());

    let era_reward = EraReward { rewards, staked };
    for era in 0..for_era {
        <PalletEraRewards<TestRuntime>>::insert(current + era, era_reward.clone());
    }
    <CurrentEra<TestRuntime>>::put(&current + for_era);
}

// helper fn to check updated storage items after claim is called
fn cleared_contract_history(
    contract: SmartContract<mock::AccountId>,
    from_era: EraIndex,
    to_era: EraIndex,
) {
    // check claim pointer moved
    assert_eq!(
        mock::DappsStaking::contract_last_claimed(&contract).unwrap_or(Zero::zero()),
        to_era
    );

    // check last staked pointer moved
    assert_eq!(
        mock::DappsStaking::contract_last_staked(&contract).unwrap_or(Zero::zero()),
        to_era
    );

    // check new contractEraStaked
    assert_ok!((mock::DappsStaking::contract_era_stake(contract, to_era))
        .is_some()
        .then(|| ())
        .ok_or("contract_era_stake not created"));

    // check history storage is cleared
    for era in from_era..to_era {
        assert_ok!((mock::DappsStaking::contract_era_stake(contract, era))
            .is_none()
            .then(|| ())
            .ok_or("contract_era_stake not cleared"));
    }
}

// helper fn to make claim() one liner for readability
fn claim(
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

// helper fn to make bond_and_stake() one liner for readability
fn bond_and_stake(
    staker_id: AccountId,
    contract: SmartContract<mock::AccountId>,
    value: BalanceOf<TestRuntime>,
) {
    assert_ok!(DappsStaking::bond_and_stake(
        Origin::signed(staker_id),
        contract.clone(),
        value,
    ));
    System::assert_last_event(mock::Event::DappsStaking(crate::Event::BondAndStake(
        staker_id,
        contract.clone(),
        value,
    )));
}
