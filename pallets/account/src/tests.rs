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

#![cfg(test)]

use super::*;
use mock::*;

use ethers::{
    contract::{Eip712, EthAbiType},
    core::types::{transaction::eip712::Eip712, Bytes},
};
use parity_scale_codec::Encode;
use sp_runtime::{traits::StaticLookup, MultiAddress};

#[test]
fn eip712_signature_verify_works() {
    /// EIP712 Payload struct
    #[derive(Eip712, EthAbiType, Clone)]
    #[eip712(
        name = "Astar EVM Claim",
        version = "1",
        chain_id = 1024,
        // mock genisis hash
        raw_salt = "0x4545454545454545454545454545454545454545454545454545454545454545"
    )]
    struct Claim {
        substrate_address: Bytes,
    }

    ExtBuilder::default().build().execute_with(|| {
        let claim = Claim {
            substrate_address: ALICE.encode().into(),
        };

        let claim_hash = EIP712Signature::<TestRuntime>::build_signing_payload(&ALICE);
        // assert signing payload is correct
        assert_eq!(
            claim.encode_eip712().unwrap(),
            claim_hash,
            "signing payload should match"
        );

        // sign the payload
        let sig = Accounts::eth_sign_prehash(&claim_hash, &alice_secret());
        assert_eq!(
            Some(Accounts::eth_address(&alice_secret())),
            EIP712Signature::<TestRuntime>::verify_signature(&ALICE, &sig),
            "signature verification should work"
        );
    });
}

#[test]
fn static_lookup_works() {
    ExtBuilder::with_alice_mapping().execute_with(|| {
        let alice_eth = Accounts::eth_address(&alice_secret());
        let bob_eth = Accounts::eth_address(&bob_secret());
        let bob_default_account_id =
            <Accounts as AddressManager<_, _>>::to_default_account_id(&bob_eth);

        // mapping should work if available
        assert_eq!(
            <Accounts as StaticLookup>::lookup(MultiAddress::Address20(alice_eth.into())).unwrap(),
            ALICE
        );

        // should use default if not mapping
        assert_eq!(
            <Accounts as StaticLookup>::lookup(MultiAddress::Address20(bob_eth.into())).unwrap(),
            bob_default_account_id
        );
    });
}

#[test]
fn on_killed_account_hook() {
    ExtBuilder::with_alice_mapping().execute_with(|| {
        let alice_eth = Accounts::eth_address(&alice_secret());

        // kill alice by transfering everything to bob
        Balances::set_balance(&ALICE, 0);

        // check killed account events
        assert!(System::events().iter().any(|r| matches!(
            &r.event,
            RuntimeEvent::System(frame_system::Event::KilledAccount { account }) if account == &ALICE
        )));

        // make sure mapping is removed
        assert_eq!(EvmAccounts::<TestRuntime>::get(ALICE), None);
        assert_eq!(NativeAccounts::<TestRuntime>::get(alice_eth), None);
    });
}

#[test]
fn account_claim_correct_signature_should_work() {}

#[test]
fn account_claim_wrong_signature_should_not_work() {}

#[test]
fn account_default_claim_works() {}
