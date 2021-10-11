use super::{pallet::pallet::Error, Event, *};
use frame_support::{assert_noop, assert_ok};
use mock::{Balances, EraIndex, MockSmartContract, *};
use sp_core::H160;
use sp_runtime::traits::{AccountIdConversion, Zero};
use testing_utils::*;

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
fn register_twice_with_same_account_nok() {
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
fn register_same_contract_twice_nok() {
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
        assert_eq!(mock::DappsStaking::contract_last_claimed(contract), None);
        assert_eq!(mock::DappsStaking::contract_last_staked(contract), None);
    })
}

#[test]
fn register_with_pre_approve_enabled() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();
        let developer = 1;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // enable pre-approval for the developers
        assert_ok!(mock::DappsStaking::enable_developer_pre_approval(
            Origin::root(),
            true
        ));
        assert!(mock::DappsStaking::pre_approval_is_enabled());

        // register new developer without pre-approval, should fail
        assert_noop!(
            DappsStaking::register(Origin::signed(developer), contract.clone()),
            Error::<TestRuntime>::RequiredContractPreApproval,
        );

        // preapprove developer
        assert_ok!(mock::DappsStaking::developer_pre_approval(
            Origin::root(),
            developer.clone()
        ));

        // try to pre-approve again same developer, should fail
        assert_noop!(
            mock::DappsStaking::developer_pre_approval(Origin::root(), developer.clone()),
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
        assert_ok!(mock::DappsStaking::enable_developer_pre_approval(
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
        verify_storage_after_unregister(&developer, &contract_id, DappsStaking::current_era());

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

        // Try to unregister and expect an error since we have unclaimed rewards.
        assert_noop!(
            DappsStaking::unregister(Origin::signed(developer), contract_id.clone()),
            Error::<TestRuntime>::ContractRewardsNotClaimed
        );

        // Claim the rewards and then try to unregister again.
        assert_ok!(DappsStaking::claim(
            Origin::signed(developer),
            contract_id.clone()
        ));

        // Ensure that contract can be unregistered
        assert_ok!(DappsStaking::unregister(
            Origin::signed(developer),
            contract_id.clone()
        ));
        System::assert_last_event(mock::Event::DappsStaking(Event::ContractRemoved(
            developer,
            contract_id,
        )));
        verify_storage_after_unregister(&developer, &contract_id, current_era);

        // Ensure ledger contains expected stake values. We have a single staked contract remaining.
        assert_eq!(staked_value_1, DappsStaking::ledger(&staker_1));
        assert_eq!(staked_value_2, DappsStaking::ledger(&staker_2));

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
fn unregister_with_incorrect_contract_is_not_ok() {
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
        assert!(ContractLastClaimed::<TestRuntime>::get(&contract_id).is_none());
        assert!(ContractLastStaked::<TestRuntime>::get(&contract_id).is_none());
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
        run_to_block(BLOCKS_PER_ERA * 10);
        let current_era = mock::DappsStaking::current_era();

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
        initialize_first_block();

        let staker_id = 1;
        let first_stake_value = 100;
        let second_stake_value = 300;
        let first_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let second_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));
        let current_era = mock::DappsStaking::current_era();

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
        verify_pallet_era_staked(current_era, total_stake_value);
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
        let current_era = mock::DappsStaking::current_era();

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
        verify_pallet_era_staked(current_era, total_stake_value);
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
        verify_pallet_era_staked(current_era, total_staked);

        // Also ensure that former values still exists even if they're beyond 'history depth'
        verify_era_staking_points(
            &contract_id,
            first_staking_amount,
            start_era,
            vec![(staker_id, first_staking_amount)],
        );
        verify_pallet_era_staked(start_era, first_staking_amount);
    })
}

#[test]
fn bond_and_stake_contract_is_not_ok() {
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
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
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
        initialize_first_block();

        let staker_id = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let original_staked_value = 300 + MINIMUM_STAKING_AMOUNT;
        let old_era = mock::DappsStaking::current_era();

        // Insert a contract under registered contracts, bond&stake it.
        register_contract(10, &contract_id);
        bond_and_stake_with_verification(staker_id, &contract_id, original_staked_value);
        run_to_block(BLOCKS_PER_ERA * 10);
        let new_era = mock::DappsStaking::current_era();

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
        verify_pallet_era_staked(new_era, new_staked_value);
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
        verify_pallet_era_staked(old_era, original_staked_value);

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
        verify_pallet_era_staked(new_era, new_staked_value);
        assert_eq!(
            new_era,
            ContractLastStaked::<TestRuntime>::get(&contract_id).unwrap()
        );
    })
}

