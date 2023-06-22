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

use super::{pallet::pallet::Error, pallet::pallet::Event, *};
use frame_support::{assert_noop, assert_ok, traits::OnInitialize, weights::Weight};
use mock::{Balances, MockSmartContract, *};
use sp_core::H160;
use sp_runtime::{
    traits::{BadOrigin, Zero},
    Perbill,
};

use testing_utils::*;

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
fn rewards_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // At the beginning, both should be 0
        assert_eq!(
            BlockRewardAccumulator::<TestRuntime>::get(),
            Default::default()
        );
        assert!(free_balance_of_dapps_staking_account().is_zero());

        // After handling imbalance, accumulator and account should be updated
        let dapps_reward = 12345;
        let stakers_reward = 9999;
        let total_reward = dapps_reward + stakers_reward;
        DappsStaking::rewards(
            Balances::issue(stakers_reward),
            Balances::issue(dapps_reward),
        );

        assert_eq!(total_reward, free_balance_of_dapps_staking_account());
        let reward_accumulator = BlockRewardAccumulator::<TestRuntime>::get();
        assert_eq!(reward_accumulator.stakers, stakers_reward);
        assert_eq!(reward_accumulator.dapps, dapps_reward);

        // After triggering a new era, accumulator should be set to 0 but account shouldn't consume any new imbalance
        DappsStaking::on_initialize(System::block_number());
        assert_eq!(
            BlockRewardAccumulator::<TestRuntime>::get(),
            Default::default()
        );
        assert_eq!(total_reward, free_balance_of_dapps_staking_account());
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

        let previous_era = current_era;
        advance_to_era(previous_era + 10);

        // Check that all reward&stakes are as expected
        let current_era = DappsStaking::current_era();
        for era in 1..current_era {
            let reward_info = GeneralEraInfo::<TestRuntime>::get(era).unwrap().rewards;
            assert_eq!(
                get_total_reward_per_era(),
                reward_info.stakers + reward_info.dapps
            );
        }
        // Current era rewards should be 0
        let era_rewards = GeneralEraInfo::<TestRuntime>::get(current_era).unwrap();
        assert_eq!(0, era_rewards.staked);
        assert_eq!(era_rewards.rewards, Default::default());
    })
}

#[test]
fn new_era_length_is_always_blocks_per_era() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();
        let blocks_per_era = mock::BLOCKS_PER_ERA;

        // go to beginning of an era
        advance_to_era(mock::DappsStaking::current_era() + 1);

        // record era number and block number
        let start_era = mock::DappsStaking::current_era();
        let starting_block_number = System::block_number();

        // go to next era
        advance_to_era(mock::DappsStaking::current_era() + 1);
        let ending_block_number = System::block_number();

        // make sure block number difference is is blocks_per_era
        assert_eq!(mock::DappsStaking::current_era(), start_era + 1);
        assert_eq!(ending_block_number - starting_block_number, blocks_per_era);
    })
}

#[test]
fn new_era_is_handled_with_maintenance_mode() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        // enable maintenance mode
        assert_ok!(DappsStaking::maintenance_mode(RuntimeOrigin::root(), true));
        assert!(PalletDisabled::<TestRuntime>::exists());
        System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::MaintenanceMode(
            true,
        )));

        // advance 9 blocks or 3 era lengths (advance_to_era() doesn't work in maintenance mode)
        run_for_blocks(mock::BLOCKS_PER_ERA * 3);

        // verify that `current block > NextEraStartingBlock` but era hasn't changed
        assert!(System::block_number() > DappsStaking::next_era_starting_block());
        assert_eq!(DappsStaking::current_era(), 1);

        // disable maintenance mode
        assert_ok!(DappsStaking::maintenance_mode(RuntimeOrigin::root(), false));
        System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::MaintenanceMode(
            false,
        )));

        // advance one era
        run_for_blocks(mock::BLOCKS_PER_ERA);

        // verify we're at block 14
        assert_eq!(System::block_number(), (4 * mock::BLOCKS_PER_ERA) + 2); // 2 from initialization, advanced 4 eras worth of blocks

        // verify era was updated and NextEraStartingBlock is 15
        assert_eq!(DappsStaking::current_era(), 2);
        assert_eq!(
            DappsStaking::next_era_starting_block(),
            (5 * mock::BLOCKS_PER_ERA)
        );
    })
}

#[test]
fn new_forced_era_length_is_always_blocks_per_era() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();
        let blocks_per_era = mock::BLOCKS_PER_ERA;

        // go to beginning of an era
        advance_to_era(mock::DappsStaking::current_era() + 1);

        // go to middle of era
        run_for_blocks(1); // can be any number between 0 and blocks_per_era

        // force new era
        <ForceEra<TestRuntime>>::put(Forcing::ForceNew);
        run_for_blocks(1); // calls on_initialize()

        // note the start block number of new (forced) era
        let start_block_number = System::block_number();

        // go to start of next era
        advance_to_era(mock::DappsStaking::current_era() + 1);

        // show the length of the forced era is equal to blocks_per_era
        let end_block_number = System::block_number();
        assert_eq!(end_block_number - start_block_number, blocks_per_era);
    })
}

#[test]
fn new_era_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // set initial era index
        advance_to_era(DappsStaking::current_era() + 10);
        let starting_era = DappsStaking::current_era();

        // verify that block reward is zero at the beginning of an era
        assert_eq!(DappsStaking::block_reward_accumulator(), Default::default());

        // Increment block by setting it to the first block in era value
        run_for_blocks(1);
        let current_era = DappsStaking::current_era();
        assert_eq!(starting_era, current_era);

        // verify that block reward is added to the block_reward_accumulator
        let block_reward = DappsStaking::block_reward_accumulator();
        assert_eq!(
            joint_block_reward(),
            block_reward.stakers + block_reward.dapps
        );

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
        System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::NewDappStakingEra(
            starting_era + 1,
        )));

        // verify that block reward accumulator is reset to 0
        let block_reward = DappsStaking::block_reward_accumulator();
        assert_eq!(block_reward, Default::default());

        let expected_era_reward = get_total_reward_per_era();
        let expected_dapps_reward = DAPP_BLOCK_REWARD * BLOCKS_PER_ERA as Balance;
        let expected_stakers_reward = STAKER_BLOCK_REWARD * BLOCKS_PER_ERA as Balance;

        // verify that .staked is copied and .reward is added
        let era_rewards = GeneralEraInfo::<TestRuntime>::get(starting_era).unwrap();
        assert_eq!(staked_amount, era_rewards.staked);
        assert_eq!(
            expected_era_reward,
            era_rewards.rewards.dapps + era_rewards.rewards.stakers
        );
        assert_eq!(expected_dapps_reward, era_rewards.rewards.dapps);
        assert_eq!(expected_stakers_reward, era_rewards.rewards.stakers);
    })
}

