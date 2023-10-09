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

use crate::test::mock::*;
use crate::test::testing_utils::*;
use crate::{
    pallet as pallet_dapp_staking, ActiveProtocolState, DAppId, EraNumber, Error, IntegratedDApps,
    Ledger, NextDAppId, PeriodType,
};

use frame_support::{assert_noop, assert_ok, error::BadOrigin, traits::Get};
use sp_runtime::traits::Zero;

#[test]
fn maintenace_mode_works() {
    ExtBuilder::build().execute_with(|| {
        // Check that maintenance mode is disabled by default
        assert!(!ActiveProtocolState::<Test>::get().maintenance);

        // Enable maintenance mode & check post-state
        assert_ok!(DappStaking::maintenance_mode(RuntimeOrigin::root(), true));
        assert!(ActiveProtocolState::<Test>::get().maintenance);

        // Call still works, even in maintenance mode
        assert_ok!(DappStaking::maintenance_mode(RuntimeOrigin::root(), true));
        assert!(ActiveProtocolState::<Test>::get().maintenance);

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

        assert_noop!(
            DappStaking::register(RuntimeOrigin::root(), 1, MockSmartContract::Wasm(1)),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::set_dapp_reward_destination(
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
            DappStaking::stake(RuntimeOrigin::signed(1), MockSmartContract::default(), 100),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(1), MockSmartContract::default(), 100),
            Error::<Test>::Disabled
        );
    })
}

#[test]
fn on_initialize_state_change_works() {
    ExtBuilder::build().execute_with(|| {
        // TODO: test `EraInfo` change and verify events. This would be good to do each time we call the helper functions to go to next era or period.

        // Sanity check
        let protocol_state = ActiveProtocolState::<Test>::get();
        assert_eq!(protocol_state.era, 1);
        assert_eq!(protocol_state.period_number(), 1);
        assert_eq!(protocol_state.period_type(), PeriodType::Voting);
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
            protocol_state.period_type(),
            PeriodType::Voting,
            "Period type should still be the same."
        );
        assert_eq!(protocol_state.era, 1);

        run_for_blocks(1);
        let protocol_state = ActiveProtocolState::<Test>::get();
        assert_eq!(protocol_state.period_type(), PeriodType::BuildAndEarn);
        assert_eq!(protocol_state.era, 2);
        assert_eq!(protocol_state.period_number(), 1);

        // Advance eras just until we reach the next voting period
        let eras_per_bep_period: EraNumber =
            <Test as pallet_dapp_staking::Config>::StandardErasPerBuildAndEarnPeriod::get();
        let blocks_per_era: BlockNumber =
            <Test as pallet_dapp_staking::Config>::StandardEraLength::get();
        for era in 2..(2 + eras_per_bep_period - 1) {
            let pre_block = System::block_number();
            advance_to_next_era();
            assert_eq!(System::block_number(), pre_block + blocks_per_era);
            let protocol_state = ActiveProtocolState::<Test>::get();
            assert_eq!(protocol_state.period_type(), PeriodType::BuildAndEarn);
            assert_eq!(protocol_state.period_number(), 1);
            assert_eq!(protocol_state.era, era + 1);
        }

        // Finaly advance over to the next era and ensure we're back to voting period
        advance_to_next_era();
        let protocol_state = ActiveProtocolState::<Test>::get();
        assert_eq!(protocol_state.period_type(), PeriodType::Voting);
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
        let limit = <Test as pallet_dapp_staking::Config>::MaxNumberOfContracts::get();
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
fn set_dapp_reward_destination_for_contract_is_ok() {
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
        assert_set_dapp_reward_destination(owner, &smart_contract, Some(3));
        assert_set_dapp_reward_destination(owner, &smart_contract, Some(5));
        assert_set_dapp_reward_destination(owner, &smart_contract, None);
    })
}

