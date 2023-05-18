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

use crate::mocks::{parachain, relay_chain, *};
use frame_support::{assert_ok, weights::Weight};
use xcm::prelude::*;
use xcm_simulator::TestExt;

// TODO: remove this when retiring pallet-xcm fork
#[test]
fn para_to_para_reserve_transfer_and_back_via_pallet_xcm() {
    MockNet::reset();

    let sibling_asset_id = 123 as u128;
    let para_a_multiloc = (Parent, Parachain(1));

    // On parachain B create an asset which representes a derivative of parachain A native asset.
    // This asset is allowed as XCM execution fee payment asset.
    ParaB::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            sibling_asset_id,
            para_a_multiloc.clone(),
            sibling_para_account_id(1),
            Some(true),
            Some(1),
            Some(1_000_000_000_000)
        ));
    });

    // Next step is to send some of parachain A native asset to parachain B.
    let withdraw_amount = 567;
    ParaA::execute_with(|| {
        assert_ok!(ParachainPalletXcm::reserve_transfer_assets(
            parachain::RuntimeOrigin::signed(ALICE),
            Box::new(MultiLocation::new(1, X1(Parachain(2))).into()),
            Box::new(
                X1(AccountId32 {
                    network: None,
                    id: ALICE.into()
                })
                .into_location()
                .into_versioned()
            ),
            Box::new((Here, withdraw_amount).into()),
            0,
        ));

        // Parachain 2 sovereign account should have it's balance increased, while Alice balance should be decreased.
        assert_eq!(
            parachain::Balances::free_balance(&sibling_para_account_id(2)),
            INITIAL_BALANCE + withdraw_amount
        );
        assert_eq!(
            parachain::Balances::free_balance(&ALICE),
            INITIAL_BALANCE - withdraw_amount
        );
    });

    // Parachain B should receive parachain A native assets and should mint their local derivate.
    // Portion of those assets should be taken as the XCM execution fee.
    let four_instructions_execution_cost =
        (parachain::UnitWeightCost::get() * 4).ref_time() as u128;
    let remaining = withdraw_amount - four_instructions_execution_cost;
    ParaB::execute_with(|| {
        // Ensure Alice received assets on ParaB (sent amount minus expenses)
        assert_eq!(
            parachain::Assets::balance(sibling_asset_id, ALICE),
            remaining
        );
    });

    // send assets back to ParaA
    ParaB::execute_with(|| {
        assert_ok!(ParachainPalletXcm::reserve_withdraw_assets(
            parachain::RuntimeOrigin::signed(ALICE),
            Box::new((Parent, Parachain(1)).into()),
            Box::new(
                AccountId32 {
                    network: None,
                    id: ALICE.into()
                }
                .into()
            ),
            Box::new((para_a_multiloc, remaining).into()),
            0
        ));
    });

    ParaA::execute_with(|| {
        // ParaB soveregin account account should have only the execution cost
        assert_eq!(
            parachain::Balances::free_balance(&sibling_para_account_id(2)),
            INITIAL_BALANCE + four_instructions_execution_cost
        );
        // ParaA alice should have initial amount backed subtracted with execution costs
        // which is 2xfour_instructions_execution_cost
        // or withdraw_amount + remaining - four_instructions_execution_cost
        // both are same
        assert_eq!(
            parachain::Balances::free_balance(&ALICE),
            INITIAL_BALANCE - withdraw_amount + remaining - four_instructions_execution_cost
        );
    });
}

