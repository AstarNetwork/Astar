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

use crate::mock::*;
use frame_support::assert_ok;
use frame_support::traits::Currency;
use pallet_contracts::{CollectEvents, DebugInfo, Determinism};
use pallet_contracts_primitives::{Code, ExecReturnValue};
use parity_scale_codec::Encode;
use sp_core::crypto::AccountId32;
use sp_runtime::DispatchError;
use std::fs;

// Those tests use the contract scheduler_example available here:
// https://github.com/AstarNetwork/chain-extension-contracts/blob/main/examples/assets
// It maps chain extension functions to ink! callable messages
// ex:
// #[ink(message)]
// pub fn burn(&mut self, asset_id: u128, who: AccountId, amount: Balance) -> Result<(), AssetsError> {
//    AssetsExtension::burn(Origin::Caller, asset_id, who, amount)?;
//     Ok(())
// }

#[test]
fn mint_works() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange - create asset and give contract mint permission (Issuer role)
            assert_ok!(Assets::create(RuntimeOrigin::signed(ALICE), 1, ALICE, 1));
            assert_ok!(Assets::set_team(
                RuntimeOrigin::signed(ALICE),
                1,
                addr.clone(),
                ALICE,
                ALICE
            ));

            // Act - Mint 1000 assets to Alice
            assert_ok!(mint(addr.clone(), 1, ALICE, 1000));

            // Assert - Alice balance is 1000
            assert_eq!(Assets::balance(1, ALICE), 1000);
        });
}

#[test]
fn burn_works() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange - create asset and give contract mint permission (Issuer role) and burn permission (Admin role)
            assert_ok!(Assets::create(RuntimeOrigin::signed(ALICE), 1, ALICE, 1));
            assert_ok!(Assets::set_team(
                RuntimeOrigin::signed(ALICE),
                1,
                addr.clone(),
                addr.clone(),
                ALICE
            ));

            // Act - Mint 1000 assets to Alice
            assert_ok!(mint(addr.clone(), 1, ALICE, 1000));
            assert_eq!(Assets::balance(1, ALICE), 1000);

            // Act - Burn 1000 of Alice tokens
            assert_ok!(burn(addr.clone(), 1, ALICE, 1000));

            // Assert - Balance of Alice is then 0
            assert_eq!(Assets::balance(1, ALICE), 0);
        });
}

#[test]
fn transfer_works() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Assert - Create, mint and transfer 1000 to contract
            assert_ok!(Assets::create(RuntimeOrigin::signed(BOB), 1, BOB, 1));
            assert_ok!(Assets::mint(RuntimeOrigin::signed(BOB), 1, BOB, 1000));
            assert_ok!(Assets::transfer(
                RuntimeOrigin::signed(BOB),
                1,
                addr.clone(),
                1000
            ));

            // Act - Transfer 1000 from contract to Alice
            assert_ok!(transfer(addr.clone(), 1, ALICE, 1000));

            // Assert - Alice balance is 1000 and contract is zero
            assert_eq!(Assets::balance(1, ALICE), 1000);
            assert_eq!(Assets::balance(1, addr.clone()), 0);
        });
}

#[test]
fn balance_of_and_total_supply() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange - create & mint 1000 to Alice
            assert_ok!(Assets::create(RuntimeOrigin::signed(ALICE), 1, ALICE, 1));
            assert_ok!(Assets::mint(RuntimeOrigin::signed(ALICE), 1, ALICE, 1000));

            // Assert - Balance and total supply is 1000
            assert_eq!(
                balance_of(addr.clone(), 1, ALICE).data[1..],
                1000u128.encode()
            );
            assert_eq!(total_supply(addr.clone(), 1).data[1..], 1000u128.encode());
        });
}

#[test]
fn approve_transfer_and_check_allowance() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange - Create and mint 1000 to contract and fund contract with ED
            assert_ok!(Assets::create(RuntimeOrigin::signed(ALICE), 1, ALICE, 1));
            assert_ok!(Assets::mint(
                RuntimeOrigin::signed(ALICE),
                1,
                addr.clone(),
                1000
            ));
            let _ = Balances::deposit_creating(&addr.clone(), 1);

            // Act - approve transfer To BOB for 100
            assert_ok!(approve_transfer(addr.clone(), 1, BOB, 100));

            // Assert - Bob has 100 allowance
            assert_eq!(
                allowance(addr.clone(), 1, addr.clone(), BOB).data[1..],
                100u128.encode()
            );
        });
}

#[test]
fn approve_transfer_and_transfer_balance() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange
            // As transfer_approved() can only be called on behalf of the contract
            // Bob creates & mint token to himself
            // and approve the contract to spend his assets
            assert_ok!(Assets::create(RuntimeOrigin::signed(BOB), 1, BOB, 1));
            assert_ok!(Assets::mint(RuntimeOrigin::signed(BOB), 1, BOB, 1000));
            assert_ok!(Assets::approve_transfer(
                RuntimeOrigin::signed(BOB),
                1,
                addr.clone(),
                100
            ));

            // Act - The contract transfer 100 from Bob to Alice
            assert_ok!(transfer_approved(addr.clone(), 1, BOB, ALICE, 100));

            // Assert - Bob has 900 and Alice 100
            assert_eq!(Assets::balance(1, BOB), 900u128);
            assert_eq!(Assets::balance(1, ALICE), 100u128);
        });
}