#[test]
fn set_dapp_reward_destination_fails() {
    ExtBuilder::build().execute_with(|| {
        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);

        // Contract doesn't exist yet
        assert_noop!(
            DappStaking::set_dapp_reward_destination(
                RuntimeOrigin::signed(owner),
                smart_contract,
                Some(5)
            ),
            Error::<Test>::ContractNotFound
        );

        // Non-owner cannnot change reward destination
        assert_register(owner, &smart_contract);
        assert_noop!(
            DappStaking::set_dapp_reward_destination(
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
fn unregister_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Prepare dApp
        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);
        assert_register(owner, &smart_contract);

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
        assert_lock(
            locker,
            <Test as pallet_dapp_staking::Config>::MinimumLockedAmount::get(),
        );
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
        let minimum_locked_amount: Balance =
            <Test as pallet_dapp_staking::Config>::MinimumLockedAmount::get();
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
        let minimum_locked_amount: Balance =
            <Test as pallet_dapp_staking::Config>::MinimumLockedAmount::get();
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
        advance_to_period(ActiveProtocolState::<Test>::get().period_info.number + 1);
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
        let minimum_locked_amount: Balance =
            <Test as pallet_dapp_staking::Config>::MinimumLockedAmount::get();
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
        for _ in 0..<Test as pallet_dapp_staking::Config>::MaxUnlockingChunks::get() {
            run_for_blocks(1);
            assert_unlock(account, unlock_amount);
        }

        // We can still unlock in the current erblocka, theoretically
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
        let unlocking_blocks: BlockNumber =
            <Test as pallet_dapp_staking::Config>::UnlockingPeriod::get();

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
        let max_unlocking_chunks: u32 =
            <Test as pallet_dapp_staking::Config>::MaxUnlockingChunks::get();
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
        let unlocking_blocks: BlockNumber =
            <Test as pallet_dapp_staking::Config>::UnlockingPeriod::get();
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

        let max_unlocking_chunks: u32 =
            <Test as pallet_dapp_staking::Config>::MaxUnlockingChunks::get();
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
        let minimum_locked_amount: Balance =
            <Test as pallet_dapp_staking::Config>::MinimumLockedAmount::get();

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
        let unlocking_blocks: BlockNumber =
            <Test as pallet_dapp_staking::Config>::UnlockingPeriod::get();
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
        let smart_contract = MockSmartContract::default();
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let lock_amount = 300;
        assert_lock(account, lock_amount);

        // 1st scenario - stake some amount, and then some more
        let (stake_amount_1, stake_amount_2) = (31, 29);
        assert_stake(account, &smart_contract, stake_amount_1);
        assert_stake(account, &smart_contract, stake_amount_2);

        // 2nd scenario - stake in the next era
        advance_to_next_era();
        let stake_amount_3 = 23;
        assert_stake(account, &smart_contract, stake_amount_3);

        // 3rd scenario - advance era again but create a gap, and then stake
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);
        let stake_amount_4 = 19;
        assert_stake(account, &smart_contract, stake_amount_4);

        // 4th scenario - advance period, and stake
        // advance_to_next_era();
        // advance_to_next_period();
        // let stake_amount_5 = 17;
        // assert_stake(account, &smart_contract, stake_amount_5);
        // TODO: this can only be tested after reward claiming has been implemented!!!
    })
}

#[test]
fn stake_with_zero_amount_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
        let account = 2;
        assert_register(1, &smart_contract);
        assert_lock(account, 300);

        // Force Build&Earn period
        ActiveProtocolState::<Test>::mutate(|state| {
            state.period_info.period_type = PeriodType::BuildAndEarn;
            state.period_info.ending_era = state.era + 1;
        });

        // Try to stake in the final era of the period, which should fail.
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::PeriodEndsInNextEra
        );
    })
}

