// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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
    pallet::Config, ActiveProtocolState, ContractStake, DAppId, DAppTierRewardsFor, DAppTiers,
    EraRewards, Error, Event, ForcingType, GenesisConfig, IntegratedDApps, Ledger, NextDAppId,
    Perbill, PeriodNumber, Permill, Safeguard, StakerInfo, StaticTierParams, Subperiod, TierConfig,
    TierThreshold,
};

use frame_support::{
    assert_noop, assert_ok, assert_storage_noop,
    error::BadOrigin,
    traits::{
        fungible::Unbalanced as FunUnbalanced, Currency, Get, OnFinalize, OnInitialize,
        ReservableCurrency,
    },
    BoundedVec,
};
use sp_runtime::{
    traits::{ConstU32, Zero},
    BoundedBTreeMap, FixedU128,
};

use astar_primitives::{
    dapp_staking::{
        CycleConfiguration, EraNumber, RankedTier, SmartContractHandle, StakingRewardHandler,
        TierSlots,
    },
    Balance, BlockNumber,
};

use std::collections::BTreeMap;

#[test]
fn maintenances_mode_works() {
    ExtBuilder::default().build_and_execute(|| {
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
fn maintenance_mode_call_filtering_works() {
    ExtBuilder::default().build_and_execute(|| {
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
        assert_noop!(
            DappStaking::unbond_and_unstake(
                RuntimeOrigin::signed(1),
                MockSmartContract::wasm(1 as AccountId),
                100
            ),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::withdraw_unbonded(RuntimeOrigin::signed(1),),
            Error::<Test>::Disabled
        );
    })
}

#[test]
fn on_initialize_is_noop_if_no_era_change() {
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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

        // Finally advance over to the next era and ensure we're back to voting period
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
    ExtBuilder::default().build_and_execute(|| {
        // Basic test
        assert_register(5, &MockSmartContract::Wasm(1));

        // Register two contracts using the same owner
        assert_register(7, &MockSmartContract::Wasm(2));
        assert_register(7, &MockSmartContract::Wasm(3));

        // Register a contract using non-root origin
        let smart_contract = MockSmartContract::Wasm(4);
        let owner = 11;
        let dapp_id = NextDAppId::<Test>::get();
        assert_ok!(DappStaking::register(
            RuntimeOrigin::signed(ContractRegisterAccount::get()),
            owner,
            smart_contract.clone()
        ));
        System::assert_last_event(RuntimeEvent::DappStaking(Event::DAppRegistered {
            owner,
            smart_contract,
            dapp_id,
        }));
    })
}

#[test]
fn register_with_incorrect_origin_fails() {
    ExtBuilder::default().build_and_execute(|| {
        // Test assumes that Contract registry & Manager origins are different.
        assert_noop!(
            DappStaking::register(
                RuntimeOrigin::signed(ManagerAccount::get()),
                3,
                MockSmartContract::Wasm(2)
            ),
            BadOrigin
        );

        // Test assumes register & unregister origins are different.
        assert_noop!(
            DappStaking::register(
                RuntimeOrigin::signed(ContractUnregisterAccount::get()),
                3,
                MockSmartContract::Wasm(2)
            ),
            BadOrigin
        );
    })
}

#[test]
fn register_already_registered_contract_fails() {
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
        // Prepare & register smart contract
        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);
        assert_register(owner, &smart_contract);

        // Update beneficiary
        assert!(IntegratedDApps::<Test>::get(&smart_contract)
            .unwrap()
            .reward_beneficiary
            .is_none());
        assert_set_dapp_reward_beneficiary(owner, &smart_contract, Some(3));
        assert_set_dapp_reward_beneficiary(owner, &smart_contract, Some(5));
        assert_set_dapp_reward_beneficiary(owner, &smart_contract, None);
    })
}

#[test]
fn set_dapp_reward_beneficiary_fails() {
    ExtBuilder::default().build_and_execute(|| {
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

        // Non-owner cannot change reward destination
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
        // Prepare dApp
        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);
        assert_register(owner, &smart_contract);

        // Nothing staked on contract, just unregister it.
        assert_unregister(&smart_contract);

        // Prepare another dApp, unregister it using non-root origin
        let smart_contract = MockSmartContract::Wasm(5);
        assert_register(owner, &smart_contract);

        assert_ok!(DappStaking::unregister(
            RuntimeOrigin::signed(ContractUnregisterAccount::get()),
            smart_contract.clone(),
        ));
        System::assert_last_event(RuntimeEvent::DappStaking(Event::DAppUnregistered {
            smart_contract: smart_contract.clone(),
            era: ActiveProtocolState::<Test>::get().era,
        }));
    })
}

#[test]
#[ignore]
/// TODO - Reestablish this test once this bug is fixed: <https://github.com/AstarNetwork/Astar/issues/1333>
fn unregister_with_active_stake_is_ok() {
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
        assert_noop!(
            DappStaking::unregister(
                RuntimeOrigin::signed(ContractRegisterAccount::get()),
                smart_contract
            ),
            BadOrigin
        );

        // Cannot unregister same contract twice
        assert_unregister(&smart_contract);
        assert_noop!(
            DappStaking::unregister(RuntimeOrigin::root(), smart_contract),
            Error::<Test>::ContractNotFound
        );
    })
}