#[test]
fn unbond_unstake_and_withdraw_value_below_staking_threshold() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let first_value_to_unstake = 300;
        let staked_value = first_value_to_unstake + MINIMUM_STAKING_AMOUNT;

        let current_era = mock::DappsStaking::current_era();

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
        verify_pallet_era_staked(current_era, Zero::zero());
    })
}

#[test]
fn unbond_unstake_and_withdraw_in_different_eras() {
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
        run_to_block(BLOCKS_PER_ERA * 50);
        let current_era = mock::DappsStaking::current_era();

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
        verify_pallet_era_staked(current_era, new_total_staked);
        assert_eq!(
            current_era,
            ContractLastStaked::<TestRuntime>::get(&contract_id).unwrap()
        );

        // Advance era, unbond with second staker and verify storage values are as expected
        run_to_block(BLOCKS_PER_ERA * 100);
        let current_era = mock::DappsStaking::current_era();

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
        verify_pallet_era_staked(current_era, new_total_staked);
        assert_eq!(
            current_era,
            ContractLastStaked::<TestRuntime>::get(&contract_id).unwrap()
        );
    })
}

#[test]
fn unbond_unstake_and_withdraw_history_depth_has_passed_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker_id = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        //////////////////////////////////////////////
        ///// FIRST ERA
        //////////////////////////////////////////////

        let start_era = DappsStaking::current_era();
        register_contract(developer, &contract_id);

        // Do the first bond&stake
        let first_staking_amount = 200;
        bond_and_stake_with_verification(staker_id, &contract_id, first_staking_amount);

        //////////////////////////////////////////////
        ///// FIRST ERA ADVANCEMENT
        //////////////////////////////////////////////

        // Advance eras beyond history depth
        let history_depth = HistoryDepth::get();
        advance_to_era(start_era + history_depth + 1);

        let first_unstake_amount = 30;
        unbond_unstake_and_withdraw_with_verification(
            staker_id,
            &contract_id,
            first_unstake_amount,
        );

        // Verify storage content
        let total_staked = first_staking_amount - first_unstake_amount;
        let current_era = DappsStaking::current_era();

        // Verify storage values related to the current era
        verify_ledger(staker_id, total_staked);
        verify_era_staking_points(
            &contract_id,
            total_staked,
            current_era,
            vec![(staker_id, total_staked)],
        );
        verify_pallet_era_staked(current_era, total_staked);

        // Also ensure that former values still exists even if they're beyond 'history depth'
        verify_era_staking_points(
            &contract_id,
            first_staking_amount,
            start_era,
            vec![(staker_id, first_staking_amount)],
        );
        verify_pallet_era_staked(start_era, first_staking_amount);

        //////////////////////////////////////////////
        ///// SECOND ERA ADVANCEMENT
        //////////////////////////////////////////////

        // Advance era again beyond the history depth
        let former_era = current_era;
        advance_to_era(former_era + history_depth + 10);
        let current_era = DappsStaking::current_era();

        let second_unstake_amount = 30;
        unbond_unstake_and_withdraw_with_verification(
            staker_id,
            &contract_id,
            second_unstake_amount,
        );

        // Verify storage content
        let former_total_staked = total_staked;
        let total_staked = total_staked - second_unstake_amount;

        // Verify storage values related to the current era
        verify_ledger(staker_id, total_staked);
        verify_era_staking_points(
            &contract_id,
            total_staked,
            current_era,
            vec![(staker_id, total_staked)],
        );
        verify_pallet_era_staked(current_era, total_staked);

        // Also ensure that former values still exists even if they're beyond 'history depth', again
        verify_era_staking_points(
            &contract_id,
            former_total_staked,
            former_era,
            vec![(staker_id, former_total_staked)],
        );
        verify_pallet_era_staked(former_era, former_total_staked);
    })
}

