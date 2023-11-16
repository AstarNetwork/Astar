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
    pallet::Config, ActiveProtocolState, DAppId, EraNumber, EraRewards, Error, ForcingType,
    IntegratedDApps, Ledger, NextDAppId, PeriodNumber, Subperiod,
};

use frame_support::{assert_noop, assert_ok, error::BadOrigin, traits::Get};
use sp_runtime::traits::Zero;

#[test]
fn print_test() {
    ExtBuilder::build().execute_with(|| {
        use crate::dsv3_weight::WeightInfo;
        println!(
            ">>> dApp tier assignment reading & calculation {:?}",
            crate::dsv3_weight::SubstrateWeight::<Test>::dapp_tier_assignment(100)
        );

        use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
        use scale_info::TypeInfo;

        #[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
        struct RewardSize;
        impl Get<u32> for RewardSize {
            fn get() -> u32 {
                1_00_u32
            }
        }
        #[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
        struct TierSize;
        impl Get<u32> for TierSize {
            fn get() -> u32 {
                4_u32
            }
        }
        println!(
            ">>> Max encoded size for dapp tier rewards: {:?}",
            crate::DAppTierRewards::<RewardSize, TierSize>::max_encoded_len()
        );

        println!(
            ">>> Max encoded size of ContractStake: {:?}",
            crate::ContractStakeAmount::max_encoded_len()
        );
    })
}

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
            DappStaking::stake(RuntimeOrigin::signed(1), MockSmartContract::default(), 100),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::unstake(RuntimeOrigin::signed(1), MockSmartContract::default(), 100),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::claim_staker_rewards(RuntimeOrigin::signed(1)),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::claim_bonus_reward(RuntimeOrigin::signed(1), MockSmartContract::default()),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::claim_dapp_reward(
                RuntimeOrigin::signed(1),
                MockSmartContract::default(),
                1
            ),
            Error::<Test>::Disabled
        );
        assert_noop!(
            DappStaking::unstake_from_unregistered(
                RuntimeOrigin::signed(1),
                MockSmartContract::default()
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
fn on_initialize_state_change_works() {
    ExtBuilder::build().execute_with(|| {
        // TODO: test `EraInfo` change and verify events. This would be good to do each time we call the helper functions to go to next era or period.

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
        let eras_per_bep_period: EraNumber =
            <Test as Config>::StandardErasPerBuildAndEarnPeriod::get();
        let blocks_per_era: BlockNumber = <Test as Config>::StandardEraLength::get();
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
        let unlocking_blocks = DappStaking::unlock_period();

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
        let unlocking_blocks = DappStaking::unlock_period();
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
        let unlocking_blocks = DappStaking::unlock_period();
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
        let smart_contract = MockSmartContract::default();
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
            state.period_info.subperiod = Subperiod::BuildAndEarn;
            state.period_info.subperiod_end_era = state.era + 1;
        });

        // Try to stake in the final era of the period, which should fail.
        assert_noop!(
            DappStaking::stake(RuntimeOrigin::signed(account), smart_contract, 100),
            Error::<Test>::PeriodEndsInNextEra
        );
    })
}

#[test]
fn stake_fails_if_unclaimed_rewards_from_past_eras_remain() {
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
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
                .subperiod_end_era
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
fn claim_bonus_reward_works() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
fn claim_dapp_reward_works() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let dev_account = 1;
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
fn unstake_from_unregistered_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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

////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////
/////// More complex & composite scenarios, maybe move them into a separate file

#[test]
fn unlock_after_staked_period_ends_is_ok() {
    ExtBuilder::build().execute_with(|| {
        // Register smart contract, lock&stake some amount
        let smart_contract = MockSmartContract::default();
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
        let smart_contract = MockSmartContract::default();
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