#[test]
fn lock_is_ok() {
    ExtBuilder::default().build_and_execute(|| {
        // Lock some amount
        let locker = 2;
        let free_balance = Balances::total_balance(&locker);
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
fn lock_with_reserve_is_ok() {
    ExtBuilder::default().build_and_execute(|| {
        // Prepare locker account
        let locker = 30;
        let minimum_locked_amount: Balance = <Test as Config>::MinimumLockedAmount::get();
        Balances::make_free_balance_be(&locker, minimum_locked_amount);
        assert_ok!(Balances::reserve(&locker, 1));
        assert_eq!(
            Balances::free_balance(&locker),
            minimum_locked_amount - 1,
            "Sanity check post-reserve."
        );

        // Lock must still work since account is not blacklisted and has enough total balance to cover the lock requirement
        assert_lock(locker, minimum_locked_amount);
    })
}

#[test]
fn lock_with_incorrect_amount_fails() {
    ExtBuilder::default().build_and_execute(|| {
        // Cannot lock "nothing"
        assert_noop!(
            DappStaking::lock(RuntimeOrigin::signed(1), Balance::zero()),
            Error::<Test>::ZeroAmount,
        );

        // Attempting to lock something after everything has been locked is same
        // as attempting to lock with "nothing"
        let locker = 1;
        assert_lock(locker, Balances::total_balance(&locker));
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
fn lock_with_blacklisted_account_fails() {
    ExtBuilder::default().build_and_execute(|| {
        Balances::make_free_balance_be(&BLACKLISTED_ACCOUNT, 100000);

        assert_noop!(
            DappStaking::lock(RuntimeOrigin::signed(BLACKLISTED_ACCOUNT), 1000),
            Error::<Test>::AccountNotAvailableForDappStaking,
        );
    })
}

#[test]
fn unbond_and_unstake_is_ok() {
    ExtBuilder::default().build_and_execute(|| {
        // Lock some amount
        let account = 2;
        let lock_amount = 101;
        assert_lock(account, lock_amount);

        // 'unbond_and_unstake' some amount, assert expected event is emitted
        let unlock_amount = 19;
        let dummy_smart_contract = MockSmartContract::Wasm(1);
        assert_ok!(DappStaking::unbond_and_unstake(
            RuntimeOrigin::signed(account),
            dummy_smart_contract,
            unlock_amount
        ));
        System::assert_last_event(RuntimeEvent::DappStaking(Event::Unlocking {
            account,
            amount: unlock_amount,
        }));
    })
}

#[test]
fn unlock_basic_example_is_ok() {
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
fn unlock_with_amount_higher_than_available_is_ok() {
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
fn withdraw_unbonded_is_ok() {
    ExtBuilder::default().build_and_execute(|| {
        // Lock & immediately unlock some amount
        let account = 2;
        let lock_amount = 97;
        let unlock_amount = 11;
        assert_lock(account, lock_amount);
        assert_unlock(account, unlock_amount);

        // Run for enough blocks so the chunk becomes claimable
        let unlocking_blocks = DappStaking::unlocking_period();
        run_for_blocks(unlocking_blocks);
        assert_ok!(DappStaking::withdraw_unbonded(RuntimeOrigin::signed(
            account
        )));
        System::assert_last_event(RuntimeEvent::DappStaking(Event::ClaimedUnlocked {
            account,
            amount: unlock_amount,
        }));
    })
}

#[test]
fn claim_unlocked_is_ok() {
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
        assert_noop!(
            DappStaking::relock_unlocking(RuntimeOrigin::signed(1)),
            Error::<Test>::NoUnlockingChunks,
        );
    })
}

#[test]
fn relock_unlocking_insufficient_lock_amount_fails() {
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
        let account = 2;
        assert_lock(account, 300);

        // Try to stake on non-existing contract
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::ContractNotFound
        );

        // Try to stake on unregistered smart contract
        assert_register(1, &smart_contract);
        assert_unregister(&smart_contract);
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::ContractNotFound
        );
    })
}

#[test]
fn stake_in_final_era_fails() {
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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

/// TODO - Reestablish this test once this bug is fixed: <https://github.com/AstarNetwork/Astar/issues/1333>
#[test]
#[ignore]
fn unstake_on_invalid_dapp_fails() {
    ExtBuilder::default().build_and_execute(|| {
        let account = 2;
        assert_lock(account, 300);

        // Try to unstake from non-existing contract
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::ContractNotFound
        );

        // Try to unstake from unregistered smart contract
        assert_register(1, &smart_contract);
        assert_stake(account, &smart_contract, 100);
        assert_unregister(&smart_contract);
        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::ContractNotFound
        );
    })
}

#[test]
fn unstake_with_exceeding_amount_fails() {
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
fn claim_staker_rewards_era_after_expiry_works() {
    ExtBuilder::default().build_and_execute(|| {
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

        // Claim must still work
        assert_claim_staker_rewards(account);
    })
}

#[test]
fn claim_staker_rewards_after_expiry_fails() {
    ExtBuilder::default().build_and_execute(|| {
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

        // Advance to the period at which rewards expire.
        advance_to_period(
            ActiveProtocolState::<Test>::get().period_number() + reward_retention_in_periods + 1,
        );

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
    ExtBuilder::default().build_and_execute(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        // Advance into Build&Earn period, and allow one era to pass.
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);

        // Disable successful reward payout
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        // Advance to next period so we can claim bonus reward
        advance_to_next_period();

        // Disable successful reward payout
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_noop!(
            DappStaking::claim_dapp_reward(RuntimeOrigin::signed(1), smart_contract, 1),
            Error::<Test>::ContractNotFound,
        );
    })
}

#[test]
fn claim_dapp_reward_from_invalid_era_fails() {
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
            Error::<Test>::NoClaimableRewards,
        );

        // We can still claim for another valid era
        let claim_era_2 = claim_era_1 + 1;
        assert_claim_dapp_reward(account, &smart_contract, claim_era_2);
    })
}