#[test]
fn unbond_unstake_and_withdraw_contract_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let unstake_value = 100;

        // Contract isn't registered, expect an error.
        let evm_contract = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_noop!(
            DappsStaking::bond_and_stake(Origin::signed(staker_id), evm_contract, unstake_value),
            Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn unbond_unstake_and_withdraw_unstake_not_possible() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let first_staker_id = 1;
        let first_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
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
        let second_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));
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
fn new_era_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // set initial era index
        run_to_block(BLOCKS_PER_ERA * 42 + 1);
        let starting_era = mock::DappsStaking::current_era();

        // verify that block reward is zero at the beginning of an era
        assert!(mock::DappsStaking::block_reward_accumulator().is_zero());

        // Increment block by setting it to the first block in era value
        run_for_blocks(1);
        let current_era = mock::DappsStaking::current_era();
        assert_eq!(starting_era, current_era);

        // verify that block reward is added to the block_reward_accumulator
        let block_reward = mock::DappsStaking::block_reward_accumulator();
        assert_eq!(BLOCK_REWARD, block_reward);

        // register and bond to verify storage item
        let staker = 2;
        let developer = 3;
        const STAKED_AMOUNT: Balance = 100;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));
        register_contract(developer, &contract);
        bond_and_stake_with_verification(staker, &contract, STAKED_AMOUNT);

        // increment (BLOCKS_PER_ERA - 1) more blocks to end the era
        // CurrentEra should be incremented
        // block_reward_accumulator should be reset to 0
        for _ in 1..=(BLOCKS_PER_ERA - 1) {
            run_for_blocks(1);
        }
        let current_era = mock::DappsStaking::current_era();
        assert_eq!(starting_era + 1, current_era);
        System::assert_last_event(mock::Event::DappsStaking(Event::NewDappStakingEra(
            starting_era + 1,
        )));

        // verify that block reward accumulator is reset to 0
        let block_reward = mock::DappsStaking::block_reward_accumulator();
        assert!(block_reward.is_zero());

        let expected_era_reward = get_total_reward_per_era();
        // verify that .staked is copied and .reward is added
        verify_pallet_era_staked_and_reward(starting_era, STAKED_AMOUNT, expected_era_reward);
    })
}

#[test]
fn new_era_forcing() {
    ExternalityBuilder::build().execute_with(|| {
        run_to_block(BLOCKS_PER_ERA * 3 + 1);
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
            DappsStaking::claim(Origin::signed(claimer), contract),
            Error::<TestRuntime>::ContractNotRegistered
        );
    })
}

#[test]
fn claim_nothing_to_claim() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer1 = 1;
        let claimer = 2;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

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
        initialize_first_block();

        let developer = 1;
        let claimer = 2;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        let start_era = DappsStaking::current_era();

        register_contract(developer, &contract);
        bond_and_stake_with_verification(claimer, &contract, 100);

        advance_to_era(3);

        let claim_era: EraIndex = DappsStaking::current_era();
        claim(claimer, contract, start_era, claim_era.clone());

        assert_noop!(
            DappsStaking::claim(Origin::signed(claimer), contract),
            Error::<TestRuntime>::AlreadyClaimedInThisEra
        );
    })
}

#[test]
fn claim_after_history_depth_has_passed_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let claimer = 2;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        let start_era = DappsStaking::current_era();
        register_contract(developer, &contract);
        bond_and_stake_with_verification(claimer, &contract, 100);

        let history_depth = HistoryDepth::get();

        advance_to_era(start_era + history_depth + 1);

        let upper_claim_era = DappsStaking::current_era();
        let lower_claim_era = upper_claim_era - history_depth;
        claim(claimer, contract, lower_claim_era, upper_claim_era.clone());

        verify_contract_history_is_cleared(contract, start_era, upper_claim_era);

        // Expect that all rewards from one era are deposited into treasury
        let treasury_id = <TestRuntime as Config>::TreasuryPalletId::get().into_account();
        assert_eq!(
            get_total_reward_per_era(),
            <TestRuntime as Config>::Currency::free_balance(&treasury_id)
        );
    })
}

