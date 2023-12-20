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

use assert_matches::assert_matches;

use astar_primitives::{dapp_staking::CycleConfiguration, BlockNumber};
use pallet_dapp_staking_v3::{ActiveProtocolState, EraNumber};

#[test]
fn protocol_state_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Prepare some mixed state in the future so not all entries are 'zero'
        advance_to_next_period();
        advance_to_next_era();

        let state = ActiveProtocolState::<Test>::get();

        let expected_outcome = PrecompileProtocolState {
            era: state.era.into(),
            period: state.period_number().into(),
            subperiod: subperiod_id(&state.subperiod()),
        };

        precompiles()
            .prepare_test(
                Alice,
                precompile_address(),
                PrecompileCall::protocol_state {},
            )
            .expect_no_logs()
            .execute_returns(expected_outcome);
    });
}

#[test]
fn unlocking_period_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        let unlocking_period_in_eras: EraNumber =
            <Test as pallet_dapp_staking_v3::Config>::UnlockingPeriod::get();
        let era_length: BlockNumber =
            <Test as pallet_dapp_staking_v3::Config>::CycleConfiguration::blocks_per_era();

        let expected_outcome = era_length * unlocking_period_in_eras;

        precompiles()
            .prepare_test(
                Alice,
                precompile_address(),
                PrecompileCall::unlocking_period {},
            )
            .expect_no_logs()
            .execute_returns(expected_outcome);
    });
}

#[test]
fn lock_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Lock some amount and verify event
        let amount = 1234;
        System::reset_events();
        precompiles()
            .prepare_test(ALICE, precompile_address(), PrecompileCall::lock { amount })
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 1);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::Locked {
                amount,
                ..
            } if amount == amount
        );
    });
}

#[test]
fn unlock_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        let lock_amount = 1234;
        assert_ok!(DappStaking::lock(
            RawOrigin::Signed(AddressMapper::into_account_id(ALICE)).into(),
            lock_amount,
        ));

        // Unlock some amount and verify event
        System::reset_events();
        let unlock_amount = 1234 / 7;
        precompiles()
            .prepare_test(
                ALICE,
                precompile_address(),
                PrecompileCall::unlock {
                    amount: unlock_amount,
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 1);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::Unlocking {
                amount,
                ..
            } if amount == unlock_amount
        );
    });
}

#[test]
fn claim_unlocked_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Lock/unlock some amount to create unlocking chunk
        let amount = 1234;
        assert_ok!(DappStaking::lock(
            RawOrigin::Signed(AddressMapper::into_account_id(ALICE)).into(),
            amount,
        ));
        assert_ok!(DappStaking::unlock(
            RawOrigin::Signed(AddressMapper::into_account_id(ALICE)).into(),
            amount,
        ));

        // Advance enough into time so unlocking chunk can be claimed
        let unlock_block =
            Ledger::<Test>::get(&AddressMapper::into_account_id(ALICE)).unlocking[0].unlock_block;
        run_to_block(unlock_block);

        // Claim unlocked chunk and verify event
        System::reset_events();
        precompiles()
            .prepare_test(
                ALICE,
                precompile_address(),
                PrecompileCall::claim_unlocked {},
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
            } if amount == amount
        );
    });
}

#[test]
fn stake_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp for staking
        let staker_h160 = ALICE;
        let smart_contract_h160 = H160::repeat_byte(0xFA);
        let smart_contract =
            <Test as pallet_dapp_staking_v3::Config>::SmartContract::evm(smart_contract_h160);
        assert_ok!(DappStaking::register(
            RawOrigin::Root.into(),
            AddressMapper::into_account_id(staker_h160),
            smart_contract.clone()
        ));

        // Lock some amount which will be used for staking
        let amount = 2000;
        assert_ok!(DappStaking::lock(
            RawOrigin::Signed(AddressMapper::into_account_id(staker_h160)).into(),
            amount,
        ));

        let smart_contract_v2 = SmartContractV2 {
            contract_type: SmartContractTypes::Evm,
            address: smart_contract_h160.as_bytes().try_into().unwrap(),
        };

        // Stake some amount and verify event
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::stake {
                    smart_contract: smart_contract_v2,
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
fn unstake_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp for staking
        let staker_h160 = ALICE;
        let smart_contract_address = [0xAF; 32];
        let smart_contract = <Test as pallet_dapp_staking_v3::Config>::SmartContract::wasm(
            smart_contract_address.into(),
        );
        assert_ok!(DappStaking::register(
            RawOrigin::Root.into(),
            AddressMapper::into_account_id(staker_h160),
            smart_contract.clone()
        ));

        // Lock & stake some amount
        let amount = 2000;
        assert_ok!(DappStaking::lock(
            RawOrigin::Signed(AddressMapper::into_account_id(staker_h160)).into(),
            amount,
        ));
        assert_ok!(DappStaking::stake(
            RawOrigin::Signed(AddressMapper::into_account_id(staker_h160)).into(),
            smart_contract.clone(),
            amount,
        ));

        let smart_contract_v2 = SmartContractV2 {
            contract_type: SmartContractTypes::Wasm,
            address: smart_contract_address.into(),
        };

        // Unstake some amount and verify event
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::unstake {
                    smart_contract: smart_contract_v2,
                    amount,
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 1);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::Unstake {
                smart_contract,
                amount,
                ..
            } if smart_contract == smart_contract && amount == amount
        );
    });
}