#[test]
fn claim_dapp_reward_for_expired_era_fails() {
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let account = 2;
        let amount = 300;
        assert_lock(account, amount);
        assert_stake(account, &smart_contract, amount);

        // Advance 2 eras so we have an entry for reward claiming
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);

        // Disable successful reward payout
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
        assert_noop!(
            DappStaking::force(RuntimeOrigin::signed(1), ForcingType::Era),
            BadOrigin
        );
    })
}

#[test]
fn force_with_safeguard_on_fails() {
    ExtBuilder::default().build_and_execute(|| {
        Safeguard::<Test>::put(true);
        assert_noop!(
            DappStaking::force(RuntimeOrigin::root(), ForcingType::Era),
            Error::<Test>::ForceNotAllowed
        );
    })
}

#[test]
fn tier_config_recalculation_works() {
    ExtBuilder::default().build_and_execute(|| {
        let init_price = NATIVE_PRICE.with(|v| v.borrow().clone());
        let init_tier_config = TierConfig::<Test>::get();

        // 1. Advance to a new era, while keeping native price the same. Expect no change in the tier config
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_for_blocks(1);

        assert_eq!(
            init_tier_config,
            TierConfig::<Test>::get(),
            "Native price didn't change so tier config should remain the same."
        );

        // 2. Increase the native price, and expect number of tiers to be increased.
        NATIVE_PRICE.with(|v| *v.borrow_mut() = init_price * FixedU128::from(3));

        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_for_blocks(1);

        let new_tier_config = TierConfig::<Test>::get();
        assert!(
            new_tier_config.total_number_of_slots() > init_tier_config.total_number_of_slots(),
            "Price has increased, therefore number of slots must increase."
        );
        assert_eq!(
            init_tier_config.slots_per_tier.len(),
            new_tier_config.slots_per_tier.len(),
            "Sanity check."
        );
        assert!(
            new_tier_config
                .slots_per_tier
                .iter()
                .zip(init_tier_config.slots_per_tier.iter())
                .all(|(new, init)| new > init),
            "Number of slots per tier should increase with higher price"
        );
        assert!(
            new_tier_config
                .tier_thresholds
                .iter()
                .zip(init_tier_config.tier_thresholds.iter())
                .all(|(new, init)| new <= init),
            "Tier threshold values should decrease with higher price"
        );

        // 3. Decrease the native price, and expect slots in tiers to be decreased.
        NATIVE_PRICE.with(|v| *v.borrow_mut() = init_price * FixedU128::from_rational(1, 2));

        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_for_blocks(1);

        let new_tier_config = TierConfig::<Test>::get();
        assert!(
            new_tier_config.total_number_of_slots() < init_tier_config.total_number_of_slots(),
            "Price has decreased, therefore number of slots must decrease."
        );
        assert_eq!(
            init_tier_config.slots_per_tier.len(),
            new_tier_config.slots_per_tier.len(),
            "Sanity check."
        );
        assert!(
            new_tier_config
                .slots_per_tier
                .iter()
                .zip(init_tier_config.slots_per_tier.iter())
                .all(|(new, init)| new < init),
            "Number of slots per tier should decrease with lower price"
        );
        assert!(
            new_tier_config
                .tier_thresholds
                .iter()
                .zip(init_tier_config.tier_thresholds.iter())
                .all(|(new, init)| new >= init),
            "Tier threshold values should increase with lower price"
        );
    })
}

