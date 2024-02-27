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

use core::str::from_utf8;
use frame_support::dispatch::GetDispatchInfo;
use frame_support::traits::Currency;
use libsecp256k1::PublicKeyFormat;
use sp_core::crypto::{AccountId32, Ss58Codec};

use crate::mock::*;

use astar_primitives::evm::EvmAddress;
use hex_literal::hex;
use parity_scale_codec::Encode;
use precompile_utils::testing::*;
use sp_core::{ecdsa, Pair};

fn precompiles() -> TestPrecompileSet<TestRuntime> {
    PrecompilesValue::get()
}

#[test]
fn dispatch_calls_on_behalf_of_lockdrop_works() {
    ExtBuilder::default().build().execute_with(|| {
        // Transfer balance to Alice
        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
            dest: ALICE,
            value: 15 * ONE,
        });
        // Sanity check - Alice holds no Balance
        assert_eq!(Balances::free_balance(ALICE), 0);

        // Get Alice EVM address based on the Public Key
        let alice_eth = crate::tests::eth_address(&alice_secret());
        // Get derived AccountId from the Blake2b hash of the compressed ECDSA Public key
        let account_id = account_id(&alice_secret());
        // Fund this account (fund the lockdrop account)
        let _ = Balances::deposit_creating(&account_id, ONE * 20);
        // Get the full 64 bytes ECDSA Public key
        let pubkey = crate::tests::public_key_full(&alice_secret());

        precompiles()
            .prepare_test(
                alice_eth,
                PRECOMPILE_ADDRESS,
                PrecompileCall::dispatch_lockdrop_call {
                    call: call.encode().into(),
                    pubkey: pubkey.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        // Get Balance of ALICE in pallet balances
        assert_eq!(Balances::free_balance(ALICE), 15 * ONE);
    });
}

#[test]
fn proper_gas_is_charged() {
    ExtBuilder::default().build().execute_with(|| {
        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
            dest: ALICE,
            value: 15 * ONE,
        });

        // Dispatch a call and ensure gas is charged properly
        // Expected gas is the constant weight of 1_000_000_000 and the weight of the call
        // In mock one unit of ref_time us charged 1
        let expected_gas = 1_000_000_000u64 + call.get_dispatch_info().weight.ref_time();

        // Get Alice EVM address based on the Public Key
        let alice_eth = crate::tests::eth_address(&alice_secret());
        // Get derived AccountId from the Blake2b hash of the compressed ECDSA Public key
        let account_id = account_id(&alice_secret());
        // Fund this account (fund the lockdrop account)
        let _ = Balances::deposit_creating(&account_id, ONE * 20);
        // Get the full 64 bytes ECDSA Public key
        let pubkey = crate::tests::public_key_full(&alice_secret());

        precompiles()
            .prepare_test(
                alice_eth,
                PRECOMPILE_ADDRESS,
                PrecompileCall::dispatch_lockdrop_call {
                    call: call.encode().into(),
                    pubkey: pubkey.into(),
                },
            )
            .expect_cost(expected_gas)
            .execute_returns(true);
    });
}

#[test]
fn pubkey_does_not_match_caller_address() {
    ExtBuilder::default().build().execute_with(|| {
        // Transfer balance to Alice
        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
            dest: ALICE,
            value: 15 * ONE,
        });
        // Sanity check - Alice holds no Balance
        assert_eq!(Balances::free_balance(ALICE), 0);

        // Get Alice EVM address based on the Public Key
        let alice_eth = crate::tests::eth_address(&alice_secret());
        // Dummy AccountId to sign the EIP712 payload with
        let account_id = DUMMY;
        // Fund this dummy account
        let _ = Balances::deposit_creating(&account_id, ONE * 20);
        // Create a dummy pubkey
        let pubkey = [10u8; 64];

        precompiles()
            .prepare_test(
                alice_eth,
                PRECOMPILE_ADDRESS,
                PrecompileCall::dispatch_lockdrop_call {
                    call: call.encode().into(),
                    pubkey: pubkey.into(),
                },
            )
            .expect_no_logs()
            .execute_reverts(|output| output == b"caller does not match the public key");

        // Get Balance of ALICE in pallet balances and ensure it has not received any funds
        assert_eq!(Balances::free_balance(ALICE), 0);
    });
}