#[test]
fn para_to_para_reserve_transfer_and_back() {
    MockNet::reset();

    let sibling_asset_id = 123 as u128;
    let para_a_multiloc = (Parent, Parachain(1));
    let alice = AccountId32 {
        network: None,
        id: ALICE.into(),
    };

    // On parachain B create an asset which representes a derivative of parachain A native asset.
    // This asset is allowed as XCM execution fee payment asset.
    ParaB::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            sibling_asset_id,
            para_a_multiloc.clone(),
            sibling_para_account_id(1),
            Some(true),
            Some(1),
            Some(1_000_000_000_000)
        ));
    });

    // Next step is to send some of parachain A native asset to parachain B.
    let withdraw_amount = 567;
    ParaA::execute_with(|| {
        assert_ok!(ParachainXtokens::transfer_multiasset(
            parachain::RuntimeOrigin::signed(ALICE),
            Box::new((Here, withdraw_amount).into()),
            Box::new((Parent, Parachain(2), alice).into()),
            Unlimited,
        ));

        // Parachain 2 sovereign account should have it's balance increased, while Alice balance should be decreased.
        assert_eq!(
            parachain::Balances::free_balance(&sibling_para_account_id(2)),
            INITIAL_BALANCE + withdraw_amount
        );
        assert_eq!(
            parachain::Balances::free_balance(&ALICE),
            INITIAL_BALANCE - withdraw_amount
        );
    });

    // Parachain B should receive parachain A native assets and should mint their local derivate.
    // Portion of those assets should be taken as the XCM execution fee.
    let four_instructions_execution_cost =
        (parachain::UnitWeightCost::get() * 4).ref_time() as u128;
    let remaining = withdraw_amount - four_instructions_execution_cost;
    ParaB::execute_with(|| {
        // Ensure Alice received assets on ParaB (sent amount minus expenses)
        assert_eq!(
            parachain::Assets::balance(sibling_asset_id, ALICE),
            remaining
        );
    });

    // send assets back to ParaA
    ParaB::execute_with(|| {
        assert_ok!(ParachainXtokens::transfer(
            parachain::RuntimeOrigin::signed(ALICE),
            sibling_asset_id,
            remaining,
            Box::new((Parent, Parachain(1), alice,).into()),
            Unlimited
        ));
    });

    ParaA::execute_with(|| {
        // ParaB soveregin account account should have only the execution cost
        assert_eq!(
            parachain::Balances::free_balance(&sibling_para_account_id(2)),
            INITIAL_BALANCE + four_instructions_execution_cost
        );
        // ParaA alice should have initial amount backed subtracted with execution costs
        // which is 2xfour_instructions_execution_cost
        // or withdraw_amount + remaining - four_instructions_execution_cost
        // both are same
        assert_eq!(
            parachain::Balances::free_balance(&ALICE),
            INITIAL_BALANCE - withdraw_amount + remaining - four_instructions_execution_cost
        );
    });
}