#[test]
fn new_era_forcing() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();
        advance_to_era(3);
        let starting_era = mock::DappsStaking::current_era();

        // call on_initialize. It is not last block in the era, but it should increment the era
        <ForceEra<TestRuntime>>::put(Forcing::ForceNew);
        run_for_blocks(1);

        // check that era is incremented
        let current = mock::DappsStaking::current_era();
        assert_eq!(starting_era + 1, current);

        // check that forcing is cleared
        assert_eq!(mock::DappsStaking::force_era(), Forcing::NotForcing);

        // check the event for the new era
        System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::NewDappStakingEra(
            starting_era + 1,
        )));
    })
}

#[test]
fn general_staker_info_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let first_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_register(10, &first_contract_id);

        let second_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));
        assert_register(11, &second_contract_id);

        let (staker_1, staker_2, staker_3) = (1, 2, 3);
        let amount = 100;

        let starting_era = 3;
        advance_to_era(starting_era);
        assert_bond_and_stake(staker_1, &first_contract_id, amount);
        assert_bond_and_stake(staker_2, &first_contract_id, amount);

        let mid_era = 7;
        advance_to_era(mid_era);
        assert_unbond_and_unstake(staker_2, &first_contract_id, amount);
        assert_bond_and_stake(staker_3, &first_contract_id, amount);
        assert_bond_and_stake(staker_3, &second_contract_id, amount);

        let final_era = 12;
        advance_to_era(final_era);

        // Check first interval
        let mut first_staker_info = DappsStaking::staker_info(&staker_1, &first_contract_id);
        let mut second_staker_info = DappsStaking::staker_info(&staker_2, &first_contract_id);
        let mut third_staker_info = DappsStaking::staker_info(&staker_3, &first_contract_id);

        for era in starting_era..mid_era {
            let contract_info = DappsStaking::contract_stake_info(&first_contract_id, era).unwrap();
            assert_eq!(2, contract_info.number_of_stakers);

            assert_eq!((era, amount), first_staker_info.claim());
            assert_eq!((era, amount), second_staker_info.claim());

            assert!(!ContractEraStake::<TestRuntime>::contains_key(
                &second_contract_id,
                era
            ));
        }

        // Check second interval
        for era in mid_era..=final_era {
            let first_contract_info =
                DappsStaking::contract_stake_info(&first_contract_id, era).unwrap();
            assert_eq!(2, first_contract_info.number_of_stakers);

            assert_eq!((era, amount), first_staker_info.claim());
            assert_eq!((era, amount), third_staker_info.claim());

            assert_eq!(
                DappsStaking::contract_stake_info(&second_contract_id, era)
                    .unwrap()
                    .number_of_stakers,
                1
            );
        }

        // Check that before starting era nothing exists
        assert!(!ContractEraStake::<TestRuntime>::contains_key(
            &first_contract_id,
            starting_era - 1
        ));
        assert!(!ContractEraStake::<TestRuntime>::contains_key(
            &second_contract_id,
            starting_era - 1
        ));
    })
}

#[test]
fn register_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let ok_contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        assert!(<TestRuntime as Config>::Currency::reserved_balance(&developer).is_zero());
        assert_register(developer, &ok_contract);
        System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::NewContract(
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
fn register_with_non_root_fails() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let ok_contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        assert_noop!(
            DappsStaking::register(RuntimeOrigin::signed(developer), developer, ok_contract),
            BadOrigin
        );
    })
}

#[test]
fn register_twice_with_same_account_fails() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let contract1 = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let contract2 = MockSmartContract::Evm(H160::repeat_byte(0x02));

        assert_register(developer, &contract1);

        System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::NewContract(
            developer, contract1,
        )));

        // now register different contract with same account
        assert_noop!(
            DappsStaking::register(RuntimeOrigin::root(), developer, contract2),
            Error::<TestRuntime>::AlreadyUsedDeveloperAccount
        );
    })
}

#[test]
fn register_same_contract_twice_fails() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer1 = 1;
        let developer2 = 2;
        let contract = MockSmartContract::Evm(H160::repeat_byte(0x01));

        assert_register(developer1, &contract);

        System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::NewContract(
            developer1, contract,
        )));

        // now register same contract by different developer
        assert_noop!(
            DappsStaking::register(RuntimeOrigin::root(), developer2, contract),
            Error::<TestRuntime>::AlreadyRegisteredContract
        );
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
            DappsStaking::unregister(RuntimeOrigin::root(), contract_id.clone()),
            Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn unregister_with_non_root() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        assert_register(developer, &contract_id);

        // Not possible to unregister if caller isn't root
        assert_noop!(
            DappsStaking::unregister(RuntimeOrigin::signed(developer), contract_id.clone()),
            BadOrigin
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
            DappsStaking::bond_and_stake(RuntimeOrigin::signed(staker), contract_id.clone(), 100),
            Error::<TestRuntime>::NotOperatedContract
        );
        assert_noop!(
            DappsStaking::unbond_and_unstake(
                RuntimeOrigin::signed(staker),
                contract_id.clone(),
                100
            ),
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

        // Claim all past rewards
        for era in 1..DappsStaking::current_era() {
            assert_claim_staker(staker_1, &contract_id);
            assert_claim_staker(staker_2, &contract_id);
            assert_claim_dapp(&contract_id, era);
        }

        // Unbond everything from the contract.
        assert_withdraw_from_unregistered(staker_1, &contract_id);
        assert_withdraw_from_unregistered(staker_2, &contract_id);

        // No additional claim ops should be possible
        assert_noop!(
            DappsStaking::claim_staker(RuntimeOrigin::signed(staker_1), contract_id.clone()),
            Error::<TestRuntime>::NotStakedContract
        );
        assert_noop!(
            DappsStaking::claim_staker(RuntimeOrigin::signed(staker_2), contract_id.clone()),
            Error::<TestRuntime>::NotStakedContract
        );
        assert_noop!(
            DappsStaking::claim_dapp(
                RuntimeOrigin::signed(developer),
                contract_id.clone(),
                DappsStaking::current_era()
            ),
            Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn withdraw_from_unregistered_when_contract_doesnt_exist() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_noop!(
            DappsStaking::withdraw_from_unregistered(RuntimeOrigin::signed(1), contract_id),
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
            DappsStaking::withdraw_from_unregistered(RuntimeOrigin::signed(1), contract_id),
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
            DappsStaking::withdraw_from_unregistered(RuntimeOrigin::signed(no_staker), contract_id),
            Error::<TestRuntime>::NotStakedContract
        );

        // Call should fail if called twice since no staked funds remain.
        assert_withdraw_from_unregistered(staker, &contract_id);
        assert_noop!(
            DappsStaking::withdraw_from_unregistered(RuntimeOrigin::signed(staker), contract_id),
            Error::<TestRuntime>::NotStakedContract
        );
    })
}