#[test]
fn get_dapp_tier_assignment_and_rewards_basic_example_works() {
    ExtBuilder::default().build_and_execute(|| {
        // Tier config is specially adapted for this test.
        TierConfig::<Test>::mutate(|config| {
            config.slots_per_tier = BoundedVec::try_from(vec![2, 5, 13, 20]).unwrap();
        });

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
                tier_config.tier_thresholds[0] + x + 1,
            );
            dapp_index += 1;
        }
        // One that won't make it into the 1st tier.
        lock_and_stake(
            dapp_index,
            &smart_contracts[dapp_index],
            tier_config.tier_thresholds[0],
        );
        dapp_index += 1;

        // 2nd tier - 1 dedicated dApp
        lock_and_stake(
            dapp_index,
            &smart_contracts[dapp_index],
            tier_config.tier_thresholds[0] - 1,
        );
        dapp_index += 1;

        // 3rd tier is empty
        // 4th tier has 2 dApps
        for x in 0..2 {
            lock_and_stake(
                dapp_index,
                &smart_contracts[dapp_index],
                tier_config.tier_thresholds[3] + x,
            );
            dapp_index += 1;
        }

        // One dApp doesn't make it into any tier
        lock_and_stake(
            dapp_index,
            &smart_contracts[dapp_index],
            tier_config.tier_thresholds[3] - 1,
        );

        // Finally, the actual test
        let protocol_state = ActiveProtocolState::<Test>::get();
        let dapp_reward_pool = 1000000;
        let (tier_assignment, counter) = DappStaking::get_dapp_tier_assignment_and_rewards(
            protocol_state.era + 1,
            protocol_state.period_number(),
            dapp_reward_pool,
        );

        // There's enough reward to satisfy 100% reward per rank.
        // Slot reward is 60_000 therefore expected rank reward is 6_000
        assert_eq!(
            tier_assignment.rank_rewards,
            BoundedVec::<Balance, ConstU32<4>>::try_from(vec![0, 6_000, 0, 0]).unwrap()
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
        let (dapp_1_tier, dapp_2_tier) = (tier_assignment.dapps[&0], tier_assignment.dapps[&1]);
        assert_eq!(dapp_1_tier, RankedTier::new_saturated(0, 0));
        assert_eq!(dapp_2_tier, RankedTier::new_saturated(0, 0));

        // 2nd tier checks
        let (dapp_3_tier, dapp_4_tier) = (tier_assignment.dapps[&2], tier_assignment.dapps[&3]);
        assert_eq!(dapp_3_tier, RankedTier::new_saturated(1, 10));
        assert_eq!(dapp_4_tier, RankedTier::new_saturated(1, 9));

        // 4th tier checks
        let (dapp_5_tier, dapp_6_tier) = (tier_assignment.dapps[&4], tier_assignment.dapps[&5]);
        assert_eq!(dapp_5_tier, RankedTier::new_saturated(3, 0));
        assert_eq!(dapp_6_tier, RankedTier::new_saturated(3, 0));

        // Sanity check - last dapp should not exists in the tier assignment
        assert!(tier_assignment
            .dapps
            .get(&dapp_index.try_into().unwrap())
            .is_none());

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
fn get_dapp_tier_assignment_and_rewards_zero_slots_per_tier_works() {
    ExtBuilder::default().build_and_execute(|| {
        // This test will rely on the configuration inside the mock file.
        // If that changes, this test might have to be updated as well.

        // Ensure that first tier has 0 slots.
        TierConfig::<Test>::mutate(|config| {
            config.slots_per_tier[0] = 0;
        });

        // Calculate tier assignment (we don't need dApps for this test)
        let protocol_state = ActiveProtocolState::<Test>::get();
        let dapp_reward_pool = 1000000;
        let (tier_assignment, counter) = DappStaking::get_dapp_tier_assignment_and_rewards(
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
    ExtBuilder::default().build_and_execute(|| {
        advance_to_period(10);
    })
}

#[test]
fn unlock_after_staked_period_ends_is_ok() {
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
    ExtBuilder::default().build_and_execute(|| {
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
fn stake_and_unstake_correctly_updates_staked_amounts() {
    ExtBuilder::default().build_and_execute(|| {
        // Register smart contract
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);
        let smart_contract_id = IntegratedDApps::<Test>::get(&smart_contract).unwrap().id;

        // Lock & stake some amount by the first staker, and lock some amount by the second staker
        let account_1 = 2;
        let amount_1 = 50;
        assert_lock(account_1, amount_1);
        assert_stake(account_1, &smart_contract, amount_1);

        let account_2 = 3;
        let amount_2 = 10;
        assert_lock(account_2, amount_2);

        // 1st scenario: repeated stake & unstake in the `Voting` subperiod
        let contract_stake_snapshot = ContractStake::<Test>::get(&smart_contract_id);

        for _ in 0..20 {
            assert_stake(account_2, &smart_contract, amount_2);
            assert_unstake(account_2, &smart_contract, amount_2);
        }

        // Check that the staked amount for the upcoming era is same as before
        let current_era = ActiveProtocolState::<Test>::get().era;
        let period_number = ActiveProtocolState::<Test>::get().period_number();
        assert_eq!(
            contract_stake_snapshot
                .get(current_era + 1, period_number)
                .expect("Entry must exist."),
            ContractStake::<Test>::get(&smart_contract_id)
                .get(current_era + 1, period_number)
                .expect("Entry must exist."),
            "Ongoing era staked amount must not change."
        );

        // 2nd scenario: repeated stake & unstake in the first era of the `Build&Earn` subperiod
        advance_to_next_era();
        let contract_stake_snapshot = ContractStake::<Test>::get(&smart_contract_id);

        for _ in 0..20 {
            assert_stake(account_2, &smart_contract, amount_2);
            assert_unstake(account_2, &smart_contract, amount_2);
        }

        // Check that the contract stake snapshot staked amount is the same as before
        let current_era = ActiveProtocolState::<Test>::get().era;
        assert_eq!(
            contract_stake_snapshot
                .get(current_era, period_number)
                .expect("Entry must exist."),
            ContractStake::<Test>::get(&smart_contract_id)
                .get(current_era, period_number)
                .expect("Entry must exist."),
            "Ongoing era staked amount must not change."
        );

        assert_eq!(
            contract_stake_snapshot
                .get(current_era, period_number)
                .expect("Entry must exist.")
                .total(),
            ContractStake::<Test>::get(&smart_contract_id)
                .get(current_era + 1, period_number)
                .expect("Entry must exist.")
                .total(),
            "Ongoing era staked amount must be equal to the upcoming era stake."
        );

        // 3rd scenario: repeated stake & unstake in the second era of the `Build&Earn` subperiod
        assert_stake(account_2, &smart_contract, amount_2);
        assert_lock(account_2, amount_2);
        advance_to_next_era();

        let contract_stake_snapshot = ContractStake::<Test>::get(&smart_contract_id);

        for _ in 0..20 {
            assert_stake(account_2, &smart_contract, amount_2);
            assert_unstake(account_2, &smart_contract, amount_2);
        }

        // Check that the contract stake snapshot staked amount is the same as before
        let current_era = ActiveProtocolState::<Test>::get().era;
        assert_eq!(
            contract_stake_snapshot
                .get(current_era, period_number)
                .expect("Entry must exist."),
            ContractStake::<Test>::get(&smart_contract_id)
                .get(current_era, period_number)
                .expect("Entry must exist."),
            "Ongoing era staked amount must not change."
        );

        // 4th scenario: Unstake with more than was staked for the next era
        let delta = 5;
        let amount_3 = amount_2 + delta;
        assert_stake(account_2, &smart_contract, amount_2);

        let contract_stake_snapshot = ContractStake::<Test>::get(&smart_contract_id);
        for _ in 0..20 {
            assert_unstake(account_2, &smart_contract, amount_3);
            assert_stake(account_2, &smart_contract, amount_3);
        }

        // Check that the contract stake snapshot staked amount is the same as before
        let current_era = ActiveProtocolState::<Test>::get().era;
        assert_eq!(
            contract_stake_snapshot
                .get(current_era, period_number)
                .expect("Entry must exist.")
                .total(),
            ContractStake::<Test>::get(&smart_contract_id)
                .get(current_era, period_number)
                .expect("Entry must exist.")
                .total()
                + delta,
            "Ongoing era stake must be reduced by the `delta` amount."
        );
    })
}

#[test]
fn stake_after_period_ends_with_max_staked_contracts() {
    ExtBuilder::default().build_and_execute(|| {
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
fn post_unlock_balance_cannot_be_transferred() {
    ExtBuilder::default().build_and_execute(|| {
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

#[test]
fn observer_pre_new_era_block_works() {
    ExtBuilder::default().build_and_execute(|| {
        fn assert_observer_value(expected: EraNumber) {
            BLOCK_BEFORE_NEW_ERA.with(|v| assert_eq!(expected, *v.borrow()));
        }

        // 1. Sanity check
        assert_observer_value(0);

        // 2. Advance to the block right before the observer value should be set.
        //    No modifications should happen.
        BLOCK_BEFORE_NEW_ERA.with(|v| {
            let _lock = v.borrow();
            run_to_block(ActiveProtocolState::<Test>::get().next_era_start - 2);
        });

        // 3. Advance to the next block, when observer value is expected to be set to the next era.
        run_for_blocks(1);
        assert_observer_value(2);

        // 4. Advance again, until the same similar scenario
        BLOCK_BEFORE_NEW_ERA.with(|v| {
            let _lock = v.borrow();
            run_for_blocks(1);
            assert_eq!(
                ActiveProtocolState::<Test>::get().subperiod(),
                Subperiod::BuildAndEarn,
                "Sanity check."
            );

            run_to_block(ActiveProtocolState::<Test>::get().next_era_start - 2);
            assert_eq!(ActiveProtocolState::<Test>::get().era, 2, "Sanity check.");
            assert_observer_value(2);
        });

        // 5. Again, check that value is set to the expected one.
        run_for_blocks(1);
        assert_observer_value(3);

        // 6. Force new era, and ensure observer value is set to the next one.
        run_for_blocks(1);
        assert_eq!(ActiveProtocolState::<Test>::get().era, 3, "Sanity check.");
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        assert_observer_value(4);
    })
}

#[test]
fn unregister_after_max_number_of_contracts_allows_register_again() {
    ExtBuilder::default().build_and_execute(|| {
        let max_number_of_contracts = <Test as Config>::MaxNumberOfContracts::get();
        let developer = 2;

        // Reach max number of contracts
        for id in 0..max_number_of_contracts {
            assert_register(developer, &MockSmartContract::Wasm(id.into()));
        }

        // Ensure we cannot register more contracts
        assert_noop!(
            DappStaking::register(
                RuntimeOrigin::root(),
                developer,
                MockSmartContract::Wasm((max_number_of_contracts).into())
            ),
            Error::<Test>::ExceededMaxNumberOfContracts
        );

        // Unregister one contract, and ensure register works again
        let smart_contract = MockSmartContract::Wasm(0);
        assert_unregister(&smart_contract);
        assert_register(developer, &smart_contract);
    })
}

#[test]
fn safeguard_on_by_default() {
    use sp_runtime::BuildStorage;
    let storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    let mut ext = sp_io::TestExternalities::from(storage);
    ext.execute_with(|| {
        assert!(Safeguard::<Test>::get());
    });
}

#[test]
fn safeguard_configurable_by_genesis_config() {
    use sp_runtime::BuildStorage;
    let mut genesis_config = GenesisConfig::<Test> {
        reward_portion: vec![
            Permill::from_percent(40),
            Permill::from_percent(30),
            Permill::from_percent(20),
            Permill::from_percent(10),
        ],
        slot_distribution: vec![
            Permill::from_percent(10),
            Permill::from_percent(20),
            Permill::from_percent(30),
            Permill::from_percent(40),
        ],
        slots_per_tier: vec![10, 20, 30, 40],
        ..Default::default()
    };

    // Test case 1: Safeguard enabled via Genesis Config
    genesis_config.safeguard = Some(true);
    let storage = genesis_config.build_storage().unwrap();
    let mut ext = sp_io::TestExternalities::from(storage);
    ext.execute_with(|| {
        assert!(Safeguard::<Test>::get());
    });

    // Test case 2: Safeguard disabled via Genesis Config
    genesis_config.safeguard = Some(false);
    let storage = genesis_config.build_storage().unwrap();
    let mut ext = sp_io::TestExternalities::from(storage);
    ext.execute_with(|| {
        assert!(!Safeguard::<Test>::get());
    });

    // Test case 3: Safeguard not set via Genesis Config
    genesis_config.safeguard = None;
    let storage = genesis_config.build_storage().unwrap();
    let mut ext = sp_io::TestExternalities::from(storage);
    ext.execute_with(|| {
        assert!(Safeguard::<Test>::get());
    });
}

#[test]
fn base_number_of_slots_is_respected() {
    ExtBuilder::default().build_and_execute(|| {
        // 0. Get expected number of slots for the base price
        let total_issuance = <Test as Config>::Currency::total_issuance();
        let base_native_price = <Test as Config>::BaseNativeCurrencyPrice::get();
        let base_number_of_slots = <Test as Config>::TierSlots::number_of_slots(base_native_price);

        // 1. Make sure base native price is set initially and calculate the new config. Store the thresholds for later comparison.
        NATIVE_PRICE.with(|v| *v.borrow_mut() = base_native_price);
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_for_blocks(1);

        assert_eq!(
            TierConfig::<Test>::get().total_number_of_slots(),
            base_number_of_slots,
            "Base number of slots is expected for base native currency price."
        );

        let base_thresholds = TierConfig::<Test>::get().tier_thresholds;

        // 2. Increase the price significantly, and ensure number of slots has increased, and thresholds have been saturated.
        let higher_price = base_native_price * FixedU128::from(1000);
        NATIVE_PRICE.with(|v| *v.borrow_mut() = higher_price);
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_for_blocks(1);

        assert!(
            TierConfig::<Test>::get().total_number_of_slots() > base_number_of_slots,
            "Price has increased, therefore number of slots must increase."
        );
        assert_eq!(
            TierConfig::<Test>::get().total_number_of_slots(),
            <Test as Config>::TierSlots::number_of_slots(higher_price),
        );

        for (amount, static_tier_threshold) in TierConfig::<Test>::get()
            .tier_thresholds
            .iter()
            .zip(StaticTierParams::<Test>::get().tier_thresholds.iter())
        {
            if let TierThreshold::DynamicPercentage {
                minimum_required_percentage,
                ..
            } = static_tier_threshold
            {
                let minimum_amount = *minimum_required_percentage * total_issuance;
                assert_eq!(*amount, minimum_amount, "Thresholds must be saturated.");
            }
        }

        // 3. Bring it back down to the base price, and expect number of slots to be the same as the base number of slots,
        // and thresholds to be the same as the base thresholds.
        NATIVE_PRICE.with(|v| *v.borrow_mut() = base_native_price);
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_for_blocks(1);

        assert_eq!(
            TierConfig::<Test>::get().total_number_of_slots(),
            base_number_of_slots,
            "Base number of slots is expected for base native currency price."
        );

        assert_eq!(
            TierConfig::<Test>::get().tier_thresholds,
            base_thresholds,
            "Thresholds must be the same as the base thresholds."
        );

        // 4. Bring it below the base price, and expect number of slots to decrease.
        let lower_price = base_native_price * FixedU128::from_rational(1, 1000);
        NATIVE_PRICE.with(|v| *v.borrow_mut() = lower_price);
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_for_blocks(1);

        assert!(
            TierConfig::<Test>::get().total_number_of_slots() < base_number_of_slots,
            "Price has decreased, therefore number of slots must decrease."
        );
        assert_eq!(
            TierConfig::<Test>::get().total_number_of_slots(),
            <Test as Config>::TierSlots::number_of_slots(lower_price),
        );

        // 5. Bring it back to the base price, and expect number of slots to be the same as the base number of slots,
        // and thresholds to be the same as the base thresholds.
        NATIVE_PRICE.with(|v| *v.borrow_mut() = base_native_price);
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_for_blocks(1);

        assert_eq!(
            TierConfig::<Test>::get().total_number_of_slots(),
            base_number_of_slots,
            "Base number of slots is expected for base native currency price."
        );

        assert_eq!(
            TierConfig::<Test>::get().tier_thresholds,
            base_thresholds,
            "Thresholds must be the same as the base thresholds."
        );
    })
}

#[test]
fn ranking_will_calc_reward_correctly() {
    ExtBuilder::default().build_and_execute(|| {
        // Tier config is specially adapted for this test.
        TierConfig::<Test>::mutate(|config| {
            config.slots_per_tier = BoundedVec::try_from(vec![2, 3, 2, 20]).unwrap();
        });

        // Register smart contracts
        let smart_contracts: Vec<_> = (1..=8u32)
            .map(|x| {
                let smart_contract = MockSmartContract::Wasm(x.into());
                assert_register(x.into(), &smart_contract);
                smart_contract
            })
            .collect();

        fn lock_and_stake(account: usize, smart_contract: &MockSmartContract, amount: Balance) {
            let account = account.try_into().unwrap();
            Balances::make_free_balance_be(&account, amount);
            assert_lock(account, amount);
            assert_stake(account, smart_contract, amount);
        }

        for (idx, amount) in [101, 102, 100, 99, 15, 49, 35, 14].into_iter().enumerate() {
            lock_and_stake(idx, &smart_contracts[idx], amount)
        }

        // Finally, the actual test
        let protocol_state = ActiveProtocolState::<Test>::get();
        let (tier_assignment, counter) = DappStaking::get_dapp_tier_assignment_and_rewards(
            protocol_state.era + 1,
            protocol_state.period_number(),
            1_000_000,
        );

        assert_eq!(
            tier_assignment,
            DAppTierRewardsFor::<Test> {
                dapps: BoundedBTreeMap::try_from(BTreeMap::from([
                    (0, RankedTier::new_saturated(0, 0)),
                    (1, RankedTier::new_saturated(0, 0)),
                    (2, RankedTier::new_saturated(1, 10)),
                    (3, RankedTier::new_saturated(1, 9)),
                    (5, RankedTier::new_saturated(2, 9)),
                    (6, RankedTier::new_saturated(2, 5)),
                    (4, RankedTier::new_saturated(3, 0)),
                ]))
                .unwrap(),
                rewards: BoundedVec::try_from(vec![200_000, 100_000, 100_000, 5_000]).unwrap(),
                period: 1,
                // Tier 0 has no ranking therefore no rank reward.
                // For tier 1 there's not enough reward to satisfy 100% reward per rank.
                // Only one slot is empty. Slot reward is 100_000 therefore expected rank reward is 100_000 / 19 (ranks_sum).
                // Tier 2 has ranking but there's no empty slot therefore no rank reward.
                // Tier 3 has no ranking therefore no rank reward.
                rank_rewards: BoundedVec::try_from(vec![0, 5_263, 0, 0]).unwrap()
            }
        );

        // one didn't make it
        assert_eq!(counter, 8);
    })
}

#[test]
fn claim_dapp_reward_with_rank() {
    ExtBuilder::default().build_and_execute(|| {
        let total_issuance = <Test as Config>::Currency::total_issuance();

        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let alice = 2;
        let amount = Perbill::from_parts(11_000_000) * total_issuance; // very close to tier 0 so will enter tier 1 with rank 9
        assert_lock(alice, amount);
        assert_stake(alice, &smart_contract, amount);

        // Advance 2 eras so we have an entry for reward claiming
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);

        let era = ActiveProtocolState::<Test>::get().era - 1;
        let tiers = DAppTiers::<Test>::get(era).unwrap();

        let slot_reward = tiers.rewards[1];
        let rank_reward = tiers.rank_rewards[1];

        // Claim dApp reward & verify event
        assert_ok!(DappStaking::claim_dapp_reward(
            RuntimeOrigin::signed(alice),
            smart_contract.clone(),
            era,
        ));

        let expected_rank = 9;
        let expected_total_reward = slot_reward + expected_rank * rank_reward;
        assert_eq!(slot_reward, 15_000_000);
        assert_eq!(rank_reward, 1_500_000); // slot_reward / 10
        assert_eq!(expected_total_reward, 28_500_000);

        System::assert_last_event(RuntimeEvent::DappStaking(Event::DAppReward {
            beneficiary: 1,
            smart_contract: smart_contract.clone(),
            tier_id: 1,
            rank: 9,
            era,
            amount: expected_total_reward,
        }));
    })
}

#[test]
fn unstake_correctly_reduces_future_contract_stake() {
    ExtBuilder::default().build_and_execute(|| {
        // 0. Register smart contract, lock&stake some amount with staker 1 during the voting subperiod
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(1, &smart_contract);

        let (staker_1, amount_1) = (1, 29);
        assert_lock(staker_1, amount_1);
        assert_stake(staker_1, &smart_contract, amount_1);

        // 1. Advance to the build&earn subperiod, stake some amount with staker 2
        advance_to_next_era();
        let (staker_2, amount_2) = (2, 11);
        assert_lock(staker_2, amount_2);
        assert_stake(staker_2, &smart_contract, amount_2);

        // 2. Advance a few eras, creating a gap but remaining within the same period.
        //    Claim all rewards for staker 1.
        //    Lock & stake some amount with staker 3.
        advance_to_era(ActiveProtocolState::<Test>::get().era + 3);
        assert_eq!(
            ActiveProtocolState::<Test>::get().period_number(),
            1,
            "Sanity check."
        );
        for _ in 0..required_number_of_reward_claims(staker_1) {
            assert_claim_staker_rewards(staker_1);
        }

        // This ensures contract stake entry is aligned to the current era, and future entry refers to the era after this one.
        //
        // This is important to reproduce an issue where the (era, amount) pairs returned by the `unstake` function don't correctly
        // cover the next era.
        let (staker_3, amount_3) = (3, 13);
        assert_lock(staker_3, amount_3);
        assert_stake(staker_3, &smart_contract, amount_3);

        // 3. Unstake from staker 1, and ensure the future stake is reduced.
        //    Unstake amount should be slightly higher than the 2nd stake amount to ensure whole b&e stake amount is removed.
        assert_unstake(staker_1, &smart_contract, amount_2 + 3);
    })
}

#[test]
fn lock_correctly_considers_unlocking_amount() {
    ExtBuilder::default().build_and_execute(|| {
        // Lock the entire amount & immediately start the unlocking process
        let (staker, unlock_amount) = (1, 13);
        let total_balance = Balances::total_balance(&staker);
        assert_lock(staker, total_balance);
        assert_unlock(staker, unlock_amount);

        assert_noop!(
            DappStaking::lock(RuntimeOrigin::signed(staker), 1),
            Error::<Test>::ZeroAmount
        );
    })
}

#[test]
fn fix_account_scenarios_work() {
    ExtBuilder::default().build_and_execute(|| {
        // 1. Lock some amount correctly, unstake it, try to fix it, and ensure the call fails
        let (account_1, lock_1) = (1, 100);
        assert_lock(account_1, lock_1);
        assert_noop!(
            DappStaking::fix_account(RuntimeOrigin::signed(11), account_1),
            Error::<Test>::AccountNotInconsistent
        );

        assert_unlock(account_1, lock_1);
        assert_noop!(
            DappStaking::fix_account(RuntimeOrigin::signed(11), account_1),
            Error::<Test>::AccountNotInconsistent
        );

        // 2. Reproduce the issue where the account has more frozen than balance
        let (account_2, unlock_2) = (2, 13);
        let lock_2 = Balances::total_balance(&account_2);
        assert_lock(account_2, lock_2);
        assert_unlock(account_2, unlock_2);

        // With the fix implemented, the scenario needs to be reproduced by hand.
        // Account calls `lock`, specifying the amount that is undergoing the unlocking process.
        // It can be either more or less, it doesn't matter for the test or the issue.

        // But first, a sanity check.
        assert_noop!(
            DappStaking::lock(RuntimeOrigin::signed(account_2), unlock_2),
            Error::<Test>::ZeroAmount,
        );

        // Now reproduce the incorrect lock/freeze operation.
        let mut ledger = Ledger::<Test>::get(&account_2);
        ledger.add_lock_amount(unlock_2);
        assert_ok!(DappStaking::update_ledger(&account_2, ledger));
        use crate::CurrentEraInfo;
        CurrentEraInfo::<Test>::mutate(|era_info| {
            era_info.add_locked(unlock_2);
        });
        assert!(
            Balances::free_balance(&account_2)
                < Ledger::<Test>::get(&account_2).total_locked_amount(),
            "Sanity check."
        );

        // Now fix the account
        assert_ok!(DappStaking::fix_account(
            RuntimeOrigin::signed(11),
            account_2
        ));
        System::assert_last_event(RuntimeEvent::DappStaking(Event::ClaimedUnlocked {
            account: account_2,
            amount: unlock_2,
        }));

        // Post-fix checks
        assert_eq!(
            Balances::free_balance(&account_2),
            Ledger::<Test>::get(&account_2).total_locked_amount(),
            "After the fix, balances should be equal."
        );

        // Cannot fix the same account again.
        assert_noop!(
            DappStaking::fix_account(RuntimeOrigin::signed(11), account_2),
            Error::<Test>::AccountNotInconsistent
        );
    })
}

#[test]
fn claim_staker_rewards_for_basic_example_is_ok() {
    ExtBuilder::default().build_and_execute(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let staker_account = 2;
        let lock_amount = 300;
        assert_lock(staker_account, lock_amount);
        let stake_amount = 93;
        assert_stake(staker_account, &smart_contract, stake_amount);

        // Advance into Build&Earn period, and allow one era to pass. Claim reward for 1 era.
        advance_to_era(ActiveProtocolState::<Test>::get().era + 2);

        // Basic checks, since the entire claim logic is already covered by other tests
        let claimer_account = 3;
        let (init_staker_balance, init_claimer_balance) = (
            Balances::free_balance(&staker_account),
            Balances::free_balance(&claimer_account),
        );
        assert_ok!(DappStaking::claim_staker_rewards_for(
            RuntimeOrigin::signed(claimer_account),
            staker_account
        ));
        System::assert_last_event(RuntimeEvent::DappStaking(Event::Reward {
            account: staker_account,
            era: ActiveProtocolState::<Test>::get().era - 1,
            // for this simple test, entire staker reward pool goes to the staker
            amount: <Test as Config>::StakingRewardHandler::staker_and_dapp_reward_pools(0).0,
        }));

        assert!(
            Balances::free_balance(&staker_account) > init_staker_balance,
            "Balance must have increased due to the reward payout."
        );
        assert_eq!(
            init_claimer_balance,
            Balances::free_balance(&claimer_account),
            "Claimer balance must not change since reward is deposited to the staker."
        );
    })
}

#[test]
fn claim_bonus_reward_for_works() {
    ExtBuilder::default().build_and_execute(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::wasm(1 as AccountId);
        assert_register(dev_account, &smart_contract);

        let staker_account = 2;
        let lock_amount = 300;
        assert_lock(staker_account, lock_amount);
        let stake_amount = 93;
        assert_stake(staker_account, &smart_contract, stake_amount);

        // Advance to the next period, and claim the bonus
        advance_to_next_period();
        let claimer_account = 3;
        let (init_staker_balance, init_claimer_balance) = (
            Balances::free_balance(&staker_account),
            Balances::free_balance(&claimer_account),
        );

        assert_ok!(DappStaking::claim_bonus_reward_for(
            RuntimeOrigin::signed(claimer_account),
            staker_account,
            smart_contract.clone()
        ));
        System::assert_last_event(RuntimeEvent::DappStaking(Event::BonusReward {
            account: staker_account,
            period: ActiveProtocolState::<Test>::get().period_number() - 1,
            smart_contract,
            // for this simple test, entire bonus reward pool goes to the staker
            amount: <Test as Config>::StakingRewardHandler::bonus_reward_pool(),
        }));

        assert!(
            Balances::free_balance(&staker_account) > init_staker_balance,
            "Balance must have increased due to the reward payout."
        );
        assert_eq!(
            init_claimer_balance,
            Balances::free_balance(&claimer_account),
            "Claimer balance must not change since reward is deposited to the staker."
        );
    })
}