#[test]
fn para_to_para_reserve_transfer_and_back_with_extra_native() {
    MockNet::reset();

    let local_asset_id = 123 as u128;
    let local_asset: MultiLocation = (PalletInstance(4u8), GeneralIndex(local_asset_id)).into();

    let para_a_local_asset = local_asset
        .clone()
        .pushed_front_with_interior(Parachain(1))
        .unwrap()
        .prepended_with(Parent)
        .unwrap();

    let para_b_native: MultiLocation = (Parent, Parachain(2)).into();
    let para_b_native_on_para_a = 456;

    let mint_amount = 300_000_000_000_000;

    let alice = AccountId32 {
        network: None,
        id: ALICE.into(),
    };

    // Local Asset registeration and minting on Parachian A
    ParaA::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            local_asset_id,
            local_asset,
            ALICE.into(),
            Some(true),
            Some(1),
            // free execution
            Some(0)
        ));

        assert_ok!(ParachainAssets::mint(
            parachain::RuntimeOrigin::signed(ALICE.into()),
            local_asset_id,
            ALICE.into(),
            mint_amount
        ));
    });

    // Registration of Local Asset of Para A on Para B
    ParaB::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            local_asset_id,
            para_a_local_asset,
            sibling_para_account_id(1),
            Some(true),
            Some(1),
            // free execution
            Some(0)
        ));
    });

    let send_amount = 123;
    ParaA::execute_with(|| {
        assert_ok!(ParachainXtokens::transfer(
            parachain::RuntimeOrigin::signed(ALICE.into()),
            local_asset_id,
            send_amount,
            Box::new((Parent, Parachain(2), alice).into()),
            Unlimited
        ));
    });

    ParaB::execute_with(|| {
        // free execution, full amount received
        assert_eq!(
            ParachainAssets::balance(local_asset_id, &ALICE.into()),
            send_amount
        );
    });

    // Registring Para B native asset on Para A
    ParaA::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            para_b_native_on_para_a,
            para_b_native.clone(),
            sibling_para_account_id(2),
            Some(true),
            Some(1),
            // free execution
            Some(0)
        ));
    });

    // Sending back Local Asset to Para A with some native asset of Para B
    ParaB::execute_with(|| {
        assert_ok!(ParachainXtokens::transfer(
            parachain::RuntimeOrigin::signed(ALICE.into()),
            local_asset_id,
            send_amount,
            Box::new((Parent, Parachain(1), alice.clone()).into()),
            Unlimited
        ));
        assert_ok!(ParachainXtokens::transfer_multiasset(
            parachain::RuntimeOrigin::signed(ALICE.into()),
            Box::new((Here, send_amount).into()),
            Box::new((Parent, Parachain(1), alice.clone()).into()),
            Unlimited
        ));
    });

    ParaA::execute_with(|| {
        assert_eq!(
            parachain::Assets::balance(local_asset_id, ALICE),
            mint_amount
        );
        assert_eq!(
            parachain::Assets::balance(para_b_native_on_para_a, ALICE),
            send_amount
        );
    })
}

#[test]
fn para_to_para_reserve_transfer_local_asset() {
    MockNet::reset();

    let asset_id = 123;
    let local_asset: MultiLocation = (PalletInstance(4u8), GeneralIndex(asset_id)).into();
    let para_a_local_asset = local_asset
        .clone()
        .pushed_front_with_interior(Parachain(1))
        .unwrap()
        .prepended_with(Parent)
        .unwrap();

    ParaA::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            asset_id,
            local_asset,
            ALICE.into(),
            Some(true),
            Some(1),
            // free execution
            Some(0)
        ));

        assert_ok!(ParachainAssets::mint(
            parachain::RuntimeOrigin::signed(ALICE.into()),
            asset_id,
            ALICE.into(),
            300000000000000
        ));
    });

    ParaB::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            asset_id,
            para_a_local_asset,
            sibling_para_account_id(1),
            Some(true),
            Some(1),
            // free execution
            Some(0)
        ));
    });

    let send_amount = 123;
    ParaA::execute_with(|| {
        assert_ok!(ParachainXtokens::transfer(
            parachain::RuntimeOrigin::signed(ALICE.into()),
            asset_id,
            send_amount,
            Box::new(
                (
                    Parent,
                    Parachain(2),
                    AccountId32 {
                        network: None,
                        id: ALICE.into()
                    }
                )
                    .into()
            ),
            Unlimited
        ));
    });

    ParaB::execute_with(|| {
        // free execution, full amount received
        assert_eq!(
            ParachainAssets::balance(asset_id, &ALICE.into()),
            send_amount
        );
    });
}

