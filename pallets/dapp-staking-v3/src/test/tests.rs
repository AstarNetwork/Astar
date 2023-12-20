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

use crate::test::{mock::*, testing_utils::*};
use crate::{
    pallet::Config, ActiveProtocolState, DAppId, EraRewards, Error, Event, ForcingType,
    IntegratedDApps, Ledger, NextDAppId, PeriodNumber, StakerInfo, Subperiod, TierConfig,
};

use frame_support::{
    assert_noop, assert_ok, assert_storage_noop,
    error::BadOrigin,
    traits::{fungible::Unbalanced as FunUnbalanced, Currency, Get, OnFinalize, OnInitialize},
};
use sp_runtime::traits::Zero;

use astar_primitives::{
    dapp_staking::{CycleConfiguration, SmartContractHandle},
    Balance, BlockNumber,
};

#[test]
fn maintenace_mode_works() {
    ExtBuilder::build().execute_with(|| {
        // Check that maintenance mode is disabled by default
        assert!(!ActiveProtocolState::<Test>::get().maintenance);

        // Enable maintenance mode & check post-state
        assert_ok!(DappStaking::maintenance_mode(RuntimeOrigin::root(), true));
        System::assert_last_event(RuntimeEvent::DappStaking(Event::MaintenanceMode {
            enabled: true,
        }));
        assert!(ActiveProtocolState::<Test>::get().maintenance);

        // Call still works, even in maintenance mode
        assert_ok!(DappStaking::maintenance_mode(RuntimeOrigin::root(), false));
        System::assert_last_event(RuntimeEvent::DappStaking(Event::MaintenanceMode {
            enabled: false,
        }));
        assert!(!ActiveProtocolState::<Test>::get().maintenance);

        // Incorrect origin doesn't work
        assert_noop!(
            DappStaking::maintenance_mode(RuntimeOrigin::signed(1), false),
            BadOrigin
        );
    })
}

#[test]
fn maintenace_mode_call_filtering_works() {
    ExtBuilder::build().execute_with(|| {
        // Enable maintenance mode & check post-state
        assert_ok!(DappStaking::maintenance_mode(RuntimeOrigin::root(), true));
        assert!(ActiveProtocolState::<Test>::get().maintenance);

        assert_storage_noop!(DappStaking::on_initialize(1));
        assert_noop!(
            DappStaking::register(RuntimeOrigin::root(), 1, MockSmartContract::Wasm(1)),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::set_dapp_reward_beneficiary(
                RuntimeOrigin::signed(1),
                MockSmartContract::Wasm(1),
                Some(2)
            ),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::set_dapp_owner(RuntimeOrigin::signed(1), MockSmartContract::Wasm(1), 2),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::unregister(RuntimeOrigin::root(), MockSmartContract::Wasm(1)),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::lock(RuntimeOrigin::signed(1), 100),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::unlock(RuntimeOrigin::signed(1), 100),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::claim_unlocked(RuntimeOrigin::signed(1)),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::relock_unlocking(RuntimeOrigin::signed(1)),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::stake(
                RuntimeOrigin::signed(1),
                MockSmartContract::wasm(1 as AccountId),
                100
            ),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::unstake(
                RuntimeOrigin::signed(1),
                MockSmartContract::wasm(1 as AccountId),
                100
            ),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::claim_staker_rewards(RuntimeOrigin::signed(1)),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::claim_bonus_reward(
                RuntimeOrigin::signed(1),
                MockSmartContract::wasm(1 as AccountId)
            ),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::claim_dapp_reward(
                RuntimeOrigin::signed(1),
                MockSmartContract::wasm(1 as AccountId),
                1
            ),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::unstake_from_unregistered(
                RuntimeOrigin::signed(1),
                MockSmartContract::wasm(1 as AccountId)
            ),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::cleanup_expired_entries(RuntimeOrigin::signed(1)),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::force(RuntimeOrigin::root(), ForcingType::Era),
            Error::<Test>::Disabled
        );
    })
}

#[test]
fn on_initialize_is_noop_if_no_era_change() {
    ExtBuilder::build().execute_with(|| {
        let protocol_state = ActiveProtocolState::<Test>::get();
        let current_block_number = System::block_number();

        assert!(
            current_block_number < protocol_state.next_era_start,
            "Sanity check, otherwise test doesn't make sense."
        );

        // Sanity check
        assert_storage_noop!(DappStaking::on_finalize(current_block_number));

        // If no era change, on_initialize should be a noop
        assert_storage_noop!(DappStaking::on_initialize(current_block_number + 1));
    })
}

#[test]
fn on_initialize_base_state_change_works() {
    ExtBuilder::build().execute_with(|| {
        // Sanity check
        let protocol_state = ActiveProtocolState::<Test>::get();
        assert_eq!(protocol_state.era, 1);
        assert_eq!(protocol_state.period_number(), 1);
        assert_eq!(protocol_state.subperiod(), Subperiod::Voting);
        assert_eq!(System::block_number(), 1);

        let blocks_per_voting_period = DappStaking::blocks_per_voting_period();
        assert_eq!(
            protocol_state.next_era_start,
            blocks_per_voting_period + 1,
            "Counting starts from block 1, hence the '+ 1'."
        );

        // Advance eras until we reach the Build&Earn period part
        run_to_block(protocol_state.next_era_start - 1);
        let protocol_state = ActiveProtocolState::<Test>::get();
        assert_eq!(
            protocol_state.subperiod(),
            Subperiod::Voting,
            "Period type should still be the same."
        );
        assert_eq!(protocol_state.era, 1);

        run_for_blocks(1);
        let protocol_state = ActiveProtocolState::<Test>::get();
        assert_eq!(protocol_state.subperiod(), Subperiod::BuildAndEarn);
        assert_eq!(protocol_state.era, 2);
        assert_eq!(protocol_state.period_number(), 1);

        // Advance eras just until we reach the next voting period
        let eras_per_bep_period =
            <Test as Config>::CycleConfiguration::eras_per_build_and_earn_subperiod();
        let blocks_per_era: BlockNumber = <Test as Config>::CycleConfiguration::blocks_per_era();
        for era in 2..(2 + eras_per_bep_period - 1) {
            let pre_block = System::block_number();
            advance_to_next_era();
            assert_eq!(System::block_number(), pre_block + blocks_per_era);
            let protocol_state = ActiveProtocolState::<Test>::get();
            assert_eq!(protocol_state.subperiod(), Subperiod::BuildAndEarn);
            assert_eq!(protocol_state.period_number(), 1);
            assert_eq!(protocol_state.era, era + 1);
        }

        // Finaly advance over to the next era and ensure we're back to voting period
        advance_to_next_era();
        let protocol_state = ActiveProtocolState::<Test>::get();
        assert_eq!(protocol_state.subperiod(), Subperiod::Voting);
        assert_eq!(protocol_state.era, 2 + eras_per_bep_period);
        assert_eq!(
            protocol_state.next_era_start,
            System::block_number() + blocks_per_voting_period
        );
        assert_eq!(protocol_state.period_number(), 2);
    })
}

#[test]
fn register_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Basic test
        assert_register(5, &MockSmartContract::Wasm(1));

        // Register two contracts using the same owner
        assert_register(7, &MockSmartContract::Wasm(2));
        assert_register(7, &MockSmartContract::Wasm(3));
    })
}

#[test]
fn register_with_incorrect_origin_fails() {
    ExtBuilder::build().execute_with(|| {
        assert_noop!(
            DappStaking::register(RuntimeOrigin::signed(1), 3, MockSmartContract::Wasm(2)),
            BadOrigin
        );
    })
}

#[test]
fn register_already_registered_contract_fails() {
    ExtBuilder::build().execute_with(|| {
        let smart_contract = MockSmartContract::Wasm(1);
        assert_register(2, &smart_contract);
        assert_noop!(
            DappStaking::register(RuntimeOrigin::root(), 2, smart_contract),
            Error::<Test>::ContractAlreadyExists
        );
    })
}

#[test]
fn register_past_max_number_of_contracts_fails() {
    ExtBuilder::build().execute_with(|| {
        let limit = <Test as Config>::MaxNumberOfContracts::get();
        for id in 1..=limit {
            assert_register(1, &MockSmartContract::Wasm(id.into()));
        }

        assert_noop!(
            DappStaking::register(
                RuntimeOrigin::root(),
                2,
                MockSmartContract::Wasm((limit + 1).into())
            ),
            Error::<Test>::ExceededMaxNumberOfContracts
        );
    })
}

#[test]
fn register_past_sentinel_value_of_id_fails() {
    ExtBuilder::build().execute_with(|| {
        // hacky approach, but good enough for test
        NextDAppId::<Test>::put(DAppId::MAX - 1);

        // First register should pass since sentinel value hasn't been reached yet
        assert_register(1, &MockSmartContract::Wasm(3));

        // Second one should fail since we've reached the sentine value and cannot add more contracts
        assert_eq!(NextDAppId::<Test>::get(), DAppId::MAX);
        assert_noop!(
            DappStaking::register(RuntimeOrigin::root(), 1, MockSmartContract::Wasm(5)),
            Error::<Test>::NewDAppIdUnavailable
        );
    })
}

