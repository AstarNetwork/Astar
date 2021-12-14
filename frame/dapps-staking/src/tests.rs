use super::{pallet::pallet::Error, Event, *};
use frame_support::{
    assert_noop, assert_ok,
    traits::{OnInitialize, OnUnbalanced},
};
use mock::{Balances, MockSmartContract, *};
use sp_core::H160;
use sp_runtime::{traits::Zero, Perbill};

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
        let era_rewards = EraRewardsAndStakes::<TestRuntime>::get(current_era).unwrap();
        assert_eq!(0, era_rewards.staked);
        assert_eq!(0, era_rewards.rewards);
    })
}

#[test]
fn staking_info_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_register(10, &contract_id);

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
        assert_bond_and_stake(staker_1, &contract_id, amount);
        assert_bond_and_stake(staker_2, &contract_id, amount);

        let mid_era = 7;
        advance_to_era(mid_era);
        assert_unbond_and_unstake(staker_2, &contract_id, amount);
        assert_bond_and_stake(staker_3, &contract_id, amount);

        let final_era = 12;
        advance_to_era(final_era);

        // Checks

        // Check first interval
        for era in starting_era..mid_era {
            let contract_info = DappsStaking::contract_staking_info(&contract_id, era);
            assert_eq!(2, contract_info.number_of_stakers);

            assert_eq!(
                amount,
                DappsStaking::staker_staking_info(&staker_1, &contract_id, era).staked
            );
            assert_eq!(
                amount,
                DappsStaking::staker_staking_info(&staker_2, &contract_id, era).staked
            );
        }

        // Check second interval
        for era in mid_era..=final_era {
            let contract_info = DappsStaking::contract_staking_info(&contract_id, era);
            assert_eq!(2, contract_info.number_of_stakers);

            assert_eq!(
                amount,
                DappsStaking::staker_staking_info(&staker_1, &contract_id, era).staked
            );
            assert_eq!(
                amount,
                DappsStaking::staker_staking_info(&staker_3, &contract_id, era).staked
            );
        }

        // Check that before starting era nothing exists
        let staking_info = DappsStaking::contract_staking_info(&contract_id, starting_era - 1);
        assert!(staking_info.number_of_stakers.is_zero());

        // TODO: Do we want such behavior?
        // Era hasn't happened yet but value is returned as if it has happened
        let overflow_era = final_era + 1;
        let staking_info = DappsStaking::contract_staking_info(&contract_id, overflow_era);
        assert_eq!(2, staking_info.number_of_stakers);
        assert_eq!(
            amount,
            DappsStaking::staker_staking_info(&staker_1, &contract_id, overflow_era).staked
        );
        assert_eq!(
            amount,
            DappsStaking::staker_staking_info(&staker_3, &contract_id, overflow_era).staked
        );
    })
}