// TODO: remove this when retiring pallet-xcm fork
//
// Send a relay asset (like DOT/KSM) to a parachain A
// and send it back from Parachain A to relaychain
#[test]
fn receive_relay_asset_from_relay_and_send_them_back_via_pallet_xcm() {
    MockNet::reset();

    let source_location = (Parent,);
    let relay_asset_id = 123_u128;
    let alice = AccountId32 {
        network: None,
        id: ALICE.into(),
    };

    // On parachain A create an asset which representes a derivative of relay native asset.
    // This asset is allowed as XCM execution fee payment asset.
    ParaA::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            relay_asset_id,
            source_location,
            parent_account_id(),
            Some(true),
            Some(1),
            Some(1_000_000_000_000)
        ));
    });

    // Next step is to send some of relay native asset to parachain A.
    let withdraw_amount = 567;
    Relay::execute_with(|| {
        assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
            relay_chain::RuntimeOrigin::signed(ALICE),
            Box::new(Parachain(1).into()),
            Box::new(alice.into()),
            Box::new((Here, withdraw_amount).into()),
            0,
        ));

        // Parachain A sovereign account should have it's balance increased, while Alice balance should be decreased.
        assert_eq!(
            relay_chain::Balances::free_balance(&child_para_account_id(1)),
            INITIAL_BALANCE + withdraw_amount
        );
        assert_eq!(
            relay_chain::Balances::free_balance(&ALICE),
            INITIAL_BALANCE - withdraw_amount
        );
    });

    // Parachain A should receive relay native assets and should mint their local derivate.
    // Portion of those assets should be taken as the XCM execution fee.
    let four_instructions_execution_cost =
        (parachain::UnitWeightCost::get() * 4).ref_time() as u128;
    let para_a_alice_expected_balance = withdraw_amount - four_instructions_execution_cost;
    ParaA::execute_with(|| {
        // Ensure Alice received assets on ParaA (sent amount minus expenses)
        assert_eq!(
            parachain::Assets::balance(relay_asset_id, ALICE),
            para_a_alice_expected_balance
        );
    });

    //
    // Send the relay assets back to relay
    //

    // Lets gather the balance before sending back money
    let mut relay_alice_balance_before_sending = 0;
    Relay::execute_with(|| {
        relay_alice_balance_before_sending = relay_chain::Balances::free_balance(&ALICE);
    });

    ParaA::execute_with(|| {
        assert_ok!(ParachainPalletXcm::reserve_withdraw_assets(
            parachain::RuntimeOrigin::signed(ALICE),
            Box::new(Parent.into()),
            Box::new(alice.into()),
            Box::new((Parent, para_a_alice_expected_balance).into()),
            0,
        ));
    });

    // The balances in ParaA alice should have been substracted
    ParaA::execute_with(|| {
        assert_eq!(parachain::Assets::balance(relay_asset_id, ALICE), 0);
    });

    // Balances in the relay should have been received
    Relay::execute_with(|| {
        // free execution,x	 full amount received
        assert!(relay_chain::Balances::free_balance(ALICE) > relay_alice_balance_before_sending);
    });
}