#[test]
fn set_dapp_reward_beneficiary_for_contract_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Prepare & register smart contract
        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);
        assert_register(owner, &smart_contract);

        // Update beneficiary
        assert!(IntegratedDApps::<Test>::get(&smart_contract)
            .unwrap()
            .reward_destination
            .is_none());
        assert_set_dapp_reward_beneficiary(owner, &smart_contract, Some(3));
        assert_set_dapp_reward_beneficiary(owner, &smart_contract, Some(5));
        assert_set_dapp_reward_beneficiary(owner, &smart_contract, None);
    })
}

#[test]
fn set_dapp_reward_beneficiary_fails() {
    ExtBuilder::build().execute_with(|| {
        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);

        // Contract doesn't exist yet
        assert_noop!(
            DappStaking::set_dapp_reward_beneficiary(
                RuntimeOrigin::signed(owner),
                smart_contract,
                Some(5)
            ),
            Error::<Test>::ContractNotFound
        );

        // Non-owner cannnot change reward destination
        assert_register(owner, &smart_contract);
        assert_noop!(
            DappStaking::set_dapp_reward_beneficiary(
                RuntimeOrigin::signed(owner + 1),
                smart_contract,
                Some(5)
            ),
            Error::<Test>::OriginNotOwner
        );
    })
}

#[test]
fn set_dapp_owner_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Prepare & register smart contract
        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);
        assert_register(owner, &smart_contract);

        // Update owner
        let new_owner = 7;
        assert_set_dapp_owner(Some(owner), &smart_contract, new_owner);
        assert_set_dapp_owner(Some(new_owner), &smart_contract, 1337);

        // Ensure manager can bypass owner
        assert_set_dapp_owner(None, &smart_contract, owner);
    })
}

#[test]
fn set_dapp_owner_fails() {
    ExtBuilder::build().execute_with(|| {
        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);

        // Contract doesn't exist yet
        assert_noop!(
            DappStaking::set_dapp_owner(RuntimeOrigin::signed(owner), smart_contract, 5),
            Error::<Test>::ContractNotFound
        );

        // Ensure non-owner cannot steal ownership
        assert_register(owner, &smart_contract);
        assert_noop!(
            DappStaking::set_dapp_owner(
                RuntimeOrigin::signed(owner + 1),
                smart_contract,
                owner + 1
            ),
            Error::<Test>::OriginNotOwner
        );
    })
}

#[test]
fn unregister_no_stake_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Prepare dApp
        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);
        assert_register(owner, &smart_contract);

        // Nothing staked on contract, just unregister it.
        assert_unregister(&smart_contract);
    })
}

#[test]
fn unregister_with_active_stake_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Prepare dApp
        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);
        assert_register(owner, &smart_contract);
        assert_lock(owner, 100);
        assert_stake(owner, &smart_contract, 100);

        // Some amount is staked, unregister must still work.
        assert_unregister(&smart_contract);
    })
}

#[test]
fn unregister_fails() {
    ExtBuilder::build().execute_with(|| {
        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);

        // Cannot unregister contract which doesn't exist
        assert_noop!(
            DappStaking::unregister(RuntimeOrigin::root(), smart_contract),
            Error::<Test>::ContractNotFound
        );

        // Cannot unregister with incorrect origin
        assert_register(owner, &smart_contract);
        assert_noop!(
            DappStaking::unregister(RuntimeOrigin::signed(owner), smart_contract),
            BadOrigin
        );

        // Cannot unregister same contract twice
        assert_unregister(&smart_contract);
        assert_noop!(
            DappStaking::unregister(RuntimeOrigin::root(), smart_contract),
            Error::<Test>::NotOperatedDApp
        );
    })
}

#[test]
fn lock_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Lock some amount
        let locker = 2;
        let free_balance = Balances::free_balance(&locker);
        assert!(free_balance > 500, "Sanity check");
        assert_lock(locker, 100);
        assert_lock(locker, 200);

        // Attempt to lock more than is available
        assert_lock(locker, free_balance - 200);

        // Ensure minimum lock amount works
        let locker = 3;
        assert_lock(locker, <Test as Config>::MinimumLockedAmount::get());
    })
}

#[test]
fn lock_with_incorrect_amount_fails() {
    ExtBuilder::build().execute_with(|| {
        // Cannot lock "nothing"
        assert_noop!(
            DappStaking::lock(RuntimeOrigin::signed(1), Balance::zero()),
            Error::<Test>::ZeroAmount,
        );

        // Attempting to lock something after everything has been locked is same
        // as attempting to lock with "nothing"
        let locker = 1;
        assert_lock(locker, Balances::free_balance(&locker));
        assert_noop!(
            DappStaking::lock(RuntimeOrigin::signed(locker), 1),
            Error::<Test>::ZeroAmount,
        );

        // Locking just below the minimum amount should fail
        let locker = 2;
        let minimum_locked_amount: Balance = <Test as Config>::MinimumLockedAmount::get();
        assert_noop!(
            DappStaking::lock(RuntimeOrigin::signed(locker), minimum_locked_amount - 1),
            Error::<Test>::LockedAmountBelowThreshold,
        );
    })
}

#[test]
fn unlock_basic_example_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Lock some amount
        let account = 2;
        let lock_amount = 101;
        assert_lock(account, lock_amount);

        // Unlock some amount in the same era that it was locked
        let first_unlock_amount = 7;
        assert_unlock(account, first_unlock_amount);

        // Advance era and unlock additional amount
        advance_to_next_era();
        assert_unlock(account, first_unlock_amount);

        // Lock a bit more, and unlock again
        assert_lock(account, lock_amount);
        assert_unlock(account, first_unlock_amount);
    })
}

#[test]
fn unlock_with_remaining_amount_below_threshold_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Lock some amount in a few eras
        let account = 2;
        let lock_amount = 101;
        assert_lock(account, lock_amount);
        advance_to_next_era();
        assert_lock(account, lock_amount);
        advance_to_era(ActiveProtocolState::<Test>::get().era + 3);

        // Unlock such amount that remaining amount is below threshold, resulting in full unlock
        let minimum_locked_amount: Balance = <Test as Config>::MinimumLockedAmount::get();
        let ledger = Ledger::<Test>::get(&account);
        assert_unlock(
            account,
            ledger.active_locked_amount() - minimum_locked_amount + 1,
        );
    })
}

#[test]
fn unlock_with_amount_higher_than_avaiable_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Lock some amount in a few eras
        let account = 2;
        let lock_amount = 101;
        assert_lock(account, lock_amount);
        advance_to_next_era();
        assert_lock(account, lock_amount);

        // Register contract & stake on it
        let smart_contract = MockSmartContract::Wasm(1);
        assert_register(1, &smart_contract);
        let stake_amount = 91;
        assert_stake(account, &smart_contract, stake_amount);

        // Try to unlock more than is available, due to active staked amount
        assert_unlock(account, lock_amount - stake_amount + 1);

        // Ensure there is no effect of staked amount once we move to the following period
        assert_lock(account, lock_amount - stake_amount); // restore previous state
        advance_to_period(ActiveProtocolState::<Test>::get().period_number() + 1);
        assert_unlock(account, lock_amount - stake_amount + 1);
    })
}

#[test]
fn unlock_advanced_examples_are_ok() {
    ExtBuilder::build().execute_with(|| {
        // Lock some amount
        let account = 2;
        let lock_amount = 101;
        assert_lock(account, lock_amount);

        // Unlock some amount in the same era that it was locked
        let unlock_amount = 7;
        assert_unlock(account, unlock_amount);

        // Advance era and unlock additional amount
        advance_to_next_era();
        assert_unlock(account, unlock_amount * 2);

        // Advance few more eras, and unlock everything
        advance_to_era(ActiveProtocolState::<Test>::get().era + 7);
        assert_unlock(account, lock_amount);
        assert!(Ledger::<Test>::get(&account)
            .active_locked_amount()
            .is_zero());

        // Advance one more era and ensure we can still lock & unlock
        advance_to_next_era();
        assert_lock(account, lock_amount);
        assert_unlock(account, unlock_amount);
    })
}

#[test]
fn unlock_everything_with_active_stake_fails() {
    ExtBuilder::build().execute_with(|| {
        let account = 2;
        let lock_amount = 101;
        assert_lock(account, lock_amount);
        advance_to_next_era();

        // We stake so the amount is just below the minimum locked amount, causing full unlock impossible.
        let minimum_locked_amount: Balance = <Test as Config>::MinimumLockedAmount::get();
        let stake_amount = minimum_locked_amount - 1;

        // Register contract & stake on it
        let smart_contract = MockSmartContract::Wasm(1);
        assert_register(1, &smart_contract);
        assert_stake(account, &smart_contract, stake_amount);

        // Try to unlock more than is available, due to active staked amount
        assert_noop!(
            DappStaking::unlock(RuntimeOrigin::signed(account), lock_amount),
            Error::<Test>::RemainingStakePreventsFullUnlock,
        );
    })
}