#[test]
fn register_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        assert!(<TestRuntime as Config>::Currency::reserved_balance(&developer).is_zero());
        assert_register(developer, &contract_id);
        System::assert_last_event(mock::Event::DappsStaking(Event::NewContract(
            developer,
            contract_id,
        )));

        let dapp_info = RegisteredDapps::<TestRuntime>::get(&contract_id).unwrap();
        assert_eq!(dapp_info.state, DAppState::Registered);
        assert_eq!(dapp_info.developer, developer);

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

        assert_register(developer, &contract1);

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

        assert_register(developer1, &contract);

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

        assert_register(developer, &contract_id);
        assert_unregister(developer, &contract_id);
        assert!(<TestRuntime as Config>::Currency::reserved_balance(&developer).is_zero());

        // Not possible to unregister a contract twice
        assert_noop!(
            DappsStaking::unregister(Origin::signed(developer), contract_id.clone()),
            Error::<TestRuntime>::NotOperatedContract
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

        assert_register(developer_1, &first_contract_id);

        // Try to unregister contract with developer who hasn't registered any contract
        assert_noop!(
            DappsStaking::unregister(Origin::signed(developer_2), first_contract_id.clone()),
            Error::<TestRuntime>::NotOwnedContract
        );

        // Register second contract with second dev and then try to unregister it using the first developer
        assert_register(developer_2, &second_contract_id);
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
        assert_register(developer, &contract_id);
        assert_bond_and_stake(staker, &contract_id, 100);
        assert_unbond_and_unstake(staker, &contract_id, 10);

        // Unregister contract and verify that stake & unstake no longer work
        assert_unregister(developer, &contract_id);

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
fn withdraw_from_unregistered_is_ok() {
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
        assert_register(developer, &contract_id);
        assert_register(dummy_developer, &dummy_contract_id);
        assert_bond_and_stake(staker_1, &contract_id, staked_value_1);
        assert_bond_and_stake(staker_2, &contract_id, staked_value_2);

        // This contract will just exist so it helps us with testing ledger content
        assert_bond_and_stake(staker_1, &dummy_contract_id, staked_value_1);

        // Advance eras. This will accumulate some rewards.
        advance_to_era(5);

        assert_unregister(developer, &contract_id);

        // Unbond everything from the contract.
        assert_withdraw_from_unregistered(staker_1, &contract_id);
        assert_withdraw_from_unregistered(staker_2, &contract_id);

        // Claim should still work for past eras
        for era in 1..DappsStaking::current_era() {
            assert_claim(staker_1, contract_id.clone(), era);
        }
    })
}

#[test]
fn withdraw_from_unregistered_when_contract_doesnt_exist() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_noop!(
            DappsStaking::withdraw_from_unregistered(Origin::signed(1), contract_id),
            Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn withdraw_from_unregistered_when_contract_is_still_registered() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_register(developer, &contract_id);

        assert_noop!(
            DappsStaking::withdraw_from_unregistered(Origin::signed(1), contract_id),
            Error::<TestRuntime>::NotUnregisteredContract
        );
    })
}

#[test]
fn withdraw_from_unregistered_when_nothing_is_staked() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_register(developer, &contract_id);

        let staker = 2;
        let no_staker = 3;
        assert_bond_and_stake(staker, &contract_id, 100);

        assert_unregister(developer, &contract_id);

        // No staked amount so call should fail.
        assert_noop!(
            DappsStaking::withdraw_from_unregistered(Origin::signed(no_staker), contract_id),
            Error::<TestRuntime>::NotStakedContract
        );

        // Call should fail if called twice since no staked funds remain.
        assert_withdraw_from_unregistered(staker, &contract_id);
        assert_noop!(
            DappsStaking::withdraw_from_unregistered(Origin::signed(staker), contract_id),
            Error::<TestRuntime>::NotStakedContract
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
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_register(20, &contract_id);

        // initially, storage values should be None
        let current_era = DappsStaking::current_era();
        assert!(ContractEraStake::<TestRuntime>::get(&contract_id, current_era).is_none());

        assert_bond_and_stake(staker_id, &contract_id, 100);

        advance_to_era(current_era + 2);

        // Stake and bond again on the same contract but using a different amount.
        assert_bond_and_stake(staker_id, &contract_id, 300);
    })
}

#[test]
fn bond_and_stake_two_different_contracts_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let first_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let second_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));

        // Insert contracts under registered contracts. Don't use the staker Id.
        assert_register(5, &first_contract_id);
        assert_register(6, &second_contract_id);

        // Stake on both contracts.
        assert_bond_and_stake(staker_id, &first_contract_id, 100);
        assert_bond_and_stake(staker_id, &second_contract_id, 300);
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

        // Insert a contract under registered contracts.
        assert_register(10, &contract_id);

        // Both stakers stake on the same contract, expect a pass.
        assert_bond_and_stake(first_staker_id, &contract_id, first_stake_value);
        assert_bond_and_stake(second_staker_id, &contract_id, second_stake_value);
    })
}

