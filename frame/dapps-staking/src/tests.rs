use super::{pallet::pallet::Error, Event, *};
use frame_support::{
    assert_noop, assert_ok,
    traits::{OnInitialize, OnUnbalanced},
};
use mock::{Balances, MockSmartContract, *};
use sp_core::H160;
use sp_runtime::traits::Zero;

use testing_utils::*;

#[test]
fn on_unbalanced_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // At the beginning, both should be 0
        assert!(BlockRewardAccumulator::<TestRuntime>::get().is_zero());
        assert!(free_balance_of_dapps_staking_account().is_zero());

        // After handling imbalance, accumulator and account should be updated
        DappsStaking::on_unbalanced(Balances::issue(BLOCK_REWARD));
        assert_eq!(BLOCK_REWARD, BlockRewardAccumulator::<TestRuntime>::get());
        assert_eq!(BLOCK_REWARD, free_balance_of_dapps_staking_account());

        // After triggering a new era, accumulator should be set to 0 but account shouldn't consume any new imbalance
        DappsStaking::on_initialize(System::block_number());
        assert!(BlockRewardAccumulator::<TestRuntime>::get().is_zero());
        assert_eq!(BLOCK_REWARD, free_balance_of_dapps_staking_account());
    })
}

#[test]
fn on_initialize_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // Before we start, era is zero
        assert!(DappsStaking::current_era().is_zero());

        // We initialize the first block and advance to second one. New era must be triggered.
        initialize_first_block();
        let current_era = DappsStaking::current_era();
        assert_eq!(1, current_era);

        // Now advance by history limit. Ensure that rewards for era 1 still exist.
        let previous_era = current_era;
        advance_to_era(previous_era + HistoryDepth::get() + 1);

        // Check that all reward&stakes are as expected
        let current_era = DappsStaking::current_era();
        for era in 1..current_era {
            let era_rewards_and_stakes = EraRewardsAndStakes::<TestRuntime>::get(era).unwrap();
            assert_eq!(get_total_reward_per_era(), era_rewards_and_stakes.rewards);
        }
        // Current era rewards should be 0
        verify_pallet_era_staked_and_reward(current_era, 0, 0);
    })
}

#[test]
fn staking_info_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        register_contract(10, &contract_id);

        let staker_1 = 1;
        let staker_2 = 2;
        let staker_3 = 3;
        let amount = 100;

        // Prepare a little scenario.
        // staker_1 --> stakes starting era, doesn't unstake
        // staker_2 --> stakes starting era, unstakes everything before final era
        // staker_3 --> stakes after starting era, doesn't unstake

        let starting_era = 3;
        advance_to_era(starting_era);
        bond_and_stake_with_verification(staker_1, &contract_id, amount);
        bond_and_stake_with_verification(staker_2, &contract_id, amount);

        let mid_era = 7;
        advance_to_era(mid_era);
        unbond_and_unstake_with_verification(staker_2, &contract_id, amount);
        bond_and_stake_with_verification(staker_3, &contract_id, amount);

        let final_era = 12;
        advance_to_era(final_era);

        // Checks

        // Check first interval
        for era in starting_era..mid_era {
            let staking_info = DappsStaking::staking_info(&contract_id, era);
            assert_eq!(2_usize, staking_info.stakers.len());
            assert!(staking_info.stakers.contains_key(&staker_1));
            assert!(staking_info.stakers.contains_key(&staker_1));
        }

        // Check second interval
        for era in mid_era..=final_era {
            let staking_info = DappsStaking::staking_info(&contract_id, era);
            assert_eq!(2_usize, staking_info.stakers.len());
            assert!(staking_info.stakers.contains_key(&staker_1));
            assert!(staking_info.stakers.contains_key(&staker_3));
        }

        // Check that before starting era nothing exists
        let staking_info = DappsStaking::staking_info(&contract_id, starting_era - 1);
        assert!(staking_info.stakers.is_empty());

        // TODO: Do we want such behavior?
        // Era hasn't happened yet but value is returned as if it has happened
        let staking_info = DappsStaking::staking_info(&contract_id, final_era + 1);
        assert_eq!(2_usize, staking_info.stakers.len());
        assert!(staking_info.stakers.contains_key(&staker_1));
        assert!(staking_info.stakers.contains_key(&staker_3));
    })
}

#[test]
fn register_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let ok_contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        assert!(<TestRuntime as Config>::Currency::reserved_balance(&developer).is_zero());
        register_contract(developer, &ok_contract);
        System::assert_last_event(mock::Event::DappsStaking(Event::NewContract(
            developer,
            ok_contract,
        )));

        assert_eq!(
            RegisterDeposit::get(),
            <TestRuntime as Config>::Currency::reserved_balance(&developer)
        );
    })
}

#[test]
fn register_twice_with_same_account_not_works() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let contract1 = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let contract2 = MockSmartContract::Evm(H160::repeat_byte(0x02));

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
fn register_same_contract_twice_not_works() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer1 = 1;
        let developer2 = 2;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        register_contract(developer1, &contract);

        System::assert_last_event(mock::Event::DappsStaking(Event::NewContract(
            developer1, contract,
        )));

        // now register same contract by different developer
        assert_noop!(
            DappsStaking::register(Origin::signed(developer2), contract),
            Error::<TestRuntime>::AlreadyRegisteredContract
        );
    })
}