#[test]
fn unlock_with_zero_amount_fails() {
    ExtBuilder::build().execute_with(|| {
        let account = 2;
        let lock_amount = 101;
        assert_lock(account, lock_amount);
        advance_to_next_era();

        // Unlock with zero fails
        assert_noop!(
            DappStaking::unlock(RuntimeOrigin::signed(account), 0),
            Error::<Test>::ZeroAmount,
        );

        // Stake everything, so available unlock amount is always zero
        let smart_contract = MockSmartContract::Wasm(1);
        assert_register(1, &smart_contract);
        assert_stake(account, &smart_contract, lock_amount);

        // Try to unlock anything, expect zero amount error
        assert_noop!(
            DappStaking::unlock(RuntimeOrigin::signed(account), lock_amount),
            Error::<Test>::ZeroAmount,
        );
    })
}

#[test]
fn unlock_with_exceeding_unlocking_chunks_storage_limits_fails() {
    ExtBuilder::build().execute_with(|| {
        // Lock some amount in a few eras
        let account = 2;
        let lock_amount = 103;
        assert_lock(account, lock_amount);

        let unlock_amount = 3;
        for _ in 0..<Test as Config>::MaxUnlockingChunks::get() {
            run_for_blocks(1);
            assert_unlock(account, unlock_amount);
        }

        // We can still unlock in the current era, theoretically
        for _ in 0..5 {
            assert_unlock(account, unlock_amount);
        }

        // Following unlock should fail due to exceeding storage limits
        run_for_blocks(1);
        assert_noop!(
            DappStaking::unlock(RuntimeOrigin::signed(account), unlock_amount),
            Error::<Test>::TooManyUnlockingChunks,
        );
    })
}

#[test]
fn claim_unlocked_is_ok() {
    ExtBuilder::build().execute_with(|| {
        let unlocking_blocks = DappStaking::unlocking_period();

        // Lock some amount in a few eras
        let account = 2;
        let lock_amount = 103;
        assert_lock(account, lock_amount);

        // Basic example
        let unlock_amount = 3;
        assert_unlock(account, unlock_amount);
        run_for_blocks(unlocking_blocks);
        assert_claim_unlocked(account);

        // Advanced example
        let max_unlocking_chunks: u32 = <Test as Config>::MaxUnlockingChunks::get();
        for _ in 0..max_unlocking_chunks {
            run_for_blocks(1);
            assert_unlock(account, unlock_amount);
        }

        // Leave two blocks remaining after the claim
        run_for_blocks(unlocking_blocks - 2);
        assert_claim_unlocked(account);

        // Claim last two blocks together
        run_for_blocks(2);
        assert_claim_unlocked(account);
        assert!(Ledger::<Test>::get(&account).unlocking.is_empty());

        // Unlock everything
        assert_unlock(account, lock_amount);
        run_for_blocks(unlocking_blocks);
        assert_claim_unlocked(account);
        assert!(!Ledger::<Test>::contains_key(&account));
    })
}

#[test]
fn claim_unlocked_no_eligible_chunks_fails() {
    ExtBuilder::build().execute_with(|| {
        // Sanity check
        let account = 2;
        assert_noop!(
            DappStaking::claim_unlocked(RuntimeOrigin::signed(account)),
            Error::<Test>::NoUnlockedChunksToClaim,
        );

        // Cannot claim if unlock period hasn't passed yet
        let lock_amount = 103;
        assert_lock(account, lock_amount);
        let unlocking_blocks = DappStaking::unlocking_period();
        run_for_blocks(unlocking_blocks - 1);
        assert_noop!(
            DappStaking::claim_unlocked(RuntimeOrigin::signed(account)),
            Error::<Test>::NoUnlockedChunksToClaim,
        );
    })
}

#[test]
fn relock_unlocking_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Lock some amount
        let account = 2;
        let lock_amount = 91;
        assert_lock(account, lock_amount);

        // Prepare some unlock chunks
        let unlock_amount = 5;
        assert_unlock(account, unlock_amount);
        run_for_blocks(2);
        assert_unlock(account, unlock_amount);

        assert_relock_unlocking(account);

        let max_unlocking_chunks: u32 = <Test as Config>::MaxUnlockingChunks::get();
        for _ in 0..max_unlocking_chunks {
            run_for_blocks(1);
            assert_unlock(account, unlock_amount);
        }

        assert_relock_unlocking(account);
    })
}

#[test]
fn relock_unlocking_no_chunks_fails() {
    ExtBuilder::build().execute_with(|| {
        assert_noop!(
            DappStaking::relock_unlocking(RuntimeOrigin::signed(1)),
            Error::<Test>::NoUnlockingChunks,
        );
    })
}

#[test]
fn relock_unlocking_insufficient_lock_amount_fails() {
    ExtBuilder::build().execute_with(|| {
        let minimum_locked_amount: Balance = <Test as Config>::MinimumLockedAmount::get();

        // lock amount should be above the threshold
        let account = 2;
        assert_lock(account, minimum_locked_amount + 1);

        // Create two unlocking chunks
        assert_unlock(account, 1);
        run_for_blocks(1);
        assert_unlock(account, minimum_locked_amount);

        // This scenario can only be achieved if minimum staking amount increases on live network.
        // Otherwise we always have a guarantee that the latest unlocking chunk at least covers the
        // minimum staking amount.
        // To test this, we will do a "dirty trick", and swap the two unlocking chunks that were just created.
        // This shoudl ensure that the latest unlocking chunk is below the minimum staking amount.
        Ledger::<Test>::mutate(&account, |ledger| {
            ledger.unlocking = ledger
                .unlocking
                .clone()
                .try_mutate(|inner| {
                    let temp_block = inner[0].unlock_block;
                    inner[0].unlock_block = inner[1].unlock_block;
                    inner[1].unlock_block = temp_block;
                    inner.swap(0, 1);
                })
                .expect("No size manipulation, only element swap.");
        });

        // Make sure only one chunk is left
        let unlocking_blocks = DappStaking::unlocking_period();
        run_for_blocks(unlocking_blocks - 1);
        assert_claim_unlocked(account);

        assert_noop!(
            DappStaking::relock_unlocking(RuntimeOrigin::signed(account)),
            Error::<Test>::LockedAmountBelowThreshold,
        );
    })
}

#[test]
fn stake_basic_example_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let lock_amount = 300;
        assert_lock(account, lock_amount);

        // Stake some amount, and then some more in the same era.
        let (stake_1, stake_2) = (31, 29);
        assert_stake(account, &smart_contract, stake_1);
        assert_stake(account, &smart_contract, stake_2);
    })
}

#[test]
fn stake_after_expiry_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        // Lock & stake some amount
        let account = 2;
        let lock_amount = 300;
        let (stake_amount_1, stake_amount_2) = (200, 100);
        assert_lock(account, lock_amount);
        assert_stake(account, &smart_contract, stake_amount_1);

        // Advance so far that the stake rewards expire.
        let reward_retention_in_periods: PeriodNumber =
            <Test as Config>::RewardRetentionInPeriods::get();
        advance_to_period(
            ActiveProtocolState::<Test>::get().period_number() + reward_retention_in_periods + 1,
        );

        // Sanity check that the rewards have expired
        assert_noop!(
            DappStaking::claim_staker_rewards(RuntimeOrigin::signed(account)),
            Error::<Test>::RewardExpired,
        );

        // Calling stake again should work, expired stake entries should be cleaned up
        assert_stake(account, &smart_contract, stake_amount_2);
        assert_stake(account, &smart_contract, stake_amount_1);
    })
}

#[test]
fn stake_with_zero_amount_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);
        let account = 2;
        assert_lock(account, 300);

        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract, 0),
            Error::<Test>::ZeroAmount,
        );
    })
}

#[test]
fn stake_on_invalid_dapp_fails() {
    ExtBuilder::build().execute_with(|| {
        let account = 2;
        assert_lock(account, 300);

        // Try to stake on non-existing contract
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::NotOperatedDApp
        );

        // Try to stake on unregistered smart contract
        assert_register(1, &smart_contract);
        assert_unregister(&smart_contract);
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::NotOperatedDApp
        );
    })
}

#[test]
fn stake_in_final_era_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        let account = 2;
        assert_register(1, &smart_contract);
        assert_lock(account, 300);

        // Force Build&Earn period
        ActiveProtocolState::<Test>::mutate(|state| {
            state.period_info.subperiod = Subperiod::BuildAndEarn;
            state.period_info.next_subperiod_start_era = state.era + 1;
        });

        // Try to stake in the final era of the period, which should fail.
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::PeriodEndsInNextEra
        );
    })
}

#[test]
fn stake_fails_if_unclaimed_staker_rewards_from_past_remain() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        let account = 2;
        assert_register(1, &smart_contract);
        assert_lock(account, 300);

        // Stake some amount, then force a few eras
        assert_stake(account, &smart_contract, 100);
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);

        // Stake must fail due to unclaimed rewards
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::UnclaimedRewards
        );

        // Should also fail in the next period
        advance_to_next_period();
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::UnclaimedRewards
        );
    })
}

