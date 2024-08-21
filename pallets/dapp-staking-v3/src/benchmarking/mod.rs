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

use super::{Pallet as DappStaking, *};

use astar_primitives::Balance;
use frame_benchmarking::v2::*;

use frame_support::assert_ok;
use frame_system::{Pallet as System, RawOrigin};
use sp_std::prelude::*;

use ::assert_matches::assert_matches;

mod utils;
use utils::*;

// A lot of benchmarks which require many blocks, eras or periods to pass have been optimized to utilize
// `force` approach, which skips the required amount of blocks that need to be produced in order to advance.
//
// Without this optimization, benchmarks can take hours to execute for production runtimes.

#[benchmarks()]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn maintenance_mode() {
        initial_config::<T>();

        #[extrinsic_call]
        _(RawOrigin::Root, true);

        assert_last_event::<T>(Event::<T>::MaintenanceMode { enabled: true }.into());
    }

    #[benchmark]
    fn register() {
        initial_config::<T>();

        let account: T::AccountId = account("dapp_owner", 0, SEED);
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);

        #[extrinsic_call]
        _(RawOrigin::Root, account.clone(), smart_contract.clone());

        assert_last_event::<T>(
            Event::<T>::DAppRegistered {
                owner: account,
                smart_contract,
                dapp_id: 0,
            }
            .into(),
        );
    }

    #[benchmark]
    fn set_dapp_reward_beneficiary() {
        initial_config::<T>();

        let owner: T::AccountId = whitelisted_caller();
        let beneficiary: Option<T::AccountId> = Some(account("beneficiary", 0, SEED));
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            owner.clone().into(),
            smart_contract.clone(),
        ));

        #[extrinsic_call]
        _(
            RawOrigin::Signed(owner),
            smart_contract.clone(),
            beneficiary.clone(),
        );

        assert_last_event::<T>(
            Event::<T>::DAppRewardDestinationUpdated {
                smart_contract,
                beneficiary,
            }
            .into(),
        );
    }

    #[benchmark]
    fn set_dapp_owner() {
        initial_config::<T>();

        let init_owner: T::AccountId = whitelisted_caller();
        let new_owner: T::AccountId = account("dapp_owner", 0, SEED);
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            init_owner.clone().into(),
            smart_contract.clone(),
        ));

        #[extrinsic_call]
        _(
            RawOrigin::Signed(init_owner),
            smart_contract.clone(),
            new_owner.clone(),
        );

        assert_last_event::<T>(
            Event::<T>::DAppOwnerChanged {
                smart_contract,
                new_owner,
            }
            .into(),
        );
    }

    #[benchmark]
    fn unregister() {
        initial_config::<T>();

        let owner: T::AccountId = whitelisted_caller();
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            owner.clone().into(),
            smart_contract.clone(),
        ));

        #[extrinsic_call]
        _(RawOrigin::Root, smart_contract.clone());

        assert_last_event::<T>(
            Event::<T>::DAppUnregistered {
                smart_contract,
                era: ActiveProtocolState::<T>::get().era,
            }
            .into(),
        );
    }

    #[benchmark]
    fn lock_new_account() {
        initial_config::<T>();

        let staker: T::AccountId = whitelisted_caller();
        let owner: T::AccountId = account("dapp_owner", 0, SEED);
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            owner.clone().into(),
            smart_contract.clone(),
        ));

        let amount = T::MinimumLockedAmount::get();
        T::BenchmarkHelper::set_balance(&staker, amount);

        #[extrinsic_call]
        lock(RawOrigin::Signed(staker.clone()), amount);

        assert_last_event::<T>(
            Event::<T>::Locked {
                account: staker,
                amount,
            }
            .into(),
        );
    }

    #[benchmark]
    fn lock_existing_account() {
        initial_config::<T>();

        let staker: T::AccountId = whitelisted_caller();
        let owner: T::AccountId = account("dapp_owner", 0, SEED);
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            owner.clone().into(),
            smart_contract.clone(),
        ));

        let amount_1 = T::MinimumLockedAmount::get();
        let amount_2 = 19;
        T::BenchmarkHelper::set_balance(&staker, amount_1 + amount_2);
        assert_ok!(DappStaking::<T>::lock(
            RawOrigin::Signed(staker.clone()).into(),
            amount_1,
        ));

        #[extrinsic_call]
        lock(RawOrigin::Signed(staker.clone()), amount_2);

        assert_last_event::<T>(
            Event::<T>::Locked {
                account: staker,
                amount: amount_2,
            }
            .into(),
        );
    }

    #[benchmark]
    fn unlock() {
        initial_config::<T>();

        let staker: T::AccountId = whitelisted_caller();
        let owner: T::AccountId = account("dapp_owner", 0, SEED);
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            owner.clone().into(),
            smart_contract.clone(),
        ));

        let amount = T::MinimumLockedAmount::get() * 2;
        T::BenchmarkHelper::set_balance(&staker, amount);
        assert_ok!(DappStaking::<T>::lock(
            RawOrigin::Signed(staker.clone()).into(),
            amount,
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(staker.clone()), 1);

        assert_last_event::<T>(
            Event::<T>::Unlocking {
                account: staker,
                amount: 1,
            }
            .into(),
        );
    }

    #[benchmark]
    fn claim_unlocked(x: Linear<0, { T::MaxNumberOfStakedContracts::get() }>) {
        initial_config::<T>();

        // Prepare staker account and lock some amount
        let staker: T::AccountId = whitelisted_caller();
        let amount = (T::MinimumStakeAmount::get() + 1)
            * Into::<Balance>::into(max_number_of_contracts::<T>())
            + Into::<Balance>::into(T::MaxUnlockingChunks::get());
        T::BenchmarkHelper::set_balance(&staker, amount);
        assert_ok!(DappStaking::<T>::lock(
            RawOrigin::Signed(staker.clone()).into(),
            amount,
        ));

        // Move over to the build&earn subperiod to ensure 'non-loyal' staking.
        // This is needed so we can achieve staker entry cleanup after claiming unlocked tokens.
        force_advance_to_next_subperiod::<T>();
        assert_eq!(
          ActiveProtocolState::<T>::get().subperiod(),
          Subperiod::BuildAndEarn,
          "Sanity check - we need to stake during build&earn for entries to be cleaned up in the next era."
        );

        // Register required number of contracts and have staker stake on them.
        // This is needed to achieve the cleanup functionality.
        for idx in 0..x {
            let smart_contract = T::BenchmarkHelper::get_smart_contract(idx as u32);
            let owner: T::AccountId = account("dapp_owner", idx.into(), SEED);

            assert_ok!(DappStaking::<T>::register(
                RawOrigin::Root.into(),
                owner.clone().into(),
                smart_contract.clone(),
            ));

            assert_ok!(DappStaking::<T>::stake(
                RawOrigin::Signed(staker.clone()).into(),
                smart_contract,
                T::MinimumStakeAmount::get() + 1,
            ));
        }

        // Unlock some amount - but we want to fill up the whole vector with chunks.
        let unlock_amount = 1;
        for _ in 0..T::MaxUnlockingChunks::get() {
            assert_ok!(DappStaking::<T>::unlock(
                RawOrigin::Signed(staker.clone()).into(),
                unlock_amount,
            ));
            run_for_blocks::<T>(One::one());
        }
        assert_eq!(
            Ledger::<T>::get(&staker).unlocking.len(),
            T::MaxUnlockingChunks::get() as usize
        );
        let unlock_amount = unlock_amount * Into::<Balance>::into(T::MaxUnlockingChunks::get());

        // Hack
        // In order to speed up the benchmark, we reduce how long it takes to unlock the chunks
        let mut counter = 1u32;
        Ledger::<T>::mutate(&staker, |ledger| {
            ledger.unlocking.iter_mut().for_each(|unlocking| {
                unlocking.unlock_block =
                    (System::<T>::block_number() + counter.into()).saturated_into();
            });
            counter += 1;
        });

        // Advance to next period to ensure the old stake entries are cleaned up.
        force_advance_to_next_period::<T>();

        // Additionally, ensure enough blocks have passed so that the unlocking chunk can be claimed.
        let unlock_block = Ledger::<T>::get(&staker)
            .unlocking
            .last()
            .expect("At least one entry must exist.")
            .unlock_block;
        run_to_block::<T>(unlock_block.into());

        #[extrinsic_call]
        _(RawOrigin::Signed(staker.clone()));

        assert_last_event::<T>(
            Event::<T>::ClaimedUnlocked {
                account: staker,
                amount: unlock_amount,
            }
            .into(),
        );
    }

    #[benchmark]
    fn relock_unlocking() {
        initial_config::<T>();

        let staker: T::AccountId = whitelisted_caller();
        let owner: T::AccountId = account("dapp_owner", 0, SEED);
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            owner.clone().into(),
            smart_contract.clone(),
        ));

        let amount =
            T::MinimumLockedAmount::get() * 2 + Into::<Balance>::into(T::MaxUnlockingChunks::get());
        T::BenchmarkHelper::set_balance(&staker, amount);
        assert_ok!(DappStaking::<T>::lock(
            RawOrigin::Signed(staker.clone()).into(),
            amount,
        ));

        // Unlock some amount - but we want to fill up the whole vector with chunks.
        let unlock_amount = 1;
        for _ in 0..T::MaxUnlockingChunks::get() {
            assert_ok!(DappStaking::<T>::unlock(
                RawOrigin::Signed(staker.clone()).into(),
                unlock_amount,
            ));
            run_for_blocks::<T>(One::one());
        }
        let unlock_amount = unlock_amount * Into::<Balance>::into(T::MaxUnlockingChunks::get());

        #[extrinsic_call]
        _(RawOrigin::Signed(staker.clone()));

        assert_last_event::<T>(
            Event::<T>::Relock {
                account: staker,
                amount: unlock_amount,
            }
            .into(),
        );
    }

    #[benchmark]
    fn stake() {
        initial_config::<T>();

        let staker: T::AccountId = whitelisted_caller();
        let owner: T::AccountId = account("dapp_owner", 0, SEED);
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            owner.clone().into(),
            smart_contract.clone(),
        ));

        let amount = T::MinimumLockedAmount::get();
        T::BenchmarkHelper::set_balance(&staker, amount);
        assert_ok!(DappStaking::<T>::lock(
            RawOrigin::Signed(staker.clone()).into(),
            amount,
        ));

        #[extrinsic_call]
        _(
            RawOrigin::Signed(staker.clone()),
            smart_contract.clone(),
            amount,
        );

        assert_last_event::<T>(
            Event::<T>::Stake {
                account: staker,
                smart_contract,
                amount,
            }
            .into(),
        );
    }

    #[benchmark]
    fn unstake() {
        initial_config::<T>();

        let staker: T::AccountId = whitelisted_caller();
        let owner: T::AccountId = account("dapp_owner", 0, SEED);
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            owner.clone().into(),
            smart_contract.clone(),
        ));

        let amount = T::MinimumLockedAmount::get() + 1;
        T::BenchmarkHelper::set_balance(&staker, amount);
        assert_ok!(DappStaking::<T>::lock(
            RawOrigin::Signed(staker.clone()).into(),
            amount,
        ));

        assert_ok!(DappStaking::<T>::stake(
            RawOrigin::Signed(staker.clone()).into(),
            smart_contract.clone(),
            amount
        ));

        let unstake_amount = 1;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(staker.clone()),
            smart_contract.clone(),
            unstake_amount,
        );

        assert_last_event::<T>(
            Event::<T>::Unstake {
                account: staker,
                smart_contract,
                amount: unstake_amount,
            }
            .into(),
        );
    }

    #[benchmark]
    fn claim_staker_rewards_past_period(x: Linear<1, { max_claim_size_past_period::<T>() }>) {
        initial_config::<T>();

        // Prepare staker & register smart contract
        let staker: T::AccountId = whitelisted_caller();
        let owner: T::AccountId = account("dapp_owner", 0, SEED);
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            owner.clone().into(),
            smart_contract.clone(),
        ));

        // Lock & stake some amount by the staker
        let amount = T::MinimumLockedAmount::get();
        T::BenchmarkHelper::set_balance(&staker, amount);
        assert_ok!(DappStaking::<T>::lock(
            RawOrigin::Signed(staker.clone()).into(),
            amount,
        ));
        assert_ok!(DappStaking::<T>::stake(
            RawOrigin::Signed(staker.clone()).into(),
            smart_contract.clone(),
            amount
        ));

        // Hacky era advancement to ensure we have the exact number of eras to claim, but are already in the next period.
        force_advance_to_era::<T>(max_claim_size_past_period::<T>() - 1);
        force_advance_to_next_period::<T>();

        // Hack - modify staker's stake so it seems as if stake was valid from the 'first stake era'.
        // Also fill up the reward span.
        //
        // This allows us to easily control how many rewards are claimed, without having to advance large amount of blocks/eras/periods
        // to find an appropriate scenario.
        let first_stake_era = max_claim_size_past_period::<T>() - x;
        Ledger::<T>::mutate(&staker, |ledger| {
            ledger.staked = ledger.staked_future.unwrap();
            ledger.staked_future = None;
            ledger.staked.era = first_stake_era;
        });

        // Just fill them up, the ledger entry will control how much claims we can make
        let mut reward_span = EraRewardSpan::<_>::new();
        for era in 0..(T::EraRewardSpanLength::get()) {
            assert_ok!(reward_span.push(
                era as EraNumber,
                EraReward {
                    staker_reward_pool: 1_000_000_000_000,
                    staked: amount,
                    dapp_reward_pool: 1_000_000_000_000,
                },
            ));
        }
        EraRewards::<T>::insert(&0, reward_span);

        // For testing purposes
        System::<T>::reset_events();

        #[extrinsic_call]
        claim_staker_rewards(RawOrigin::Signed(staker.clone()));

        // No need to do precise check of values, but predetermined amount of 'Reward' events is expected.
        let dapp_staking_events = dapp_staking_events::<T>();
        assert_eq!(dapp_staking_events.len(), x as usize);
        dapp_staking_events.iter().for_each(|e| {
            assert_matches!(e, Event::Reward { .. });
        });
    }

    #[benchmark]
    fn claim_staker_rewards_ongoing_period(x: Linear<1, { max_claim_size_ongoing_period::<T>() }>) {
        initial_config::<T>();

        // Prepare staker & register smart contract
        let staker: T::AccountId = whitelisted_caller();
        let owner: T::AccountId = account("dapp_owner", 0, SEED);
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            owner.clone().into(),
            smart_contract.clone(),
        ));

        // Lock & stake some amount by the staker
        let amount = T::MinimumLockedAmount::get();
        T::BenchmarkHelper::set_balance(&staker, amount);
        assert_ok!(DappStaking::<T>::lock(
            RawOrigin::Signed(staker.clone()).into(),
            amount,
        ));
        assert_ok!(DappStaking::<T>::stake(
            RawOrigin::Signed(staker.clone()).into(),
            smart_contract.clone(),
            amount
        ));

        // Advance to era at the end of the first period or first span.
        force_advance_to_era::<T>(max_claim_size_ongoing_period::<T>());
        assert_eq!(
            ActiveProtocolState::<T>::get().period_number(),
            1,
            "Sanity check, we must still be in the first period."
        );

        // Hack - modify staker's stake so it seems as if stake was valid from the 'first stake era'/
        // Also fill up the reward span.
        //
        // This allows us to easily control how many rewards are claimed, without having to advance large amount of blocks/eras/periods
        // to find an appropriate scenario.
        let first_stake_era = max_claim_size_ongoing_period::<T>() - x;
        Ledger::<T>::mutate(&staker, |ledger| {
            ledger.staked = ledger.staked_future.unwrap();
            ledger.staked_future = None;
            ledger.staked.era = first_stake_era;
        });

        // Just fill them up, the ledger entry will control how much claims we can make
        let mut reward_span = EraRewardSpan::<_>::new();
        for era in 0..(T::EraRewardSpanLength::get()) {
            assert_ok!(reward_span.push(
                era as EraNumber,
                EraReward {
                    staker_reward_pool: 1_000_000_000_000,
                    staked: amount,
                    dapp_reward_pool: 1_000_000_000_000,
                },
            ));
        }
        EraRewards::<T>::insert(&0, reward_span);

        // For testing purposes
        System::<T>::reset_events();

        #[extrinsic_call]
        claim_staker_rewards(RawOrigin::Signed(staker.clone()));

        // No need to do precise check of values, but predetermined amount of 'Reward' events is expected.
        let dapp_staking_events = dapp_staking_events::<T>();
        assert_eq!(dapp_staking_events.len(), x as usize);
        dapp_staking_events.iter().for_each(|e| {
            assert_matches!(e, Event::Reward { .. });
        });
    }

    #[benchmark]
    fn claim_bonus_reward() {
        initial_config::<T>();

        // Prepare staker & register smart contract
        let staker: T::AccountId = whitelisted_caller();
        let owner: T::AccountId = account("dapp_owner", 0, SEED);
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            owner.clone().into(),
            smart_contract.clone(),
        ));

        // Lock & stake some amount by the staker
        let amount = T::MinimumLockedAmount::get();
        T::BenchmarkHelper::set_balance(&staker, amount);
        assert_ok!(DappStaking::<T>::lock(
            RawOrigin::Signed(staker.clone()).into(),
            amount,
        ));
        assert_ok!(DappStaking::<T>::stake(
            RawOrigin::Signed(staker.clone()).into(),
            smart_contract.clone(),
            amount
        ));

        // Advance to the next period so we can claim the bonus reward.
        force_advance_to_next_period::<T>();

        #[extrinsic_call]
        _(RawOrigin::Signed(staker.clone()), smart_contract.clone());

        // No need to do precise check of values, but last event must be 'BonusReward'.
        assert_matches!(
            dapp_staking_events::<T>().last(),
            Some(Event::BonusReward { .. })
        );
    }

    #[benchmark]
    fn claim_dapp_reward() {
        initial_config::<T>();

        // Register a dApp & stake on it.
        // This is the dApp for which we'll claim rewards for.
        let owner: T::AccountId = whitelisted_caller();
        let smart_contract = T::BenchmarkHelper::get_smart_contract(0);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            owner.clone().into(),
            smart_contract.clone(),
        ));

        let amount = MIN_TIER_THRESHOLD * 1000;
        T::BenchmarkHelper::set_balance(&owner, amount);
        assert_ok!(DappStaking::<T>::lock(
            RawOrigin::Signed(owner.clone()).into(),
            amount,
        ));
        assert_ok!(DappStaking::<T>::stake(
            RawOrigin::Signed(owner.clone()).into(),
            smart_contract.clone(),
            amount
        ));

        // Register & stake up to max number of contracts.
        // The reason is we want to have reward vector filled up to the capacity.
        for idx in 1..T::MaxNumberOfContracts::get() {
            let owner: T::AccountId = account("dapp_owner", idx.into(), SEED);
            let smart_contract = T::BenchmarkHelper::get_smart_contract(idx as u32);
            assert_ok!(DappStaking::<T>::register(
                RawOrigin::Root.into(),
                owner.clone().into(),
                smart_contract.clone(),
            ));

            let staker: T::AccountId = account("staker", idx.into(), SEED);
            T::BenchmarkHelper::set_balance(&staker, amount);
            assert_ok!(DappStaking::<T>::lock(
                RawOrigin::Signed(staker.clone()).into(),
                amount,
            ));
            assert_ok!(DappStaking::<T>::stake(
                RawOrigin::Signed(staker.clone()).into(),
                smart_contract.clone(),
                amount
            ));
        }

        // Advance enough eras so dApp reward can be claimed.
        force_advance_to_next_subperiod::<T>();

        // This is a hacky part to ensure we accommodate max number of contracts.
        TierConfig::<T>::mutate(|config| {
            let max_number_of_contracts: u16 = T::MaxNumberOfContracts::get().try_into().unwrap();
            config.slots_per_tier[0] = max_number_of_contracts;
            config.slots_per_tier[1..].iter_mut().for_each(|x| *x = 0);
            config.tier_thresholds[0] = 1;
        });
        force_advance_to_next_era::<T>();
        let claim_era = ActiveProtocolState::<T>::get().era - 1;

        assert_eq!(
            DAppTiers::<T>::get(claim_era)
                .expect("Must exist since it's from past build&earn era.")
                .dapps
                .len(),
            T::MaxNumberOfContracts::get() as usize,
            "Sanity check to ensure we have filled up the vector completely."
        );

        #[extrinsic_call]
        _(
            RawOrigin::Signed(owner.clone()),
            smart_contract.clone(),
            claim_era,
        );

        // No need to do precise check of values, but last event must be 'DAppReward'.
        assert_matches!(
            dapp_staking_events::<T>().last(),
            Some(Event::DAppReward { .. })
        );
    }

    #[benchmark]
    fn unstake_from_unregistered() {
        initial_config::<T>();

        let staker: T::AccountId = whitelisted_caller();
        let owner: T::AccountId = account("dapp_owner", 0, SEED);
        let smart_contract = T::BenchmarkHelper::get_smart_contract(1);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            owner.clone().into(),
            smart_contract.clone(),
        ));

        let amount = T::MinimumLockedAmount::get();
        T::BenchmarkHelper::set_balance(&staker, amount);
        assert_ok!(DappStaking::<T>::lock(
            RawOrigin::Signed(staker.clone()).into(),
            amount,
        ));

        assert_ok!(DappStaking::<T>::stake(
            RawOrigin::Signed(staker.clone()).into(),
            smart_contract.clone(),
            amount
        ));

        assert_ok!(DappStaking::<T>::unregister(
            RawOrigin::Root.into(),
            smart_contract.clone(),
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(staker.clone()), smart_contract.clone());

        assert_last_event::<T>(
            Event::<T>::UnstakeFromUnregistered {
                account: staker,
                smart_contract,
                amount,
            }
            .into(),
        );
    }

    #[benchmark]
    fn cleanup_expired_entries(x: Linear<1, { T::MaxNumberOfStakedContracts::get() }>) {
        initial_config::<T>();

        // Move over to the build&earn subperiod to ensure 'non-loyal' staking.
        force_advance_to_next_subperiod::<T>();

        // Prepare staker & lock some amount
        let staker: T::AccountId = whitelisted_caller();
        let amount = T::MinimumLockedAmount::get()
            * Into::<Balance>::into(T::MaxNumberOfStakedContracts::get());
        T::BenchmarkHelper::set_balance(&staker, amount);
        assert_ok!(DappStaking::<T>::lock(
            RawOrigin::Signed(staker.clone()).into(),
            amount,
        ));

        // Register dApps up the the limit
        for idx in 0..x {
            let owner: T::AccountId = account("dapp_owner", idx.into(), SEED);
            let smart_contract = T::BenchmarkHelper::get_smart_contract(idx as u32);
            assert_ok!(DappStaking::<T>::register(
                RawOrigin::Root.into(),
                owner.clone().into(),
                smart_contract.clone(),
            ));

            assert_ok!(DappStaking::<T>::stake(
                RawOrigin::Signed(staker.clone()).into(),
                smart_contract.clone(),
                T::MinimumStakeAmount::get(),
            ));
        }

        // Move over to the next period, marking the entries as expired since they don't have the loyalty flag.
        force_advance_to_next_period::<T>();

        #[extrinsic_call]
        _(RawOrigin::Signed(staker.clone()));

        assert_last_event::<T>(
            Event::<T>::ExpiredEntriesRemoved {
                account: staker,
                count: x.try_into().unwrap(),
            }
            .into(),
        );
    }

    #[benchmark]
    fn force() {
        initial_config::<T>();

        let forcing_type = ForcingType::Subperiod;

        #[extrinsic_call]
        _(RawOrigin::Root, forcing_type);

        assert_last_event::<T>(Event::<T>::Force { forcing_type }.into());
    }

    #[benchmark]
    fn on_initialize_voting_to_build_and_earn() {
        initial_config::<T>();

        let state = ActiveProtocolState::<T>::get();
        assert_eq!(state.subperiod(), Subperiod::Voting, "Sanity check.");

        // Register & stake contracts, just so we don't have empty stakes.
        prepare_contracts_for_tier_assignment::<T>(max_number_of_contracts::<T>());

        run_to_block::<T>((state.next_era_start - 1).into());
        DappStaking::<T>::on_finalize((state.next_era_start - 1).into());
        System::<T>::set_block_number(state.next_era_start.into());

        #[block]
        {
            DappStaking::<T>::era_and_period_handler(state.next_era_start, TierAssignment::Dummy);
        }

        assert_eq!(
            ActiveProtocolState::<T>::get().subperiod(),
            Subperiod::BuildAndEarn
        );
    }

    #[benchmark]
    fn on_initialize_build_and_earn_to_voting() {
        initial_config::<T>();

        // Get started
        assert_eq!(
            ActiveProtocolState::<T>::get().subperiod(),
            Subperiod::Voting,
            "Sanity check."
        );

        // Register & stake contracts, just so we don't have empty stakes.
        prepare_contracts_for_tier_assignment::<T>(max_number_of_contracts::<T>());

        // Force advance enough periods into the future so we can ensure that history
        // cleanup marker will be updated on the next period change.
        let period_before_expiry_starts =
            ActiveProtocolState::<T>::get().period_number() + T::RewardRetentionInPeriods::get();
        force_advance_to_period::<T>(period_before_expiry_starts);

        // Advance to build&earn subperiod
        force_advance_to_next_subperiod::<T>();
        let snapshot_state = ActiveProtocolState::<T>::get();

        // Advance over to the last era of the subperiod, and then again to the last block of that era.
        advance_to_era::<T>(
            ActiveProtocolState::<T>::get()
                .period_info
                .next_subperiod_start_era
                - 1,
        );
        run_to_block::<T>((ActiveProtocolState::<T>::get().next_era_start - 1).into());

        // Some sanity checks, we should still be in the build&earn subperiod, and in the same period as when snapshot was taken.
        assert_eq!(
            ActiveProtocolState::<T>::get().subperiod(),
            Subperiod::BuildAndEarn
        );
        assert_eq!(
            ActiveProtocolState::<T>::get().period_number(),
            snapshot_state.period_number(),
        );

        let new_era_start_block = ActiveProtocolState::<T>::get().next_era_start;
        DappStaking::<T>::on_finalize((new_era_start_block - 1).into());
        System::<T>::set_block_number(new_era_start_block.into());

        let pre_cleanup_marker = HistoryCleanupMarker::<T>::get();

        #[block]
        {
            DappStaking::<T>::era_and_period_handler(new_era_start_block, TierAssignment::Dummy);
        }

        assert_eq!(
            ActiveProtocolState::<T>::get().subperiod(),
            Subperiod::Voting
        );
        assert_eq!(
            ActiveProtocolState::<T>::get().period_number(),
            snapshot_state.period_number() + 1,
        );
        assert!(
            HistoryCleanupMarker::<T>::get().oldest_valid_era > pre_cleanup_marker.oldest_valid_era
        );
    }

    #[benchmark]
    fn on_initialize_build_and_earn_to_build_and_earn() {
        initial_config::<T>();

        // Register & stake contracts, just so we don't have empty stakes.
        prepare_contracts_for_tier_assignment::<T>(max_number_of_contracts::<T>());

        // Advance to build&earn subperiod
        force_advance_to_next_subperiod::<T>();
        let snapshot_state = ActiveProtocolState::<T>::get();

        // Advance over to the next era, and then again to the last block of that era.
        force_advance_to_next_era::<T>();
        run_to_block::<T>((ActiveProtocolState::<T>::get().next_era_start - 1).into());

        // Some sanity checks, we should still be in the build&earn subperiod, and in the first period.
        assert_eq!(
            ActiveProtocolState::<T>::get().subperiod(),
            Subperiod::BuildAndEarn
        );
        assert_eq!(
            ActiveProtocolState::<T>::get().period_number(),
            snapshot_state.period_number(),
        );

        let new_era_start_block = ActiveProtocolState::<T>::get().next_era_start;
        DappStaking::<T>::on_finalize((new_era_start_block - 1).into());
        System::<T>::set_block_number(new_era_start_block.into());

        #[block]
        {
            DappStaking::<T>::era_and_period_handler(new_era_start_block, TierAssignment::Dummy);
        }

        assert_eq!(
            ActiveProtocolState::<T>::get().subperiod(),
            Subperiod::BuildAndEarn
        );
        assert_eq!(
            ActiveProtocolState::<T>::get().period_number(),
            snapshot_state.period_number(),
        );
    }

    // Investigate why the PoV size is so large here, even after removing read of `IntegratedDApps` storage.
    // Relevant file: polkadot-sdk/substrate/utils/frame/benchmarking-cli/src/pallet/writer.rs
    // UPDATE: after some investigation, it seems that PoV size benchmarks are very imprecise
    // - the worst case measured is usually very far off the actual value that is consumed on chain.
    // There's an ongoing item to improve it (mentioned on roundtable meeting).
    #[benchmark]
    fn dapp_tier_assignment(x: Linear<0, { max_number_of_contracts::<T>() }>) {
        // Prepare init config (protocol state, tier params & config, etc.)
        initial_config::<T>();

        // Register & stake contracts, to prepare for tier assignment.
        prepare_contracts_for_tier_assignment::<T>(x);
        force_advance_to_next_era::<T>();

        // Need to ensure settings remain unchanged even after the era change
        init_tier_settings::<T>();

        let reward_era = ActiveProtocolState::<T>::get().era;
        let reward_period = ActiveProtocolState::<T>::get().period_number();
        let reward_pool = Balance::from(10_000 * UNIT as u128);

        #[block]
        {
            let (dapp_tiers, _count) = Pallet::<T>::get_dapp_tier_assignment_and_rewards(
                reward_era,
                reward_period,
                reward_pool,
            );
            assert_eq!(dapp_tiers.dapps.len(), x as usize);
        }
    }

    #[benchmark]
    fn on_idle_cleanup() {
        // Prepare init config (protocol state, tier params & config, etc.)
        initial_config::<T>();

        // Hack
        // Manually prepare state prior to the cleanup to ensure worst case.
        let cleanup_marker = CleanupMarker {
            era_reward_index: 0,
            dapp_tiers_index: 0,
            oldest_valid_era: T::EraRewardSpanLength::get().into(),
        };
        HistoryCleanupMarker::<T>::put(cleanup_marker);

        // Prepare completely filled up reward span and insert it into storage.
        let mut reward_span = EraRewardSpan::<_>::new();
        (0..T::EraRewardSpanLength::get()).for_each(|era| {
            assert_ok!(reward_span.push(
                era as EraNumber,
                EraReward {
                    staker_reward_pool: 1_000_000_000_000,
                    staked: 1_000_000_000_000,
                    dapp_reward_pool: 1_000_000_000_000,
                },
            ));
        });
        EraRewards::<T>::insert(&cleanup_marker.era_reward_index, reward_span);

        // Prepare completely filled up tier rewards and insert it into storage.
        DAppTiers::<T>::insert(
            &cleanup_marker.dapp_tiers_index,
            DAppTierRewardsFor::<T> {
                dapps: (0..T::MaxNumberOfContracts::get())
                    .map(|dapp_id| (dapp_id as DAppId, RankedTier::new_saturated(0, 0)))
                    .collect::<BTreeMap<DAppId, RankedTier>>()
                    .try_into()
                    .expect("Using `MaxNumberOfContracts` as length; QED."),
                rewards: vec![1_000_000_000_000; T::NumberOfTiers::get() as usize]
                    .try_into()
                    .expect("Using `NumberOfTiers` as length; QED."),
                period: 1,
                rank_rewards: vec![0; T::NumberOfTiers::get() as usize]
                    .try_into()
                    .expect("Using `NumberOfTiers` as length; QED."),
            },
        );

        let block_number = System::<T>::block_number();
        #[block]
        {
            DappStaking::<T>::on_idle(block_number, Weight::MAX);
        }

        assert!(
            !EraRewards::<T>::contains_key(cleanup_marker.era_reward_index),
            "Reward span should have been cleaned up."
        );
        assert!(
            !DAppTiers::<T>::contains_key(cleanup_marker.dapp_tiers_index),
            "Period end info should have been cleaned up."
        );
    }

    impl_benchmark_test_suite!(
        Pallet,
        crate::benchmarking::tests::new_test_ext(),
        crate::test::mock::Test,
    );
}

#[cfg(test)]
mod tests {
    use crate::test::mock;
    use sp_io::TestExternalities;

    pub fn new_test_ext() -> TestExternalities {
        mock::ExtBuilder::default().build()
    }
}