#[test]
fn getters_works() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange
            // Alice creates & mint token
            assert_ok!(Assets::create(RuntimeOrigin::signed(ALICE), 1, ALICE, 1));
            assert_ok!(Assets::mint(RuntimeOrigin::signed(ALICE), 1, ALICE, 1000));
            assert_ok!(Assets::approve_transfer(
                RuntimeOrigin::signed(ALICE),
                1,
                BOB,
                100
            ));
            assert_ok!(Assets::set_metadata(
                RuntimeOrigin::signed(ALICE),
                1,
                "Token".as_bytes().to_vec(),
                "TKN".as_bytes().to_vec(),
                1
            ));

            // Assert - verify state using chain extension getters
            assert_eq!(
                allowance(addr.clone(), 1, ALICE, BOB).data[1..],
                100u128.encode()
            );
            assert_eq!(
                balance_of(addr.clone(), 1, ALICE).data[1..],
                1000u128.encode()
            );
            assert_eq!(total_supply(addr.clone(), 1).data[1..], 1000u128.encode());
            assert_eq!(metadata_decimals(addr.clone(), 1).data[1..], [1u8]);
            assert_eq!(metadata_name(addr.clone(), 1).data[1..], "Token".encode());
            assert_eq!(metadata_symbol(addr.clone(), 1).data[1..], "TKN".encode());
            assert_eq!(minimum_balance(addr.clone(), 1).data[1..], 1u128.encode());
        });
}

fn instantiate() -> AccountId32 {
    let code = fs::read("../../tests/ink-contracts/pallet_assets_extension.wasm")
        .expect("could not read .wasm file");
    let _ = Balances::deposit_creating(&ALICE, ONE * 1000);
    let _ = Balances::deposit_creating(&BOB, ONE * 1000);
    let instance_selector: Vec<u8> = [0x9b, 0xae, 0x9d, 0x5e].to_vec();
    Contracts::bare_instantiate(
        ALICE,
        0,
        GAS_LIMIT,
        Some(ONE),
        Code::Upload(code),
        instance_selector,
        vec![],
        DebugInfo::Skip,
        CollectEvents::UnsafeCollect,
    )
    .result
    .unwrap()
    .account_id
}

fn transfer(
    addr: AccountId32,
    asset_id: u128,
    target: AccountId32,
    amount: u128,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0x84, 0xa1, 0x5d, 0xa1].to_vec(),
        (asset_id, target, amount).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0)
}

fn transfer_approved(
    addr: AccountId32,
    asset_id: u128,
    owner: AccountId32,
    dest: AccountId32,
    amount: u128,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0x31, 0x05, 0x59, 0x75].to_vec(),
        (asset_id, owner, dest, amount).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0)
}

fn mint(
    addr: AccountId32,
    asset_id: u128,
    beneficiary: AccountId32,
    amount: u128,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0xcf, 0xdd, 0x9a, 0xa2].to_vec(),
        (asset_id, beneficiary, amount).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0)
}

fn burn(
    addr: AccountId32,
    asset_id: u128,
    who: AccountId32,
    amount: u128,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0xb1, 0xef, 0xc1, 0x7b].to_vec(),
        (asset_id, who, amount).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0)
}

fn approve_transfer(
    addr: AccountId32,
    asset_id: u128,
    delegate: AccountId32,
    amount: u128,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0x8e, 0x7c, 0x3e, 0xe9].to_vec(),
        (asset_id, delegate, amount).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0)
}

fn balance_of(addr: AccountId32, asset_id: u128, who: AccountId32) -> ExecReturnValue {
    let data = [[0x0f, 0x75, 0x5a, 0x56].to_vec(), (asset_id, who).encode()].concat();
    do_bare_call(addr, data, 0).unwrap()
}

fn allowance(
    addr: AccountId32,
    asset_id: u128,
    owner: AccountId32,
    delegate: AccountId32,
) -> ExecReturnValue {
    let data = [
        [0x6a, 0x00, 0x16, 0x5e].to_vec(),
        (asset_id, owner, delegate).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0).unwrap()
}

fn metadata_name(addr: AccountId32, asset_id: u128) -> ExecReturnValue {
    let data = [[0xf5, 0xcd, 0xdb, 0xc1].to_vec(), asset_id.encode()].concat();
    do_bare_call(addr, data, 0).unwrap()
}

fn metadata_symbol(addr: AccountId32, asset_id: u128) -> ExecReturnValue {
    let data = [[0x7c, 0xdc, 0xaf, 0xc1].to_vec(), asset_id.encode()].concat();
    do_bare_call(addr, data, 0).unwrap()
}

fn metadata_decimals(addr: AccountId32, asset_id: u128) -> ExecReturnValue {
    let data = [[0x25, 0x54, 0x47, 0x3b].to_vec(), asset_id.encode()].concat();
    do_bare_call(addr, data, 0).unwrap()
}

fn total_supply(addr: AccountId32, asset_id: u128) -> ExecReturnValue {
    let data = [[0xdb, 0x63, 0x75, 0xa8].to_vec(), asset_id.encode()].concat();
    do_bare_call(addr, data, 0).unwrap()
}

fn minimum_balance(addr: AccountId32, asset_id: u128) -> ExecReturnValue {
    let data = [[0x1a, 0xa4, 0x88, 0x63].to_vec(), asset_id.encode()].concat();
    do_bare_call(addr, data, 0).unwrap()
}

fn do_bare_call(
    addr: AccountId32,
    input: Vec<u8>,
    value: u128,
) -> Result<ExecReturnValue, DispatchError> {
    Contracts::bare_call(
        ALICE,
        addr.into(),
        value.into(),
        GAS_LIMIT,
        None,
        input,
        DebugInfo::Skip,
        CollectEvents::UnsafeCollect,
        Determinism::Relaxed,
    )
    .result
}