#[test]
fn bond_and_stake_different_value_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // Insert a contract under registered contracts.
        assert_register(20, &contract_id);

        // Bond&stake almost the entire available balance of the staker.
        let staker_free_balance =
            Balances::free_balance(&staker_id).saturating_sub(MINIMUM_REMAINING_AMOUNT);
        assert_bond_and_stake(staker_id, &contract_id, staker_free_balance - 1);

        // Bond&stake again with less than existential deposit but this time expect a pass
        // since we're only increasing the already staked amount.
        assert_bond_and_stake(staker_id, &contract_id, 1);

        // Bond&stake more than what's available in funds. Verify that only what's available is bonded&staked.
        let staker_id = 2;
        let staker_free_balance = Balances::free_balance(&staker_id);
        assert_bond_and_stake(staker_id, &contract_id, staker_free_balance + 1);

        // Verify the minimum transferable amount of stakers account
        let transferable_balance =
            Balances::free_balance(&staker_id) - Ledger::<TestRuntime>::get(staker_id).locked;
        assert_eq!(MINIMUM_REMAINING_AMOUNT, transferable_balance);

        // Bond&stake some amount, a bit less than free balance
        let staker_id = 3;
        let staker_free_balance =
            Balances::free_balance(&staker_id).saturating_sub(MINIMUM_REMAINING_AMOUNT);
        assert_bond_and_stake(staker_id, &contract_id, staker_free_balance - 200);

        // Try to bond&stake more than we have available (since we already locked most of the free balance).
        assert_bond_and_stake(staker_id, &contract_id, 500);
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
        assert_register(developer, &contract_id);

        // Do the first bond&stake
        assert_bond_and_stake(staker_id, &contract_id, 200);

        // Advance eras beyond history depth
        let history_depth = HistoryDepth::get();
        advance_to_era(start_era + history_depth + 1);

        // Bond&stake again
        assert_bond_and_stake(staker_id, &contract_id, 350);
    })
}

#[test]
fn bond_and_stake_on_unregistered_contract() {
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
        assert_register(20, &contract_id);

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
        assert_register(10, &contract_id);

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
        assert_register(10, &contract_id);
        assert_bond_and_stake(staker_id, &contract_id, original_staked_value);
        advance_to_era(old_era + 1);
        let new_era = DappsStaking::current_era();

        // Unstake such an amount so there will remain staked funds on the contract
        let unstaked_value = 100;
        assert_unbond_and_unstake(staker_id, &contract_id, unstaked_value);

        // Verify era staking info
        let new_staked_value = original_staked_value - unstaked_value;
        verify_contract_staking_info(
            &contract_id,
            new_staked_value,
            new_era,
            vec![(staker_id, new_staked_value)],
        );
        // Also verify that the storage values for the old era haven't been changed due to unstaking
        verify_contract_staking_info(
            &contract_id,
            original_staked_value,
            old_era,
            vec![(staker_id, original_staked_value)],
        );

        // Unbond yet again, but don't advance era
        // Unstake such an amount so there will remain staked funds on the contract
        let unstaked_value = 50;
        assert_unbond_and_unstake(staker_id, &contract_id, unstaked_value);
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
        assert_register(10, &contract_id);
        assert_bond_and_stake(staker_id, &contract_id, staked_value);

        // Unstake such an amount that exactly minimum staking amount will remain staked.
        assert_unbond_and_unstake(staker_id, &contract_id, first_value_to_unstake);

        // Unstake 1 token and expect that the entire staked amount will be unstaked.
        assert_unbond_and_unstake(staker_id, &contract_id, 1);
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
        assert_register(10, &contract_id);
        assert_bond_and_stake(first_staker_id, &contract_id, staked_value);
        assert_bond_and_stake(second_staker_id, &contract_id, staked_value);
        let total_staked_value = 2 * staked_value;

        // Advance era, unbond&withdraw with first staker, verify that it was successful
        let current_era = DappsStaking::current_era();
        advance_to_era(current_era + 10);
        let current_era = DappsStaking::current_era();

        let first_unstake_value = 100;
        assert_unbond_and_unstake(first_staker_id, &contract_id, first_unstake_value);

        // Verify that storage values are as expected for both stakers and total staked value
        let new_total_staked = total_staked_value - first_unstake_value;
        let first_staked_value = staked_value - first_unstake_value;
        verify_contract_staking_info(
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
        assert_unbond_and_unstake(second_staker_id, &contract_id, second_unstake_value);

        // Verify that storage values are as expected for both stakers and total staked value
        let new_total_staked = new_total_staked - second_unstake_value;
        let second_staked_value = staked_value - second_unstake_value;
        verify_contract_staking_info(
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
        assert_register(developer, &contract_id);

        // Do the first bond&stake
        let first_staking_amount = 200;
        assert_bond_and_stake(staker_id, &contract_id, first_staking_amount);

        // Advance eras beyond history depth
        let history_depth = HistoryDepth::get();
        advance_to_era(start_era + history_depth + 1);

        let first_unstake_amount = 30;
        assert_unbond_and_unstake(staker_id, &contract_id, first_unstake_amount);

        // Advance era again beyond the history depth
        advance_to_era(DappsStaking::current_era() + history_depth + 10);

        let second_unstake_amount = 30;
        assert_unbond_and_unstake(staker_id, &contract_id, second_unstake_amount);
    })
}