#[test]
fn withdraw_from_unregistered_when_unclaimed_rewards_remaining() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_register(developer, &contract_id);

        let staker = 2;
        assert_bond_and_stake(staker, &contract_id, 100);

        // Advance eras. This will accumulate some rewards.
        advance_to_era(5);

        assert_unregister(developer, &contract_id);

        for _ in 1..DappsStaking::current_era() {
            assert_noop!(
                DappsStaking::withdraw_from_unregistered(
                    RuntimeOrigin::signed(staker),
                    contract_id
                ),
                Error::<TestRuntime>::UnclaimedRewardsRemaining
            );
            assert_claim_staker(staker, &contract_id);
        }

        // Withdraw should work after all rewards have been claimed
        assert_withdraw_from_unregistered(staker, &contract_id);
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
        assert!(DappsStaking::contract_stake_info(&contract_id, current_era).is_none());

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
fn bond_and_stake_on_unregistered_contract_fails() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let stake_value = 100;

        // Check not registered contract. Expect an error.
        let evm_contract = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_noop!(
            DappsStaking::bond_and_stake(
                RuntimeOrigin::signed(staker_id),
                evm_contract,
                stake_value
            ),
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
                RuntimeOrigin::signed(staker_id),
                contract_id.clone(),
                MINIMUM_STAKING_AMOUNT - 1
            ),
            Error::<TestRuntime>::InsufficientValue
        );

        // Now bond&stake the entire stash so we lock all the available funds.
        let staker_free_balance = Balances::free_balance(&staker_id);
        assert_bond_and_stake(staker_id, &contract_id, staker_free_balance);

        // Now try to bond&stake some additional funds and expect an error since we cannot bond&stake 0.
        assert_noop!(
            DappsStaking::bond_and_stake(RuntimeOrigin::signed(staker_id), contract_id.clone(), 1),
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
            assert_bond_and_stake(staker_id.into(), &contract_id, 100);
        }

        // Now try to stake with an additional staker and expect an error.
        assert_noop!(
            DappsStaking::bond_and_stake(
                RuntimeOrigin::signed((1 + MAX_NUMBER_OF_STAKERS).into()),
                contract_id.clone(),
                100
            ),
            Error::<TestRuntime>::MaxNumberOfStakersExceeded
        );
    })
}

#[test]
fn bond_and_stake_too_many_era_stakes() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        // Insert a contract under registered contracts.
        assert_register(10, &contract_id);

        // Stake with MAX_NUMBER_OF_STAKERS - 1 on the same contract. It must work.
        let start_era = DappsStaking::current_era();
        for offset in 1..MAX_ERA_STAKE_VALUES {
            assert_bond_and_stake(staker_id, &contract_id, 100);
            advance_to_era(start_era + offset);
        }

        // Now try to stake with an additional staker and expect an error.
        assert_noop!(
            DappsStaking::bond_and_stake(RuntimeOrigin::signed(staker_id), contract_id, 100),
            Error::<TestRuntime>::TooManyEraStakeValues
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

        // Unstake such an amount so there will remain staked funds on the contract
        let unstaked_value = 100;
        assert_unbond_and_unstake(staker_id, &contract_id, unstaked_value);

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

        let (first_staker_id, second_staker_id) = (1, 2);
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let staked_value = 500;

        // Insert a contract under registered contracts, bond&stake it with two different stakers.
        assert_register(10, &contract_id);
        assert_bond_and_stake(first_staker_id, &contract_id, staked_value);
        assert_bond_and_stake(second_staker_id, &contract_id, staked_value);

        // Advance era, unbond&withdraw with first staker, verify that it was successful
        advance_to_era(DappsStaking::current_era() + 10);
        let current_era = DappsStaking::current_era();
        assert_unbond_and_unstake(first_staker_id, &contract_id, 100);

        // Advance era, unbond with second staker and verify storage values are as expected
        advance_to_era(current_era + 10);
        assert_unbond_and_unstake(second_staker_id, &contract_id, 333);
    })
}

#[test]
fn unbond_and_unstake_calls_in_same_era_can_exceed_max_chunks() {
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
            DappsStaking::unbond_and_unstake(RuntimeOrigin::signed(1), contract_id, 0),
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
            DappsStaking::unbond_and_unstake(RuntimeOrigin::signed(1), contract_id, 100),
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
                RuntimeOrigin::signed(staker),
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
            DappsStaking::unbond_and_unstake(RuntimeOrigin::signed(1), contract_id, 10),
            Error::<TestRuntime>::NotStakedContract,
        );
    })
}