// Send a relay asset (like DOT/KSM) to a parachain A
// and send it back from Parachain A to relaychain
#[test]
fn receive_relay_asset_from_relay_and_send_them_back() {
    MockNet::reset();

    let source_location = (Parent,);
    let relay_asset_id = 123_u128;
    let alice = AccountId32 {
        network: None,
        id: ALICE.into(),
    };

    // On parachain A create an asset which representes a derivative of relay native asset.
    // This asset is allowed as XCM execution fee payment asset.
    ParaA::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            relay_asset_id,
            source_location,
            parent_account_id(),
            Some(true),
            Some(1),
            Some(1_000_000_000_000)
        ));
    });

    // Next step is to send some of relay native asset to parachain A.
    let withdraw_amount = 567;
    Relay::execute_with(|| {
        assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
            relay_chain::RuntimeOrigin::signed(ALICE),
            Box::new(Parachain(1).into()),
            Box::new(alice.into()),
            Box::new((Here, withdraw_amount).into()),
            0,
        ));

        // Parachain A sovereign account should have it's balance increased, while Alice balance should be decreased.
        assert_eq!(
            relay_chain::Balances::free_balance(&child_para_account_id(1)),
            INITIAL_BALANCE + withdraw_amount
        );
        assert_eq!(
            relay_chain::Balances::free_balance(&ALICE),
            INITIAL_BALANCE - withdraw_amount
        );
    });

    // Parachain A should receive relay native assets and should mint their local derivate.
    // Portion of those assets should be taken as the XCM execution fee.
    let four_instructions_execution_cost =
        (parachain::UnitWeightCost::get() * 4).ref_time() as u128;
    let para_a_alice_expected_balance = withdraw_amount - four_instructions_execution_cost;
    ParaA::execute_with(|| {
        // Ensure Alice received assets on ParaA (sent amount minus expenses)
        assert_eq!(
            parachain::Assets::balance(relay_asset_id, ALICE),
            para_a_alice_expected_balance
        );
    });

    //
    // Send the relay assets back to relay
    //

    // Lets gather the balance before sending back money
    let mut relay_alice_balance_before_sending = 0;
    Relay::execute_with(|| {
        relay_alice_balance_before_sending = relay_chain::Balances::free_balance(&ALICE);
    });

    ParaA::execute_with(|| {
        assert_ok!(ParachainXtokens::transfer(
            parachain::RuntimeOrigin::signed(ALICE),
            relay_asset_id,
            para_a_alice_expected_balance,
            Box::new((Parent, alice).into(),),
            Unlimited
        ));
    });

    // The balances in ParaA alice should have been substracted
    ParaA::execute_with(|| {
        assert_eq!(parachain::Assets::balance(relay_asset_id, ALICE), 0);
    });

    // Balances in the relay should have been received
    Relay::execute_with(|| {
        // free execution,x	 full amount received
        assert!(relay_chain::Balances::free_balance(ALICE) > relay_alice_balance_before_sending);
    });
}

// Send relay asset (like DOT) back from Parachain A to Parachain B
#[test]
fn para_a_send_relay_asset_to_para_b() {
    MockNet::reset();

    let source_location = (Parent,);
    let relay_asset_id = 123_u128;
    let alice = AccountId32 {
        network: None,
        id: ALICE.into(),
    };

    // On parachain A create an asset which representes a derivative of relay native asset.
    // This asset is allowed as XCM execution fee payment asset.
    // Register relay asset in ParaA
    ParaA::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            relay_asset_id,
            source_location,
            parent_account_id(),
            Some(true),
            Some(123),
            // free execution
            Some(0)
        ));
    });

    // register relay asset in ParaB
    ParaB::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            relay_asset_id,
            source_location,
            parent_account_id(),
            Some(true),
            Some(123),
            Some(1_000_000_000_000)
        ));
    });

    // Next step is to send some of relay native asset to parachain A.
    // same as previous test
    let withdraw_amount = 54321;
    Relay::execute_with(|| {
        assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
            relay_chain::RuntimeOrigin::signed(ALICE),
            Box::new(Parachain(1).into()),
            Box::new(alice.into_location().into_versioned()),
            Box::new((Here, withdraw_amount).into()),
            0,
        ));
    });

    ParaA::execute_with(|| {
        // Free execution, full amount received
        assert_eq!(
            parachain::Assets::balance(relay_asset_id, ALICE),
            withdraw_amount
        );
    });

    // send relay asset to ParaB
    ParaA::execute_with(|| {
        assert_ok!(ParachainXtokens::transfer(
            parachain::RuntimeOrigin::signed(ALICE),
            relay_asset_id,
            withdraw_amount,
            Box::new((Parent, Parachain(2), alice).into(),),
            Unlimited,
        ));
    });

    // Para A balances should have been substracted
    ParaA::execute_with(|| {
        assert_eq!(parachain::Assets::balance(relay_asset_id, ALICE), 0);
    });

    // Para B balances should have been credited
    ParaB::execute_with(|| {
        assert_eq!(
            parachain::Assets::balance(relay_asset_id, ALICE),
            withdraw_amount - 40
        );
    });
}