#[test]
fn stake_fails_if_claimable_bonus_rewards_from_past_remain() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        let account = 2;
        assert_register(1, &smart_contract);
        assert_lock(account, 300);
        assert_stake(account, &smart_contract, 100);

        // Advance to next period, claim all staker rewards
        advance_to_next_period();
        for _ in 0..required_number_of_reward_claims(account) {
            assert_claim_staker_rewards(account);
        }

        // Try to stake again on the same contract, expect an error due to unclaimed bonus rewards
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::UnclaimedRewards
        );
    })
}

#[test]
fn stake_fails_if_not_enough_stakeable_funds_available() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contracts & lock some amount
        let smart_contract_1 = MockSmartContract::Wasm(1);
        let smart_contract_2 = MockSmartContract::Wasm(2);
        let account = 3;
        assert_register(1, &smart_contract_1);
        assert_register(2, &smart_contract_2);
        let lock_amount = 100;
        assert_lock(account, lock_amount);

        // Stake some amount on the first contract, and second contract
        assert_stake(account, &smart_contract_1, 50);
        assert_stake(account, &smart_contract_2, 40);

        // Try to stake more than is available, expect failure
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract_1.clone(), 11),
            Error::<Test>::UnavailableStakeFunds
        );
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract_2.clone(), 11),
            Error::<Test>::UnavailableStakeFunds
        );

        // Stake exactly up to available funds, expect a pass
        assert_stake(account, &smart_contract_2, 10);
    })
}

#[test]
fn stake_fails_due_to_too_small_staking_amount() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let smart_contract_1 = MockSmartContract::Wasm(1);
        let smart_contract_2 = MockSmartContract::Wasm(2);
        let account = 3;
        assert_register(1, &smart_contract_1);
        assert_register(2, &smart_contract_2);
        assert_lock(account, 300);

        // Stake with too small amount, expect a failure
        let min_stake_amount: Balance = <Test as Config>::MinimumStakeAmount::get();
        assert_noop!(
            DappStaking::stake(
                RuntimeOrigin::signed(account),
                smart_contract_1.clone(),
                min_stake_amount - 1
            ),
            Error::<Test>::InsufficientStakeAmount
        );

        // Staking with minimum amount must work. Also, after a successful stake, we can stake with arbitrary small amount on the contract.
        assert_stake(account, &smart_contract_1, min_stake_amount);
        assert_stake(account, &smart_contract_1, 1);

        // Even though account is staking already, trying to stake with too small amount on a different
        // smart contract should once again fail.
        assert_noop!(
            DappStaking::stake(
                RuntimeOrigin::signed(account),
                smart_contract_2.clone(),
                min_stake_amount - 1
            ),
            Error::<Test>::InsufficientStakeAmount
        );
    })
}

#[test]
fn stake_fails_due_to_too_many_staked_contracts() {
    ExtBuilder::build().execute_with(|| {
        let max_number_of_contracts: u32 = <Test as Config>::MaxNumberOfStakedContracts::get();

        // Lock amount by staker
        let account = 1;
        assert_lock(account, 100 as Balance * max_number_of_contracts as Balance);

        // Advance to build&earn subperiod so we ensure non-loyal staking
        advance_to_next_subperiod();

        // Register smart contracts up to the max allowed number
        for id in 1..=max_number_of_contracts {
            let smart_contract = MockSmartContract::Wasm(id.into());
            assert_register(2, &MockSmartContract::Wasm(id.into()));
            assert_stake(account, &smart_contract, 10);
        }

        let excess_smart_contract = MockSmartContract::Wasm((max_number_of_contracts + 1).into());
        assert_register(2, &excess_smart_contract);

        // Max number of staked contract entries has been exceeded.
        assert_noop!(
            DappStaking::stake(
                RuntimeOrigin::signed(account),
                excess_smart_contract.clone(),
                10
            ),
            Error::<Test>::TooManyStakedContracts
        );

        // Advance into next period, error should still happen
        advance_to_next_period();
        for _ in 0..required_number_of_reward_claims(account) {
            assert_claim_staker_rewards(account);
        }
        assert_noop!(
            DappStaking::stake(
                RuntimeOrigin::signed(account),
                excess_smart_contract.clone(),
                10
            ),
            Error::<Test>::TooManyStakedContracts
        );
    })
}

#[test]
fn unstake_basic_example_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let lock_amount = 400;
        assert_lock(account, lock_amount);

        // Prep step - stake some amount
        let stake_amount_1 = 83;
        assert_stake(account, &smart_contract, stake_amount_1);

        // Unstake some amount, in the current era.
        let unstake_amount_1 = 3;
        assert_unstake(account, &smart_contract, unstake_amount_1);
    })
}

#[test]
fn unstake_with_leftover_amount_below_minimum_works() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);

        let min_stake_amount: Balance = <Test as Config>::MinimumStakeAmount::get();
        assert_stake(account, &smart_contract, min_stake_amount);

        // Unstake some amount, bringing it below the minimum
        assert_unstake(account, &smart_contract, 1);
    })
}

#[test]
fn unstake_with_zero_amount_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);
        let account = 2;
        assert_lock(account, 300);
        assert_stake(account, &smart_contract, 100);

        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(account), smart_contract, 0),
            Error::<Test>::ZeroAmount,
        );
    })
}

#[test]
fn unstake_on_invalid_dapp_fails() {
    ExtBuilder::build().execute_with(|| {
        let account = 2;
        assert_lock(account, 300);

        // Try to unstake from non-existing contract
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::NotOperatedDApp
        );

        // Try to unstake from unregistered smart contract
        assert_register(1, &smart_contract);
        assert_stake(account, &smart_contract, 100);
        assert_unregister(&smart_contract);
        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::NotOperatedDApp
        );
    })
}

#[test]
fn unstake_with_exceeding_amount_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contracts & lock some amount
        let smart_contract_1 = MockSmartContract::Wasm(1);
        let smart_contract_2 = MockSmartContract::Wasm(2);
        assert_register(1, &smart_contract_1);
        assert_register(1, &smart_contract_2);
        let account = 2;
        assert_lock(account, 300);

        // 1st scenario - stake some amount on the first contract, and try to unstake more than was staked
        let stake_amount_1 = 100;
        assert_stake(account, &smart_contract_1, stake_amount_1);
        assert_noop!(
            DappStaking::unstake(
                RuntimeOrigin::signed(account),
                smart_contract_1,
                stake_amount_1 + 1
            ),
            Error::<Test>::UnstakeAmountTooLarge
        );

        // 2nd scenario - have some stake on two distinct contracts, but unstaking more than staked per contract still fails
        let stake_amount_2 = 50;
        assert_stake(account, &smart_contract_2, stake_amount_2);
        assert_noop!(
            DappStaking::unstake(
                RuntimeOrigin::signed(account),
                smart_contract_2,
                stake_amount_2 + 1
            ),
            Error::<Test>::UnstakeAmountTooLarge
        );
    })
}

#[test]
fn unstake_from_non_staked_contract_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contracts & lock some amount
        let smart_contract_1 = MockSmartContract::Wasm(1);
        let smart_contract_2 = MockSmartContract::Wasm(2);
        assert_register(1, &smart_contract_1);
        assert_register(1, &smart_contract_2);
        let account = 2;
        assert_lock(account, 300);

        // Stake some amount on the first contract.
        let stake_amount = 100;
        assert_stake(account, &smart_contract_1, stake_amount);

        // Try to unstake from the 2nd contract, which isn't staked on.
        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(account), smart_contract_2, 1,),
            Error::<Test>::NoStakingInfo
        );
    })
}

#[test]
fn unstake_with_unclaimed_rewards_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::Wasm(1);
        assert_register(1, &smart_contract);
        let account = 2;
        assert_lock(account, 300);
        let stake_amount = 100;
        assert_stake(account, &smart_contract, stake_amount);

        // Advance 1 era, try to unstake and it should work since we're modifying the current era stake.
        advance_to_next_era();
        assert_unstake(account, &smart_contract, 1);

        // Advance 1 more era, creating claimable rewards. Unstake should fail now.
        advance_to_next_era();
        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(account), smart_contract, 1),
            Error::<Test>::UnclaimedRewards
        );
    })
}

#[test]
fn unstake_from_past_period_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let smart_contract = MockSmartContract::Wasm(1);
        assert_register(1, &smart_contract);
        let account = 2;
        assert_lock(account, 300);

        // Stake some amount, and advance to the next period
        let stake_amount = 100;
        assert_stake(account, &smart_contract, stake_amount);
        advance_to_next_period();

        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(account), smart_contract, stake_amount),
            Error::<Test>::UnstakeFromPastPeriod
        );
    })
}