#[test]
fn claim_staker_rewards_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp and stake on it
        let staker_h160 = ALICE;
        let smart_contract_address = [0xAF; 32];
        let smart_contract = <Test as pallet_dapp_staking_v3::Config>::SmartContract::wasm(
            smart_contract_address.into(),
        );
        let amount = 1234;
        register_and_stake(staker_h160, smart_contract.clone(), amount);

        // Advance a few eras so we can claim a few rewards
        let target_era = 7;
        advance_to_era(target_era);
        let number_of_claims = (2..target_era).count();

        // Claim staker rewards and verify events
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::claim_staker_rewards {},
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
fn claim_bonus_reward_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp and stake on it, loyally
        let staker_h160 = ALICE;
        let smart_contract_address = [0xAF; 32];
        let smart_contract = <Test as pallet_dapp_staking_v3::Config>::SmartContract::wasm(
            smart_contract_address.into(),
        );
        let amount = 1234;
        register_and_stake(staker_h160, smart_contract.clone(), amount);

        // Advance to the next period
        advance_to_next_period();

        let smart_contract_v2 = SmartContractV2 {
            contract_type: SmartContractTypes::Wasm,
            address: smart_contract_address.into(),
        };

        // Claim bonus reward and verify event
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::claim_bonus_reward {
                   smart_contract: smart_contract_v2,
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 1);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::BonusReward { smart_contract, .. } if smart_contract == smart_contract
        );
    });
}

#[test]
fn claim_dapp_reward_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp and stake on it
        let staker_h160 = ALICE;
        let smart_contract_address = [0xAF; 32];
        let smart_contract = <Test as pallet_dapp_staking_v3::Config>::SmartContract::wasm(
            smart_contract_address.into(),
        );
        let amount = 1234;
        register_and_stake(staker_h160, smart_contract.clone(), amount);

        // Advance to 3rd era so we claim rewards for the 2nd era
        advance_to_era(3);

        let smart_contract_v2 = SmartContractV2 {
            contract_type: SmartContractTypes::Wasm,
            address: smart_contract_address.into(),
        };

        // Claim dApp reward and verify event
        let claim_era: EraNumber = 2;
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::claim_dapp_reward {
                   smart_contract: smart_contract_v2,
                   era: claim_era.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 1);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::DAppReward { era, smart_contract, .. } if era == claim_era && smart_contract == smart_contract
        );
    });
}

#[test]
fn unstake_from_unregistered_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Register a dApp and stake on it
        let staker_h160 = ALICE;
        let smart_contract_address = [0xAF; 32];
        let smart_contract = <Test as pallet_dapp_staking_v3::Config>::SmartContract::wasm(
            smart_contract_address.into(),
        );
        let amount = 1234;
        register_and_stake(staker_h160, smart_contract.clone(), amount);

        // Unregister the dApp
        assert_ok!(DappStaking::unregister(
            RawOrigin::Root.into(),
            smart_contract.clone()
        ));

        let smart_contract_v2 = SmartContractV2 {
            contract_type: SmartContractTypes::Wasm,
            address: smart_contract_address.into(),
        };

        // Unstake from the unregistered dApp and verify event
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::unstake_from_unregistered {
                   smart_contract: smart_contract_v2,
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 1);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::UnstakeFromUnregistered { smart_contract, amount, .. } if smart_contract == smart_contract && amount == amount
        );
    });
}

#[test]
fn cleanup_expired_entries_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        initialize();

        // Advance over to the Build&Earn subperiod
        advance_to_next_subperiod();
        assert_eq!(
            ActiveProtocolState::<Test>::get().subperiod(),
            Subperiod::BuildAndEarn,
            "Sanity check."
        );

        // Register a dApp and stake on it
        let staker_h160 = ALICE;
        let smart_contract_address = [0xAF; 32];
        let smart_contract = <Test as pallet_dapp_staking_v3::Config>::SmartContract::wasm(
            smart_contract_address.into(),
        );
        let amount = 1234;
        register_and_stake(staker_h160, smart_contract.clone(), amount);

        // Advance over to the next period so the entry for dApp becomes expired
        advance_to_next_period();

        // Cleanup single expired entry and verify event
        System::reset_events();
        precompiles()
            .prepare_test(
                staker_h160,
                precompile_address(),
                PrecompileCall::cleanup_expired_entries {},
            )
            .expect_no_logs()
            .execute_returns(true);

        let events = dapp_staking_events();
        assert_eq!(events.len(), 1);
        assert_matches!(
            events[0].clone(),
            pallet_dapp_staking_v3::Event::ExpiredEntriesRemoved { count, .. } if count == 1
        );
    });
}
