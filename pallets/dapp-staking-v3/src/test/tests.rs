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
    pallet as pallet_dapp_staking, ActiveProtocolState, DAppId, Error, IntegratedDApps, Ledger,
    NextDAppId, StakeInfo,
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
fn lock_with_too_many_chunks_fails() {
    ExtBuilder::build().execute_with(|| {
        let max_locked_chunks = <Test as pallet_dapp_staking::Config>::MaxLockedChunks::get();
        let minimum_locked_amount: Balance =
            <Test as pallet_dapp_staking::Config>::MinimumLockedAmount::get();

        // Fill up the locked chunks to the limit
        let locker = 1;
        assert_lock(locker, minimum_locked_amount);
        for current_era in 1..max_locked_chunks {
            advance_to_era(current_era + 1);
            assert_lock(locker, 1);
        }

        // Ensure we can still lock in the current era since number of chunks should not increase
        for _ in 0..10 {
            assert_lock(locker, 1);
        }

        // Advance to the next era and ensure it's not possible to add additional chunks
        advance_to_era(ActiveProtocolState::<Test>::get().era + 1);
        assert_noop!(
            DappStaking::lock(RuntimeOrigin::signed(locker), 1),
            Error::<Test>::TooManyLockedBalanceChunks,
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
        advance_to_era(ActiveProtocolState::<Test>::get().era + 1);
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
        advance_to_era(ActiveProtocolState::<Test>::get().era + 1);
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
        advance_to_era(ActiveProtocolState::<Test>::get().era + 1);
        assert_lock(account, lock_amount);

        // Hacky, maybe improve later when staking is implemented?
        let stake_amount = 91;
        Ledger::<Test>::mutate(&account, |ledger| {
            ledger.staked = StakeInfo {
                amount: stake_amount,
                period: ActiveProtocolState::<Test>::get().period,
            }
        });

        // Try to unlock more than is available, due to active staked amount
        assert_unlock(account, lock_amount - stake_amount + 1);

        // Ensure there is no effect of staked amount once we move to the following period
        assert_lock(account, lock_amount - stake_amount); // restore previous state
        advance_to_period(ActiveProtocolState::<Test>::get().period + 1);
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
        advance_to_era(ActiveProtocolState::<Test>::get().era + 1);
        assert_unlock(account, unlock_amount * 2);

        // Advance few more eras, and unlock everything
        advance_to_era(ActiveProtocolState::<Test>::get().era + 7);
        assert_unlock(account, lock_amount);
        assert!(Ledger::<Test>::get(&account)
            .active_locked_amount()
            .is_zero());

        // Advance one more era and ensure we can still lock & unlock
        advance_to_era(ActiveProtocolState::<Test>::get().era + 1);
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
        advance_to_era(ActiveProtocolState::<Test>::get().era + 1);

        // We stake so the amount is just below the minimum locked amount, causing full unlock impossible.
        let minimum_locked_amount: Balance =
            <Test as pallet_dapp_staking::Config>::MinimumLockedAmount::get();
        let stake_amount = minimum_locked_amount - 1;
        // Hacky, maybe improve later when staking is implemented?
        Ledger::<Test>::mutate(&account, |ledger| {
            ledger.staked = StakeInfo {
                amount: stake_amount,
                period: ActiveProtocolState::<Test>::get().period,
            }
        });

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
        advance_to_era(ActiveProtocolState::<Test>::get().era + 1);

        // Unlock with zero fails
        assert_noop!(
            DappStaking::unlock(RuntimeOrigin::signed(account), 0),
            Error::<Test>::ZeroAmount,
        );

        // Stake everything, so available unlock amount is always zero
        // Hacky, maybe improve later when staking is implemented?
        Ledger::<Test>::mutate(&account, |ledger| {
            ledger.staked = StakeInfo {
                amount: lock_amount,
                period: ActiveProtocolState::<Test>::get().period,
            }
        });

        // Try to unlock anything, expect zero amount error
        assert_noop!(
            DappStaking::unlock(RuntimeOrigin::signed(account), lock_amount),
            Error::<Test>::ZeroAmount,
        );
    })
}

#[test]
fn unlock_with_exceeding_locked_storage_limits_fails() {
    ExtBuilder::build().execute_with(|| {
        let account = 2;
        let lock_amount = 103;
        assert_lock(account, lock_amount);

        let unlock_amount = 3;
        for _ in 0..<Test as pallet_dapp_staking::Config>::MaxLockedChunks::get() {
            advance_to_era(ActiveProtocolState::<Test>::get().era + 1);
            assert_unlock(account, unlock_amount);
        }

        // We can still unlock in the current era, theoretically
        for _ in 0..5 {
            assert_unlock(account, unlock_amount);
        }

        // Following unlock should fail due to exceeding storage limits
        advance_to_era(ActiveProtocolState::<Test>::get().era + 1);
        assert_noop!(
            DappStaking::unlock(RuntimeOrigin::signed(account), unlock_amount),
            Error::<Test>::TooManyLockedBalanceChunks,
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