#[test]
fn register_with_pre_approve_enabled() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();
        let developer = 1;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // enable pre-approval for the developers
        assert_ok!(DappsStaking::enable_developer_pre_approval(
            Origin::root(),
            true
        ));
        assert!(DappsStaking::pre_approval_is_enabled());

        // register new developer without pre-approval, should fail
        assert_noop!(
            DappsStaking::register(Origin::signed(developer), contract.clone()),
            Error::<TestRuntime>::RequiredContractPreApproval,
        );

        // preapprove developer
        assert_ok!(DappsStaking::developer_pre_approval(
            Origin::root(),
            developer.clone()
        ));

        // try to pre-approve again same developer, should fail
        assert_noop!(
            DappsStaking::developer_pre_approval(Origin::root(), developer.clone()),
            Error::<TestRuntime>::AlreadyPreApprovedDeveloper
        );

        // register new contract by pre-approved developer
        assert_ok!(DappsStaking::register(
            Origin::signed(developer),
            contract.clone()
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::NewContract(
            developer, contract,
        )));

        // disable pre_approval and register contract2
        let developer2 = 2;
        let contract2 = MockSmartContract::Evm(H160::repeat_byte(0x02));
        assert_ok!(DappsStaking::enable_developer_pre_approval(
            Origin::root(),
            false
        ));
        assert_ok!(DappsStaking::register(
            Origin::signed(developer2),
            contract2.clone()
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::NewContract(
            developer2, contract2,
        )));
    })
}

#[test]
fn unregister_after_register_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        register_contract(developer, &contract_id);

        // Ensure that contract can be unregistered
        assert_ok!(DappsStaking::unregister(
            Origin::signed(developer),
            contract_id.clone()
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::ContractRemoved(
            developer,
            contract_id,
        )));
        verify_storage_after_unregister(&developer, &contract_id);

        assert!(<TestRuntime as Config>::Currency::reserved_balance(&developer).is_zero());
    })
}

#[test]
fn unregister_with_staked_contracts_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let dummy_developer = 2;
        let staker_1 = 3;
        let staker_2 = 4;
        let staked_value_1 = 150;
        let staked_value_2 = 330;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let dummy_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x05));

        // Register both contracts and stake them
        register_contract(developer, &contract_id);
        register_contract(dummy_developer, &dummy_contract_id);
        bond_and_stake_with_verification(staker_1, &contract_id, staked_value_1);
        bond_and_stake_with_verification(staker_2, &contract_id, staked_value_2);

        // This contract will just exist so it helps us with testing ledger content
        bond_and_stake_with_verification(staker_1, &dummy_contract_id, staked_value_1);
        bond_and_stake_with_verification(staker_2, &dummy_contract_id, staked_value_2);

        // Advance eras. This will accumulate some rewards.
        advance_to_era(5);
        let current_era = DappsStaking::current_era();

        // Ensure that era reward&stake are as expected. Later we will verify that this value is reduced.
        assert_eq!(
            (staked_value_1 + staked_value_2) * 2,
            DappsStaking::era_reward_and_stake(&current_era)
                .unwrap()
                .staked
        );

        // Ensure that contract can be unregistered
        assert_ok!(DappsStaking::unregister(
            Origin::signed(developer),
            contract_id.clone()
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::ContractRemoved(
            developer,
            contract_id,
        )));
        verify_storage_after_unregister(&developer, &contract_id);

        // Ensure ledger contains expected stake values. We have a single staked contract remaining.
        assert_eq!(staked_value_1, DappsStaking::ledger(&staker_1).locked);
        assert_eq!(staked_value_2, DappsStaking::ledger(&staker_2).locked);

        // Ensure that era reward&stake has been updated
        assert_eq!(
            staked_value_1 + staked_value_2,
            DappsStaking::era_reward_and_stake(&current_era)
                .unwrap()
                .staked
        );
    })
}

#[test]
fn unregister_with_incorrect_contract_does_not_work() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer_1 = 1;
        let developer_2 = 2;
        let first_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let second_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));

        register_contract(developer_1, &first_contract_id);

        // Try to unregister contract with developer who hasn't registered any contract
        assert_noop!(
            DappsStaking::unregister(Origin::signed(developer_2), first_contract_id.clone()),
            Error::<TestRuntime>::NotOwnedContract
        );

        // Register second contract with second dev and then try to unregister it using the first developer
        register_contract(developer_2, &second_contract_id);
        assert_noop!(
            DappsStaking::unregister(Origin::signed(developer_1), second_contract_id.clone()),
            Error::<TestRuntime>::NotOwnedContract
        );
    })
}

#[test]
fn unregister_stake_and_unstake_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // Register contract, stake it, unstake a bit
        register_contract(developer, &contract_id);
        bond_and_stake_with_verification(staker, &contract_id, 100);
        unbond_and_unstake_with_verification(staker, &contract_id, 10);

        // Unregister contract and verify that stake & unstake no longer work
        assert_ok!(DappsStaking::unregister(
            Origin::signed(developer),
            contract_id.clone()
        ));

        assert_noop!(
            DappsStaking::bond_and_stake(Origin::signed(staker), contract_id.clone(), 100),
            Error::<TestRuntime>::NotOperatedContract
        );
        assert_noop!(
            DappsStaking::unbond_and_unstake(Origin::signed(staker), contract_id.clone(), 100),
            Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn on_initialize_when_dapp_staking_enabled_in_mid_of_an_era_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // Set a block number in mid of an era
        System::set_block_number(2);

        // Verify that current era is 0 since dapps staking hasn't been initialized yet
        assert_eq!(0u32, DappsStaking::current_era());

        // Call on initialize in the mid of an era (according to block number calculation)
        // but since no era was initialized before, it will trigger a new era init.
        DappsStaking::on_initialize(System::block_number());
        assert_eq!(1u32, DappsStaking::current_era());
    })
}