#[test]
fn unbond_and_unstake_too_many_era_stakes() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_register(10, &contract_id);

        // Fill up the `EraStakes` vec
        let start_era = DappsStaking::current_era();
        for offset in 1..MAX_ERA_STAKE_VALUES {
            assert_bond_and_stake(staker_id, &contract_id, 100);
            advance_to_era(start_era + offset);
        }

        // At this point, we have max allowed amount of `EraStake` values so we cannot create
        // an additional one.
        assert_noop!(
            DappsStaking::unbond_and_unstake(RuntimeOrigin::signed(staker_id), contract_id, 10),
            Error::<TestRuntime>::TooManyEraStakeValues
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
            DappsStaking::unbond_and_unstake(
                RuntimeOrigin::signed(staker_id),
                contract_id.clone(),
                20
            ),
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
        advance_to_era(initial_era + UNBONDING_PERIOD - 1);
        assert_noop!(
            DappsStaking::withdraw_unbonded(RuntimeOrigin::signed(staker_id)),
            Error::<TestRuntime>::NothingToWithdraw
        );

        // Advance one additional era and expect that the first chunk can be withdrawn
        advance_to_era(DappsStaking::current_era() + 1);
        assert_ok!(DappsStaking::withdraw_unbonded(RuntimeOrigin::signed(
            staker_id
        ),));
        System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::Withdrawn(
            staker_id,
            first_unbond_value,
        )));

        // Advance one additional era and expect that the first chunk can be withdrawn
        advance_to_era(DappsStaking::current_era() + 1);
        assert_ok!(DappsStaking::withdraw_unbonded(RuntimeOrigin::signed(
            staker_id
        ),));
        System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::Withdrawn(
            staker_id,
            second_unbond_value,
        )));

        // Advance one additional era but since we have nothing else to withdraw, expect an error
        advance_to_era(initial_era + UNBONDING_PERIOD - 1);
        assert_noop!(
            DappsStaking::withdraw_unbonded(RuntimeOrigin::signed(staker_id)),
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

        // Now clean up all that are eligible for clean-up
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
            DappsStaking::withdraw_unbonded(RuntimeOrigin::signed(1)),
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
            DappsStaking::withdraw_unbonded(RuntimeOrigin::signed(staker_id)),
            Error::<TestRuntime>::NothingToWithdraw,
        );

        // Advance an era and expect successful withdrawal
        advance_to_era(DappsStaking::current_era() + 1);
        assert_withdraw_unbonded(staker_id);
    })
}

#[test]
fn nomination_transfer_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let origin_developer = 1;
        let target_developer = 2;
        let staker = 3;
        let origin_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let target_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));

        assert_register(origin_developer, &origin_contract_id);
        assert_register(target_developer, &target_contract_id);
        assert_bond_and_stake(staker, &origin_contract_id, MINIMUM_STAKING_AMOUNT * 2);

        // The first transfer will ensure that both contracts are staked after operation is complete
        assert_nomination_transfer(
            staker,
            &origin_contract_id,
            MINIMUM_STAKING_AMOUNT,
            &target_contract_id,
        );
        assert!(
            !GeneralStakerInfo::<TestRuntime>::get(&staker, &origin_contract_id)
                .latest_staked_value()
                .is_zero()
        );

        // The second operation should fully unstake origin contract since it takes it below minimum staking amount
        assert_nomination_transfer(
            staker,
            &origin_contract_id,
            MINIMUM_STAKING_AMOUNT,
            &target_contract_id,
        );
        assert!(
            GeneralStakerInfo::<TestRuntime>::get(&staker, &origin_contract_id)
                .latest_staked_value()
                .is_zero()
        );
    })
}

#[test]
fn nomination_transfer_to_same_contract_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();
        let developer = 1;
        let staker = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        assert_register(developer, &contract_id);

        assert_noop!(
            DappsStaking::nomination_transfer(
                RuntimeOrigin::signed(staker),
                contract_id,
                100,
                contract_id,
            ),
            Error::<TestRuntime>::NominationTransferToSameContract
        );
    })
}

#[test]
fn nomination_transfer_to_inactive_contracts_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let origin_developer = 1;
        let target_developer = 2;
        let staker = 3;
        let origin_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let target_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));

        // 1. Neither contract is registered
        assert_noop!(
            DappsStaking::nomination_transfer(
                RuntimeOrigin::signed(staker),
                origin_contract_id,
                100,
                target_contract_id,
            ),
            Error::<TestRuntime>::NotOperatedContract
        );

        // 2. Only first contract is registered
        assert_register(origin_developer, &origin_contract_id);
        assert_noop!(
            DappsStaking::nomination_transfer(
                RuntimeOrigin::signed(staker),
                origin_contract_id,
                100,
                target_contract_id,
            ),
            Error::<TestRuntime>::NotOperatedContract
        );

        // 3. Both are registered but then target contract gets unregistered
        assert_register(target_developer, &target_contract_id);
        assert_bond_and_stake(staker, &origin_contract_id, 100);
        assert_nomination_transfer(staker, &origin_contract_id, 100, &target_contract_id);

        assert_unregister(target_developer, &target_contract_id);
        assert_noop!(
            DappsStaking::nomination_transfer(
                RuntimeOrigin::signed(staker),
                origin_contract_id,
                100,
                target_contract_id,
            ),
            Error::<TestRuntime>::NotOperatedContract
        );

        // 4. Origin contract is unregistered
        assert_unregister(origin_developer, &origin_contract_id);
        assert_noop!(
            DappsStaking::nomination_transfer(
                RuntimeOrigin::signed(staker),
                origin_contract_id,
                100,
                target_contract_id,
            ),
            Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn nomination_transfer_from_not_staked_contract() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let origin_developer = 1;
        let target_developer = 2;
        let staker = 3;
        let origin_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let target_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));

        assert_register(origin_developer, &origin_contract_id);
        assert_register(target_developer, &target_contract_id);

        assert_noop!(
            DappsStaking::nomination_transfer(
                RuntimeOrigin::signed(staker),
                origin_contract_id.clone(),
                20,
                target_contract_id.clone()
            ),
            Error::<TestRuntime>::NotStakedContract
        );
    })
}

#[test]
fn nomination_transfer_with_no_value() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let origin_developer = 1;
        let target_developer = 2;
        let staker = 3;
        let origin_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let target_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));

        assert_register(origin_developer, &origin_contract_id);
        assert_register(target_developer, &target_contract_id);
        assert_bond_and_stake(staker, &origin_contract_id, 100);

        assert_noop!(
            DappsStaking::nomination_transfer(
                RuntimeOrigin::signed(staker),
                origin_contract_id.clone(),
                Zero::zero(),
                target_contract_id.clone()
            ),
            Error::<TestRuntime>::UnstakingWithNoValue
        );
    })
}

#[test]
fn nomination_transfer_with_insufficient_value() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let origin_developer = 1;
        let target_developer = 2;
        let staker = 3;
        let origin_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let target_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));

        assert_register(origin_developer, &origin_contract_id);
        assert_register(target_developer, &target_contract_id);
        assert_bond_and_stake(staker, &origin_contract_id, 100);

        assert_noop!(
            DappsStaking::nomination_transfer(
                RuntimeOrigin::signed(staker),
                origin_contract_id.clone(),
                MINIMUM_STAKING_AMOUNT - 1,
                target_contract_id.clone()
            ),
            Error::<TestRuntime>::InsufficientValue
        );
    })
}