#[test]
fn unbond_and_unstake_in_same_era_can_exceed_max_chunks() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_register(10, &contract_id);

        let staker = 1;

        assert_bond_and_stake(staker, &contract_id, 200 * MAX_UNLOCKING_CHUNKS as Balance);

        // Ensure that we can unbond up to a limited amount of time.
        for _ in 0..MAX_UNLOCKING_CHUNKS * 2 {
            assert_unbond_and_unstake(1, &contract_id, 10);
            assert_eq!(1, Ledger::<TestRuntime>::get(&staker).unbonding_info.len());
        }
    })
}

#[test]
fn unbond_and_unstake_with_zero_value_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_register(10, &contract_id);

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
        assert_register(10, &contract_id);

        let staker = 1;
        let unstake_amount = 10;
        let stake_amount =
            MINIMUM_STAKING_AMOUNT * 10 + unstake_amount * MAX_UNLOCKING_CHUNKS as Balance;

        assert_bond_and_stake(staker, &contract_id, stake_amount);

        // Ensure that we can unbond up to a limited amount of time.
        for _ in 0..MAX_UNLOCKING_CHUNKS {
            advance_to_era(DappsStaking::current_era() + 1);
            assert_unbond_and_unstake(staker, &contract_id, unstake_amount);
        }

        // Ensure that we're at the max but can still add new chunks since it should be merged with the existing one
        assert_eq!(
            MAX_UNLOCKING_CHUNKS,
            DappsStaking::ledger(&staker).unbonding_info.len()
        );
        assert_unbond_and_unstake(staker, &contract_id, unstake_amount);

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
        assert_register(10, &contract_id);

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
        assert_register(10, &contract_id);

        let staker_id = 1;
        assert_bond_and_stake(staker_id, &contract_id, 100);

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
        assert_register(10, &contract_id);

        let staker_id = 1;
        assert_bond_and_stake(staker_id, &contract_id, 1000);

        let first_unbond_value = 75;
        let second_unbond_value = 39;
        let initial_era = DappsStaking::current_era();

        // Unbond some amount in the initial era
        assert_unbond_and_unstake(staker_id, &contract_id, first_unbond_value);

        // Advance one era and then unbond some more
        advance_to_era(initial_era + 1);
        assert_unbond_and_unstake(staker_id, &contract_id, second_unbond_value);

        // Now advance one era before first chunks finishes the unbonding process
        advance_to_era(initial_era + UNBONDING_PERIOD);
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
        assert_register(10, &contract_id);

        let staker_id = 1;
        assert_bond_and_stake(staker_id, &contract_id, 1000);

        // Repeatedly start unbonding and advance era to create unlocking chunks
        let init_unbonding_amount = 15;
        for x in 1..=MAX_UNLOCKING_CHUNKS {
            assert_unbond_and_unstake(staker_id, &contract_id, init_unbonding_amount * x as u128);
            advance_to_era(DappsStaking::current_era() + 1);
        }

        // Now clean up all that are eligible for cleanu-up
        assert_withdraw_unbonded(staker_id);

        // This is a sanity check for the test. Some chunks should remain, otherwise test isn't testing realistic unbonding period.
        assert!(!Ledger::<TestRuntime>::get(&staker_id)
            .unbonding_info
            .is_empty());

        while !Ledger::<TestRuntime>::get(&staker_id)
            .unbonding_info
            .is_empty()
        {
            advance_to_era(DappsStaking::current_era() + 1);
            assert_withdraw_unbonded(staker_id);
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
        assert_register(10, &contract_id);

        let staker_id = 1;
        assert_bond_and_stake(staker_id, &contract_id, 100);
        assert_unbond_and_unstake(staker_id, &contract_id, 20);

        // Try to withdraw but expect an error since current era hasn't passed yet
        assert_noop!(
            DappsStaking::withdraw_unbonded(Origin::signed(staker_id)),
            Error::<TestRuntime>::NothingToWithdraw,
        );

        // Advance an era and expect successful withdrawal
        advance_to_era(DappsStaking::current_era() + 1);
        assert_withdraw_unbonded(staker_id);
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
        assert_register(developer, &contract);
        assert_bond_and_stake(staker, &contract, staked_amount);

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
        let era_rewards = EraRewardsAndStakes::<TestRuntime>::get(starting_era).unwrap();
        assert_eq!(staked_amount, era_rewards.staked);
        assert_eq!(expected_era_reward, era_rewards.rewards);
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
        assert_eq!(mock::DappsStaking::force_era(), Forcing::NotForcing);

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

        assert_register(developer1, &contract);

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

        assert_register(developer, &contract);
        assert_bond_and_stake(claimer, &contract, 100);

        advance_to_era(DappsStaking::current_era() + 1);

        let claim_era = DappsStaking::current_era() - 1;
        assert_claim(claimer, contract, claim_era);

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

        assert_register(developer1, &contract);
        assert_bond_and_stake(claimer, &contract, 100);

        // Advance past the history depth
        advance_to_era(DappsStaking::current_era() + HistoryDepth::get() + 1);
        let current_era = DappsStaking::current_era();

        // All eras must be claimable
        for era in (current_era - HistoryDepth::get())..current_era {
            assert_claim(claimer, contract.clone(), era);
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

        assert_register(developer, &contract);
        assert_bond_and_stake(claimer, &contract, 100);

        advance_to_era(start_era + 3);

        let issuance_before_claim = <TestRuntime as Config>::Currency::total_issuance();
        let claim_era = DappsStaking::current_era() - 1;

        assert_claim(claimer, contract, claim_era);

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
        assert_register(developer, &contract);
        assert_bond_and_stake(staker, &contract, stake_amount_1);

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
            assert_claim(staker, contract.clone(), era);
        }

        // Advance some more eras
        advance_to_era(unregistered_era + 5);
        let current_era = DappsStaking::current_era();
        for era in unregistered_era..current_era {
            assert_noop!(
                DappsStaking::claim(Origin::signed(developer), contract.clone(), era),
                Error::<TestRuntime>::NotOperatedContract,
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
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // Register contracts, bond&stake them with two stakers on the contract.
        let start_era = DappsStaking::current_era();
        assert_register(developer, &contract);

        assert_bond_and_stake(staker1, &contract, stake_amount_1);

        // Advance some eras to be able to claim rewards. Verify storage is consolidated
        advance_to_era(start_era + 1);

        assert_claim(staker1, contract, start_era);
        assert_claim(developer, contract, start_era);
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
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // Register contracts, bond&stake them with two stakers on the contract.
        assert_register(developer, &contract);
        assert_bond_and_stake(staker1, &contract, stake_amount_1);
        assert_bond_and_stake(staker2, &contract, stake_amount_2);

        // Advance some eras to be able to claim rewards. Verify storage is consolidated
        advance_to_era(DappsStaking::current_era() + 3);
        let claim_era = DappsStaking::current_era() - 1;
        assert_claim(staker1, contract, claim_era);
        assert_claim(staker2, contract, claim_era);
        assert_claim(developer, contract, claim_era);
    })
}

