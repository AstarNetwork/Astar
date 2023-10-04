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
use frame_support::{assert_noop, assert_ok};
use mock::*;

use ethers::{
    contract::{Eip712, EthAbiType},
    core::types::{transaction::eip712::Eip712, Bytes},
};
use parity_scale_codec::Encode;
use sp_runtime::{traits::StaticLookup, AccountId32, MultiAddress};

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

/// Build the signature payload for given native account and eth private key
fn get_evm_signature(who: &AccountId32, secret: &libsecp256k1::SecretKey) -> [u8; 65] {
    // sign the payload
    UnifiedAccounts::eth_sign_prehash(
        &Claim {
            substrate_address: who.encode().into(),
        }
        .encode_eip712()
        .unwrap(),
        secret,
    )
}

/// Create the mappings for the accounts
fn connect_accounts(who: &AccountId32, secret: &libsecp256k1::SecretKey) {
    assert_ok!(UnifiedAccounts::claim_evm_address(
        RuntimeOrigin::signed(who.clone()),
        UnifiedAccounts::eth_address(secret),
        get_evm_signature(who, secret)
    ));
}

#[test]
fn eip712_signature_verify_works() {
    ExtBuilder::default().build().execute_with(|| {
        let claim = Claim {
            substrate_address: ALICE.encode().into(),
        };

        let claim_hash = UnifiedAccounts::build_signing_payload(&ALICE);
        // assert signing payload is correct
        assert_eq!(
            claim.encode_eip712().unwrap(),
            claim_hash,
            "signing payload should match"
        );

        // sign the payload
        let sig = UnifiedAccounts::eth_sign_prehash(&claim_hash, &alice_secret());
        assert_eq!(
            Some(UnifiedAccounts::eth_address(&alice_secret())),
            UnifiedAccounts::verify_signature(&ALICE, &sig),
            "signature verification should work"
        );
    });
}

#[test]
fn static_lookup_works() {
    ExtBuilder::default().build().execute_with(|| {
        let alice_eth = UnifiedAccounts::eth_address(&alice_secret());
        let bob_eth = UnifiedAccounts::eth_address(&bob_secret());
        let bob_default_account_id =
            <UnifiedAccounts as UnifiedAddressMapper<_>>::to_default_account_id(&bob_eth);

        // create mappings for alice
        connect_accounts(&ALICE, &alice_secret());

        // mapping should work if available
        assert_eq!(
            <UnifiedAccounts as StaticLookup>::lookup(MultiAddress::Address20(alice_eth.into()))
                .unwrap(),
            ALICE
        );

        // should use default if not mapping
        assert_eq!(
            <UnifiedAccounts as StaticLookup>::lookup(MultiAddress::Address20(bob_eth.into()))
                .unwrap(),
            bob_default_account_id
        );
    });
}

#[test]
fn on_killed_account_hook() {
    ExtBuilder::default().build().execute_with(|| {
        let alice_eth = UnifiedAccounts::eth_address(&alice_secret());

        // create the mappings
        connect_accounts(&ALICE, &alice_secret());

        // kill alice by transfering everything to bob
        Balances::set_balance(&ALICE, 0);

        // check killed account events
        assert!(System::events().iter().any(|r| matches!(
            &r.event,
            RuntimeEvent::System(frame_system::Event::KilledAccount { account }) if account == &ALICE
        )));

        // make sure mapping is removed
        assert_eq!(NativeToEvm::<TestRuntime>::get(ALICE), None);
        assert_eq!(EvmToNative::<TestRuntime>::get(alice_eth), None);
    });
}

#[test]
fn account_claim_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let alice_eth = UnifiedAccounts::eth_address(&alice_secret());
        // default ss58 account associated with eth address
        let alice_eth_old_account =  <TestRuntime as Config>::DefaultEvmToNative::into_account_id(alice_eth.clone());
        let signature = get_evm_signature(&ALICE, &alice_secret());

        // transfer some funds to alice_eth (H160)
        assert_ok!(Balances::transfer_allow_death(
            RuntimeOrigin::signed(BOB),
            alice_eth_old_account.clone().into(),
            1001
        ));

        // claim the account
        assert_ok!(UnifiedAccounts::claim_evm_address(
            RuntimeOrigin::signed(ALICE),
            alice_eth,
            signature
        ));

        // check if all of balances is transfered to new account (ALICE) from
        // old account (alice_eth_old_account)
        assert!(System::events().iter().any(|r| matches!(
            &r.event,
            RuntimeEvent::System(frame_system::Event::KilledAccount { account }) if account == &alice_eth_old_account
        )));

        // check for claim account event
        System::assert_last_event(
            RuntimeEvent::UnifiedAccounts(crate::Event::AccountClaimed { account_id: ALICE.clone(), evm_address: alice_eth.clone()})
        );

        // make sure mappings are in place
        assert_eq!(
			EvmToNative::<TestRuntime>::get(alice_eth).unwrap(), ALICE
		);
        assert_eq!(
            NativeToEvm::<TestRuntime>::get(ALICE).unwrap(), alice_eth
        )
    });
}