#[test]
fn claim_staker_rewards_basic_example_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let lock_amount = 300;
        assert_lock(account, lock_amount);
        let stake_amount = 93;
        assert_stake(account, &smart_contract, stake_amount);

        // Advance into Build&Earn period, and allow one era to pass. Claim reward for 1 era.
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);
        assert_claim_staker_rewards(account);

        // Advance a few more eras, and claim multiple rewards this time.
        advance_to_era(ActiveProtocolState::<Test>::get().era + 3);
        assert_eq!(
            ActiveProtocolState::<Test>::get().period_number(),
            1,
            "Sanity check, we must still be in the 1st period."
        );
        assert_claim_staker_rewards(account);

        // Advance into the next period, make sure we can still claim old rewards.
        advance_to_next_period();
        for _ in 0..required_number_of_reward_claims(account) {
            assert_claim_staker_rewards(account);
        }
    })
}

#[test]
fn claim_staker_rewards_double_call_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let lock_amount = 300;
        assert_lock(account, lock_amount);
        let stake_amount = 93;
        assert_stake(account, &smart_contract, stake_amount);

        // Advance into the next period, claim all eligible rewards
        advance_to_next_period();
        for _ in 0..required_number_of_reward_claims(account) {
            assert_claim_staker_rewards(account);
        }

        assert_noop!(
            DappStaking::claim_staker_rewards(RuntimeOrigin::signed(account)),
            Error::<Test>::NoClaimableRewards,
        );
    })
}

#[test]
fn claim_staker_rewards_no_claimable_rewards_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let lock_amount = 300;
        assert_lock(account, lock_amount);

        // 1st scenario - try to claim with no stake at all.
        assert_noop!(
            DappStaking::claim_staker_rewards(RuntimeOrigin::signed(account)),
            Error::<Test>::NoClaimableRewards,
        );

        // 2nd scenario - stake some amount, and try to claim in the same era.
        // It's important this is the 1st era, when no `EraRewards` entry exists.
        assert_eq!(ActiveProtocolState::<Test>::get().era, 1, "Sanity check");
        assert!(EraRewards::<Test>::iter().next().is_none(), "Sanity check");
        let stake_amount = 93;
        assert_stake(account, &smart_contract, stake_amount);
        assert_noop!(
            DappStaking::claim_staker_rewards(RuntimeOrigin::signed(account)),
            Error::<Test>::NoClaimableRewards,
        );

        // 3rd scenario - move over to the next era, but we still expect failure because
        // stake is valid from era 2 (current era), and we're trying to claim rewards for era 1.
        advance_to_next_era();
        assert!(EraRewards::<Test>::iter().next().is_some(), "Sanity check");
        assert_noop!(
            DappStaking::claim_staker_rewards(RuntimeOrigin::signed(account)),
            Error::<Test>::NoClaimableRewards,
        );
    })
}

#[test]
fn claim_staker_rewards_after_expiry_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let lock_amount = 300;
        assert_lock(account, lock_amount);
        let stake_amount = 93;
        assert_stake(account, &smart_contract, stake_amount);

        let reward_retention_in_periods: PeriodNumber =
            <Test as Config>::RewardRetentionInPeriods::get();

        // Advance to the block just before the 'expiry' period starts
        advance_to_period(
            ActiveProtocolState::<Test>::get().period_number() + reward_retention_in_periods,
        );
        advance_to_next_subperiod();
        advance_to_era(
            ActiveProtocolState::<Test>::get()
                .period_info
                .next_subperiod_start_era
                - 1,
        );
        assert_claim_staker_rewards(account);

        // Ensure we're still in the first period for the sake of test validity
        assert_eq!(
            Ledger::<Test>::get(&account).staked.period,
            1,
            "Sanity check."
        );

        // Trigger next period, rewards should be marked as expired
        advance_to_next_era();
        assert_eq!(
            ActiveProtocolState::<Test>::get().period_number(),
            reward_retention_in_periods + 2
        );
        assert_noop!(
            DappStaking::claim_staker_rewards(RuntimeOrigin::signed(account)),
            Error::<Test>::RewardExpired,
        );
    })
}

#[test]
fn claim_staker_rewards_fails_due_to_payout_failure() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        // Advance into Build&Earn period, and allow one era to pass.
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);

        // Disable successfull reward payout
        DOES_PAYOUT_SUCCEED.with(|v| *v.borrow_mut() = false);
        assert_noop!(
            DappStaking::claim_staker_rewards(RuntimeOrigin::signed(account)),
            Error::<Test>::RewardPayoutFailed,
        );

        // Re-enable it again, claim should work again
        DOES_PAYOUT_SUCCEED.with(|v| *v.borrow_mut() = true);
        assert_claim_staker_rewards(account);
    })
}

#[test]
fn claim_bonus_reward_works() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let lock_amount = 300;
        assert_lock(account, lock_amount);
        let stake_amount = 93;
        assert_stake(account, &smart_contract, stake_amount);

        // 1st scenario - advance to the next period, first claim bonus reward, then staker rewards
        advance_to_next_period();
        assert_claim_bonus_reward(account, &smart_contract);
        for _ in 0..required_number_of_reward_claims(account) {
            assert_claim_staker_rewards(account);
        }

        // 2nd scenario - stake again, advance to next period, this time first claim staker rewards, then bonus reward.
        assert_stake(account, &smart_contract, stake_amount);
        advance_to_next_period();
        for _ in 0..required_number_of_reward_claims(account) {
            assert_claim_staker_rewards(account);
        }
        assert!(
            Ledger::<Test>::get(&account).staked.is_empty(),
            "Sanity check."
        );
        assert_claim_bonus_reward(account, &smart_contract);
    })
}

#[test]
fn claim_bonus_reward_double_call_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let lock_amount = 300;
        assert_lock(account, lock_amount);
        let stake_amount = 93;
        assert_stake(account, &smart_contract, stake_amount);

        // Advance to the next period, claim bonus reward, then try to do it again
        advance_to_next_period();
        assert_claim_bonus_reward(account, &smart_contract);

        assert_noop!(
            DappStaking::claim_bonus_reward(RuntimeOrigin::signed(account), smart_contract),
            Error::<Test>::NoClaimableRewards,
        );
    })
}

#[test]
fn claim_bonus_reward_when_nothing_to_claim_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let lock_amount = 300;
        assert_lock(account, lock_amount);

        // 1st - try to claim bonus reward when no stake is present
        assert_noop!(
            DappStaking::claim_bonus_reward(RuntimeOrigin::signed(account), smart_contract),
            Error::<Test>::NoClaimableRewards,
        );

        // 2nd - try to claim bonus reward for the ongoing period
        let stake_amount = 93;
        assert_stake(account, &smart_contract, stake_amount);
        assert_noop!(
            DappStaking::claim_bonus_reward(RuntimeOrigin::signed(account), smart_contract),
            Error::<Test>::NoClaimableRewards,
        );
    })
}

#[test]
fn claim_bonus_reward_with_only_build_and_earn_stake_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let lock_amount = 300;
        assert_lock(account, lock_amount);

        // Stake in Build&Earn period type, advance to next era and try to claim bonus reward
        advance_to_next_subperiod();
        assert_eq!(
            ActiveProtocolState::<Test>::get().subperiod(),
            Subperiod::BuildAndEarn,
            "Sanity check."
        );
        let stake_amount = 93;
        assert_stake(account, &smart_contract, stake_amount);

        advance_to_next_period();
        assert_noop!(
            DappStaking::claim_bonus_reward(RuntimeOrigin::signed(account), smart_contract),
            Error::<Test>::NotEligibleForBonusReward,
        );
    })
}

#[test]
fn claim_bonus_reward_after_expiry_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let lock_amount = 300;
        assert_lock(account, lock_amount);
        assert_stake(account, &smart_contract, lock_amount);

        // 1st scenario - Advance to one period before the expiry, claim should still work.
        let reward_retention_in_periods: PeriodNumber =
            <Test as Config>::RewardRetentionInPeriods::get();
        advance_to_period(
            ActiveProtocolState::<Test>::get().period_number() + reward_retention_in_periods,
        );
        assert_claim_bonus_reward(account, &smart_contract);
        for _ in 0..required_number_of_reward_claims(account) {
            assert_claim_staker_rewards(account);
        }

        // 2nd scenario - advance past the expiry, call must fail
        assert_stake(account, &smart_contract, lock_amount);
        advance_to_period(
            ActiveProtocolState::<Test>::get().period_number() + reward_retention_in_periods + 1,
        );
        assert_noop!(
            DappStaking::claim_bonus_reward(RuntimeOrigin::signed(account), smart_contract),
            Error::<Test>::RewardExpired,
        );
    })
}

#[test]
fn claim_bonus_reward_fails_due_to_payout_failure() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        // Advance to next period so we can claim bonus reward
        advance_to_next_period();

        // Disable successfull reward payout
        DOES_PAYOUT_SUCCEED.with(|v| *v.borrow_mut() = false);
        assert_noop!(
            DappStaking::claim_bonus_reward(RuntimeOrigin::signed(account), smart_contract),
            Error::<Test>::RewardPayoutFailed,
        );

        // Re-enable it again, claim should work again
        DOES_PAYOUT_SUCCEED.with(|v| *v.borrow_mut() = true);
        assert_claim_bonus_reward(account, &smart_contract);
    })
}