#[test]
fn nomination_transfer_contracts_have_too_many_era_stake_values() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let origin_developer = 1;
        let target_developer = 2;
        let staker = 3;
        let origin_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let target_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));

        assert_register(origin_developer, &origin_contract_id);
        assert_register(target_developer, &target_contract_id);

        // Ensure we fill up era stakes vector
        for _ in 1..MAX_ERA_STAKE_VALUES {
            // We use bond&stake since its only limiting factor is max era stake values
            assert_bond_and_stake(staker, &origin_contract_id, 15);
            advance_to_era(DappsStaking::current_era() + 1);
        }
        assert_noop!(
            DappsStaking::bond_and_stake(
                RuntimeOrigin::signed(staker),
                origin_contract_id.clone(),
                15
            ),
            Error::<TestRuntime>::TooManyEraStakeValues
        );

        // Ensure it's not possible to transfer from origin contract since it's era stake values are maxed
        assert_noop!(
            DappsStaking::nomination_transfer(
                RuntimeOrigin::signed(staker),
                origin_contract_id.clone(),
                15,
                target_contract_id.clone()
            ),
            Error::<TestRuntime>::TooManyEraStakeValues
        );

        // Swap origin and target to verify that same is true if target contract era stake values is maxed out
        let (origin_contract_id, target_contract_id) = (target_contract_id, origin_contract_id);
        assert_bond_and_stake(staker, &origin_contract_id, 15);
        assert_noop!(
            DappsStaking::nomination_transfer(
                RuntimeOrigin::signed(staker),
                origin_contract_id.clone(),
                15,
                target_contract_id.clone()
            ),
            Error::<TestRuntime>::TooManyEraStakeValues
        );
    })
}

#[test]
fn nomination_transfer_max_number_of_stakers_exceeded() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let origin_developer = 1;
        let target_developer = 2;
        // This one will only stake on origin contract
        let first_staker = 3;
        // This one will stake on both origin and target contracts
        let second_staker = 4;
        let origin_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let target_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));

        // Register contracts and bond&stake them with both stakers
        assert_register(origin_developer, &origin_contract_id);
        assert_register(target_developer, &target_contract_id);

        assert_bond_and_stake(first_staker, &origin_contract_id, 23);
        assert_bond_and_stake(second_staker, &target_contract_id, 37);
        assert_bond_and_stake(second_staker, &target_contract_id, 41);

        // Fill up the second contract with stakers until max number of stakers limit has been reached
        for temp_staker in (second_staker + 1)..(MAX_NUMBER_OF_STAKERS as u64 + second_staker) {
            Balances::resolve_creating(&temp_staker, Balances::issue(100));
            assert_bond_and_stake(temp_staker, &target_contract_id, 13);
        }
        // Sanity check + assurance that first_staker isn't staking on target contract
        assert_noop!(
            DappsStaking::bond_and_stake(
                RuntimeOrigin::signed(first_staker),
                target_contract_id.clone(),
                19
            ),
            Error::<TestRuntime>::MaxNumberOfStakersExceeded
        );

        // Now attempt transfer nomination and expect an error
        assert_noop!(
            DappsStaking::nomination_transfer(
                RuntimeOrigin::signed(first_staker),
                origin_contract_id.clone(),
                19,
                target_contract_id.clone(),
            ),
            Error::<TestRuntime>::MaxNumberOfStakersExceeded
        );
    })
}

#[test]
fn claim_not_staked_contract() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        assert_register(developer, &contract_id);

        assert_noop!(
            DappsStaking::claim_staker(RuntimeOrigin::signed(staker), contract_id),
            Error::<TestRuntime>::NotStakedContract
        );

        advance_to_era(DappsStaking::current_era() + 1);
        assert_noop!(
            DappsStaking::claim_dapp(RuntimeOrigin::signed(developer), contract_id, 1),
            Error::<TestRuntime>::NotStakedContract
        );
    })
}

#[test]
fn claim_not_operated_contract() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        assert_register(developer, &contract_id);
        assert_bond_and_stake(staker, &contract_id, 100);

        // Advance one era and unregister the contract
        advance_to_era(DappsStaking::current_era() + 1);
        assert_unregister(developer, &contract_id);

        // First claim should pass but second should fail because contract was unregistered
        assert_claim_staker(staker, &contract_id);
        assert_noop!(
            DappsStaking::claim_staker(RuntimeOrigin::signed(staker), contract_id),
            Error::<TestRuntime>::NotOperatedContract
        );

        assert_claim_dapp(&contract_id, 1);
        assert_noop!(
            DappsStaking::claim_dapp(RuntimeOrigin::signed(developer), contract_id, 2),
            Error::<TestRuntime>::NotOperatedContract
        );
    })
}

#[test]
fn claim_invalid_era() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        let start_era = DappsStaking::current_era();
        assert_register(developer, &contract_id);
        assert_bond_and_stake(staker, &contract_id, 100);
        advance_to_era(start_era + 5);

        for era in start_era..DappsStaking::current_era() {
            assert_claim_staker(staker, &contract_id);
            assert_claim_dapp(&contract_id, era);
        }

        assert_noop!(
            DappsStaking::claim_staker(RuntimeOrigin::signed(staker), contract_id),
            Error::<TestRuntime>::EraOutOfBounds
        );
        assert_noop!(
            DappsStaking::claim_dapp(
                RuntimeOrigin::signed(developer),
                contract_id,
                DappsStaking::current_era()
            ),
            Error::<TestRuntime>::EraOutOfBounds
        );
    })
}

#[test]
fn claim_dapp_same_era_twice() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        let start_era = DappsStaking::current_era();
        assert_register(developer, &contract_id);
        assert_bond_and_stake(staker, &contract_id, 100);
        advance_to_era(start_era + 1);

        assert_claim_dapp(&contract_id, start_era);
        assert_noop!(
            DappsStaking::claim_dapp(RuntimeOrigin::signed(developer), contract_id, start_era),
            Error::<TestRuntime>::AlreadyClaimedInThisEra
        );
    })
}

