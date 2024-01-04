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

use crate::setup::*;
use assets_chain_extension_types::selector_bytes;
use parity_scale_codec::Encode;
use sp_io::hashing::blake2_256;

const ASSETS_CE: &'static str = "pallet_assets_extension";

const ASSET_ID: u128 = 200;

#[test]
fn mint_transfer_burn_works() {
    new_test_ext().execute_with(|| {
        let contract_id = deploy_wasm_contract(ASSETS_CE);

        assert_ok!(Assets::create(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID.into(),
            ALICE.into(),
            1,
        ));

        // Give to contract mint permission (Issuer role) and burn permission (Admin role)
        assert_ok!(Assets::set_team(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID.into(),
            contract_id.clone().into(),
            contract_id.clone().into(),
            contract_id.clone().into(),
        ));

        // Call mint to mint 1000 to contract
        assert_eq!(
            call_wasm_contract_method::<Result<(), ()>>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("mint").to_vec(),
                    ASSET_ID.encode(),
                    contract_id.encode(),
                    1000u128.encode()
                ]
                .concat()
            ),
            Ok(())
        );

        // Assert contract balance
        assert_eq!(
            call_wasm_contract_method::<u128>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("balance_of").to_vec(),
                    ASSET_ID.encode(),
                    contract_id.encode()
                ]
                .concat()
            ),
            1000u128
        );

        // Transfer 100 from contract to Alice
        assert_eq!(
            call_wasm_contract_method::<Result<(), ()>>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("transfer").to_vec(),
                    ASSET_ID.encode(),
                    ALICE.encode(),
                    100u128.encode()
                ]
                .concat()
            ),
            Ok(())
        );

        // Assert Alice balance
        assert_eq!(
            call_wasm_contract_method::<u128>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("balance_of").to_vec(),
                    ASSET_ID.encode(),
                    ALICE.encode()
                ]
                .concat()
            ),
            100u128
        );

        // Contract burn 50 tokens
        assert_eq!(
            call_wasm_contract_method::<Result<(), ()>>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("burn").to_vec(),
                    ASSET_ID.encode(),
                    contract_id.encode(),
                    50u128.encode()
                ]
                .concat()
            ),
            Ok(())
        );

        // // Check that Balance of Alice is reduced to 850
        assert_eq!(
            call_wasm_contract_method::<u128>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("balance_of").to_vec(),
                    ASSET_ID.encode(),
                    contract_id.encode()
                ]
                .concat()
            ),
            850u128
        );

        // Check that total supply has been reduced to 950
        assert_eq!(
            call_wasm_contract_method::<u128>(
                ALICE,
                contract_id.clone(),
                [selector_bytes!("total_supply").to_vec(), ASSET_ID.encode()].concat()
            ),
            950u128
        );
    });
}

#[test]
fn approve_works() {
    new_test_ext().execute_with(|| {
        let contract_id = deploy_wasm_contract(ASSETS_CE);

        assert_ok!(Assets::create(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID.into(),
            ALICE.into(),
            1,
        ));

        // Contract approve Alice to spend 100 tokens
        // First the contract need Existential DDeposit
        assert_ok!(Balances::transfer(
            RuntimeOrigin::signed(ALICE),
            contract_id.clone().into(),
            UNIT,
        ));

        assert_eq!(
            call_wasm_contract_method::<Result<(), ()>>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("approve_transfer").to_vec(),
                    ASSET_ID.encode(),
                    ALICE.encode(),
                    100u128.encode()
                ]
                .concat()
            ),
            Ok(())
        );

        // Check that Allowance of Alice is 100
        assert_eq!(
            call_wasm_contract_method::<u128>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("allowance").to_vec(),
                    ASSET_ID.encode(),
                    contract_id.encode(),
                    ALICE.encode()
                ]
                .concat()
            ),
            100u128
        );
    });
}

#[test]
fn getters_works() {
    new_test_ext().execute_with(|| {
        let contract_id = deploy_wasm_contract(ASSETS_CE);

        assert_ok!(Assets::create(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID.into(),
            ALICE.into(),
            1,
        ));

        assert_ok!(Assets::set_metadata(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID.into(),
            "Token".encode(),
            "TKN".encode(),
            1
        ));

        assert_eq!(
            call_wasm_contract_method::<u8>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("metadata_decimals").to_vec(),
                    ASSET_ID.encode()
                ]
                .concat()
            ),
            1u8
        );

        assert_eq!(
            call_wasm_contract_method::<Vec<u8>>(
                ALICE,
                contract_id.clone(),
                [selector_bytes!("metadata_name").to_vec(), ASSET_ID.encode()].concat()
            ),
            "Token".encode()
        );

        assert_eq!(
            call_wasm_contract_method::<Vec<u8>>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("metadata_symbol").to_vec(),
                    ASSET_ID.encode()
                ]
                .concat()
            ),
            "TKN".encode()
        );

        // Check Minimum Balance
        assert_eq!(
            call_wasm_contract_method::<u128>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("minimum_balance").to_vec(),
                    ASSET_ID.encode()
                ]
                .concat()
            ),
            1u128
        );
    });
}

#[test]
fn transfer_approved_works() {
    new_test_ext().execute_with(|| {
        let contract_id = deploy_wasm_contract(ASSETS_CE);

        assert_ok!(Assets::create(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID.into(),
            ALICE.into(),
            1,
        ));

        // Mint 1000 tokens to Alice
        assert_ok!(Assets::mint(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID.into(),
            ALICE.into(),
            1000
        ));

        // Alice approve the contract to spend 100 on her behalf
        assert_ok!(Assets::approve_transfer(
            RuntimeOrigin::signed(ALICE),
            ASSET_ID.into(),
            contract_id.clone().into(),
            100
        ));

        // The contract transfer 100 tokens from Alice to itself
        assert_eq!(
            call_wasm_contract_method::<Result<(), ()>>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("transfer_approved").to_vec(),
                    ASSET_ID.encode(),
                    ALICE.encode(),
                    contract_id.encode(),
                    100u128.encode()
                ]
                .concat()
            ),
            Ok(())
        );

        // Check that contract received the 100 and that Alice balance is 900
        assert_eq!(
            call_wasm_contract_method::<u128>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("balance_of").to_vec(),
                    ASSET_ID.encode(),
                    contract_id.encode()
                ]
                .concat()
            ),
            100u128
        );

        assert_eq!(
            call_wasm_contract_method::<u128>(
                ALICE,
                contract_id.clone(),
                [
                    selector_bytes!("balance_of").to_vec(),
                    ASSET_ID.encode(),
                    ALICE.encode()
                ]
                .concat()
            ),
            900u128
        );
    });
}