#[test]
fn stake_fails_if_unclaimed_rewards_from_past_period_remain() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let smart_contract = MockSmartContract::default();
        let account = 2;
        assert_register(1, &smart_contract);
        assert_lock(account, 300);

        // Stake some amount, then force next period
        assert_stake(account, &smart_contract, 100);
        advance_to_next_period();
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::UnclaimedRewardsFromPastPeriods
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
fn stake_fails_due_to_too_many_chunks() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let smart_contract = MockSmartContract::default();
        let account = 3;
        assert_register(1, &smart_contract);
        let lock_amount = 500;
        assert_lock(account, lock_amount);

        // Keep on staking & creating chunks until capacity is reached
        for _ in 0..(<Test as pallet_dapp_staking::Config>::MaxStakingChunks::get()) {
            advance_to_next_era();
            assert_stake(account, &smart_contract, 10);
        }

        // Ensure we can still stake in the current era since an entry exists
        assert_stake(account, &smart_contract, 10);

        // Staking in the next era results in error due to too many chunks
        advance_to_next_era();
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract.clone(), 10),
            Error::<Test>::TooManyStakeChunks
        );
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
        let min_stake_amount: Balance =
            <Test as pallet_dapp_staking::Config>::MinimumStakeAmount::get();
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
fn unstake_basic_example_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::default();
        assert_register(dev_account, &smart_contract);

        let account = 2;
        let lock_amount = 400;
        assert_lock(account, lock_amount);

        // Prep step - stake some amount
        let stake_amount_1 = 83;
        assert_stake(account, &smart_contract, stake_amount_1);

        // 1st scenario - unstake some amount, in the current era.
        let unstake_amount_1 = 3;
        assert_unstake(account, &smart_contract, unstake_amount_1);

        // 2nd scenario - advance to next era/period type, and unstake some more
        let unstake_amount_2 = 7;
        let unstake_amount_3 = 11;
        advance_to_next_era();
        assert_eq!(
            ActiveProtocolState::<Test>::get().period_type(),
            PeriodType::BuildAndEarn,
            "Sanity check, period type change must happe."
        );
        assert_unstake(account, &smart_contract, unstake_amount_2);
        assert_unstake(account, &smart_contract, unstake_amount_3);

        // 3rd scenario - advance few eras to create a gap, and unstake some more
        advance_to_era(ActiveProtocolState::<Test>::get().era + 3);
        assert_unstake(account, &smart_contract, unstake_amount_3);
        assert_unstake(account, &smart_contract, unstake_amount_2);
    })
}

#[test]
fn unstake_with_zero_amount_fails() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract & lock some amount
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
        // TODO: need to implement reward claiming for this check to work!
        // assert_stake(account, &smart_contract_1, stake_amount);
        // Try to unstake from the 2nd contract, which is no longer staked on due to period change.
        // assert_noop!(
        //     DappStaking::unstake(
        //         RuntimeOrigin::signed(account),
        //         smart_contract_2,
        //         1,
        //     ),
        //     Error::<Test>::UnstakeFromPastPeriod
        // );
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

        // 1st scenario - stake some amount on the first contract, and try to unstake more than was staked
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
fn unstake_fails_due_to_too_many_chunks() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract,lock & stake some amount
        let smart_contract = MockSmartContract::default();
        let account = 2;
        assert_register(1, &smart_contract);
        let lock_amount = 1000;
        assert_lock(account, lock_amount);
        assert_stake(account, &smart_contract, lock_amount);

        // Keep on unstaking & creating chunks until capacity is reached
        for _ in 0..(<Test as pallet_dapp_staking::Config>::MaxStakingChunks::get()) {
            advance_to_next_era();
            assert_unstake(account, &smart_contract, 11);
        }

        // Ensure we can still unstake in the current era since an entry exists
        assert_unstake(account, &smart_contract, 10);

        // Staking in the next era results in error due to too many chunks
        advance_to_next_era();
        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(account), smart_contract.clone(), 10),
            Error::<Test>::TooManyStakeChunks
        );
    })
}