#[test]
fn claim_two_contracts_three_stakers() {
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

        // Register 2 contracts, bond&stake with two stakers on first contract.
        let start_era = DappsStaking::current_era();
        assert_register(developer1, &contract1);
        assert_register(developer2, &contract2);
        assert_bond_and_stake(staker1, &contract1, staker_1_amount);
        assert_bond_and_stake(staker2, &contract1, staker_2_amount_1);
        let first_claim_era = start_era;

        // Advance eras and then bond&stake with two stakers on second contract.
        advance_to_era(start_era + 3);

        assert_bond_and_stake(staker2, &contract2, staker_2_amount_2);
        assert_bond_and_stake(staker3, &contract2, staker_3_amount);

        // Advance era again by one, so rewards can be claimed for previous era as well.
        let current_era = DappsStaking::current_era();
        let second_claim_era = current_era;
        advance_to_era(current_era + 1);

        // Claim first contract rewards.
        for era in first_claim_era..DappsStaking::current_era() {
            assert_claim(developer1, contract1.clone(), era);
            assert_claim(staker1, contract1.clone(), era);
            assert_claim(staker2, contract1.clone(), era);
        }

        // Claim second contract rewards.
        for era in second_claim_era..DappsStaking::current_era() {
            assert_claim(developer2, contract2.clone(), era);
            assert_claim(staker2, contract2.clone(), era);
            assert_claim(staker3, contract2.clone(), era);
        }
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

#[test]
fn account_ledger_is_empty() {
    let mut account_ledger = AccountLedger::<MockSmartContract<AccountId>, Balance>::default();
    assert!(account_ledger.is_empty());

    account_ledger.locked = 1;
    assert!(!account_ledger.is_empty());

    let mut account_ledger = AccountLedger::<MockSmartContract<AccountId>, Balance>::default();
    account_ledger.unbonding_info.add(UnlockingChunk::default());
    assert!(!account_ledger.is_empty());
}

#[test]
fn account_ledger_contract_staked() {
    let mut account_ledger = AccountLedger::<MockSmartContract<AccountId>, Balance>::default();
    assert!(account_ledger.staked_contracts.is_empty());

    let contract_1 = MockSmartContract::<AccountId>::Evm(H160::repeat_byte(0x01));
    let contract_2 = MockSmartContract::<AccountId>::Evm(H160::repeat_byte(0x02));
    let contract_3 = MockSmartContract::<AccountId>::Evm(H160::repeat_byte(0x03));

    // Inform that contract was staked. Repeating the op shouldn't affect anything.
    for _ in 1..5 {
        account_ledger.contract_staked(&contract_1);
        assert_eq!(account_ledger.staked_contracts.len(), 1 as usize);
        assert_eq!(account_ledger.staked_contracts[&contract_1], None);
    }

    account_ledger.contract_staked(&contract_2);
    assert_eq!(account_ledger.staked_contracts.len(), 2 as usize);
    account_ledger.contract_staked(&contract_3);
    assert_eq!(account_ledger.staked_contracts.len(), 3 as usize);

    assert_eq!(account_ledger.staked_contracts[&contract_1], None);
    assert_eq!(account_ledger.staked_contracts[&contract_2], None);
    assert_eq!(account_ledger.staked_contracts[&contract_3], None);
}

#[test]
fn account_ledger_contract_unstaked() {
    let mut account_ledger = AccountLedger::<MockSmartContract<AccountId>, Balance>::default();

    let contract_1 = MockSmartContract::<AccountId>::Evm(H160::repeat_byte(0x01));
    let contract_2 = MockSmartContract::<AccountId>::Evm(H160::repeat_byte(0x02));
    let contract_3 = MockSmartContract::<AccountId>::Evm(H160::repeat_byte(0x03));

    let first_era = 10;
    let history_depth = 15;

    account_ledger.contract_staked(&contract_1);
    account_ledger.contract_staked(&contract_2);
    account_ledger.contract_staked(&contract_3);

    // Unstake first contract, nothing should be removed
    account_ledger.contract_unstaked(&contract_1, first_era, history_depth);
    assert_eq!(account_ledger.staked_contracts.len(), 3 as usize);
    assert_eq!(account_ledger.staked_contracts[&contract_1], Some(10));
    assert_eq!(account_ledger.staked_contracts[&contract_2], None);
    assert_eq!(account_ledger.staked_contracts[&contract_3], None);

    // Second era is just one era shy of the first contract being removable
    let second_era = first_era + history_depth - 1;
    account_ledger.contract_unstaked(&contract_2, second_era, history_depth);
    assert_eq!(account_ledger.staked_contracts.len(), 3 as usize);
    assert_eq!(account_ledger.staked_contracts[&contract_1], Some(10));
    assert_eq!(
        account_ledger.staked_contracts[&contract_2],
        Some(second_era)
    );
    assert_eq!(account_ledger.staked_contracts[&contract_3], None);

    // Third era is just enough for the first contract to be removed from history
    let third_era = second_era + 1;
    account_ledger.contract_unstaked(&contract_3, third_era, history_depth);
    assert_eq!(account_ledger.staked_contracts.len(), 2 as usize);
    assert_eq!(
        account_ledger.staked_contracts[&contract_2],
        Some(second_era)
    );
    assert_eq!(
        account_ledger.staked_contracts[&contract_3],
        Some(third_era)
    );
}

#[test]
fn developer_staker_split() {
    // Normal example
    let contract_info = EraStakingPoints::<Balance> {
        total: 200,
        number_of_stakers: 10,
    };
    let reward_and_stake = EraRewardAndStake::<Balance> {
        rewards: 150,
        staked: 600,
    };
    let dev_percentage = Perbill::from_percent(25);
    let contract_reward = Perbill::from_rational(contract_info.total, reward_and_stake.staked)
        * reward_and_stake.rewards;

    let (dev_reward, staker_reward) =
        DappsStaking::dev_stakers_split(&contract_info, &reward_and_stake, &dev_percentage);
    assert_eq!(dev_reward, dev_percentage * contract_reward);
    assert_eq!(
        staker_reward,
        contract_reward - dev_percentage * contract_reward
    );

    // Dev percentage is 0%
    let dev_percentage = Perbill::from_percent(0);
    let (dev_reward, staker_reward) =
        DappsStaking::dev_stakers_split(&contract_info, &reward_and_stake, &dev_percentage);
    assert_eq!(dev_reward, 0);
    assert_eq!(staker_reward, contract_reward);

    // Dev percentage is 100%
    let dev_percentage = Perbill::from_percent(100);
    let (dev_reward, staker_reward) =
        DappsStaking::dev_stakers_split(&contract_info, &reward_and_stake, &dev_percentage);
    assert_eq!(dev_reward, contract_reward);
    assert_eq!(staker_reward, 0);

    // Semi-normal scenario where contract isn't staked at all
    let dev_percentage = Perbill::from_percent(80);
    let empty_contract_info = EraStakingPoints::<Balance>::default();
    let (dev_reward, staker_reward) =
        DappsStaking::dev_stakers_split(&empty_contract_info, &reward_and_stake, &dev_percentage);
    assert_eq!(dev_reward, 0);
    assert_eq!(staker_reward, 0);
}