#[test]
fn bond_and_stake_different_eras_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let first_stake_value = 100;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        let current_era = DappsStaking::current_era();

        // Insert a contract under registered contracts.
        register_contract(20, &contract_id);

        // initially, storage values should be None
        assert!(ContractEraStake::<TestRuntime>::get(&contract_id, current_era).is_none());

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
        verify_pallet_era_staked(current_era, first_stake_value);

        // Prepare new values and advance some eras.
        let second_stake_value = 300;
        let total_stake_value = first_stake_value + second_stake_value;

        advance_to_era(current_era + 2);
        let current_era = DappsStaking::current_era();

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
        verify_pallet_era_staked(current_era, total_stake_value);
    })
}

#[test]
fn bond_and_stake_two_different_contracts_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let first_stake_value = 100;
        let second_stake_value = 300;
        let first_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let second_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));
        let current_era = DappsStaking::current_era();

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
        assert_eq!(
            EraRewardsAndStakes::<TestRuntime>::get(current_era)
                .unwrap()
                .staked,
            total_stake_value,
        );
    })
}

#[test]
fn bond_and_stake_two_stakers_one_contract_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let first_staker_id = 1;
        let second_staker_id = 2;
        let first_stake_value = 50;
        let second_stake_value = 235;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let current_era = DappsStaking::current_era();

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
        assert_eq!(
            EraRewardsAndStakes::<TestRuntime>::get(current_era)
                .unwrap()
                .staked,
            total_stake_value,
        );
    })
}

#[test]
fn bond_and_stake_different_value_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // Insert a contract under registered contracts.
        register_contract(20, &contract_id);

        // Bond&stake almost the entire available balance of the staker.
        let staker_free_balance =
            Balances::free_balance(&staker_id).saturating_sub(MINIMUM_REMAINING_AMOUNT);
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
            staker_free_balance.saturating_sub(MINIMUM_REMAINING_AMOUNT),
        )));
        // Verify the minimum transferable amount of stakers account
        let transferable_balance =
            Balances::free_balance(&staker_id) - Ledger::<TestRuntime>::get(staker_id).locked;
        assert_eq!(MINIMUM_REMAINING_AMOUNT, transferable_balance);

        // Bond&stake some amount, a bit less than free balance
        let staker_id = 3;
        let staker_free_balance =
            Balances::free_balance(&staker_id).saturating_sub(MINIMUM_REMAINING_AMOUNT);
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
fn bond_and_stake_history_depth_has_passed_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker_id = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        let start_era = DappsStaking::current_era();
        register_contract(developer, &contract_id);

        // Do the first bond&stake
        let first_staking_amount = 200;
        bond_and_stake_with_verification(staker_id, &contract_id, first_staking_amount);

        // Advance eras beyond history depth
        let history_depth = HistoryDepth::get();
        advance_to_era(start_era + history_depth + 1);

        // Bond&stake again
        let second_staking_amount = 350;
        bond_and_stake_with_verification(staker_id, &contract_id, second_staking_amount);

        // Verify storage content
        let total_staked = first_staking_amount + second_staking_amount;
        let current_era = DappsStaking::current_era();

        // Verify storage values related to the current era
        verify_ledger(staker_id, total_staked);
        verify_era_staking_points(
            &contract_id,
            total_staked,
            current_era,
            vec![(staker_id, total_staked)],
        );
        assert_eq!(
            EraRewardsAndStakes::<TestRuntime>::get(current_era)
                .unwrap()
                .staked,
            total_staked,
        );

        // Also ensure that former values still exists even if they're beyond 'history depth'
        verify_era_staking_points(
            &contract_id,
            first_staking_amount,
            start_era,
            vec![(staker_id, first_staking_amount)],
        );
        assert_eq!(
            EraRewardsAndStakes::<TestRuntime>::get(current_era)
                .unwrap()
                .staked,
            total_staked,
        );
    })
}

#[test]
fn bond_and_stake_on_unregistered_contract_not_works() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let stake_value = 100;

        // Check not registered contract. Expect an error.
        let evm_contract = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_noop!(
            DappsStaking::bond_and_stake(Origin::signed(staker_id), evm_contract, stake_value),
            Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn bond_and_stake_insufficient_value() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();
        let staker_id = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // Insert a contract under registered contracts.
        register_contract(20, &contract_id);

        // If user tries to make an initial bond&stake with less than minimum amount, raise an error.
        assert_noop!(
            DappsStaking::bond_and_stake(
                Origin::signed(staker_id),
                contract_id.clone(),
                MINIMUM_STAKING_AMOUNT - 1
            ),
            Error::<TestRuntime>::InsufficientValue
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
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        // Insert a contract under registered contracts.
        register_contract(10, &contract_id);

        // Stake with MAX_NUMBER_OF_STAKERS on the same contract. It must work.
        for staker_id in 1..=MAX_NUMBER_OF_STAKERS {
            assert_ok!(DappsStaking::bond_and_stake(
                Origin::signed(staker_id.into()),
                contract_id.clone(),
                100,
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
fn unbond_and_unstake_multiple_time_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let original_staked_value = 300 + MINIMUM_STAKING_AMOUNT;
        let old_era = DappsStaking::current_era();

        // Insert a contract under registered contracts, bond&stake it.
        register_contract(10, &contract_id);
        bond_and_stake_with_verification(staker_id, &contract_id, original_staked_value);
        advance_to_era(old_era + 1);
        let new_era = DappsStaking::current_era();

        // Unstake such an amount so there will remain staked funds on the contract
        let unstaked_value = 100;
        unbond_and_unstake_with_verification(staker_id, &contract_id, unstaked_value);

        // Verify era staking info
        let new_staked_value = original_staked_value - unstaked_value;
        verify_era_staking_points(
            &contract_id,
            new_staked_value,
            new_era,
            vec![(staker_id, new_staked_value)],
        );
        // Also verify that the storage values for the old era haven't been changed due to unstaking
        verify_era_staking_points(
            &contract_id,
            original_staked_value,
            old_era,
            vec![(staker_id, original_staked_value)],
        );

        // Unbond yet again, but don't advance era
        // Unstake such an amount so there will remain staked funds on the contract
        let unstaked_value = 50;
        unbond_and_unstake_with_verification(staker_id, &contract_id, unstaked_value);
    })
}

#[test]
fn unbond_and_unstake_value_below_staking_threshold() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let first_value_to_unstake = 300;
        let staked_value = first_value_to_unstake + MINIMUM_STAKING_AMOUNT;

        // Insert a contract under registered contracts, bond&stake it.
        register_contract(10, &contract_id);
        bond_and_stake_with_verification(staker_id, &contract_id, staked_value);

        // Unstake such an amount that exactly minimum staking amount will remain staked.
        unbond_and_unstake_with_verification(staker_id, &contract_id, first_value_to_unstake);

        // Unstake 1 token and expect that the entire staked amount will be unstaked.
        unbond_and_unstake_with_verification(staker_id, &contract_id, 1);
    })
}