#[test]
fn claim_after_unclaimed_history_depth_has_passed_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let claimer = 2;
        let treasury_id = <TestRuntime as Config>::TreasuryPalletId::get().into_account();
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        let start_era = DappsStaking::current_era();
        register_contract(developer, &contract);
        bond_and_stake_with_verification(claimer, &contract, 100);

        // Get and calculate history information
        let history_depth = HistoryDepth::get();
        let unclaimed_history_depth = DappsStaking::get_unclaimed_reward_history_limit();
        let history_delta = unclaimed_history_depth - history_depth;

        // Take snapshot of free balance prior to claim
        let treasury_starting_balance =
            <TestRuntime as Config>::Currency::free_balance(&treasury_id);
        let developer_starting_balance =
            <TestRuntime as Config>::Currency::free_balance(&developer);
        let claimer_starting_balance = <TestRuntime as Config>::Currency::free_balance(&claimer);

        // Advance eras so we move past unclaimed history depth. Some rewards must be slashed now.
        advance_to_era(start_era + unclaimed_history_depth + 1);

        let upper_claim_era = DappsStaking::current_era();
        let lower_claim_era = upper_claim_era.saturating_sub(history_depth).max(1);
        claim(claimer, contract, lower_claim_era, upper_claim_era.clone());

        verify_contract_history_is_cleared(contract, start_era, upper_claim_era);

        // Calculate how much was earned by everyone
        let treasury_earned = <TestRuntime as Config>::Currency::free_balance(&treasury_id)
            - treasury_starting_balance;
        let developer_earned = <TestRuntime as Config>::Currency::free_balance(&developer)
            - developer_starting_balance;
        let claimer_earned =
            <TestRuntime as Config>::Currency::free_balance(&claimer) - claimer_starting_balance;

        // Now assert that reward distribution is as expected
        let reward_per_era = get_total_reward_per_era();

        // Both dev and claimer should have earned the rewards for each of the eras in the 'history depth'
        assert_eq!(
            reward_per_era * history_depth as u128,
            developer_earned + claimer_earned
        );

        // Past the history depth, we enter 'unclaimed history depth' where we claim rewards for the treasury
        assert_eq!(reward_per_era * history_delta as u128, treasury_earned);
    })
}

#[test]
fn claim_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let claimer = 2;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));
        const SKIP_ERA: EraIndex = 3;

        let start_era = DappsStaking::current_era();

        register_contract(developer, &contract);
        bond_and_stake_with_verification(claimer, &contract, 100);

        advance_to_era(start_era + SKIP_ERA);

        let claim_era = DappsStaking::current_era();
        claim(claimer, contract, start_era, claim_era.clone());
        verify_contract_history_is_cleared(contract, start_era, claim_era);

        // Nothing should be deposited into treasury as unclaimed reward
        let treasury_id = <TestRuntime as Config>::TreasuryPalletId::get().into_account();
        assert!(<TestRuntime as Config>::Currency::free_balance(&treasury_id).is_zero());
    })
}

#[test]
fn claim_one_contract_one_staker() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker1 = 2;

        const STAKE_AMOUNT1: mock::Balance = 1000;
        const INITIAL_STAKE: mock::Balance = 1000;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));
        const SKIP_ERA: EraIndex = 4;

        // Store initial free balaces of the developer and the stakers
        let free_balance_staker1 = <mock::TestRuntime as Config>::Currency::free_balance(&staker1);

        // Register contracts, bond&stake them with two stakers on the contract.
        let start_era = DappsStaking::current_era();
        register_contract(developer, &contract);
        let free_developer_balance =
            <mock::TestRuntime as Config>::Currency::free_balance(&developer);
        bond_and_stake_with_verification(staker1, &contract, STAKE_AMOUNT1);

        // Advance some eras to be able to claim rewards. Verify storage is consolidated
        advance_to_era(start_era + SKIP_ERA);
        let claim_era = DappsStaking::current_era();
        claim(staker1, contract, start_era, claim_era.clone());
        verify_contract_history_is_cleared(contract, start_era, claim_era);
        let eras_eligible_for_reward = (claim_era - start_era) as u128;
        // calculate reward per stakers
        let total_era_dapps_reward = get_total_reward_per_era();
        let expected_staker1_reward = calc_expected_staker_reward(
            total_era_dapps_reward,
            INITIAL_STAKE,
            INITIAL_STAKE,
            STAKE_AMOUNT1,
        );

        // calculate reward per developer
        let expected_developer_reward =
            calc_expected_developer_reward(total_era_dapps_reward, INITIAL_STAKE, INITIAL_STAKE);

        // check balances to see if the rewards are paid out
        check_rewards_on_balance_and_storage(
            &contract,
            &staker1,
            free_balance_staker1,
            eras_eligible_for_reward as EraIndex,
            expected_staker1_reward,
        );
        check_rewards_on_balance_and_storage(
            &contract,
            &developer,
            free_developer_balance,
            eras_eligible_for_reward as EraIndex,
            expected_developer_reward,
        );
        let expected_contract_reward =
            eras_eligible_for_reward * (expected_staker1_reward + expected_developer_reward);
        check_paidout_rewards_for_contract(&contract, expected_contract_reward);
    })
}