#[test]
fn send_relay_asset_to_para_b_with_extra_native() {
    MockNet::reset();

    let source_location = (Parent,);
    let relay_asset_id = 123_u128;
    let alice = AccountId32 {
        network: None,
        id: ALICE.into(),
    };

    let para_a_native: MultiLocation = (Parent, Parachain(1)).into();
    let para_a_native_on_para_b = 456;

    // On parachain A create an asset which representes a derivative of relay native asset.
    // This asset is allowed as XCM execution fee payment asset.
    // Register relay asset in ParaA
    ParaA::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            relay_asset_id,
            source_location,
            parent_account_id(),
            Some(true),
            Some(123),
            Some(0)
        ));
    });

    // register relay asset in ParaB
    ParaB::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            relay_asset_id,
            source_location,
            parent_account_id(),
            Some(true),
            Some(123),
            Some(0)
        ));
    });

    // register Para A native on Para B
    ParaB::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            para_a_native_on_para_b,
            para_a_native,
            sibling_para_account_id(1),
            Some(true),
            Some(123),
            Some(0)
        ));
    });

    // Next step is to send some of relay native asset to parachain A.
    // same as previous test
    let withdraw_amount = 54321;
    Relay::execute_with(|| {
        assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
            relay_chain::RuntimeOrigin::signed(ALICE),
            Box::new(Parachain(1).into()),
            Box::new(alice.into_location().into_versioned()),
            Box::new((Here, withdraw_amount).into()),
            0,
        ));
    });

    ParaA::execute_with(|| {
        // Free execution, full amount received
        assert_eq!(
            parachain::Assets::balance(relay_asset_id, ALICE),
            withdraw_amount
        );
    });

    // send relay asset with some Para A native to ParaB
    ParaA::execute_with(|| {
        assert_ok!(ParachainXtokens::transfer(
            parachain::RuntimeOrigin::signed(ALICE.into()),
            relay_asset_id,
            withdraw_amount,
            Box::new((Parent, Parachain(2), alice).into()),
            Unlimited,
        ));
        assert_ok!(ParachainXtokens::transfer_multiasset(
            parachain::RuntimeOrigin::signed(ALICE.into()),
            Box::new((Here, withdraw_amount).into()),
            Box::new((Parent, Parachain(2), alice).into()),
            Unlimited,
        ));
    });

    // Para A balances should have been substracted
    ParaA::execute_with(|| {
        assert_eq!(parachain::Assets::balance(relay_asset_id, ALICE), 0);
        assert_eq!(
            parachain::Balances::free_balance(ALICE),
            INITIAL_BALANCE - withdraw_amount
        );
    });

    // Para B balances should have been added
    ParaB::execute_with(|| {
        assert_eq!(
            parachain::Assets::balance(relay_asset_id, ALICE),
            withdraw_amount
        );
        assert_eq!(
            parachain::Assets::balance(para_a_native_on_para_b, ALICE),
            withdraw_amount
        );
    });
}

#[test]
fn receive_asset_with_no_sufficients_not_possible_if_non_existent_account() {
    MockNet::reset();

    let relay_asset_id = 123 as u128;
    let source_location = (Parent,);
    let fresh_account = [2u8; 32];

    // On parachain A create an asset which representes a derivative of relay native asset.
    ParaA::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            relay_asset_id,
            source_location,
            parent_account_id(),
            Some(false),
            Some(1),
            // free execution
            Some(0)
        ));
    });

    // Next step is to send some of relay native asset to parachain A.
    let withdraw_amount = 123;
    Relay::execute_with(|| {
        assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
            relay_chain::RuntimeOrigin::signed(ALICE),
            Box::new(Parachain(1).into()),
            Box::new(
                AccountId32 {
                    network: None,
                    id: fresh_account.into()
                }
                .into()
            ),
            Box::new((Here, withdraw_amount).into()),
            0,
        ));
    });

    // parachain should not have received assets
    ParaA::execute_with(|| {
        assert_eq!(
            ParachainAssets::balance(relay_asset_id, &fresh_account.into()),
            0
        );
    });

    // Send native token to fresh_account
    ParaA::execute_with(|| {
        assert_ok!(ParachainBalances::transfer(
            parachain::RuntimeOrigin::signed(ALICE.into()),
            fresh_account.into(),
            100
        ));
    });

    // Re-send tokens
    Relay::execute_with(|| {
        assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
            relay_chain::RuntimeOrigin::signed(ALICE),
            Box::new(Parachain(1).into()),
            Box::new(
                AccountId32 {
                    network: None,
                    id: fresh_account.into()
                }
                .into()
            ),
            Box::new((Here, withdraw_amount).into()),
            0,
        ));
    });

    // parachain should have received assets
    ParaA::execute_with(|| {
        // free execution, full amount received
        assert_eq!(
            ParachainAssets::balance(relay_asset_id, &fresh_account.into()),
            withdraw_amount
        );
    });
}