#[test]
fn claim_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let first_developer = 1;
        let second_developer = 2;
        let first_staker = 3;
        let second_staker = 4;
        let first_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let second_contract_id = MockSmartContract::Evm(H160::repeat_byte(0x02));

        let start_era = DappsStaking::current_era();

        // Prepare a scenario with different stakes

        assert_register(first_developer, &first_contract_id);
        assert_register(second_developer, &second_contract_id);
        assert_bond_and_stake(first_staker, &first_contract_id, 100);
        assert_bond_and_stake(second_staker, &first_contract_id, 45);

        // Just so ratio isn't 100% in favor of the first contract
        assert_bond_and_stake(first_staker, &second_contract_id, 33);
        assert_bond_and_stake(second_staker, &second_contract_id, 22);

        let eras_advanced = 3;
        advance_to_era(start_era + eras_advanced);

        for x in 0..eras_advanced.into() {
            assert_bond_and_stake(first_staker, &first_contract_id, 20 + x * 3);
            assert_bond_and_stake(second_staker, &first_contract_id, 5 + x * 5);
            advance_to_era(DappsStaking::current_era() + 1);
        }

        // Ensure that all past eras can be claimed
        let current_era = DappsStaking::current_era();
        for era in start_era..current_era {
            assert_claim_staker(first_staker, &first_contract_id);
            assert_claim_dapp(&first_contract_id, era);
            assert_claim_staker(second_staker, &first_contract_id);
        }

        // Shouldn't be possible to claim current era.
        // Also, previous claim calls should have claimed everything prior to current era.
        assert_noop!(
            DappsStaking::claim_staker(
                RuntimeOrigin::signed(first_staker),
                first_contract_id.clone()
            ),
            Error::<TestRuntime>::EraOutOfBounds
        );
        assert_noop!(
            DappsStaking::claim_dapp(
                RuntimeOrigin::signed(first_developer),
                first_contract_id,
                current_era
            ),
            Error::<TestRuntime>::EraOutOfBounds
        );
    })
}

#[test]
fn claim_after_unregister_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        let start_era = DappsStaking::current_era();
        assert_register(developer, &contract_id);
        let stake_value = 100;
        assert_bond_and_stake(staker, &contract_id, stake_value);

        // Advance few eras, then unstake everything
        advance_to_era(start_era + 5);
        assert_unbond_and_unstake(staker, &contract_id, stake_value);
        let full_unstake_era = DappsStaking::current_era();
        let number_of_staking_eras = full_unstake_era - start_era;

        // Few eras pass, then staker stakes again
        advance_to_era(DappsStaking::current_era() + 3);
        let stake_value = 75;
        let restake_era = DappsStaking::current_era();
        assert_bond_and_stake(staker, &contract_id, stake_value);

        // Again, few eras pass then contract is unregistered
        advance_to_era(DappsStaking::current_era() + 3);
        assert_unregister(developer, &contract_id);
        let unregister_era = DappsStaking::current_era();
        let number_of_staking_eras = number_of_staking_eras + unregister_era - restake_era;
        advance_to_era(DappsStaking::current_era() + 2);

        // Ensure that staker can claim all the eras that he had an active stake
        for _ in 0..number_of_staking_eras {
            assert_claim_staker(staker, &contract_id);
        }
        assert_noop!(
            DappsStaking::claim_staker(RuntimeOrigin::signed(staker), contract_id.clone()),
            Error::<TestRuntime>::NotOperatedContract
        );

        // Ensure the same for dapp reward
        for era in start_era..unregister_era {
            if era >= full_unstake_era && era < restake_era {
                assert_noop!(
                    DappsStaking::claim_dapp(
                        RuntimeOrigin::signed(developer),
                        contract_id.clone(),
                        era
                    ),
                    Error::<TestRuntime>::NotStakedContract
                );
            } else {
                assert_claim_dapp(&contract_id, era);
            }
        }
    })
}

#[test]
fn claim_only_payout_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // stake some tokens
        let start_era = DappsStaking::current_era();
        assert_register(developer, &contract_id);
        let stake_value = 100;
        assert_bond_and_stake(staker, &contract_id, stake_value);

        // disable reward restaking
        advance_to_era(start_era + 1);
        assert_set_reward_destination(staker, RewardDestination::FreeBalance);

        // ensure it's claimed correctly
        assert_claim_staker(staker, &contract_id);
    })
}

#[test]
fn claim_with_zero_staked_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        let start_era = DappsStaking::current_era();
        assert_register(developer, &contract_id);

        // stake some tokens and wait for an era
        let stake_value = 100;
        assert_bond_and_stake(staker, &contract_id, stake_value);
        advance_to_era(start_era + 1);

        // ensure reward_destination is set to StakeBalance
        assert_set_reward_destination(staker, RewardDestination::StakeBalance);

        // unstake all the tokens
        assert_unbond_and_unstake(staker, &contract_id, stake_value);

        // ensure claimed value goes to claimer's free balance
        assert_claim_staker(staker, &contract_id);
    })
}

#[test]
fn claims_with_different_reward_destination_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // stake some tokens
        let start_era = DappsStaking::current_era();
        assert_register(developer, &contract_id);
        let stake_value = 100;
        assert_bond_and_stake(staker, &contract_id, stake_value);

        // disable compounding mode, wait 3 eras
        assert_set_reward_destination(staker, RewardDestination::FreeBalance);
        advance_to_era(start_era + 1);
        // ensure staker can claim rewards to wallet
        assert_claim_staker(staker, &contract_id);

        // enable compounding mode, wait 3 eras
        assert_set_reward_destination(staker, RewardDestination::StakeBalance);
        advance_to_era(start_era + 2);
        // ensure staker can claim with compounding
        assert_claim_staker(staker, &contract_id);
    })
}

#[test]
fn claiming_when_stakes_full_without_compounding_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let staker_id = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        // Insert a contract under registered contracts.
        assert_register(10, &contract_id);

        // Stake with MAX_ERA_STAKE_VALUES - 1 on the same contract. It must work.
        let start_era = DappsStaking::current_era();
        for offset in 1..MAX_ERA_STAKE_VALUES {
            assert_bond_and_stake(staker_id, &contract_id, 100);
            advance_to_era(start_era + offset * 5);
        }

        // Make sure reward_destination is set to StakeBalance
        assert_set_reward_destination(staker_id, RewardDestination::StakeBalance);

        // claim and restake once, so there's a claim record for the for the current era in the stakes vec
        assert_claim_staker(staker_id, &contract_id);

        // making another gap in eras and trying to claim and restake would exceed MAX_ERA_STAKE_VALUES
        advance_to_era(DappsStaking::current_era() + 1);
        assert_noop!(
            DappsStaking::claim_staker(RuntimeOrigin::signed(staker_id), contract_id),
            Error::<TestRuntime>::TooManyEraStakeValues
        );

        // set reward_destination to FreeBalance (disable restaking)
        assert_set_reward_destination(staker_id, RewardDestination::FreeBalance);

        // claiming should work again
        assert_claim_staker(staker_id, &contract_id);
    })
}