#[test]
fn pubkey_derive_to_proper_ss58() {
    ExtBuilder::default().build().execute_with(|| {
        // Transfer balance to Alice
        let call = RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
            dest: ALICE,
            value: 15 * ONE,
        });
        // Sanity check - Alice holds no Balance
        assert_eq!(Balances::free_balance(ALICE), 0);

        // The seed "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        // should resolve to the SS58 address "5EGynCAEvv8NLeHx8vDMvb8hTcEcMYUMWCDQEEncNEfNWB2W"
        // If we fund this account, it will be able to dispatch the Transfer call
        let pair = ecdsa::Pair::from_seed(&hex!(
            "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        ));
        let pubkey = crate::tests::public_key_full_from_compressed(pair.public().as_ref());
        let alice_eth = EvmAddress::from_slice(&sp_io::hashing::keccak_256(&pubkey)[12..]);
        let account_id =
            AccountId::from_ss58check("5EGynCAEvv8NLeHx8vDMvb8hTcEcMYUMWCDQEEncNEfNWB2W").unwrap();
        // Fund this account
        let _ = Balances::deposit_creating(&account_id, ONE * 20);

        precompiles()
            .prepare_test(
                alice_eth,
                PRECOMPILE_ADDRESS,
                PrecompileCall::dispatch_lockdrop_call {
                    call: call.encode().into(),
                    pubkey: pubkey.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        // Assert that the call (Transfer) was successful
        assert_eq!(Balances::free_balance(ALICE), 15 * ONE);
    });
}

#[test]
fn decode_limit_too_high() {
    ExtBuilder::default().build().execute_with(|| {
        let mut nested_call =
            RuntimeCall::System(frame_system::Call::remark { remark: Vec::new() });

        // More than 8 depth
        for _ in 0..9 {
            nested_call = RuntimeCall::Utility(pallet_utility::Call::as_derivative {
                index: 0,
                call: Box::new(nested_call),
            });
        }

        // Get Alice EVM address based on the Public Key
        let alice_eth = crate::tests::eth_address(&alice_secret());
        // Get derived AccountId from the Blake2b hash of the compressed ECDSA Public key
        let account_id = account_id(&alice_secret());
        // Fund this account (fund the lockdrop account)
        let _ = Balances::deposit_creating(&account_id, ONE * 20);
        // Get the full 64 bytes ECDSA Public key
        let pubkey = crate::tests::public_key_full(&alice_secret());

        precompiles()
            .prepare_test(
                alice_eth,
                PRECOMPILE_ADDRESS,
                PrecompileCall::dispatch_lockdrop_call {
                    call: nested_call.encode().into(),
                    pubkey: pubkey.into(),
                },
            )
            .expect_no_logs()
            .execute_reverts(|output| from_utf8(output).unwrap().contains("could not decode call"));
    });
}

#[test]
fn decode_limit_ok() {
    ExtBuilder::default().build().execute_with(|| {
        let mut nested_call =
            RuntimeCall::System(frame_system::Call::remark { remark: Vec::new() });

        for _ in 0..8 {
            nested_call = RuntimeCall::Utility(pallet_utility::Call::as_derivative {
                index: 0,
                call: Box::new(nested_call),
            });
        }

        // Get Alice EVM address based on the Public Key
        let alice_eth = crate::tests::eth_address(&alice_secret());
        // Get derived AccountId from the Blake2b hash of the compressed ECDSA Public key
        let account_id = account_id(&alice_secret());
        // Fund this account (fund the lockdrop account)
        let _ = Balances::deposit_creating(&account_id, ONE * 20);
        // Get the full 64 bytes ECDSA Public key
        let pubkey = crate::tests::public_key_full(&alice_secret());

        precompiles()
            .prepare_test(
                alice_eth,
                PRECOMPILE_ADDRESS,
                PrecompileCall::dispatch_lockdrop_call {
                    call: nested_call.encode().into(),
                    pubkey: pubkey.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(true);
    });
}

#[test]
fn only_whitelisted_calls_can_be_dispatched() {
    ExtBuilder::default().build().execute_with(|| {
        // Transfer balance to Alice
        let call = RuntimeCall::System(frame_system::Call::remark_with_event {
            remark: b"Hello World".to_vec(),
        });

        // Get Alice EVM address based on the Public Key
        let alice_eth = crate::tests::eth_address(&alice_secret());
        // Get derived AccountId from the Blake2b hash of the compressed ECDSA Public key
        let account_id = crate::tests::account_id(&alice_secret());
        // Fund this account (fund the lockdrop account)
        let _ = Balances::deposit_creating(&account_id, ONE * 20);
        // Get the full 64 bytes ECDSA Public key
        let pubkey = crate::tests::public_key_full(&alice_secret());

        precompiles()
            .prepare_test(
                alice_eth,
                PRECOMPILE_ADDRESS,
                PrecompileCall::dispatch_lockdrop_call {
                    call: call.encode().into(),
                    pubkey: pubkey.into(),
                },
            )
            .expect_no_logs()
            .execute_reverts(|output| output == b"invalid Call");
    });
}

fn account_id(secret: &libsecp256k1::SecretKey) -> AccountId32 {
    sp_io::hashing::blake2_256(
        ecdsa::Public::from_full(
            &libsecp256k1::PublicKey::from_secret_key(secret).serialize()[1..65],
        )
        .unwrap()
        .as_ref(),
    )
    .into()
}

fn eth_address(secret: &libsecp256k1::SecretKey) -> EvmAddress {
    EvmAddress::from_slice(
        &sp_io::hashing::keccak_256(
            &libsecp256k1::PublicKey::from_secret_key(secret).serialize()[1..65],
        )[12..],
    )
}

fn public_key_full_from_compressed(pubkey: &[u8]) -> [u8; 64] {
    libsecp256k1::PublicKey::parse_slice(pubkey, Some(PublicKeyFormat::Compressed))
        .unwrap()
        .serialize()[1..65]
        .try_into()
        .unwrap()
}

fn public_key_full(secret: &libsecp256k1::SecretKey) -> [u8; 64] {
    libsecp256k1::PublicKey::from_secret_key(secret).serialize()[1..65]
        .try_into()
        .unwrap()
}