#[test]
fn receive_assets_with_sufficients_true_allows_non_funded_account_to_receive_assets() {
    MockNet::reset();

    let relay_asset_id = 123 as u128;
    let source_location = (Parent,);
    let fresh_account = [2u8; 32];

    // On parachain A create an asset which representes a derivative of relay native asset.
    ParaA::execute_with(|| {
        assert_ok!(register_and_setup_xcm_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            relay_asset_id,
            source_location,
            parent_account_id(),
            Some(true),
            Some(1),
            // free execution
            Some(0)
        ));
    });

    // Next step is to send some of relay native asset to parachain A.
    // Since min balance is configured to 1, 123 should be fine
    let withdraw_amount = 123;
    Relay::execute_with(|| {
        assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
            relay_chain::RuntimeOrigin::signed(ALICE),
            Box::new(Parachain(1).into()),
            Box::new(
                AccountId32 {
                    network: None,
                    id: fresh_account.into()
                }
                .into()
            ),
            Box::new((Here, withdraw_amount).into()),
            0,
        ));
    });

    // parachain should have received assets
    ParaA::execute_with(|| {
        // free execution, full amount received
        assert_eq!(
            ParachainAssets::balance(relay_asset_id, &fresh_account.into()),
            withdraw_amount
        );
    });
}

#[test]
fn para_asset_trap_and_claim() {
    MockNet::reset();
    let send_amount = 1222;

    let bob = AccountId32 {
        network: None,
        id: BOB.into(),
    };

    // Asset Trapped
    ParaA::execute_with(|| {
        let xcm = Xcm(vec![WithdrawAsset((Here, send_amount).into())]);

        assert_ok!(ParachainPalletXcm::execute(
            parachain::RuntimeOrigin::signed(ALICE.into()),
            Box::new(VersionedXcm::V3(xcm)),
            Weight::from_parts(100_000_000_000, 1024 * 1024)
        ));

        //  Alice's Balnce is reduced by the sent amount
        assert_eq!(
            parachain::Balances::free_balance(ALICE),
            INITIAL_BALANCE - send_amount
        );
        // Making sure that Bob doesn't have any free balance before transfer
        assert_eq!(parachain::Balances::free_balance(BOB), 0);
    });

    ParaA::execute_with(|| {
        // Claiming Trapped Assets
        let xcm = Xcm(vec![
            ClaimAsset {
                assets: (Here, send_amount).into(),
                ticket: (Here).into(),
            },
            BuyExecution {
                fees: (Here, send_amount).into(),
                weight_limit: Unlimited,
            },
            DepositAsset {
                assets: All.into(),
                beneficiary: bob.clone().into(),
            },
        ]);

        assert_ok!(ParachainPalletXcm::execute(
            parachain::RuntimeOrigin::signed(ALICE.into()),
            Box::new(VersionedXcm::V3(xcm)),
            Weight::from_parts(100_000_000_000, 1024 * 1024)
        ));

        // Bob's Balance increased after assets claimed
        assert_eq!(parachain::Balances::free_balance(BOB), send_amount);
    });
}