#[test]
fn claim_dapp_reward_works() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        // Advance 2 eras so we have an entry for reward claiming
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);
        assert_eq!(ActiveProtocolState::<Test>::get().era, 3, "Sanity check");

        assert_claim_dapp_reward(
            account,
            &smart_contract,
            ActiveProtocolState::<Test>::get().era - 1,
        );

        // Advance to next era, and ensure rewards can be paid out to a custom beneficiary
        let new_beneficiary = 17;
        assert_set_dapp_reward_beneficiary(dev_account, &smart_contract, Some(new_beneficiary));
        advance_to_next_era();
        assert_claim_dapp_reward(
            account,
            &smart_contract,
            ActiveProtocolState::<Test>::get().era - 1,
        );
    })
}

#[test]
fn claim_dapp_reward_from_non_existing_contract_fails() {
    ExtBuilder::build().execute_with(|| {
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_noop!(
            DappStaking::claim_dapp_reward(RuntimeOrigin::signed(1), smart_contract, 1),
            Error::<Test>::ContractNotFound,
        );
    })
}

#[test]
fn claim_dapp_reward_from_invalid_era_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        // Advance 2 eras and try to claim from the ongoing era.
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);
        assert_noop!(
            DappStaking::claim_dapp_reward(
                RuntimeOrigin::signed(1),
                smart_contract,
                ActiveProtocolState::<Test>::get().era
            ),
            Error::<Test>::InvalidClaimEra,
        );

        // Try to claim from the era which corresponds to the voting period. No tier info should
        assert_noop!(
            DappStaking::claim_dapp_reward(RuntimeOrigin::signed(1), smart_contract, 1),
            Error::<Test>::NoDAppTierInfo,
        );
    })
}

#[test]
fn claim_dapp_reward_if_dapp_not_in_any_tier_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract_1 = MockSmartContract::Wasm(3);
        let smart_contract_2 = MockSmartContract::Wasm(5);
        assert_register(1, &smart_contract_1);
        assert_register(1, &smart_contract_2);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract_1, amount);

        // Advance 2 eras and try to claim reward for non-staked dApp.
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);
        let account = 2;
        let claim_era = ActiveProtocolState::<Test>::get().era - 1;
        assert_noop!(
            DappStaking::claim_dapp_reward(
                RuntimeOrigin::signed(account),
                smart_contract_2,
                claim_era
            ),
            Error::<Test>::NoClaimableRewards,
        );
        // Staked dApp should still be able to claim.
        assert_claim_dapp_reward(account, &smart_contract_1, claim_era);
    })
}

#[test]
fn claim_dapp_reward_twice_for_same_era_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        // Advance 3 eras and claim rewards.
        advance_to_era(ActiveProtocolState::<Test>::get().era + 3);

        // We can only claim reward ONCE for a particular era
        let claim_era_1 = ActiveProtocolState::<Test>::get().era - 2;
        assert_claim_dapp_reward(account, &smart_contract, claim_era_1);
        assert_noop!(
            DappStaking::claim_dapp_reward(
                RuntimeOrigin::signed(account),
                smart_contract,
                claim_era_1
            ),
            Error::<Test>::DAppRewardAlreadyClaimed,
        );

        // We can still claim for another valid era
        let claim_era_2 = claim_era_1 + 1;
        assert_claim_dapp_reward(account, &smart_contract, claim_era_2);
    })
}

#[test]
fn claim_dapp_reward_for_expired_era_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        let reward_retention_in_periods: PeriodNumber =
            <Test as Config>::RewardRetentionInPeriods::get();

        // Advance to period before the rewards expire. Claim reward must still work.
        advance_to_period(
            ActiveProtocolState::<Test>::get().period_number() + reward_retention_in_periods,
        );
        assert_claim_dapp_reward(account, &smart_contract, 2);

        // Advance to the next era, expiring some rewards.
        advance_to_next_period();
        assert_noop!(
            DappStaking::claim_dapp_reward(RuntimeOrigin::signed(account), smart_contract, 3),
            Error::<Test>::RewardExpired,
        );
    })
}

#[test]
fn claim_dapp_reward_fails_due_to_payout_failure() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        // Advance 2 eras so we have an entry for reward claiming
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);

        // Disable successfull reward payout
        DOES_PAYOUT_SUCCEED.with(|v| *v.borrow_mut() = false);
        assert_noop!(
            DappStaking::claim_dapp_reward(
                RuntimeOrigin::signed(account),
                smart_contract,
                ActiveProtocolState::<Test>::get().era - 1
            ),
            Error::<Test>::RewardPayoutFailed,
        );

        // Re-enable it again, claim should work again
        DOES_PAYOUT_SUCCEED.with(|v| *v.borrow_mut() = true);
        assert_claim_dapp_reward(
            account,
            &smart_contract,
            ActiveProtocolState::<Test>::get().era - 1,
        );
    })
}

#[test]
fn unstake_from_unregistered_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        // Unregister the smart contract, and unstake from it.
        assert_unregister(&smart_contract);
        assert_unstake_from_unregistered(account, &smart_contract);
    })
}

#[test]
fn unstake_from_unregistered_fails_for_active_contract() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        assert_noop!(
            DappStaking::unstake_from_unregistered(RuntimeOrigin::signed(account), smart_contract),
            Error::<Test>::ContractStillActive
        );
    })
}

#[test]
fn unstake_from_unregistered_fails_for_not_staked_contract() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);
        assert_unregister(&smart_contract);

        assert_noop!(
            DappStaking::unstake_from_unregistered(RuntimeOrigin::signed(2), smart_contract),
            Error::<Test>::NoStakingInfo
        );
    })
}

#[test]
fn unstake_from_unregistered_fails_for_past_period() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        // Unregister smart contract & advance to next period
        assert_unregister(&smart_contract);
        advance_to_next_period();

        assert_noop!(
            DappStaking::unstake_from_unregistered(RuntimeOrigin::signed(account), smart_contract),
            Error::<Test>::UnstakeFromPastPeriod
        );
    })
}

#[test]
fn cleanup_expired_entries_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contracts
        let contracts: Vec<_> = (1..=5).map(|id| MockSmartContract::Wasm(id)).collect();
        contracts.iter().for_each(|smart_contract| {
            assert_register(1, smart_contract);
        });
        let account = 2;
        assert_lock(account, 1000);

        // Scenario:
        // - 1st contract will be staked in the period that expires due to exceeded reward retention
        // - 2nd contract will be staked in the period on the edge of expiry, with loyalty flag
        // - 3rd contract will be be staked in the period on the edge of expiry, without loyalty flag
        // - 4th contract will be staked in the period right before the current one, with loyalty flag
        // - 5th contract will be staked in the period right before the current one, without loyalty flag
        //
        // Expectation: 1, 3, 5 should be removed, 2 & 4 should remain

        // 1st
        assert_stake(account, &contracts[0], 13);

        // 2nd & 3rd
        advance_to_next_period();
        for _ in 0..required_number_of_reward_claims(account) {
            assert_claim_staker_rewards(account);
        }
        assert_stake(account, &contracts[1], 17);
        advance_to_next_subperiod();

        assert_stake(account, &contracts[2], 19);

        // 4th & 5th
        let reward_retention_in_periods: PeriodNumber =
            <Test as Config>::RewardRetentionInPeriods::get();
        assert!(
            reward_retention_in_periods >= 2,
            "Sanity check, otherwise the test doesn't make sense."
        );
        advance_to_period(reward_retention_in_periods + 1);
        for _ in 0..required_number_of_reward_claims(account) {
            assert_claim_staker_rewards(account);
        }
        assert_stake(account, &contracts[3], 23);
        advance_to_next_subperiod();
        assert_stake(account, &contracts[4], 29);

        // Finally do the test
        advance_to_next_period();
        assert_cleanup_expired_entries(account);

        // Additional sanity check according to the described scenario
        assert!(!StakerInfo::<Test>::contains_key(account, &contracts[0]));
        assert!(!StakerInfo::<Test>::contains_key(account, &contracts[2]));
        assert!(!StakerInfo::<Test>::contains_key(account, &contracts[4]));

        assert!(StakerInfo::<Test>::contains_key(account, &contracts[1]));
        assert!(StakerInfo::<Test>::contains_key(account, &contracts[3]));
    })
}

#[test]
fn cleanup_expired_entries_fails_with_no_entries() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contracts
        let (contract_1, contract_2) = (MockSmartContract::Wasm(1), MockSmartContract::Wasm(2));
        assert_register(1, &contract_1);
        assert_register(1, &contract_2);

        let account = 2;
        assert_lock(account, 1000);
        assert_stake(account, &contract_1, 13);
        assert_stake(account, &contract_2, 17);

        // Advance only one period, rewards should still be valid.
        let reward_retention_in_periods: PeriodNumber =
            <Test as Config>::RewardRetentionInPeriods::get();
        assert!(
            reward_retention_in_periods >= 1,
            "Sanity check, otherwise the test doesn't make sense."
        );
        advance_to_next_period();

        assert_noop!(
            DappStaking::cleanup_expired_entries(RuntimeOrigin::signed(account)),
            Error::<Test>::NoExpiredEntries
        );
    })
}