#[test]
fn account_default_claim_works() {
    ExtBuilder::default().build().execute_with(|| {
        let alice_default_evm =
            <TestRuntime as Config>::DefaultNativeToEvm::into_h160(ALICE.into());

        // claim default account
        assert_ok!(UnifiedAccounts::claim_default_evm_address(
            RuntimeOrigin::signed(ALICE)
        ));
        System::assert_last_event(RuntimeEvent::UnifiedAccounts(
            crate::Event::AccountClaimed {
                account_id: ALICE.clone(),
                evm_address: alice_default_evm.clone(),
            },
        ));

        // check UnifiedAddressMapper's mapping works
        assert_eq!(
            <UnifiedAccounts as UnifiedAddressMapper<_>>::to_h160(&ALICE),
            Some(alice_default_evm)
        );
        assert_eq!(
            <UnifiedAccounts as UnifiedAddressMapper<_>>::to_account_id(&alice_default_evm),
            Some(ALICE)
        );

        // should not allow to claim afterwards
        assert_noop!(
            UnifiedAccounts::claim_evm_address(
                RuntimeOrigin::signed(ALICE),
                UnifiedAccounts::eth_address(&alice_secret()),
                get_evm_signature(&ALICE, &alice_secret())
            ),
            Error::<TestRuntime>::AlreadyMapped
        );
    });
}

#[test]
fn account_default_claim_should_not_work_if_collision() {
    ExtBuilder::default().build().execute_with(|| {
        let bob_default_h160 = <UnifiedAccounts as UnifiedAddressMapper<_>>::to_default_h160(&BOB);

        // create mapping of alice native with bob's default address
        // in real world possibilty of this happening is minuscule
        EvmToNative::<TestRuntime>::insert(&bob_default_h160, &ALICE);
        NativeToEvm::<TestRuntime>::insert(&ALICE, &bob_default_h160);

        // bob try claiming default h160 address, it should fail since alice already
        // has mapping in place with it.
        assert_noop!(
            UnifiedAccounts::claim_default_evm_address(RuntimeOrigin::signed(BOB)),
            Error::<TestRuntime>::AlreadyMapped
        );

        // check mappings are consistent
        assert_eq!(
            <UnifiedAccounts as UnifiedAddressMapper<_>>::to_h160(&ALICE),
            Some(bob_default_h160)
        );
        assert_eq!(
            <UnifiedAccounts as UnifiedAddressMapper<_>>::to_account_id(&bob_default_h160),
            Some(ALICE)
        );
    });
}

#[test]
fn replay_attack_should_not_be_possible() {
    ExtBuilder::default().build().execute_with(|| {
        let alice_eth = UnifiedAccounts::eth_address(&alice_secret());
        let alice_signature = get_evm_signature(&ALICE, &alice_secret());

        // alice claim her eth address first
        assert_ok!(UnifiedAccounts::claim_evm_address(
            RuntimeOrigin::signed(ALICE),
            alice_eth,
            alice_signature
        ));

        // bob intercepted alice signature and tries to perform
        // replay attack to claim alice eth address as his own,
        // this should fail.
        assert_noop!(
            UnifiedAccounts::claim_evm_address(
                RuntimeOrigin::signed(BOB),
                alice_eth,
                alice_signature
            ),
            Error::<TestRuntime>::AlreadyMapped
        );
    });
}

#[test]
fn frontrun_attack_should_not_be_possible() {
    ExtBuilder::default().build().execute_with(|| {
        let alice_eth = UnifiedAccounts::eth_address(&alice_secret());
        let alice_signature = get_evm_signature(&ALICE, &alice_secret());

        // bob intercepted alice signature and tries to perform
        // frontrun attack to claim alice eth address as his own
        // this should fail with InvalidSignature.
        assert_noop!(
            UnifiedAccounts::claim_evm_address(
                RuntimeOrigin::signed(BOB),
                alice_eth,
                alice_signature
            ),
            Error::<TestRuntime>::InvalidSignature
        );

        // alice can claim her eth address
        assert_ok!(UnifiedAccounts::claim_evm_address(
            RuntimeOrigin::signed(ALICE),
            alice_eth,
            alice_signature
        ));
    });
}

#[test]
fn connecting_mapped_accounts_should_not_work() {
    ExtBuilder::default().build().execute_with(|| {
        // connect ALICE accounts
        connect_accounts(&ALICE, &alice_secret());

        // AccountId already mapped
        // ALICE attempts to connect another evm address
        assert_noop!(
            UnifiedAccounts::claim_evm_address(
                RuntimeOrigin::signed(ALICE),
                UnifiedAccounts::eth_address(&bob_secret()),
                get_evm_signature(&BOB, &bob_secret())
            ),
            Error::<TestRuntime>::AlreadyMapped
        );

        // eth address already mapped
        // BOB attempts to connect alice_eth that is already mapped
        assert_noop!(
            UnifiedAccounts::claim_evm_address(
                RuntimeOrigin::signed(BOB),
                UnifiedAccounts::eth_address(&alice_secret()),
                get_evm_signature(&ALICE, &alice_secret())
            ),
            Error::<TestRuntime>::AlreadyMapped
        );
    });
}
