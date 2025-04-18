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

use crate::setup::*;
use astar_primitives::dapp_staking::SmartContractHandle;
use pallet_dapp_staking::ForcingType;
use parity_scale_codec::Decode;

#[test]
fn test_utility_call_pass_for_any() {
    new_test_ext().execute_with(|| {
        // Any proxy should be allowed to make balance transfer call
        assert_ok!(Proxy::add_proxy(
            RuntimeOrigin::signed(ALICE),
            MultiAddress::Id(BOB),
            ProxyType::Any,
            0
        ));

        // Preparing Utility call
        let transfer_call = RuntimeCall::Balances(BalancesCall::transfer_allow_death {
            dest: MultiAddress::Id(CAT),
            value: 100_000_000_000,
        });
        let inner = Box::new(transfer_call);
        let call = Box::new(RuntimeCall::Utility(UtilityCall::batch {
            calls: vec![*inner],
        }));

        // Utility call passed through filter
        assert_ok!(Proxy::proxy(
            RuntimeOrigin::signed(BOB),
            MultiAddress::Id(ALICE),
            None,
            call.clone()
        ));
        expect_events(vec![
            UtilityEvent::BatchCompleted.into(),
            ProxyEvent::ProxyExecuted { result: Ok(()) }.into(),
        ]);
    });
}

#[test]
fn test_utility_call_pass_for_balances() {
    new_test_ext().execute_with(|| {
        // Balances proxy should be allowed to make balance transfer call
        assert_ok!(Proxy::add_proxy(
            RuntimeOrigin::signed(ALICE),
            MultiAddress::Id(BOB),
            ProxyType::Balances,
            0
        ));

        // Preparing Utility call
        let transfer_call = RuntimeCall::Balances(BalancesCall::transfer_allow_death {
            dest: MultiAddress::Id(CAT),
            value: 100_000_000_000,
        });
        let inner = Box::new(transfer_call);
        let call = Box::new(RuntimeCall::Utility(UtilityCall::batch {
            calls: vec![*inner],
        }));

        // Utility call passed through filter
        assert_ok!(Proxy::proxy(
            RuntimeOrigin::signed(BOB),
            MultiAddress::Id(ALICE),
            None,
            call.clone()
        ));
        expect_events(vec![
            UtilityEvent::BatchCompleted.into(),
            ProxyEvent::ProxyExecuted { result: Ok(()) }.into(),
        ]);
    });
}

#[test]
fn test_utility_call_fail_non_transfer() {
    new_test_ext().execute_with(|| {
        // NonTransfer proxy shouldn't be allowed to make balance transfer call
        assert_ok!(Proxy::add_proxy(
            RuntimeOrigin::signed(ALICE),
            MultiAddress::Id(BOB),
            ProxyType::NonTransfer,
            0
        ));

        // Preparing Utility call
        let transfer_call = RuntimeCall::Balances(BalancesCall::transfer_allow_death {
            dest: MultiAddress::Id(CAT),
            value: 100_000_000_000,
        });
        let inner = Box::new(transfer_call);
        let call = Box::new(RuntimeCall::Utility(UtilityCall::batch {
            calls: vec![*inner],
        }));

        assert_ok!(Proxy::proxy(
            RuntimeOrigin::signed(BOB),
            MultiAddress::Id(ALICE),
            None,
            call.clone()
        ));

        // Utility call filtered out
        expect_events(vec![
            UtilityEvent::BatchInterrupted {
                index: 0,
                error: SystemError::CallFiltered.into(),
            }
            .into(),
            ProxyEvent::ProxyExecuted { result: Ok(()) }.into(),
        ]);
    });
}

#[test]
fn test_utility_call_fail_for_dappstaking() {
    new_test_ext().execute_with(|| {
        // Dappstaking proxy shouldn't be allowed to make balance transfer call
        assert_ok!(Proxy::add_proxy(
            RuntimeOrigin::signed(ALICE),
            MultiAddress::Id(BOB),
            ProxyType::DappStaking,
            0
        ));

        // Preparing Utility call
        let transfer_call = RuntimeCall::Balances(BalancesCall::transfer_allow_death {
            dest: MultiAddress::Id(CAT),
            value: 100_000_000_000,
        });
        let inner = Box::new(transfer_call);
        let call = Box::new(RuntimeCall::Utility(UtilityCall::batch {
            calls: vec![*inner],
        }));

        assert_ok!(Proxy::proxy(
            RuntimeOrigin::signed(BOB),
            MultiAddress::Id(ALICE),
            None,
            call.clone()
        ));
        // Utility call filtered out
        expect_events(vec![
            UtilityEvent::BatchInterrupted {
                index: 0,
                error: SystemError::CallFiltered.into(),
            }
            .into(),
            ProxyEvent::ProxyExecuted { result: Ok(()) }.into(),
        ]);
    });
}

#[test]
fn test_staker_reward_claim_proxy_works() {
    new_test_ext().execute_with(|| {
        // Make CAT delegate for StakerRewardClaim proxy
        assert_ok!(Proxy::add_proxy(
            RuntimeOrigin::signed(BOB),
            MultiAddress::Id(CAT),
            ProxyType::StakerRewardClaim,
            0
        ));

        let contract =
            <Runtime as pallet_dapp_staking::Config>::SmartContract::evm(H160::repeat_byte(0x01));
        let staker_reward_claim_call =
            RuntimeCall::DappStaking(DappStakingCall::Call::claim_staker_rewards {});
        let call = Box::new(staker_reward_claim_call);

        // contract must be registered
        assert_ok!(DappStaking::register(
            RuntimeOrigin::root(),
            ALICE.clone(),
            contract.clone()
        ));

        // some amount must be locked&staked
        let amount = 600 * UNIT;
        assert_ok!(DappStaking::lock(RuntimeOrigin::signed(BOB), amount));
        assert_ok!(DappStaking::stake(
            RuntimeOrigin::signed(BOB),
            contract.clone(),
            amount,
        ));

        // Generate some rewards
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_to_block(System::block_number() + 1);
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_to_block(System::block_number() + 1);

        // CAT making proxy call on behalf of staker (BOB)
        assert_ok!(Proxy::proxy(
            RuntimeOrigin::signed(CAT),
            MultiAddress::Id(BOB),
            None,
            call.clone()
        ));

        expect_events(vec![ProxyEvent::ProxyExecuted { result: Ok(()) }.into()]);
    })
}

#[test]
fn test_set_keys_pass_for_non_transfer_proxy() {
    new_test_ext().execute_with(|| {
        assert_ok!(Proxy::add_proxy(
            RuntimeOrigin::signed(ALICE),
            MultiAddress::Id(BOB),
            ProxyType::NonTransfer,
            0
        ));

        let keys = [0u8; 128];
        let set_keys_call = Box::new(RuntimeCall::Session(SessionCall::set_keys {
            keys: Decode::decode(&mut &keys[..]).unwrap(),
            proof: Vec::new(),
        }));

        assert_ok!(Proxy::proxy(
            RuntimeOrigin::signed(BOB),
            MultiAddress::Id(ALICE),
            None,
            set_keys_call
        ));
        expect_events(vec![ProxyEvent::ProxyExecuted { result: Ok(()) }.into()]);
    });
}