#[test]
fn claim_one_contract_two_stakers() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker1 = 2;
        let staker2 = 3;

        const STAKE_AMOUNT1: mock::Balance = 400;
        const STAKE_AMOUNT2: mock::Balance = 600;
        const INITIAL_STAKE: mock::Balance = 1000;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));
        const SKIP_ERA: EraIndex = 2;

        // Store initial free balaces of the developer and the stakers
        let free_balance_staker1 = <mock::TestRuntime as Config>::Currency::free_balance(&staker1);
        let free_balance_staker2 = <mock::TestRuntime as Config>::Currency::free_balance(&staker2);

        // Register contracts, bond&stake them with two stakers on the contract.
        let start_era = DappsStaking::current_era();
        register_contract(developer, &contract);
        let free_developer_balance =
            <mock::TestRuntime as Config>::Currency::free_balance(&developer);
        bond_and_stake_with_verification(staker1, &contract, STAKE_AMOUNT1);
        bond_and_stake_with_verification(staker2, &contract, STAKE_AMOUNT2);

        // Advance some eras to be able to claim rewards. Verify storage is consolidated
        advance_to_era(start_era + SKIP_ERA);
        let claim_era = DappsStaking::current_era();
        claim(staker1, contract, start_era, claim_era.clone());
        verify_contract_history_is_cleared(contract, start_era, claim_era);

        // calculate reward per stakers
        let total_era_dapps_reward = get_total_reward_per_era();
        let expected_staker1_reward = calc_expected_staker_reward(
            total_era_dapps_reward,
            INITIAL_STAKE,
            INITIAL_STAKE,
            STAKE_AMOUNT1,
        );
        let expected_staker2_reward = calc_expected_staker_reward(
            total_era_dapps_reward,
            INITIAL_STAKE,
            INITIAL_STAKE,
            STAKE_AMOUNT2,
        );

        // calculate reward per developer
        let expected_developer_reward =
            calc_expected_developer_reward(total_era_dapps_reward, INITIAL_STAKE, INITIAL_STAKE);

        // check balances to see if the rewards are paid out
        let eras_eligible_for_reward = (claim_era - start_era) as u128;
        check_rewards_on_balance_and_storage(
            &contract,
            &staker1,
            free_balance_staker1,
            eras_eligible_for_reward as EraIndex,
            expected_staker1_reward,
        );
        check_rewards_on_balance_and_storage(
            &contract,
            &staker2,
            free_balance_staker2,
            eras_eligible_for_reward as EraIndex,
            expected_staker2_reward,
        );
        check_rewards_on_balance_and_storage(
            &contract,
            &developer,
            free_developer_balance,
            eras_eligible_for_reward as EraIndex,
            expected_developer_reward,
        );
        let expected_contract_reward = eras_eligible_for_reward
            * (expected_staker1_reward + expected_staker2_reward + expected_developer_reward);
        check_paidout_rewards_for_contract(&contract, expected_contract_reward);
    })
}

// claim_two_contracts_three_stakers() test will exercise following scenario
// era=2
//      register(contract1)
//      register(contract2)
//      bond_and_stake(staker1, &contract1, STAKER1_AMOUNT)
//      bond_and_stake(staker2, &contract1, STAKER2_AMOUNT1)
// era=5
//      bond_and_stake(staker2, &contract2, STAKER2_AMOUNT2);
//      bond_and_stake(staker3, &contract2, STAKER3_AMOUNT);
// era=7
//      claim(staker1, contract1); claim for era 2 - 6
// era=11
//      claim(staker2, contract2); claim for eras 5 - 10

