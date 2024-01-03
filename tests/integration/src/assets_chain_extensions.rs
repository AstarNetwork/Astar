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
use parity_scale_codec::Encode;

const ASSETS_CE: &'static str = "pallet_assets_extension";

const TRANSFER: [u8; 4] = [0x84, 0xa1, 0x5d, 0xa1];
const TRANSFER_APPROVED: [u8; 4] = [0x31, 0x05, 0x59, 0x75];
const MINT: [u8; 4] = [0xcf, 0xdd, 0x9a, 0xa2];
const BURN: [u8; 4] = [0xb1, 0xef, 0xc1, 0x7b];
const APPROVE_TRANSFER: [u8; 4] = [0x8e, 0x7c, 0x3e, 0xe9];
const BALANCE_OF: [u8; 4] = [0x0f, 0x75, 0x5a, 0x56];
const ALLOWANCE: [u8; 4] = [0x6a, 0x00, 0x16, 0x5e];
const METADATA_NAME: [u8; 4] = [0xf5, 0xcd, 0xdb, 0xc1];
const METADATA_SYMBOL: [u8; 4] = [0x7c, 0xdc, 0xaf, 0xc1];
const METADATA_DECIMALS: [u8; 4] = [0x25, 0x54, 0x47, 0x3b];
const TOTAL_SUPPLY: [u8; 4] = [0xdb, 0x63, 0x75, 0xa8];
const MINIMUM_BALANCE: [u8; 4] = [0x1a, 0xa4, 0x88, 0x63];

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
                    MINT.to_vec(),
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
                [BALANCE_OF.to_vec(), ASSET_ID.encode(), contract_id.encode()].concat()
            ),
            1000u128
        );

        // Transfer 100 from contract to Alice
        assert_eq!(
            call_wasm_contract_method::<Result<(), ()>>(
                ALICE,
                contract_id.clone(),
                [
                    TRANSFER.to_vec(),
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
                [BALANCE_OF.to_vec(), ASSET_ID.encode(), ALICE.encode()].concat()
            ),
            100u128
        );

        // Contract burn 50 tokens
        assert_eq!(
            call_wasm_contract_method::<Result<(), ()>>(
                ALICE,
                contract_id.clone(),
                [
                    BURN.to_vec(),
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
                [BALANCE_OF.to_vec(), ASSET_ID.encode(), contract_id.encode()].concat()
            ),
            850u128
        );

        // Check that total supply has been reduced to 950
        assert_eq!(
            call_wasm_contract_method::<u128>(
                ALICE,
                contract_id.clone(),
                [TOTAL_SUPPLY.to_vec(), ASSET_ID.encode()].concat()
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
                    APPROVE_TRANSFER.to_vec(),
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
                    ALLOWANCE.to_vec(),
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
                [METADATA_DECIMALS.to_vec(), ASSET_ID.encode()].concat()
            ),
            1u8
        );

        assert_eq!(
            call_wasm_contract_method::<Vec<u8>>(
                ALICE,
                contract_id.clone(),
                [METADATA_NAME.to_vec(), ASSET_ID.encode()].concat()
            ),
            "Token".encode()
        );

        assert_eq!(
            call_wasm_contract_method::<Vec<u8>>(
                ALICE,
                contract_id.clone(),
                [METADATA_SYMBOL.to_vec(), ASSET_ID.encode()].concat()
            ),
            "TKN".encode()
        );

        // Check Minimum Balance
        assert_eq!(
            call_wasm_contract_method::<u128>(
                ALICE,
                contract_id.clone(),
                [MINIMUM_BALANCE.to_vec(), ASSET_ID.encode()].concat()
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
                    TRANSFER_APPROVED.to_vec(),
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
                [BALANCE_OF.to_vec(), ASSET_ID.encode(), contract_id.encode()].concat()
            ),
            100u128
        );

        assert_eq!(
            call_wasm_contract_method::<u128>(
                ALICE,
                contract_id.clone(),
                [BALANCE_OF.to_vec(), ASSET_ID.encode(), ALICE.encode()].concat()
            ),
            900u128
        );
    });
}