#[test]
fn unbond_and_unstake_in_different_eras() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let first_staker_id = 1;
        let second_staker_id = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let staked_value = 500;

        // Insert a contract under registered contracts, bond&stake it with two different stakers.
        register_contract(10, &contract_id);
        bond_and_stake_with_verification(first_staker_id, &contract_id, staked_value);
        bond_and_stake_with_verification(second_staker_id, &contract_id, staked_value);
        let total_staked_value = 2 * staked_value;

        // Advance era, unbond&withdraw with first staker, verify that it was successful
        let current_era = DappsStaking::current_era();
        advance_to_era(current_era + 10);
        let current_era = DappsStaking::current_era();

        let first_unstake_value = 100;
        unbond_and_unstake_with_verification(first_staker_id, &contract_id, first_unstake_value);

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

        // Advance era, unbond with second staker and verify storage values are as expected
        advance_to_era(current_era + 10);
        let current_era = DappsStaking::current_era();

        let second_unstake_value = 333;
        unbond_and_unstake_with_verification(second_staker_id, &contract_id, second_unstake_value);

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
    })
}

#[test]
fn unbond_and_unstake_history_depth_has_passed_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker_id = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        let start_era = DappsStaking::current_era();
        register_contract(developer, &contract_id);

        // Do the first bond&stake
        let first_staking_amount = 200;
        bond_and_stake_with_verification(staker_id, &contract_id, first_staking_amount);

        // Advance eras beyond history depth
        let history_depth = HistoryDepth::get();
        advance_to_era(start_era + history_depth + 1);

        let first_unstake_amount = 30;
        unbond_and_unstake_with_verification(staker_id, &contract_id, first_unstake_amount);

        // Advance era again beyond the history depth
        advance_to_era(DappsStaking::current_era() + history_depth + 10);

        let second_unstake_amount = 30;
        unbond_and_unstake_with_verification(staker_id, &contract_id, second_unstake_amount);
    })
}

#[test]
fn unbond_and_unstake_in_same_era_can_exceed_max_chunks() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        register_contract(10, &contract_id);

        let staker = 1;

        bond_and_stake_with_verification(
            staker,
            &contract_id,
            200 * MAX_UNLOCKING_CHUNKS as Balance,
        );

        // Ensure that we can unbond up to a limited amount of time.
        for _ in 0..MAX_UNLOCKING_CHUNKS * 2 {
            unbond_and_unstake_with_verification(1, &contract_id, 10);
            assert_eq!(1, Ledger::<TestRuntime>::get(&staker).unbonding_info.len());
        }
    })
}

#[test]
fn unbond_and_unstake_with_zero_value_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        register_contract(10, &contract_id);

        assert_noop!(
            DappsStaking::unbond_and_unstake(Origin::signed(1), contract_id, 0),
            Error::<TestRuntime>::UnstakingWithNoValue
        );
    })
}

#[test]
fn unbond_and_unstake_on_not_operated_contract_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_noop!(
            DappsStaking::unbond_and_unstake(Origin::signed(1), contract_id, 100),
            Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn unbond_and_unstake_too_many_unlocking_chunks_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        register_contract(10, &contract_id);

        let staker = 1;
        let unstake_amount = 10;
        let stake_amount =
            MINIMUM_STAKING_AMOUNT * 10 + unstake_amount * MAX_UNLOCKING_CHUNKS as Balance;

        bond_and_stake_with_verification(staker, &contract_id, stake_amount);

        // Ensure that we can unbond up to a limited amount of time.
        for _ in 0..MAX_UNLOCKING_CHUNKS {
            advance_to_era(DappsStaking::current_era() + 1);
            unbond_and_unstake_with_verification(staker, &contract_id, unstake_amount);
        }

        // Ensure that we're at the max but can still add new chunks since it should be merged with the existing one
        assert_eq!(
            MAX_UNLOCKING_CHUNKS,
            DappsStaking::ledger(&staker).unbonding_info.len()
        );
        unbond_and_unstake_with_verification(staker, &contract_id, unstake_amount);

        // Ensure that further unbonding attempts result in an error.
        advance_to_era(DappsStaking::current_era() + 1);
        assert_noop!(
            DappsStaking::unbond_and_unstake(
                Origin::signed(staker),
                contract_id.clone(),
                unstake_amount
            ),
            Error::<TestRuntime>::TooManyUnlockingChunks,
        );
    })
}

