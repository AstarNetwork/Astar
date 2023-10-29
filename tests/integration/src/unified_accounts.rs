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
use astar_primitives::evm::UnifiedAddress;
use parity_scale_codec::Encode;
use sp_io::hashing::keccak_256;

const AU_CE_GETTER: &'static str = "au_ce_getters";

#[test]
fn transfer_to_h160_via_lookup() {
    new_test_ext().execute_with(|| {
        let eth_address = H160::from_slice(&keccak_256(b"Alice")[0..20]);

        // make sure account is empty
        assert!(EVM::is_account_empty(&eth_address));

        // tranfer to evm account
        assert_ok!(Balances::transfer(
            RuntimeOrigin::signed(ALICE),
            MultiAddress::Address20(eth_address.clone().into()),
            UNIT,
        ));

        // evm account should have recieved the funds
        let (account, _) = EVM::account_basic(&eth_address);
        assert_eq!(account.balance, (UNIT - ExistentialDeposit::get()).into());
    });
}

#[test]
fn unified_accounts_chain_extension_works() {
    const GET_H160: [u8; 4] = [0x00, 0x00, 0x00, 0x2a];
    const GET_H160_OR_DEFAULT: [u8; 4] = [0x00, 0x00, 0x00, 0x2b];
    const GET_NATIVE: [u8; 4] = [0x00, 0x00, 0x00, 0x2c];
    const GET_NATIVE_OR_DEFAULT: [u8; 4] = [0x00, 0x00, 0x00, 0x2d];

    new_test_ext().execute_with(|| {
        let contract_id = deploy_wasm_contract(AU_CE_GETTER);

        // mapped h160 address should None
        assert_eq!(
            call_wasm_contract_method::<Option<H160>>(
                ALICE,
                contract_id.clone(),
                [GET_H160.to_vec(), ALICE.encode()].concat()
            ),
            None
        );
        // default h160 address should match
        assert_eq!(
            call_wasm_contract_method::<UnifiedAddress<H160>>(
                ALICE,
                contract_id.clone(),
                [GET_H160_OR_DEFAULT.to_vec(), ALICE.encode()].concat()
            ),
            UnifiedAccounts::to_h160_or_default(&ALICE)
        );
        // mapped native address should be None
        assert_eq!(
            call_wasm_contract_method::<Option<AccountId>>(
                ALICE,
                contract_id.clone(),
                [GET_NATIVE.to_vec(), alith().encode()].concat()
            ),
            None
        );
        // default native address should match
        assert_eq!(
            call_wasm_contract_method::<UnifiedAddress<AccountId>>(
                ALICE,
                contract_id.clone(),
                [GET_NATIVE_OR_DEFAULT.to_vec(), alith().encode()].concat()
            ),
            UnifiedAccounts::to_account_id_or_default(&alith())
        );

        //
        // Create account mappings
        //
        connect_accounts(&ALICE, &alith_secret_key());

        // ALICE mapped h160 address should be alith
        assert_eq!(
            call_wasm_contract_method::<Option<H160>>(
                ALICE,
                contract_id.clone(),
                [GET_H160.to_vec(), ALICE.encode()].concat()
            ),
            Some(alith())
        );

        // alith mapped native address should ALICE
        assert_eq!(
            call_wasm_contract_method::<Option<AccountId>>(
                ALICE,
                contract_id.clone(),
                [GET_NATIVE.to_vec(), alith().encode()].concat()
            ),
            Some(ALICE)
        );
    });
}
