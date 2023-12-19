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

extern crate alloc;
use crate::{test::mock::*, *};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use precompile_utils::testing::*;
use sp_core::H160;
use sp_runtime::traits::Zero;

use assert_matches::assert_matches;

use pallet_dapp_staking_v3::{ActiveProtocolState, EraNumber, EraRewards};

#[test]
fn read_current_era_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        precompiles()
            .prepare_test(
                Alice,
                precompile_address(),
                PrecompileCall::read_current_era {},
            )
            .expect_no_logs()
            .execute_returns(ActiveProtocolState::<Test>::get().era);

        // advance a few eras, check value again
        advance_to_era(7);
        precompiles()
            .prepare_test(
                Alice,
                precompile_address(),
                PrecompileCall::read_current_era {},
            )
            .expect_no_logs()
            .execute_returns(ActiveProtocolState::<Test>::get().era);
    });
}

#[test]
fn read_unbonding_period_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        let unlocking_period_in_eras: EraNumber =
            <Test as pallet_dapp_staking_v3::Config>::UnlockingPeriod::get();

        precompiles()
            .prepare_test(
                Alice,
                precompile_address(),
                PrecompileCall::read_unbonding_period {},
            )
            .expect_no_logs()
            .execute_returns(unlocking_period_in_eras);
    });
}

#[test]
fn read_era_reward_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Check historic era for rewards
        let era = 3;
        advance_to_era(era + 1);

        let span_index = DAppStaking::<Test>::era_reward_span_index(era);

        let era_rewards_span = EraRewards::<Test>::get(span_index).expect("Entry must exist.");
        let expected_reward = era_rewards_span
            .get(era)
            .map(|r| r.staker_reward_pool + r.dapp_reward_pool)
            .expect("It's history era so it must exist.");
        assert!(expected_reward > 0, "Sanity check.");

        precompiles()
            .prepare_test(
                Alice,
                precompile_address(),
                PrecompileCall::read_era_reward { era },
            )
            .expect_no_logs()
            .execute_returns(expected_reward);

        // Check current era for rewards, must be zero
        precompiles()
            .prepare_test(
                Alice,
                precompile_address(),
                PrecompileCall::read_era_reward { era: era + 1 },
            )
            .expect_no_logs()
            .execute_returns(Balance::zero());
    });
}

#[test]
fn read_era_staked_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        let staker_h160 = ALICE;
        let smart_contract_address = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address);
        let amount = 1_000_000_000_000;
        register_and_stake(staker_h160, smart_contract.clone(), amount);
        let anchor_era = ActiveProtocolState::<Test>::get().era;

        // 1. Current era stake must be zero, since stake is only valid from the next era.
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_era_staked { era: anchor_era },
            )
            .expect_no_logs()
            .execute_returns(Balance::zero());

        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_era_staked {
                    era: anchor_era + 1,
                },
            )
            .expect_no_logs()
            .execute_returns(amount);

        // 2. Advance to next era, and check next era after the anchor.
        advance_to_era(anchor_era + 5);
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_era_staked {
                    era: anchor_era + 1,
                },
            )
            .expect_no_logs()
            .execute_returns(amount);

        // 3. Check era after the next one, must throw an error.
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_era_staked {
                    era: ActiveProtocolState::<Test>::get().era + 2,
                },
            )
            .expect_no_logs()
            .execute_reverts(|output| output == b"Era is in the future");
    });
}