#[test]
fn unbond_and_unstake_on_not_staked_contract_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        register_contract(10, &contract_id);

        assert_noop!(
            DappsStaking::unbond_and_unstake(Origin::signed(1), contract_id, 10),
            Error::<TestRuntime>::NotStakedContract,
        );
    })
}

#[ignore]
#[test]
fn unbond_and_unstake_with_no_chunks_allowed() {
    // UT can be used to verify situation when MaxUnlockingChunks = 0. Requires mock modification.
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        // Sanity check
        assert_eq!(<TestRuntime as Config>::MaxUnlockingChunks::get(), 0);

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        register_contract(10, &contract_id);

        let staker_id = 1;
        bond_and_stake_with_verification(staker_id, &contract_id, 100);

        assert_noop!(
            DappsStaking::unbond_and_unstake(Origin::signed(staker_id), contract_id.clone(), 20),
            Error::<TestRuntime>::TooManyUnlockingChunks,
        );
    })
}

#[test]
fn withdraw_unbonded_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        register_contract(10, &contract_id);

        let staker_id = 1;
        bond_and_stake_with_verification(staker_id, &contract_id, 1000);

        let first_unbond_value = 75;
        let second_unbond_value = 39;
        let initial_era = DappsStaking::current_era();

        // Unbond some amount in the initial era
        unbond_and_unstake_with_verification(staker_id, &contract_id, first_unbond_value);

        // Advance one era and then unbond some more
        advance_to_era(initial_era + 1);
        unbond_and_unstake_with_verification(staker_id, &contract_id, second_unbond_value);

        // Now advance one era before first chunks finishes the unbonding process
        advance_to_era(initial_era + UNBONDING_PERIOD - 1);
        assert_noop!(
            DappsStaking::withdraw_unbonded(Origin::signed(staker_id)),
            Error::<TestRuntime>::NothingToWithdraw
        );

        // Advance one additional era and expect that the first chunk can be withdrawn
        advance_to_era(DappsStaking::current_era() + 1);
        assert_ok!(DappsStaking::withdraw_unbonded(Origin::signed(staker_id),));
        System::assert_last_event(mock::Event::DappsStaking(Event::Withdrawn(
            staker_id,
            first_unbond_value,
        )));

        // Advance one additional era and expect that the first chunk can be withdrawn
        advance_to_era(DappsStaking::current_era() + 1);
        assert_ok!(DappsStaking::withdraw_unbonded(Origin::signed(staker_id),));
        System::assert_last_event(mock::Event::DappsStaking(Event::Withdrawn(
            staker_id,
            second_unbond_value,
        )));

        // Advance one additional era but since we have nothing else to withdraw, expect an error
        advance_to_era(initial_era + UNBONDING_PERIOD);
        assert_noop!(
            DappsStaking::withdraw_unbonded(Origin::signed(staker_id)),
            Error::<TestRuntime>::NothingToWithdraw
        );
    })
}

#[test]
fn withdraw_unbonded_full_vector_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        register_contract(10, &contract_id);

        let staker_id = 1;
        bond_and_stake_with_verification(staker_id, &contract_id, 1000);

        // Repeatedly start unbonding and advance era to create unlocking chunks
        let init_unbonding_amount = 15;
        for x in 1..=MAX_UNLOCKING_CHUNKS {
            unbond_and_unstake_with_verification(
                staker_id,
                &contract_id,
                init_unbonding_amount * x as u128,
            );
            advance_to_era(DappsStaking::current_era() + 1);
        }

        // Now clean up all that are eligible for cleanu-up
        withdraw_unbonded_with_verification(staker_id);

        // This is a sanity check for the test. Some chunks should remain, otherwise test isn't testing realistic unbonding period.
        assert!(!Ledger::<TestRuntime>::get(&staker_id)
            .unbonding_info
            .is_empty());

        while !Ledger::<TestRuntime>::get(&staker_id)
            .unbonding_info
            .is_empty()
        {
            advance_to_era(DappsStaking::current_era() + 1);
            withdraw_unbonded_with_verification(staker_id);
        }
    })
}

#[test]
fn withdraw_unbonded_no_value_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        assert_noop!(
            DappsStaking::withdraw_unbonded(Origin::signed(1)),
            Error::<TestRuntime>::NothingToWithdraw,
        );
    })
}

#[ignore]
#[test]
fn withdraw_unbonded_no_unbonding_period() {
    // UT can be used to verify situation when UnbondingPeriod = 0. Requires mock modification.
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        // Sanity check
        assert_eq!(<TestRuntime as Config>::UnbondingPeriod::get(), 0);

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        register_contract(10, &contract_id);

        let staker_id = 1;
        bond_and_stake_with_verification(staker_id, &contract_id, 100);
        unbond_and_unstake_with_verification(staker_id, &contract_id, 20);

        // Ensure that funds can be immediately withdrawn
        withdraw_unbonded_with_verification(staker_id);
    })
}

#[test]
fn new_era_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // set initial era index
        advance_to_era(DappsStaking::current_era() + 10);
        let starting_era = DappsStaking::current_era();

        // verify that block reward is zero at the beginning of an era
        assert!(DappsStaking::block_reward_accumulator().is_zero());

        // Increment block by setting it to the first block in era value
        run_for_blocks(1);
        let current_era = DappsStaking::current_era();
        assert_eq!(starting_era, current_era);

        // verify that block reward is added to the block_reward_accumulator
        let block_reward = DappsStaking::block_reward_accumulator();
        assert_eq!(BLOCK_REWARD, block_reward);

        // register and bond to verify storage item
        let staker = 2;
        let developer = 3;
        let staked_amount = 100;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));
        register_contract(developer, &contract);
        bond_and_stake_with_verification(staker, &contract, staked_amount);

        // CurrentEra should be incremented
        // block_reward_accumulator should be reset to 0
        advance_to_era(DappsStaking::current_era() + 1);

        let current_era = DappsStaking::current_era();
        assert_eq!(starting_era + 1, current_era);
        System::assert_last_event(mock::Event::DappsStaking(Event::NewDappStakingEra(
            starting_era + 1,
        )));

        // verify that block reward accumulator is reset to 0
        let block_reward = DappsStaking::block_reward_accumulator();
        assert!(block_reward.is_zero());

        let expected_era_reward = get_total_reward_per_era();
        // verify that .staked is copied and .reward is added
        verify_pallet_era_staked_and_reward(starting_era, staked_amount, expected_era_reward);
    })
}