#[test]
fn force_era_works() {
    ExtBuilder::build().execute_with(|| {
        // 1. Force new era in the voting subperiod
        let init_state = ActiveProtocolState::<Test>::get();
        assert!(
            init_state.next_era_start > System::block_number() + 1,
            "Sanity check, new era cannot start in next block, otherwise the test doesn't guarantee it tests what's expected."
        );
        assert_eq!(
            init_state.subperiod(),
            Subperiod::Voting,
            "Sanity check."
        );
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        System::assert_last_event(RuntimeEvent::DappStaking(Event::Force {
            forcing_type: ForcingType::Era,
        }));

        // Verify state change
        assert_eq!(
            ActiveProtocolState::<Test>::get().next_era_start,
            System::block_number() + 1,
        );
        assert_eq!(
            ActiveProtocolState::<Test>::get().next_subperiod_start_era(),
            init_state.next_subperiod_start_era(),
        );

        // Go to the next block, and ensure new era is started
        run_for_blocks(1);
        assert_eq!(
            ActiveProtocolState::<Test>::get().era,
            init_state.era + 1,
            "New era must be started."
        );
        assert_eq!(
            ActiveProtocolState::<Test>::get().subperiod(),
            Subperiod::BuildAndEarn,
        );

        // 2. Force new era in the build&earn subperiod
        let init_state = ActiveProtocolState::<Test>::get();
        assert!(
            init_state.next_era_start > System::block_number() + 1,
            "Sanity check, new era cannot start in next block, otherwise the test doesn't guarantee it tests what's expected."
        );
        assert!(init_state.next_subperiod_start_era() > init_state.era + 1, "Sanity check, otherwise the test doesn't guarantee it tests what's expected.");
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        System::assert_last_event(RuntimeEvent::DappStaking(Event::Force {
            forcing_type: ForcingType::Era,
        }));

        // Verify state change
        assert_eq!(
            ActiveProtocolState::<Test>::get().next_era_start,
            System::block_number() + 1,
        );
        assert_eq!(
            ActiveProtocolState::<Test>::get().next_subperiod_start_era(),
            init_state.next_subperiod_start_era(),
            "Only era is bumped, but we don't expect to switch over to the next subperiod."
        );

        run_for_blocks(1);
        assert_eq!(
            ActiveProtocolState::<Test>::get().era,
            init_state.era + 1,
            "New era must be started."
        );
        assert_eq!(
            ActiveProtocolState::<Test>::get().subperiod(),
            Subperiod::BuildAndEarn,
            "We're expected to remain in the same subperiod."
        );
    })
}

#[test]
fn force_subperiod_works() {
    ExtBuilder::build().execute_with(|| {
        // 1. Force new subperiod in the voting subperiod
        let init_state = ActiveProtocolState::<Test>::get();
        assert!(
            init_state.next_era_start > System::block_number() + 1,
            "Sanity check, new era cannot start in next block, otherwise the test doesn't guarantee it tests what's expected."
        );
        assert_eq!(
            init_state.subperiod(),
            Subperiod::Voting,
            "Sanity check."
        );
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Subperiod));
        System::assert_last_event(RuntimeEvent::DappStaking(Event::Force {
            forcing_type: ForcingType::Subperiod,
        }));

        // Verify state change
        assert_eq!(
            ActiveProtocolState::<Test>::get().next_era_start,
            System::block_number() + 1,
        );
        assert_eq!(
            ActiveProtocolState::<Test>::get().next_subperiod_start_era(),
            init_state.era + 1,
            "The switch to the next subperiod must happen in the next era."
        );

        // Go to the next block, and ensure new era is started
        run_for_blocks(1);
        assert_eq!(
            ActiveProtocolState::<Test>::get().era,
            init_state.era + 1,
            "New era must be started."
        );
        assert_eq!(
            ActiveProtocolState::<Test>::get().subperiod(),
            Subperiod::BuildAndEarn,
            "New subperiod must be started."
        );
        assert_eq!(ActiveProtocolState::<Test>::get().period_number(), init_state.period_number(), "Period must remain the same.");

        // 2. Force new era in the build&earn subperiod
        let init_state = ActiveProtocolState::<Test>::get();
        assert!(
            init_state.next_era_start > System::block_number() + 1,
            "Sanity check, new era cannot start in next block, otherwise the test doesn't guarantee it tests what's expected."
        );
        assert!(init_state.next_subperiod_start_era() > init_state.era + 1, "Sanity check, otherwise the test doesn't guarantee it tests what's expected.");
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Subperiod));
        System::assert_last_event(RuntimeEvent::DappStaking(Event::Force {
            forcing_type: ForcingType::Subperiod,
        }));

        // Verify state change
        assert_eq!(
            ActiveProtocolState::<Test>::get().next_era_start,
            System::block_number() + 1,
        );
        assert_eq!(
            ActiveProtocolState::<Test>::get().next_subperiod_start_era(),
            init_state.era + 1,
            "The switch to the next subperiod must happen in the next era."
        );

        run_for_blocks(1);
        assert_eq!(
            ActiveProtocolState::<Test>::get().era,
            init_state.era + 1,
            "New era must be started."
        );
        assert_eq!(
            ActiveProtocolState::<Test>::get().subperiod(),
            Subperiod::Voting,
            "New subperiod must be started."
        );
        assert_eq!(ActiveProtocolState::<Test>::get().period_number(), init_state.period_number() + 1, "New period must be started.");
    })
}

#[test]
fn force_with_incorrect_origin_fails() {
    ExtBuilder::build().execute_with(|| {
        assert_noop!(
            DappStaking::force(RuntimeOrigin::signed(1), ForcingType::Era),
            BadOrigin
        );
    })
}

#[test]
fn get_dapp_tier_assignment_basic_example_works() {
    ExtBuilder::build().execute_with(|| {
        // This test will rely on the configuration inside the mock file.
        // If that changes, this test will have to be updated as well.

        // Scenario:
        // - 1st tier is filled up, with one dApp satisfying the threshold but not making it due to lack of tier capacity
        // - 2nd tier has 2 dApps - 1 that could make it into the 1st tier and one that's supposed to be in the 2nd tier
        // - 3rd tier has no dApps
        // - 4th tier has 2 dApps
        // - 1 dApp doesn't make it into any tier

        // Register smart contracts
        let tier_config = TierConfig::<Test>::get();
        let number_of_smart_contracts = tier_config.slots_per_tier[0] + 1 + 1 + 0 + 2 + 1;
        let smart_contracts: Vec<_> = (1..=number_of_smart_contracts)
            .map(|x| {
                let smart_contract = MockSmartContract::Wasm(x.into());
                assert_register(x.into(), &smart_contract);
                smart_contract
            })
            .collect();
        let mut dapp_index: usize = 0;

        fn lock_and_stake(account: usize, smart_contract: &MockSmartContract, amount: Balance) {
            let account = account.try_into().unwrap();
            Balances::make_free_balance_be(&account, amount);
            assert_lock(account, amount);
            assert_stake(account, smart_contract, amount);
        }

        // 1st tier is completely filled up, with 1 more dApp not making it inside
        for x in 0..tier_config.slots_per_tier[0] as Balance {
            lock_and_stake(
                dapp_index,
                &smart_contracts[dapp_index],
                tier_config.tier_thresholds[0].threshold() + x + 1,
            );
            dapp_index += 1;
        }
        // One that won't make it into the 1st tier.
        lock_and_stake(
            dapp_index,
            &smart_contracts[dapp_index],
            tier_config.tier_thresholds[0].threshold(),
        );
        dapp_index += 1;

        // 2nd tier - 1 dedicated dApp
        lock_and_stake(
            dapp_index,
            &smart_contracts[dapp_index],
            tier_config.tier_thresholds[0].threshold() - 1,
        );
        dapp_index += 1;

        // 3rd tier is empty
        // 4th tier has 2 dApps
        for x in 0..2 {
            lock_and_stake(
                dapp_index,
                &smart_contracts[dapp_index],
                tier_config.tier_thresholds[3].threshold() + x,
            );
            dapp_index += 1;
        }

        // One dApp doesn't make it into any tier
        lock_and_stake(
            dapp_index,
            &smart_contracts[dapp_index],
            tier_config.tier_thresholds[3].threshold() - 1,
        );

        // Finally, the actual test
        let protocol_state = ActiveProtocolState::<Test>::get();
        let dapp_reward_pool = 1000000;
        let (tier_assignment, counter) = DappStaking::get_dapp_tier_assignment(
            protocol_state.era + 1,
            protocol_state.period_number(),
            dapp_reward_pool,
        );

        // Basic checks
        let number_of_tiers: u32 = <Test as Config>::NumberOfTiers::get();
        assert_eq!(tier_assignment.period, protocol_state.period_number());
        assert_eq!(tier_assignment.rewards.len(), number_of_tiers as usize);
        assert_eq!(
            tier_assignment.dapps.len(),
            number_of_smart_contracts as usize - 1,
            "One contract doesn't make it into any tier."
        );
        assert_eq!(counter, number_of_smart_contracts);

        // 1st tier checks
        let (entry_1, entry_2) = (tier_assignment.dapps[0], tier_assignment.dapps[1]);
        assert_eq!(entry_1.dapp_id, 0);
        assert_eq!(entry_1.tier_id, Some(0));

        assert_eq!(entry_2.dapp_id, 1);
        assert_eq!(entry_2.tier_id, Some(0));

        // 2nd tier checks
        let (entry_3, entry_4) = (tier_assignment.dapps[2], tier_assignment.dapps[3]);
        assert_eq!(entry_3.dapp_id, 2);
        assert_eq!(entry_3.tier_id, Some(1));

        assert_eq!(entry_4.dapp_id, 3);
        assert_eq!(entry_4.tier_id, Some(1));

        // 4th tier checks
        let (entry_5, entry_6) = (tier_assignment.dapps[4], tier_assignment.dapps[5]);
        assert_eq!(entry_5.dapp_id, 4);
        assert_eq!(entry_5.tier_id, Some(3));

        assert_eq!(entry_6.dapp_id, 5);
        assert_eq!(entry_6.tier_id, Some(3));

        // Sanity check - last dapp should not exists in the tier assignment
        assert!(tier_assignment
            .dapps
            .binary_search_by(|x| x.dapp_id.cmp(&(dapp_index.try_into().unwrap())))
            .is_err());

        // Check that rewards are calculated correctly
        tier_config
            .reward_portion
            .iter()
            .zip(tier_config.slots_per_tier.iter())
            .enumerate()
            .for_each(|(idx, (reward_portion, slots))| {
                let total_tier_allocation = *reward_portion * dapp_reward_pool;
                let tier_reward: Balance = total_tier_allocation / (*slots as Balance);

                assert_eq!(tier_assignment.rewards[idx], tier_reward,);
            });
    })
}