#[test]
fn read_staked_amount_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        let staker_h160 = ALICE;
        let dynamic_addresses = into_dynamic_addresses(staker_h160);

        // 1. Sanity checks - must be zero before anything is staked.
        for staker in &dynamic_addresses {
            precompiles()
                .prepare_test(
                    staker_h160,
                    precompile_address(),
                    PrecompileCall::read_staked_amount {
                        staker: staker.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(Balance::zero());
        }

        // 2. Stake some amount and check again
        let smart_contract_address = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address);
        let amount = 1_000_000_000_000;
        register_and_stake(staker_h160, smart_contract.clone(), amount);
        for staker in &dynamic_addresses {
            precompiles()
                .prepare_test(
                    staker_h160,
                    precompile_address(),
                    PrecompileCall::read_staked_amount {
                        staker: staker.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(amount);
        }

        // 3. Advance into next period, it should be reset back to zero
        advance_to_next_period();
        for staker in &dynamic_addresses {
            precompiles()
                .prepare_test(
                    staker_h160,
                    precompile_address(),
                    PrecompileCall::read_staked_amount {
                        staker: staker.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(Balance::zero());
        }
    });
}

#[test]
fn read_staked_amount_on_contract_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        let staker_h160 = ALICE;
        let smart_contract_address = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address);
        let dynamic_addresses = into_dynamic_addresses(staker_h160);

        // 1. Sanity checks - must be zero before anything is staked.
        for staker in &dynamic_addresses {
            precompiles()
                .prepare_test(
                    staker_h160,
                    precompile_address(),
                    PrecompileCall::read_staked_amount_on_contract {
                        contract_h160: smart_contract_address.into(),
                        staker: staker.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(Balance::zero());
        }

        // 2. Stake some amount and check again
        let amount = 1_000_000_000_000;
        register_and_stake(staker_h160, smart_contract.clone(), amount);
        for staker in &dynamic_addresses {
            precompiles()
                .prepare_test(
                    staker_h160,
                    precompile_address(),
                    PrecompileCall::read_staked_amount_on_contract {
                        contract_h160: smart_contract_address.into(),
                        staker: staker.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(amount);
        }

        // 3. Advance into next period, it should be reset back to zero
        advance_to_next_period();
        for staker in &dynamic_addresses {
            precompiles()
                .prepare_test(
                    staker_h160,
                    precompile_address(),
                    PrecompileCall::read_staked_amount_on_contract {
                        contract_h160: smart_contract_address.into(),
                        staker: staker.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(Balance::zero());
        }
    });
}

#[test]
fn read_contract_stake_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        let staker_h160 = ALICE;
        let smart_contract_address = H160::repeat_byte(0xFA);

        // 1. Sanity checks - must be zero before anything is staked.
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_contract_stake {
                    contract_h160: smart_contract_address.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(Balance::zero());

        // 2. Stake some amount and check again
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address);
        let amount = 1_000_000_000_000;
        register_and_stake(staker_h160, smart_contract.clone(), amount);

        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_contract_stake {
                    contract_h160: smart_contract_address.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(amount);

        // 3. Advance into next period, it should be reset back to zero
        advance_to_next_period();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::read_contract_stake {
                    contract_h160: smart_contract_address.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(Balance::zero());
    });
}

#[test]
fn register_is_unsupported() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        precompiles()
            .prepare_test(
                ALICE,
                precompile_address(),
                PrecompileCall::register {
                    _address: Default::default(),
                },
            )
            .expect_no_logs()
            .execute_reverts(|output| output == b"register via evm precompile is not allowed");
    });
}

#[test]
fn set_reward_destination_is_unsupported() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        precompiles()
            .prepare_test(
                ALICE,
                precompile_address(),
                PrecompileCall::set_reward_destination { _destination: 0 },
            )
            .expect_no_logs()
            .execute_reverts(|output| {
                output == b"Setting reward destination is no longer supported."
            });
    });
}

#[test]
fn bond_and_stake_with_two_calls_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp for staking
        let staker_h160 = ALICE;
        let smart_contract_address = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address);
        assert_ok!(DappStaking::register(
            RawOrigin::Root.into(),
            AddressMapper::into_account_id(staker_h160),
            smart_contract.clone()
        ));

        // Lock some amount, but not enough to cover the `bond_and_stake` call.
        let pre_lock_amount = 500;
        let stake_amount = 1_000_000;
        assert_ok!(DappStaking::lock(
            RawOrigin::Signed(AddressMapper::into_account_id(staker_h160)).into(),
            pre_lock_amount,
        ));

        // Execute legacy call, expect missing funds to be locked.
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::bond_and_stake {
                    contract_h160: smart_contract_address.into(),
                    amount: stake_amount,
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 2);
        let additional_lock_amount = stake_amount - pre_lock_amount;
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::Locked {
                amount,
                ..
            } if amount == additional_lock_amount
        );
        assert_matches!(
            events[1].clone(),
            pallet_dapp_staking_v3::Event::Stake {
                smart_contract,
                amount,
                ..
            } if smart_contract == smart_contract && amount == stake_amount
        );
    });
}