#[test]
fn new_era_forcing() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();
        advance_to_era(3);
        let starting_era = mock::DappsStaking::current_era();

        // call on_initilize. It is not last block in the era, but it should increment the era
        <ForceEra<TestRuntime>>::put(Forcing::ForceNew);
        run_for_blocks(1);

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
        initialize_first_block();

        let claimer = 2;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        assert_noop!(
            DappsStaking::claim(Origin::signed(claimer), contract, 1),
            Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn claim_invalid_eras() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer1 = 1;
        let claimer = 2;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        register_contract(developer1, &contract);

        // Advance way past the history depth
        advance_to_era(HistoryDepth::get() * 2);

        let too_old_era = DappsStaking::current_era() - HistoryDepth::get() - 1;
        assert_noop!(
            DappsStaking::claim(Origin::signed(claimer), contract, too_old_era),
            Error::<TestRuntime>::EraOutOfBounds,
        );

        let future_era = DappsStaking::current_era() + 1;
        assert_noop!(
            DappsStaking::claim(Origin::signed(claimer), contract, future_era),
            Error::<TestRuntime>::EraOutOfBounds,
        );

        let current_era = DappsStaking::current_era();
        assert_noop!(
            DappsStaking::claim(Origin::signed(claimer), contract, current_era,),
            Error::<TestRuntime>::EraOutOfBounds,
        );

        let non_staked_era = current_era - 1;
        assert_noop!(
            DappsStaking::claim(Origin::signed(claimer), contract, non_staked_era,),
            Error::<TestRuntime>::NotStaked,
        );
    })
}

#[test]
fn claim_twice_in_same_era() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let claimer = 2;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        register_contract(developer, &contract);
        bond_and_stake_with_verification(claimer, &contract, 100);

        advance_to_era(DappsStaking::current_era() + 1);

        let claim_era = DappsStaking::current_era() - 1;
        claim_with_verification(claimer, contract, claim_era);

        assert_noop!(
            DappsStaking::claim(Origin::signed(claimer), contract, claim_era),
            Error::<TestRuntime>::AlreadyClaimedInThisEra
        );
    })
}

#[test]
fn claim_for_all_valid_history_eras_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer1 = 1;
        let claimer = 2;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        register_contract(developer1, &contract);
        bond_and_stake_with_verification(claimer, &contract, 100);

        // Advance past the history depth
        advance_to_era(DappsStaking::current_era() + HistoryDepth::get() + 1);
        let current_era = DappsStaking::current_era();

        // All eras must be claimable
        for era in (current_era - HistoryDepth::get())..current_era {
            claim_with_verification(claimer, contract.clone(), era);
        }
    })
}

#[test]
fn claim_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let claimer = 2;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        let start_era = DappsStaking::current_era();

        register_contract(developer, &contract);
        bond_and_stake_with_verification(claimer, &contract, 100);

        advance_to_era(start_era + 3);

        let issuance_before_claim = <TestRuntime as Config>::Currency::total_issuance();
        let claim_era = DappsStaking::current_era() - 1;

        claim_with_verification(claimer, contract, claim_era);

        // Claim shouldn't mint new tokens, instead it should just transfer from the dapps staking pallet account
        let issuance_after_claim = <TestRuntime as Config>::Currency::total_issuance();
        assert_eq!(issuance_before_claim, issuance_after_claim);
    })
}

#[test]
fn claim_after_unregister_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();
        let developer = 1;
        let staker = 2;
        let stake_amount_1 = 100;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // Register contract, stake it
        register_contract(developer, &contract);
        bond_and_stake_with_verification(staker, &contract, stake_amount_1);

        // Advance by some eras
        advance_to_era(5);

        // Unregister contract, without claiming it!
        assert_ok!(DappsStaking::unregister(
            Origin::signed(developer),
            contract.clone()
        ));
        let unregistered_era = DappsStaking::current_era();

        // Ensure that contract can still be claimed.
        let current_era = DappsStaking::current_era();
        for era in 1..current_era {
            claim_with_verification(staker, contract.clone(), era);
        }

        // Advance some more eras
        advance_to_era(unregistered_era + 5);
        let current_era = DappsStaking::current_era();
        for era in unregistered_era..current_era {
            assert_noop!(
                DappsStaking::claim(Origin::signed(developer), contract.clone(), era),
                Error::<TestRuntime>::NotStaked,
            );
        }
    })
}