#[test]
fn changing_reward_destination_for_empty_ledger_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();
        let staker = 1;
        assert_noop!(
            DappsStaking::set_reward_destination(
                RuntimeOrigin::signed(staker),
                RewardDestination::FreeBalance
            ),
            Error::<TestRuntime>::NotActiveStaker
        );
    });
}

#[test]
fn claim_dapp_with_zero_stake_periods_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 2;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // Prepare scenario: <staked eras><not staked eras><staked eras><not staked eras>

        let start_era = DappsStaking::current_era();
        assert_register(developer, &contract_id);
        let stake_value = 100;
        assert_bond_and_stake(staker, &contract_id, stake_value);

        advance_to_era(start_era + 5);
        let first_full_unstake_era = DappsStaking::current_era();
        assert_unbond_and_unstake(staker, &contract_id, stake_value);

        advance_to_era(DappsStaking::current_era() + 7);
        let restake_era = DappsStaking::current_era();
        assert_bond_and_stake(staker, &contract_id, stake_value);

        advance_to_era(DappsStaking::current_era() + 4);
        let second_full_unstake_era = DappsStaking::current_era();
        assert_unbond_and_unstake(staker, &contract_id, stake_value);
        advance_to_era(DappsStaking::current_era() + 10);

        // Ensure that first interval can be claimed
        for era in start_era..first_full_unstake_era {
            assert_claim_dapp(&contract_id, era);
        }

        // Ensure that the empty interval cannot be claimed
        for era in first_full_unstake_era..restake_era {
            assert_noop!(
                DappsStaking::claim_dapp(
                    RuntimeOrigin::signed(developer),
                    contract_id.clone(),
                    era
                ),
                Error::<TestRuntime>::NotStakedContract
            );
        }

        // Ensure that second interval can be claimed
        for era in restake_era..second_full_unstake_era {
            assert_claim_dapp(&contract_id, era);
        }

        // Ensure no more claims are possible since contract was fully unstaked
        assert_noop!(
            DappsStaking::claim_dapp(
                RuntimeOrigin::signed(developer),
                contract_id.clone(),
                second_full_unstake_era
            ),
            Error::<TestRuntime>::NotStakedContract
        );

        // Now stake again and ensure contract can once again be claimed
        let last_claim_era = DappsStaking::current_era();
        assert_bond_and_stake(staker, &contract_id, stake_value);
        advance_to_era(last_claim_era + 1);
        assert_claim_dapp(&contract_id, last_claim_era);
    })
}

#[test]
fn maintenance_mode_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        assert_ok!(DappsStaking::ensure_pallet_enabled());
        assert!(!PalletDisabled::<TestRuntime>::exists());

        assert_ok!(DappsStaking::maintenance_mode(RuntimeOrigin::root(), true));
        assert!(PalletDisabled::<TestRuntime>::exists());
        System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::MaintenanceMode(
            true,
        )));

        let account = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        //
        // 1
        assert_noop!(
            DappsStaking::register(RuntimeOrigin::root(), account, contract_id),
            Error::<TestRuntime>::Disabled
        );
        assert_noop!(
            DappsStaking::unregister(RuntimeOrigin::root(), contract_id),
            Error::<TestRuntime>::Disabled
        );
        assert_noop!(
            DappsStaking::withdraw_from_unregistered(RuntimeOrigin::signed(account), contract_id),
            Error::<TestRuntime>::Disabled
        );

        //
        // 2
        assert_noop!(
            DappsStaking::bond_and_stake(RuntimeOrigin::signed(account), contract_id, 100),
            Error::<TestRuntime>::Disabled
        );
        assert_noop!(
            DappsStaking::unbond_and_unstake(RuntimeOrigin::signed(account), contract_id, 100),
            Error::<TestRuntime>::Disabled
        );
        assert_noop!(
            DappsStaking::claim_dapp(RuntimeOrigin::signed(account), contract_id, 5),
            Error::<TestRuntime>::Disabled
        );
        assert_noop!(
            DappsStaking::claim_staker(RuntimeOrigin::signed(account), contract_id),
            Error::<TestRuntime>::Disabled
        );
        assert_noop!(
            DappsStaking::withdraw_unbonded(RuntimeOrigin::signed(account)),
            Error::<TestRuntime>::Disabled
        );
        assert_noop!(
            DappsStaking::nomination_transfer(
                RuntimeOrigin::signed(account),
                contract_id,
                100,
                contract_id,
            ),
            Error::<TestRuntime>::Disabled
        );

        //
        // 3
        assert_noop!(
            DappsStaking::force_new_era(RuntimeOrigin::root()),
            Error::<TestRuntime>::Disabled
        );
        // shouldn't do anything since we're in maintenance mode
        assert_eq!(DappsStaking::on_initialize(3), Weight::zero());

        //
        // 4
        assert_ok!(DappsStaking::maintenance_mode(RuntimeOrigin::root(), false));
        System::assert_last_event(mock::RuntimeEvent::DappsStaking(Event::MaintenanceMode(
            false,
        )));
        assert_register(account, &contract_id);
    })
}

#[test]
fn maintenance_mode_no_change() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        // Expect an error since maintenance mode is already disabled
        assert_ok!(DappsStaking::ensure_pallet_enabled());
        assert_noop!(
            DappsStaking::maintenance_mode(RuntimeOrigin::root(), false),
            Error::<TestRuntime>::NoMaintenanceModeChange
        );

        // Same for the case when maintenance mode is already enabled
        assert_ok!(DappsStaking::maintenance_mode(RuntimeOrigin::root(), true));
        assert_noop!(
            DappsStaking::maintenance_mode(RuntimeOrigin::root(), true),
            Error::<TestRuntime>::NoMaintenanceModeChange
        );
    })
}

#[test]
fn dev_stakers_split_util() {
    let base_stakers_reward = 7 * 11 * 13 * 17;
    let base_dapps_reward = 19 * 23 * 31;
    let staked_on_contract = 123456;
    let total_staked = staked_on_contract * 3;

    // Prepare structs
    let staking_points = ContractStakeInfo::<Balance> {
        total: staked_on_contract,
        number_of_stakers: 10,
        contract_reward_claimed: false,
    };
    let era_info = EraInfo::<Balance> {
        rewards: RewardInfo {
            dapps: base_dapps_reward,
            stakers: base_stakers_reward,
        },
        staked: total_staked,
        locked: total_staked,
    };

    let (dev_reward, stakers_reward) = DappsStaking::dev_stakers_split(&staking_points, &era_info);

    let contract_stake_ratio = Perbill::from_rational(staked_on_contract, total_staked);
    let calculated_stakers_reward = contract_stake_ratio * base_stakers_reward;
    let calculated_dev_reward = contract_stake_ratio * base_dapps_reward;
    assert_eq!(calculated_dev_reward, dev_reward);
    assert_eq!(calculated_stakers_reward, stakers_reward);

    assert_eq!(
        calculated_stakers_reward + calculated_dev_reward,
        dev_reward + stakers_reward
    );
}