#[test]
fn bond_and_stake_with_single_call_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp for staking
        let staker_h160 = ALICE;
        let smart_contract_address = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address);
        assert_ok!(DappStaking::register(
            RawOrigin::Root.into(),
            AddressMapper::into_account_id(staker_h160),
            smart_contract.clone()
        ));

        // Lock enough amount to cover `bond_and_stake` call.
        let amount = 3000;
        assert_ok!(DappStaking::lock(
            RawOrigin::Signed(AddressMapper::into_account_id(staker_h160)).into(),
            amount,
        ));

        // Execute legacy call, expect only single stake to be executed.
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::bond_and_stake {
                    contract_h160: smart_contract_address.into(),
                    amount,
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 1);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::Stake {
                smart_contract,
                amount,
                ..
            } if smart_contract == smart_contract && amount == amount
        );
    });
}

#[test]
fn unbond_and_unstake_with_two_calls_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp for staking
        let staker_h160 = ALICE;
        let smart_contract_address = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address);
        let amount = 1_000_000_000_000;
        register_and_stake(staker_h160, smart_contract.clone(), amount);

        // Execute legacy call, expect funds to first unstaked, and then unlocked
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::unbond_and_unstake {
                    contract_h160: smart_contract_address.into(),
                    amount,
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 2);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::Unstake {
                smart_contract,
                amount,
                ..
            }if smart_contract == smart_contract && amount == amount
        );
        assert_matches!(
            events[1].clone(),
            pallet_dapp_staking_v3::Event::Unlocking { amount, .. } if amount == amount
        );
    });
}

#[test]
fn unbond_and_unstake_with_single_calls_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp for staking
        let staker_h160 = ALICE;
        let smart_contract_address = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address);
        let amount = 1_000_000_000_000;
        register_and_stake(staker_h160, smart_contract.clone(), amount);

        // Unstake the entire amount, so only unlock call is expected.
        assert_ok!(DappStaking::unstake(
            RawOrigin::Signed(AddressMapper::into_account_id(staker_h160)).into(),
            smart_contract.clone(),
            amount,
        ));

        // Execute legacy call, expect funds to be unlocked
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::unbond_and_unstake {
                    contract_h160: smart_contract_address.into(),
                    amount,
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 1);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::Unlocking { amount, .. } if amount == amount
        );
    });
}

#[test]
fn withdraw_unbonded_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp for staking
        let staker_h160 = ALICE;
        let staker_native = AddressMapper::into_account_id(staker_h160);
        let smart_contract_address = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address);
        let amount = 1_000_000_000_000;
        register_and_stake(staker_h160, smart_contract.clone(), amount);

        // Unlock some amount
        assert_ok!(DappStaking::unstake(
            RawOrigin::Signed(staker_native.clone()).into(),
            smart_contract.clone(),
            amount,
        ));
        let unlock_amount = amount / 7;
        assert_ok!(DappStaking::unlock(
            RawOrigin::Signed(staker_native.clone()).into(),
            unlock_amount,
        ));

        // Advance enough into time so unlocking chunk can be claimed
        let unlock_block = Ledger::<Test>::get(&staker_native).unlocking[0].unlock_block;
        run_to_block(unlock_block);

        // Execute legacy call, expect unlocked funds to be claimed back
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::withdraw_unbonded {},
            )
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 1);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::ClaimedUnlocked {
                amount,
                ..
            } if amount == unlock_amount
        );
    });
}

#[test]
fn claim_dapp_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp for staking
        let staker_h160 = ALICE;
        let smart_contract_address = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address);
        let amount = 1_000_000_000_000;
        register_and_stake(staker_h160, smart_contract.clone(), amount);

        // Advance enough eras so we can claim dApp reward
        advance_to_era(3);
        let claim_era = 2;

        // Execute legacy call, expect dApp rewards to be claimed
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::claim_dapp {
                    contract_h160: smart_contract_address.into(),
                    era: claim_era,
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 1);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::DAppReward {
                era,
                smart_contract,
                ..
            } if era as u128 == claim_era && smart_contract == smart_contract
        );
    });
}