#[test]
fn claim_one_contract_one_staker() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker1 = 2;

        // We use a small amount so staked amount is less than rewards
        let stake_amount_1 = 50;
        let initial_stake = 50;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // Store initial free balaces of the developer and the stakers
        let free_balance_staker1 = <TestRuntime as Config>::Currency::free_balance(&staker1);

        // Register contracts, bond&stake them with two stakers on the contract.
        let start_era = DappsStaking::current_era();
        register_contract(developer, &contract);
        let free_developer_balance = <TestRuntime as Config>::Currency::free_balance(&developer);
        bond_and_stake_with_verification(staker1, &contract, stake_amount_1);

        // Advance some eras to be able to claim rewards. Verify storage is consolidated
        advance_to_era(start_era + 1);
        let claim_era = DappsStaking::current_era() - 1;
        claim_with_verification(staker1, contract, claim_era);
        // calculate reward per stakers
        let expected_staker1_reward =
            calc_expected_staker_reward(claim_era, initial_stake, stake_amount_1);

        // calculate reward per developer
        let expected_developer_reward = calc_expected_developer_reward(claim_era, initial_stake);

        // check balances to see if the rewards are paid out
        check_rewards_on_balance_and_storage(
            &staker1,
            free_balance_staker1,
            expected_staker1_reward,
        );
        check_rewards_on_balance_and_storage(
            &developer,
            free_developer_balance,
            expected_developer_reward,
        );

        let expected_contract_reward = expected_staker1_reward + expected_developer_reward;
        check_paidout_rewards_for_contract(&contract, claim_era, expected_contract_reward);
    })
}

#[test]
fn claim_one_contract_two_stakers() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker1 = 2;
        let staker2 = 3;

        let stake_amount_1 = 400;
        let stake_amount_2 = 600;
        let initial_stake = stake_amount_1 + stake_amount_2;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // Store initial free balaces of the developer and the stakers
        let free_balance_staker1 = <TestRuntime as Config>::Currency::free_balance(&staker1);
        let free_balance_staker2 = <TestRuntime as Config>::Currency::free_balance(&staker2);

        // Register contracts, bond&stake them with two stakers on the contract.
        let start_era = DappsStaking::current_era();
        register_contract(developer, &contract);
        let free_developer_balance = <TestRuntime as Config>::Currency::free_balance(&developer);
        bond_and_stake_with_verification(staker1, &contract, stake_amount_1);
        bond_and_stake_with_verification(staker2, &contract, stake_amount_2);

        // Advance some eras to be able to claim rewards. Verify storage is consolidated
        advance_to_era(start_era + 3);
        let claim_era = DappsStaking::current_era() - 1;
        claim_with_verification(staker1, contract, claim_era);

        // calculate reward per stakers
        let expected_staker1_reward =
            calc_expected_staker_reward(claim_era, initial_stake, stake_amount_1);
        let expected_staker2_reward =
            calc_expected_staker_reward(claim_era, initial_stake, stake_amount_2);

        // calculate reward per developer
        let expected_developer_reward = calc_expected_developer_reward(claim_era, initial_stake);

        // check balances to see if the rewards are paid out
        check_rewards_on_balance_and_storage(
            &staker1,
            free_balance_staker1,
            expected_staker1_reward,
        );
        check_rewards_on_balance_and_storage(
            &staker2,
            free_balance_staker2,
            expected_staker2_reward,
        );
        check_rewards_on_balance_and_storage(
            &developer,
            free_developer_balance,
            expected_developer_reward,
        );
        let expected_contract_reward =
            expected_staker1_reward + expected_staker2_reward + expected_developer_reward;
        check_paidout_rewards_for_contract(&contract, claim_era, expected_contract_reward);
    })
}