#[test]
fn get_dapp_tier_assignment_zero_slots_per_tier_works() {
    ExtBuilder::build().execute_with(|| {
        // This test will rely on the configuration inside the mock file.
        // If that changes, this test might have to be updated as well.

        // Ensure that first tier has 0 slots.
        TierConfig::<Test>::mutate(|config| {
            let slots_in_first_tier = config.slots_per_tier[0];
            config.number_of_slots = config.number_of_slots - slots_in_first_tier;
            config.slots_per_tier[0] = 0;
        });

        // Calculate tier assignment (we don't need dApps for this test)
        let protocol_state = ActiveProtocolState::<Test>::get();
        let dapp_reward_pool = 1000000;
        let (tier_assignment, counter) = DappStaking::get_dapp_tier_assignment(
            protocol_state.era,
            protocol_state.period_number(),
            dapp_reward_pool,
        );

        // Basic checks
        let number_of_tiers: u32 = <Test as Config>::NumberOfTiers::get();
        assert_eq!(tier_assignment.period, protocol_state.period_number());
        assert_eq!(tier_assignment.rewards.len(), number_of_tiers as usize);
        assert!(tier_assignment.dapps.is_empty());
        assert!(counter.is_zero());

        assert!(
            tier_assignment.rewards[0].is_zero(),
            "1st tier has no slots so no rewards should be assigned to it."
        );

        // Regardless of that, other tiers shouldn't benefit from this
        assert!(tier_assignment.rewards.iter().sum::<Balance>() < dapp_reward_pool);
    })
}

#[test]
fn advance_for_some_periods_works() {
    ExtBuilder::build().execute_with(|| {
        advance_to_period(10);
    })
}

////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////
/////// More complex & composite scenarios, maybe move them into a separate file

#[test]
fn unlock_after_staked_period_ends_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let account = 2;
        let amount = 101;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        // Advance to the next period, and ensure stake is reset and can be fully unlocked
        advance_to_next_period();
        assert!(Ledger::<Test>::get(&account)
            .staked_amount(ActiveProtocolState::<Test>::get().period_number())
            .is_zero());
        assert_unlock(account, amount);
        assert_eq!(Ledger::<Test>::get(&account).unlocking_amount(), amount);
    })
}

#[test]
fn unstake_from_a_contract_staked_in_past_period_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let smart_contract_1 = MockSmartContract::Wasm(1);
        let smart_contract_2 = MockSmartContract::Wasm(2);
        assert_register(1, &smart_contract_1);
        assert_register(1, &smart_contract_2);
        let account = 2;
        assert_lock(account, 300);

        // Stake some amount on the 2nd contract.
        let stake_amount = 100;
        assert_stake(account, &smart_contract_2, stake_amount);

        // Advance to the next period, and stake on the 1st contract.
        advance_to_next_period();
        for _ in 0..required_number_of_reward_claims(account) {
            assert_claim_staker_rewards(account);
        }

        // Try to unstake from the 2nd contract, which is no longer staked on due to period change.
        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(account), smart_contract_2, 1,),
            Error::<Test>::UnstakeFromPastPeriod
        );

        // Staking on the 1st contract should succeed since we haven't staked on it before so there are no bonus rewards to claim
        assert_stake(account, &smart_contract_1, stake_amount);

        // Even with active stake on the 1st contract, unstake from 2nd should still fail since period change reset its stake.
        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(account), smart_contract_2, 1,),
            Error::<Test>::UnstakeFromPastPeriod
        );
    })
}

#[test]
fn stake_and_unstake_after_reward_claim_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let amount = 400;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount - 100);

        // Advance 2 eras so we have claimable rewards. Both stake & unstake should fail.
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract, 1),
            Error::<Test>::UnclaimedRewards
        );
        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(account), smart_contract, 1),
            Error::<Test>::UnclaimedRewards
        );

        // Claim rewards, unstake should work now.
        for _ in 0..required_number_of_reward_claims(account) {
            assert_claim_staker_rewards(account);
        }
        assert_stake(account, &smart_contract, 1);
        assert_unstake(account, &smart_contract, 1);
    })
}

#[test]
fn stake_after_period_ends_with_max_staked_contracts() {
    ExtBuilder::build().execute_with(|| {
        let max_number_of_contracts: u32 = <Test as Config>::MaxNumberOfStakedContracts::get();

        // Lock amount by staker
        let account = 1;
        assert_lock(account, 100 as Balance * max_number_of_contracts as Balance);

        // Register smart contracts up to the max allowed number
        for id in 1..=max_number_of_contracts {
            let smart_contract = MockSmartContract::Wasm(id.into());
            assert_register(2, &smart_contract);
            assert_stake(account, &smart_contract, 10);
        }

        // Advance to the next period, and claim ALL rewards
        advance_to_next_period();
        for _ in 0..required_number_of_reward_claims(account) {
            assert_claim_staker_rewards(account);
        }
        for id in 1..=max_number_of_contracts {
            let smart_contract = MockSmartContract::Wasm(id.into());
            assert_claim_bonus_reward(account, &smart_contract);
        }

        // Make sure it's possible to stake again
        for id in 1..=max_number_of_contracts {
            let smart_contract = MockSmartContract::Wasm(id.into());
            assert_stake(account, &smart_contract, 10);
        }
    })
}

#[test]
fn post_unlock_balance_cannot_be_transfered() {
    ExtBuilder::build().execute_with(|| {
        let staker = 2;

        // Lock some of the free balance
        let init_free_balance = Balances::free_balance(&staker);
        let lock_amount = init_free_balance / 3;
        assert_lock(staker, lock_amount);

        // Make sure second account is empty
        let other_account = 42;
        assert_ok!(Balances::write_balance(&other_account, 0));

        // 1. Ensure we can only transfer what is not locked/frozen.
        assert_ok!(Balances::transfer_all(
            RuntimeOrigin::signed(staker),
            other_account,
            true
        ));
        assert_eq!(
            Balances::free_balance(&other_account),
            init_free_balance - lock_amount,
            "Only what is locked can be transferred."
        );

        // 2. Start the 'unlocking process' for the locked amount, but ensure it still cannot be transferred.
        assert_unlock(staker, lock_amount);

        assert_ok!(Balances::write_balance(&other_account, 0));
        assert_ok!(Balances::transfer_all(
            RuntimeOrigin::signed(staker),
            other_account,
            true
        ));
        assert!(
            Balances::free_balance(&other_account).is_zero(),
            "Nothing could have been transferred since it's still locked/frozen."
        );

        // 3. Claim the unlocked chunk, and ensure it can be transferred afterwards.
        run_to_block(Ledger::<Test>::get(&staker).unlocking[0].unlock_block);
        assert_claim_unlocked(staker);

        assert_ok!(Balances::write_balance(&other_account, 0));
        assert_ok!(Balances::transfer_all(
            RuntimeOrigin::signed(staker),
            other_account,
            false
        ));
        assert_eq!(
            Balances::free_balance(&other_account),
            lock_amount,
            "Everything should have been transferred."
        );
        assert!(Balances::free_balance(&staker).is_zero());
    })
}