#[test]
fn claim_staker_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp for staking
        let staker_h160 = ALICE;
        let smart_contract_address = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address);
        let amount = 1_000_000_000_000;
        register_and_stake(staker_h160, smart_contract.clone(), amount);

        // Advance enough eras so we can claim dApp reward
        let target_era = 5;
        advance_to_era(target_era);
        let number_of_claims = (2..target_era).count();

        // Execute legacy call, expect dApp rewards to be claimed
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::claim_staker {
                    _contract_h160: smart_contract_address.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        // We expect multiple reward to be claimed
        let events = dapp_staking_events();
        assert_eq!(events.len(), number_of_claims as usize);
        for era in 2..target_era {
            assert_matches!(
                events[era as usize - 2].clone(),
                pallet_dapp_staking_v3::Event::Reward { era, .. } if era == era
            );
        }
    });
}

#[test]
fn withdraw_from_unregistered_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp for staking
        let staker_h160 = ALICE;
        let smart_contract_address = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address);
        let amount = 1_000_000_000_000;
        register_and_stake(staker_h160, smart_contract.clone(), amount);

        // Unregister the dApp
        assert_ok!(DappStaking::unregister(
            RawOrigin::Root.into(),
            smart_contract.clone()
        ));

        // Execute legacy call, expect funds to be unstaked & withdrawn
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::withdraw_from_unregistered {
                    contract_h160: smart_contract_address.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 1);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::UnstakeFromUnregistered {
                smart_contract,
                amount,
                ..
            } if smart_contract == smart_contract && amount == amount
        );
    });
}

#[test]
fn nomination_transfer_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register the first dApp, and stke on it.
        let staker_h160 = ALICE;
        let staker_native = AddressMapper::into_account_id(staker_h160);
        let smart_contract_address_1 = H160::repeat_byte(0xFA);
        let smart_contract_1 =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address_1);
        let amount = 1_000_000_000_000;
        register_and_stake(staker_h160, smart_contract_1.clone(), amount);

        // Register the second dApp.
        let smart_contract_address_2 = H160::repeat_byte(0xBF);
        let smart_contract_2 =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_address_2);
        assert_ok!(DappStaking::register(
            RawOrigin::Root.into(),
            staker_native.clone(),
            smart_contract_2.clone()
        ));

        // 1st scenario - transfer enough amount from the first to second dApp to cover the stake,
        //                but not enough for full unstake.
        let minimum_stake_amount: Balance =
            <Test as pallet_dapp_staking_v3::Config>::MinimumStakeAmount::get();

        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::nomination_transfer {
                    origin_contract_h160: smart_contract_address_1.into(),
                    amount: minimum_stake_amount,
                    target_contract_h160: smart_contract_address_2.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        // We expect the same amount to be staked on the second contract
        let events = dapp_staking_events();
        assert_eq!(events.len(), 2);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::Unstake {
                smart_contract,
                amount,
                ..
            } if smart_contract == smart_contract_1 && amount == minimum_stake_amount
        );
        assert_matches!(
            events[1].clone(),
            pallet_dapp_staking_v3::Event::Stake {
                smart_contract,
                amount,
                ..
            } if smart_contract == smart_contract_2 && amount == minimum_stake_amount
        );

        // 2nd scenario - transfer almost the entire amount from the first to second dApp.
        //                The amount is large enough to trigger full unstake of the first contract.
        let unstake_amount = amount - minimum_stake_amount - 1;
        let expected_stake_unstake_amount = amount - minimum_stake_amount;

        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::nomination_transfer {
                    origin_contract_h160: smart_contract_address_1.into(),
                    amount: unstake_amount,
                    target_contract_h160: smart_contract_address_2.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        // We expect the same amount to be staked on the second contract
        let events = dapp_staking_events();
        assert_eq!(events.len(), 2);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::Unstake {
                smart_contract,
                amount,
                ..
            } if smart_contract == smart_contract_1 && amount == expected_stake_unstake_amount
        );
        assert_matches!(
            events[1].clone(),
            pallet_dapp_staking_v3::Event::Stake {
                smart_contract,
                amount,
                ..
            } if smart_contract == smart_contract_2 && amount == expected_stake_unstake_amount
        );
    });
}