#[test]
fn claim_two_contracts_three_stakers_new() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer1 = 1;
        let developer2 = 10;

        let staker1 = 2;
        let staker2 = 3; // will stake on 2 contracts
        let staker3 = 4;

        let staker_1_amount = 400;
        let staker_2_amount_1 = 600;
        let staker_2_amount_2 = 100;
        let staker_3_amount = 400;

        let contract1 = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let contract2 = MockSmartContract::Evm(H160::repeat_byte(0x02));

        // Store initial free balaces of developers and stakers
        let free_balance_staker1 = <TestRuntime as Config>::Currency::free_balance(&staker1);
        let free_balance_staker2 = <TestRuntime as Config>::Currency::free_balance(&staker2);
        let free_balance_staker3 = <TestRuntime as Config>::Currency::free_balance(&staker3);

        // Register 2 contracts, bond&stake with two stakers on first contract.
        let start_era = DappsStaking::current_era();
        register_contract(developer1, &contract1);
        register_contract(developer2, &contract2);
        let free_balance_developer1 = <TestRuntime as Config>::Currency::free_balance(&developer1);
        let free_balance_developer2 = <TestRuntime as Config>::Currency::free_balance(&developer2);
        bond_and_stake_with_verification(staker1, &contract1, staker_1_amount);
        bond_and_stake_with_verification(staker2, &contract1, staker_2_amount_1);
        let contract_1_stake = staker_1_amount + staker_2_amount_1;
        let first_claim_era = start_era;

        // Advance eras and then bond&stake with two stakers on second contract.
        advance_to_era(start_era + 3);

        bond_and_stake_with_verification(staker2, &contract2, staker_2_amount_2);
        bond_and_stake_with_verification(staker3, &contract2, staker_3_amount);
        let contract_2_stake = staker_2_amount_2 + staker_3_amount;

        // Advance era again by one, so rewards can be claimed for previous era as well.
        let current_era = DappsStaking::current_era();
        let second_claim_era = current_era;
        advance_to_era(current_era + 1);

        // Claim first contract rewards for the two prepared eras and verify storage content is as expected.
        claim_with_verification(staker1, contract1.clone(), first_claim_era);
        claim_with_verification(staker1, contract1.clone(), second_claim_era);

        // Calculate staker1 rewards for the two claimed eras
        let expected_c1_staker1_e1_reward =
            calc_expected_staker_reward(first_claim_era, contract_1_stake, staker_1_amount);
        let expected_c1_staker1_e2_reward =
            calc_expected_staker_reward(second_claim_era, contract_1_stake, staker_1_amount);
        let expected_c1_staker1_reward_total =
            expected_c1_staker1_e1_reward + expected_c1_staker1_e2_reward;
        check_rewards_on_balance_and_storage(
            &staker1,
            free_balance_staker1,
            expected_c1_staker1_reward_total,
        );

        // Calculate staker2 rewards for the two claimed eras
        let expected_c1_staker2_e1_reward =
            calc_expected_staker_reward(first_claim_era, contract_1_stake, staker_2_amount_1);
        let expected_c1_staker2_e2_reward =
            calc_expected_staker_reward(second_claim_era, contract_1_stake, staker_2_amount_1);
        let expected_c1_staker2_reward_total =
            expected_c1_staker2_e1_reward + expected_c1_staker2_e2_reward;
        check_rewards_on_balance_and_storage(
            &staker2,
            free_balance_staker2,
            expected_c1_staker2_reward_total,
        );

        // Calculate developer1 rewards for the two claimed eras
        let expected_c1_dev1_e1_reward =
            calc_expected_developer_reward(first_claim_era, contract_1_stake);
        let expected_c1_dev1_e2_reward =
            calc_expected_developer_reward(second_claim_era, contract_1_stake);
        let expected_c1_developer1_reward_total =
            expected_c1_dev1_e1_reward + expected_c1_dev1_e2_reward;
        check_rewards_on_balance_and_storage(
            &developer1,
            free_balance_developer1,
            expected_c1_developer1_reward_total,
        );

        // Verify total paid out rewards for the claimed eras
        let expected_contract1_e1_reward = expected_c1_staker1_e1_reward
            + expected_c1_staker2_e1_reward
            + expected_c1_dev1_e1_reward;
        check_paidout_rewards_for_contract(
            &contract1,
            first_claim_era,
            expected_contract1_e1_reward,
        );
        let expected_contract1_e2_reward = expected_c1_staker1_e2_reward
            + expected_c1_staker2_e2_reward
            + expected_c1_dev1_e2_reward;
        check_paidout_rewards_for_contract(
            &contract1,
            second_claim_era,
            expected_contract1_e2_reward,
        );

        claim_with_verification(staker2, contract2.clone(), second_claim_era);

        // Calculate staker 2 rewards for the second contract and a single era
        let expected_c2_staker2_e2_reward =
            calc_expected_staker_reward(second_claim_era, contract_2_stake, staker_2_amount_2);
        check_rewards_on_balance_and_storage(
            &staker2,
            free_balance_staker2,
            expected_c2_staker2_e2_reward + expected_c1_staker2_reward_total,
        );

        // Calculate staker 3 rewards for the second contract and a single era
        let expected_c2_staker3_e2_reward =
            calc_expected_staker_reward(second_claim_era, contract_2_stake, staker_3_amount);
        check_rewards_on_balance_and_storage(
            &staker3,
            free_balance_staker3,
            expected_c2_staker3_e2_reward,
        );

        // Calculate developer2 rewards for the single claimed era
        let expected_c2_dev2_e2_reward =
            calc_expected_developer_reward(second_claim_era, contract_2_stake);
        check_rewards_on_balance_and_storage(
            &developer2,
            free_balance_developer2,
            expected_c2_dev2_e2_reward,
        );

        let expected_contract2_reward = expected_c2_staker2_e2_reward
            + expected_c2_staker3_e2_reward
            + expected_c2_dev2_e2_reward;
        check_paidout_rewards_for_contract(&contract2, second_claim_era, expected_contract2_reward);
    })
}

#[test]
fn unbonding_info_test() {
    let mut unbonding_info = UnbondingInfo::<Balance>::default();

    // assert basic ops on empty info
    assert!(unbonding_info.is_empty());
    assert!(unbonding_info.len().is_zero());
    let (first_info, second_info) = unbonding_info.clone().partition(2);
    assert!(first_info.is_empty());
    assert!(second_info.is_empty());

    // Prepare unlocking chunks.
    let count = 5;
    let base_amount: Balance = 100;
    let base_unlock_era = 4 * count;
    let mut chunks = vec![];
    for x in 1_u32..=count as u32 {
        chunks.push(UnlockingChunk {
            amount: base_amount * x as Balance,
            unlock_era: base_unlock_era - 3 * x,
        });
    }

    // Add one unlocking chunk and verify basic ops.
    unbonding_info.add(chunks[0 as usize]);

    assert!(!unbonding_info.is_empty());
    assert_eq!(1, unbonding_info.len());
    assert_eq!(chunks[0 as usize].amount, unbonding_info.sum());

    let (first_info, second_info) = unbonding_info.clone().partition(base_unlock_era);
    assert_eq!(1, first_info.len());
    assert_eq!(chunks[0 as usize].amount, first_info.sum());
    assert!(second_info.is_empty());

    // Add remainder and verify basic ops
    for x in unbonding_info.len() as usize..chunks.len() {
        unbonding_info.add(chunks[x]);
        // Ensure internal vec is sorted
        assert!(unbonding_info
            .vec()
            .windows(2)
            .all(|w| w[0].unlock_era <= w[1].unlock_era));
    }
    assert_eq!(chunks.len(), unbonding_info.len() as usize);
    let total: Balance = chunks.iter().map(|c| c.amount).sum();
    assert_eq!(total, unbonding_info.sum());

    let partition_era = chunks[2].unlock_era + 1;
    let (first_info, second_info) = unbonding_info.clone().partition(partition_era);
    assert_eq!(3, first_info.len());
    assert_eq!(2, second_info.len());
    assert_eq!(unbonding_info.sum(), first_info.sum() + second_info.sum());
}