#[test]
fn claim_two_contracts_three_stakers() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer1 = 1;
        let developer2 = 10;
        let staker1: mock::AccountId = 2;
        let staker2: mock::AccountId = 3; // will stake on 2 contracts
        let staker3: mock::AccountId = 4;
        const STAKER1_AMOUNT: mock::Balance = 400;
        const STAKER2_AMOUNT1: mock::Balance = 600;
        const STAKER2_AMOUNT2: mock::Balance = 100;
        const STAKER3_AMOUNT: mock::Balance = 400;
        const CONTRACT1_STAKE: mock::Balance = 1000;
        const CONTRACT2_STAKE: mock::Balance = 500;
        const ERA_STAKED1: mock::Balance = 1000; // 400 + 600
        const ERA_STAKED2: mock::Balance = 1500; // 1000 + 100 + 400
        let contract1 = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let contract2 = MockSmartContract::Evm(H160::repeat_byte(0x02));
        const SKIP_ERA: EraIndex = 3;

        // Store initial free balaces of developers and stakers
        let free_balance_staker1 = <mock::TestRuntime as Config>::Currency::free_balance(&staker1);
        let free_balance_staker2 = <mock::TestRuntime as Config>::Currency::free_balance(&staker2);
        let free_balance_staker3 = <mock::TestRuntime as Config>::Currency::free_balance(&staker3);

        // Register 2 contracts, bond&stake with two stakers on first contract.
        // era=2
        //      register(contract1)
        //      register(contract2)
        //      bond_and_stake(staker1, &contract1, STAKER1_AMOUNT)
        //      bond_and_stake(staker2, &contract1, STAKER2_AMOUNT1)
        let start_era = DappsStaking::current_era();
        register_contract(developer1, &contract1);
        register_contract(developer2, &contract2);
        let free_balance_developer1 =
            <mock::TestRuntime as Config>::Currency::free_balance(&developer1);
        let free_balance_developer2 =
            <mock::TestRuntime as Config>::Currency::free_balance(&developer2);
        bond_and_stake_with_verification(staker1, &contract1, STAKER1_AMOUNT);
        bond_and_stake_with_verification(staker2, &contract1, STAKER2_AMOUNT1);

        // Advance eras and then bond&stake with two stakers on second contract.
        // era=5
        //      bond_and_stake(staker2, &contract2, STAKER2_AMOUNT2);
        //      bond_and_stake(staker3, &contract2, STAKER3_AMOUNT);
        advance_to_era(start_era + SKIP_ERA);
        let current_era = DappsStaking::current_era();
        let eras_eligible_for_reward1 = (current_era - start_era) as u128;

        let start_staking_era_for_c2 = current_era;
        bond_and_stake_with_verification(staker2, &contract2, STAKER2_AMOUNT2);
        bond_and_stake_with_verification(staker3, &contract2, STAKER3_AMOUNT);

        // Advance era again by one, so rewards can be claimed for previous era as well.
        let skip_another_eras = 2;
        advance_to_era(current_era + skip_another_eras);

        // Claim rewards for first contract and verify storage content is as expected.
        // era=7
        //      claim(staker1, contract1); claim for era 2 - 6
        let mut claim_era: EraIndex = DappsStaking::current_era();
        let eras_eligible_for_reward2 = skip_another_eras as u128;
        claim(staker1, contract1.clone(), start_era, claim_era.clone());
        verify_contract_history_is_cleared(contract1, start_era, claim_era);

        // calculate reward per stakers in contract1
        let expected_era_reward = get_total_reward_per_era();
        let expected_c1_staker1_e1_reward = calc_expected_staker_reward(
            expected_era_reward,
            ERA_STAKED1,
            CONTRACT1_STAKE,
            STAKER1_AMOUNT,
        );
        let expected_c1_staker1_e2_reward = calc_expected_staker_reward(
            expected_era_reward,
            ERA_STAKED2,
            CONTRACT1_STAKE,
            STAKER1_AMOUNT,
        );
        let expected_c1_staker2_e1_reward = calc_expected_staker_reward(
            expected_era_reward,
            ERA_STAKED1,
            CONTRACT1_STAKE,
            STAKER2_AMOUNT1,
        );
        let expected_c1_staker2_e2_reward = calc_expected_staker_reward(
            expected_era_reward,
            ERA_STAKED2,
            CONTRACT1_STAKE,
            STAKER2_AMOUNT1,
        );
        // calculate reward per developer contract 1
        let expected_c1_dev1_e1_reward =
            calc_expected_developer_reward(expected_era_reward, ERA_STAKED1, CONTRACT1_STAKE);
        let expected_c1_dev1_e2_reward =
            calc_expected_developer_reward(expected_era_reward, ERA_STAKED2, CONTRACT1_STAKE);

        let expected_c1_staker1_reward_total = eras_eligible_for_reward1
            * expected_c1_staker1_e1_reward
            + eras_eligible_for_reward2 * expected_c1_staker1_e2_reward;
        check_rewards_on_balance_and_storage(
            &contract1,
            &staker1,
            free_balance_staker1,
            1 as EraIndex, // use 1 since the multiplication with era is alreday done
            expected_c1_staker1_reward_total,
        );
        // staker2 staked on both contracts. Memorize this reward for staker2 on contract1
        let expected_c1_staker2_reward_total = eras_eligible_for_reward1
            * expected_c1_staker2_e1_reward
            + eras_eligible_for_reward2 * expected_c1_staker2_e2_reward;
        check_rewards_on_balance_and_storage(
            &contract1,
            &staker2,
            free_balance_staker2,
            1 as EraIndex, // use 1 since the multiplication with era is alreday done
            expected_c1_staker2_reward_total,
        );

        let expected_c1_developer1_reward_total = eras_eligible_for_reward1
            * expected_c1_dev1_e1_reward
            + eras_eligible_for_reward2 * expected_c1_dev1_e2_reward;
        check_rewards_on_balance_and_storage(
            &contract1,
            &developer1,
            free_balance_developer1,
            1 as EraIndex, // use 1 since the multiplication with era is alreday done
            expected_c1_developer1_reward_total,
        );
        let expected_contract1_reward = expected_c1_staker1_reward_total
            + expected_c1_staker2_reward_total
            + expected_c1_developer1_reward_total;
        check_paidout_rewards_for_contract(&contract1, expected_contract1_reward);

        // claim rewards for contract2  4 eras later
        // era=11
        //      claim(staker2, contract2); claim for eras 5 - 10
        advance_to_era(claim_era + 4);
        claim_era = DappsStaking::current_era();

        claim(
            staker2,
            contract2.clone(),
            start_staking_era_for_c2,
            claim_era.clone(),
        );

        // calculate reward per stakers in contract2
        let expected_c2_staker2_e2_reward = calc_expected_staker_reward(
            expected_era_reward,
            ERA_STAKED2,
            CONTRACT2_STAKE,
            STAKER2_AMOUNT2,
        );
        let expected_c2_staker3_e2_reward = calc_expected_staker_reward(
            expected_era_reward,
            ERA_STAKED2,
            CONTRACT2_STAKE,
            STAKER3_AMOUNT,
        );

        // calculate reward per developer
        let expected_c2_dev2_e2_reward =
            calc_expected_developer_reward(expected_era_reward, ERA_STAKED2, CONTRACT2_STAKE);

        let eras_eligible_for_reward = (claim_era - start_staking_era_for_c2) as u128; // all skipped eras plus era when it was last claimed

        // check balances to see if the rewards are paid out
        let expected_c2_staker2_reward_total =
            eras_eligible_for_reward * expected_c2_staker2_e2_reward;
        assert_eq!(
            <mock::TestRuntime as Config>::Currency::free_balance(&staker2),
            free_balance_staker2
                + expected_c2_staker2_reward_total
                + expected_c1_staker2_reward_total
        );

        // we do not use check_rewards_on_balance_and_storage() here since
        // this counter check is for the contract2 only.
        // It does not include reward for the contract1
        assert_eq!(
            mock::DappsStaking::rewards_claimed(contract2, staker2),
            expected_c2_staker2_reward_total
        );

        check_rewards_on_balance_and_storage(
            &contract2,
            &staker3,
            free_balance_staker3,
            eras_eligible_for_reward as EraIndex,
            expected_c2_staker3_e2_reward,
        );

        check_rewards_on_balance_and_storage(
            &contract2,
            &developer2,
            free_balance_developer2,
            eras_eligible_for_reward as EraIndex,
            expected_c2_dev2_e2_reward,
        );
        let expected_contract2_reward = eras_eligible_for_reward
            * (expected_c2_staker3_e2_reward + expected_c2_dev2_e2_reward)
            + expected_c2_staker2_reward_total;
        check_paidout_rewards_for_contract(&contract2, expected_contract2_reward);
    })
}