#[test]
pub fn tvl_util_test() {
    ExternalityBuilder::build().execute_with(|| {
        // Ensure TVL is zero before first block and also after first block
        assert!(DappsStaking::tvl().is_zero());
        initialize_first_block();
        assert!(DappsStaking::tvl().is_zero());

        let developer = 1;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_register(developer, &contract_id);

        // Expect TVL to change as we bond&stake more
        let iterations = 10;
        let stake_value = 100;
        for x in 1..=iterations {
            assert_bond_and_stake(developer, &contract_id, stake_value);
            assert_eq!(DappsStaking::tvl(), stake_value * x);
        }

        // Era advancement should have no effect on TVL
        advance_to_era(5);
        assert_eq!(DappsStaking::tvl(), stake_value * iterations);
    })
}

#[test]
pub fn set_contract_stake_info() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));
        assert_register(10, &contract_id);

        let staker_id = 1;
        assert_bond_and_stake(staker_id, &contract_id, 1000);

        // Read current contract stake info, then overwrite it with different value
        let original_contract_stake_info =
            DappsStaking::contract_stake_info(&contract_id, 1).unwrap();
        let mut modified_info = original_contract_stake_info.clone();
        modified_info.total = modified_info.total + 17;
        ContractEraStake::<TestRuntime>::insert(&contract_id, 1, modified_info);

        // Ensure only root can call it
        assert_noop!(
            DappsStaking::set_contract_stake_info(
                RuntimeOrigin::signed(1),
                contract_id.clone(),
                1,
                original_contract_stake_info.clone()
            ),
            BadOrigin
        );

        // Verify we can fix the corrupted stroage
        assert_ne!(
            ContractEraStake::<TestRuntime>::get(&contract_id, 1).unwrap(),
            original_contract_stake_info
        );
        assert_ok!(DappsStaking::set_contract_stake_info(
            RuntimeOrigin::root(),
            contract_id.clone(),
            1,
            original_contract_stake_info.clone()
        ));
        assert_eq!(
            ContractEraStake::<TestRuntime>::get(&contract_id, 1).unwrap(),
            original_contract_stake_info
        );
    })
}

#[test]
fn custom_max_encoded_len() {
    let max_unbonding_info_len = 10 * (4 + 16) + 1;
    assert_eq!(
        UnbondingInfo::<u128>::max_encoded_len(),
        max_unbonding_info_len as usize
    );

    let max_staker_info_len = 10 * (4 + 16) + 1;
    assert_eq!(
        StakerInfo::<u128>::max_encoded_len(),
        max_staker_info_len as usize
    );
}

#[test]
fn burn_stale_reward_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 3;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        let start_era = DappsStaking::current_era();

        // Register & stake on contract
        assert_register(developer, &contract_id);
        assert_bond_and_stake(staker, &contract_id, 100);

        // Advance enough eras so stale rewards become burnable
        let eras_advanced = REWARD_RETENTION_PERIOD + 1;
        advance_to_era(start_era + eras_advanced);
        assert_unregister(developer, &contract_id);

        assert_burn_stale_reward(&contract_id, start_era);
    })
}

#[test]
fn burn_stale_reward_from_registered_dapp_fails() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 3;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        let start_era = DappsStaking::current_era();

        // Register & stake on contract
        assert_register(developer, &contract_id);
        assert_bond_and_stake(staker, &contract_id, 100);

        // Advance enough eras so stale rewards would become burnable, in case dapp was unregistered
        let eras_advanced = REWARD_RETENTION_PERIOD;
        advance_to_era(start_era + eras_advanced);

        // Rewards shouldn't be burnable since retention period hasn't expired yet
        assert_noop!(
            DappsStaking::burn_stale_reward(RuntimeOrigin::root(), contract_id, start_era,),
            Error::<TestRuntime>::NotUnregisteredContract
        );
    })
}

#[test]
fn burn_stale_reward_before_retention_period_finished_fails() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 3;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        let start_era = DappsStaking::current_era();

        // Register & stake on contract
        assert_register(developer, &contract_id);
        assert_bond_and_stake(staker, &contract_id, 100);

        // Advance enough eras so stale rewards become burnable
        let eras_advanced = REWARD_RETENTION_PERIOD;
        advance_to_era(start_era + eras_advanced);
        assert_unregister(developer, &contract_id);

        // Rewards shouldn't be burnable since retention period hasn't expired yet
        assert_noop!(
            DappsStaking::burn_stale_reward(RuntimeOrigin::root(), contract_id, start_era,),
            Error::<TestRuntime>::EraOutOfBounds
        );
    })
}

#[test]
fn burn_stale_reward_negative_checks() {
    ExternalityBuilder::build().execute_with(|| {
        initialize_first_block();

        let developer = 1;
        let staker = 3;
        let contract_id = MockSmartContract::Evm(H160::repeat_byte(0x01));

        // Cannot burn from non-existing contract
        assert_noop!(
            DappsStaking::burn_stale_reward(RuntimeOrigin::root(), contract_id, 1,),
            Error::<TestRuntime>::NotOperatedContract
        );

        // Cannot burn unless called with root privileges
        assert_noop!(
            DappsStaking::burn_stale_reward(RuntimeOrigin::signed(developer), contract_id, 1,),
            BadOrigin
        );

        // Register & stake on contract
        assert_register(developer, &contract_id);
        assert_bond_and_stake(staker, &contract_id, 100);

        // Advance enough eras so stale rewards become burnable
        let start_era = DappsStaking::current_era();
        let eras_advanced = REWARD_RETENTION_PERIOD + 2;
        advance_to_era(start_era + eras_advanced);
        assert_unregister(developer, &contract_id);

        // Claim them (before they are burned)
        assert_claim_dapp(&contract_id, start_era);

        // No longer possible to burn if reward was claimed
        assert_noop!(
            DappsStaking::burn_stale_reward(RuntimeOrigin::root(), contract_id, start_era,),
            Error::<TestRuntime>::AlreadyClaimedInThisEra
        );
    })
}
