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

use frame_support::{assert_ok, traits::IsType, weights::Weight};
use pallet_contracts::Determinism;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::{
    traits::{Bounded, StaticLookup},
    DispatchResult,
};
use xcm::prelude::*;
use xcm_simulator::TestExt;

fn register_asset<Runtime, AssetId>(
    origin: Runtime::RuntimeOrigin,
    asset_id: AssetId,
    asset_location: impl Into<MultiLocation> + Clone,
    asset_controller: <Runtime::Lookup as StaticLookup>::Source,
    is_sufficent: Option<bool>,
    min_balance: Option<Runtime::Balance>,
    units_per_second: Option<u128>,
) -> DispatchResult
where
    Runtime: pallet_xc_asset_config::Config + pallet_assets::Config,
    AssetId: IsType<<Runtime as pallet_xc_asset_config::Config>::AssetId>
        + IsType<<Runtime as pallet_assets::Config>::AssetId>
        + Clone,
{
    pallet_assets::Pallet::<Runtime>::force_create(
        origin.clone(),
        <Runtime as pallet_assets::Config>::AssetIdParameter::from(asset_id.clone().into()),
        asset_controller,
        is_sufficent.unwrap_or(true),
        min_balance.unwrap_or(Bounded::min_value()),
    )?;

    pallet_xc_asset_config::Pallet::<Runtime>::register_asset_location(
        origin.clone(),
        Box::new(asset_location.clone().into().into_versioned()),
        asset_id.into(),
    )?;

    pallet_xc_asset_config::Pallet::<Runtime>::set_asset_units_per_second(
        origin,
        Box::new(asset_location.into().into_versioned()),
        units_per_second.unwrap_or(1_000_000_000_000),
    )
}

#[test]
fn basic_dmp() {
    MockNet::reset();

    let remark = parachain::RuntimeCall::System(
        frame_system::Call::<parachain::Runtime>::remark_with_event {
            remark: vec![1, 2, 3],
        },
    );

    // A remote `Transact` is sent to the parachain A.
    // No need to pay for the execution time since parachain is configured to allow unpaid execution from parents.
    Relay::execute_with(|| {
        assert_ok!(RelayChainPalletXcm::send_xcm(
            Here,
            Parachain(1),
            Xcm(vec![Transact {
                origin_kind: OriginKind::SovereignAccount,
                require_weight_at_most: Weight::from_parts(1_000_000_000, 1024 * 1024),
                call: remark.encode().into(),
            }]),
        ));
    });

    // Execute remote transact and verify that `Remarked` event is emitted.
    ParaA::execute_with(|| {
        use parachain::{RuntimeEvent, System};
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::System(frame_system::Event::Remarked { .. })
        )));
    });
}

#[test]
fn basic_ump() {
    MockNet::reset();

    let remark = relay_chain::RuntimeCall::System(
        frame_system::Call::<relay_chain::Runtime>::remark_with_event {
            remark: vec![1, 2, 3],
        },
    );

    // A remote `Transact` is sent to the relaychain.
    // No need to pay for the execution time since relay chain is configured to allow unpaid execution from everything.
    ParaA::execute_with(|| {
        assert_ok!(ParachainPalletXcm::send_xcm(
            Here,
            Parent,
            Xcm(vec![Transact {
                origin_kind: OriginKind::SovereignAccount,
                require_weight_at_most: Weight::from_parts(1_000_000_000, 1024 * 1024),
                call: remark.encode().into(),
            }]),
        ));
    });

    Relay::execute_with(|| {
        use relay_chain::{RuntimeEvent, System};
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::System(frame_system::Event::Remarked { .. })
        )));
    });
}

#[test]
fn basic_xcmp() {
    MockNet::reset();

    let remark = parachain::RuntimeCall::System(
        frame_system::Call::<parachain::Runtime>::remark_with_event {
            remark: vec![1, 2, 3],
        },
    );
    ParaA::execute_with(|| {
        assert_ok!(ParachainPalletXcm::send_xcm(
            Here,
            (Parent, Parachain(2)),
            Xcm(vec![
                WithdrawAsset((Here, 100_000_000_000_u128).into()),
                BuyExecution {
                    fees: (Here, 100_000_000_000_u128).into(),
                    weight_limit: Unlimited
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    require_weight_at_most: Weight::from_parts(1_000_000_000, 1024 * 1024),
                    call: remark.encode().into(),
                }
            ]),
        ));
    });

    ParaB::execute_with(|| {
        use parachain::{RuntimeEvent, System};
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::System(frame_system::Event::Remarked { .. })
        )));
    });
}

#[test]
fn para_to_para_reserve_transfer_and_back() {
    MockNet::reset();

    let sibling_asset_id = 123 as u128;
    let para_a_multiloc = (Parent, Parachain(1));

    // On parachain B create an asset which representes a derivative of parachain A native asset.
    // This asset is allowed as XCM execution fee payment asset.
    ParaB::execute_with(|| {
        assert_ok!(register_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            sibling_asset_id,
            para_a_multiloc.clone(),
            sibling_account_id(1),
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
            parachain::Balances::free_balance(&sibling_account_id(2)),
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
            parachain::Balances::free_balance(&sibling_account_id(2)),
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
        assert_ok!(register_asset::<parachain::Runtime, _>(
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
        assert_ok!(register_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            asset_id,
            para_a_local_asset,
            sibling_account_id(1),
            Some(true),
            Some(1),
            // free execution
            Some(0)
        ));
    });

    let send_amount = 123;
    ParaA::execute_with(|| {
        assert_ok!(ParachainPalletXcm::reserve_transfer_assets(
            parachain::RuntimeOrigin::signed(ALICE.into()),
            Box::new((Parent, Parachain(2)).into()),
            Box::new(
                AccountId32 {
                    network: None,
                    id: ALICE.into(),
                }
                .into(),
            ),
            Box::new((local_asset, send_amount).into()),
            0,
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
// Send a relay asset (like DOT/KSM) to a parachain A
#[test]
fn receive_relay_asset_from_relay() {
    MockNet::reset();

    let relay_asset_id = 123 as u128;
    let source_location = (Parent,);

    // On parachain A create an asset which representes a derivative of relay native asset.
    // This asset is allowed as XCM execution fee payment asset.
    ParaA::execute_with(|| {
        assert_ok!(register_asset::<parachain::Runtime, _>(
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
            Box::new(
                AccountId32 {
                    network: None,
                    id: ALICE.into()
                }
                .into_location()
                .into_versioned()
            ),
            Box::new((Here, withdraw_amount).into()),
            0,
        ));

        // Parachain A sovereign account should have it's balance increased, while Alice balance should be decreased.
        assert_eq!(
            relay_chain::Balances::free_balance(&child_account_id(1)),
            INITIAL_BALANCE + withdraw_amount
        );
        assert_eq!(
            relay_chain::Balances::free_balance(&ALICE),
            INITIAL_BALANCE - withdraw_amount
        );
    });

    // Parachain A should receive relay native assets and should mint their local derivate.
    // Portion of those assets should be taken as the XCM execution fee.
    ParaA::execute_with(|| {
        // Ensure Alice received assets on ParaA (sent amount minus expenses)
        let four_instructions_execution_cost = parachain::UnitWeightCost::get() * 4;
        assert_eq!(
            parachain::Assets::balance(relay_asset_id, ALICE),
            withdraw_amount - four_instructions_execution_cost.ref_time() as u128
        );
    });
}

// Send relay asset (like DOT) back from Parachain A to relaychain
#[test]
fn send_relay_asset_to_relay() {
    MockNet::reset();

    let source_location = (Parent,);
    let relay_asset_id = 123_u128;
    let alice = AccountId32 {
        network: None,
        id: ALICE.into(),
    };

    // On parachain A create an asset which representes a derivative of relay native asset.
    // This asset is allowed as XCM execution fee payment asset.
    // Register relay asset in paraA
    ParaA::execute_with(|| {
        assert_ok!(register_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            relay_asset_id,
            source_location,
            parent_account_id(),
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

    // Lets gather the balance before sending back money
    let mut balance_before_sending = 0;
    Relay::execute_with(|| {
        balance_before_sending = relay_chain::Balances::free_balance(&ALICE);
    });

    ParaA::execute_with(|| {
        assert_ok!(ParachainPalletXcm::reserve_withdraw_assets(
            parachain::RuntimeOrigin::signed(ALICE),
            Box::new(Parent.into()),
            Box::new(alice.into_location().into_versioned()),
            Box::new((Parent, withdraw_amount).into()),
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
        assert!(relay_chain::Balances::free_balance(ALICE) > balance_before_sending);
    });

    // // To get logs
    // std::thread::sleep(std::time::Duration::from_millis(4000));
}

// Send relay asset (like DOT) back from Parachain A to Parachain B
#[test]
fn send_relay_asset_to_para_b() {
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
        assert_ok!(register_asset::<parachain::Runtime, _>(
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
        assert_ok!(register_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            relay_asset_id,
            source_location,
            parent_account_id(),
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

    // send relay asset to ParaB
    ParaA::execute_with(|| {
        let xcm = Xcm(vec![
            // withdraw relay native asset
            WithdrawAsset((Parent, withdraw_amount).into()),
            InitiateReserveWithdraw {
                assets: All.into(),
                reserve: source_location.clone().into(),
                xcm: Xcm(vec![
                    BuyExecution {
                        fees: (Here, withdraw_amount).into(),
                        weight_limit: Unlimited,
                    },
                    // deposit into ParaB
                    DepositReserveAsset {
                        assets: All.into(),
                        dest: Parachain(2).into(),
                        xcm: Xcm(vec![
                            BuyExecution {
                                // for sake of sanity, let's assume half the
                                // amount is still available
                                fees: (Parent, withdraw_amount / 2).into(),
                                weight_limit: Unlimited,
                            },
                            // deposit into ParaB's alice
                            DepositAsset {
                                assets: All.into(),
                                beneficiary: alice.clone().into(),
                            },
                        ]),
                    },
                ]),
            },
        ]);

        assert_ok!(ParachainPalletXcm::execute(
            parachain::RuntimeOrigin::signed(ALICE.into()),
            Box::new(VersionedXcm::V3(xcm)),
            Weight::from_parts(100_000_000_000, 1024 * 1024)
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
        assert_ok!(register_asset::<parachain::Runtime, _>(
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
        assert_ok!(register_asset::<parachain::Runtime, _>(
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
fn error_when_not_paying_enough() {
    MockNet::reset();

    let source_location: MultiLocation = (Parent,).into();
    let source_id: parachain::AssetId = 123;

    let dest: MultiLocation = Junction::AccountId32 {
        network: None,
        id: ALICE.into(),
    }
    .into();
    // This time we are gonna put a rather high number of units per second
    // we know later we will divide by 1e12
    // Lets put 1e6 as units per second
    ParaA::execute_with(|| {
        assert_ok!(register_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            source_id,
            source_location,
            parent_account_id(),
            Some(true),
            Some(1),
            Some(2500000000000u128)
        ));
    });

    // We are sending 100 tokens from relay.
    // If we set the dest weight to be 1e7, we know the buy_execution will spend 1e7*1e6/1e12 = 10
    // Therefore with no refund, we should receive 10 tokens less
    Relay::execute_with(|| {
        assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
            relay_chain::RuntimeOrigin::signed(ALICE),
            Box::new(Parachain(1).into()),
            Box::new(VersionedMultiLocation::V3(dest).clone().into()),
            Box::new((Here, 5).into()),
            0,
        ));
    });

    ParaA::execute_with(|| {
        // amount not received as it is not paying enough
        assert_eq!(ParachainAssets::balance(source_id, &ALICE.into()), 0);
    });
}

#[test]
fn remote_dapps_staking_staker_claim() {
    MockNet::reset();

    // The idea of this test case is to remotely claim dApps staking staker rewards.
    // Remote claim will be sent from parachain A to parachain B.

    let smart_contract = parachain::SmartContract::Wasm(1337);
    let stake_amount = 100_000_000;

    // 1st step
    // Register contract & stake on it. Advance a few blocks until at least era 4 since we need 3 claimable rewards.
    // Enable parachain A sovereign account to claim on Alice's behalf.
    ParaB::execute_with(|| {
        assert_ok!(parachain::DappsStaking::register(
            parachain::RuntimeOrigin::root(),
            ALICE,
            smart_contract.clone(),
        ));
        assert_ok!(parachain::DappsStaking::bond_and_stake(
            parachain::RuntimeOrigin::signed(ALICE),
            smart_contract.clone(),
            stake_amount,
        ));

        // advance enough blocks so we at least get to era 4
        advance_parachain_block_to(20);
        assert!(parachain::DappsStaking::current_era() >= 4);

        // Register para A sovereign account as proxy with dApps staking privileges
        assert_ok!(parachain::Proxy::add_proxy(
            parachain::RuntimeOrigin::signed(ALICE),
            sibling_account_id(1),
            parachain::ProxyType::StakerRewardClaim,
            0
        ));
    });

    let claim_staker_call = parachain::RuntimeCall::DappsStaking(pallet_dapps_staking::Call::<
        parachain::Runtime,
    >::claim_staker {
        contract_id: smart_contract.clone(),
    });

    // 2nd step
    // Dispatch remote `claim_staker` call from Para A to Para B
    ParaA::execute_with(|| {
        let proxy_call =
            parachain::RuntimeCall::Proxy(pallet_proxy::Call::<parachain::Runtime>::proxy {
                real: ALICE,
                force_proxy_type: None,
                call: Box::new(claim_staker_call.clone()),
            });

        // Send the remote transact operation
        assert_ok!(ParachainPalletXcm::send_xcm(
            Here,
            MultiLocation::new(1, X1(Parachain(2))),
            Xcm(vec![
                WithdrawAsset((Here, 100_000_000_000_u128).into()),
                BuyExecution {
                    fees: (Here, 100_000_000_000_u128).into(),
                    weight_limit: Unlimited
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    require_weight_at_most: Weight::from_parts(1_000_000_000, 1024 * 1024),
                    call: proxy_call.encode().into(),
                },
            ]),
        ));
    });

    // 3rd step
    // Receive claim & verify it was successful
    ParaB::execute_with(|| {
        // We expect exactly one `Reward` event
        assert_eq!(
            parachain::System::events()
                .iter()
                .filter(|r| matches!(
                    r.event,
                    parachain::RuntimeEvent::DappsStaking(
                        pallet_dapps_staking::Event::Reward { .. }
                    )
                ))
                .count(),
            1
        );

        // Extra check to ensure reward was claimed for `Alice`
        let staker_info = parachain::DappsStaking::staker_info(&ALICE, &smart_contract);
        assert!(staker_info.latest_staked_value() > stake_amount);

        // Cleanup events
        parachain::System::reset_events();
    });

    // 4th step
    // Dispatch two remote `claim_staker` calls from Para A to Para B, but as a batch
    ParaA::execute_with(|| {
        let batch_call =
            parachain::RuntimeCall::Utility(pallet_utility::Call::<parachain::Runtime>::batch {
                calls: vec![claim_staker_call.clone(), claim_staker_call.clone()],
            });

        let proxy_call =
            parachain::RuntimeCall::Proxy(pallet_proxy::Call::<parachain::Runtime>::proxy {
                real: ALICE,
                force_proxy_type: None,
                call: Box::new(batch_call),
            });

        // Send the remote transact operation
        assert_ok!(ParachainPalletXcm::send_xcm(
            Here,
            MultiLocation::new(1, X1(Parachain(2))),
            Xcm(vec![
                WithdrawAsset((Here, 100_000_000_000_u128).into()),
                BuyExecution {
                    fees: (Here, 100_000_000_000_u128).into(),
                    weight_limit: Unlimited
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    require_weight_at_most: Weight::from_parts(1_000_000_000, 1024 * 1024),
                    call: proxy_call.encode().into(),
                }
            ]),
        ));
    });

    // 5th step
    // Receive two claims & verify they were successful
    ParaB::execute_with(|| {
        // We expect exactly two `Reward` events
        assert_eq!(
            parachain::System::events()
                .iter()
                .filter(|r| matches!(
                    r.event,
                    parachain::RuntimeEvent::DappsStaking(
                        pallet_dapps_staking::Event::Reward { .. }
                    )
                ))
                .count(),
            2
        );
    });
}

/// Scenario:
/// User transfers an NFT from ParaA to ParaB.
/// NFT is first minted on ParaA pallet-uniques.
/// On ParaB, a derivative NFT is minted on smart contract.
#[test]
fn transfer_nft_to_smart_contract() {
    MockNet::reset();
    let collection = 1;
    let item = 42;
    // let uniques_pallet_instance = 13u8;
    // Alice owns an NFT on the ParaA chain.
    ParaA::execute_with(|| {
        assert_eq!(
            parachain::Uniques::owner(collection, item),
            Some(child_account_id(1))
        );
    });

    // let sibling_asset_id = 123 as u128;
    // let para_a_multiloc = (Parent, Parachain(1));

    // Deploy and initialize flipper contract with `true` in ParaB
    const SELECTOR_CONSTRUCTOR: [u8; 4] = [0x9b, 0xae, 0x9d, 0x5e];
    const SELECTOR_GET: [u8; 4] = [0x2f, 0x86, 0x5b, 0xd9];
    // const SELECTOR_FLIP: [u8; 4] = [0x63, 0x3a, 0xa5, 0x51];
    const GAS_LIMIT: Weight = Weight::from_parts(100_000_000_000, 3 * 1024 * 1024);
    let mut contract_id = [0u8; 32].into();
    ParaB::execute_with(|| {
        (contract_id, _) = deploy_contract::<parachain::Runtime>(
            "flipper",
            ALICE.into(),
            0,
            GAS_LIMIT,
            None,
            // selector + true
            [SELECTOR_CONSTRUCTOR.to_vec(), vec![0x01]].concat(),
        );
        
        println!("#######Contract ID: {:?}", contract_id);
        // check for flip status
        let outcome = ParachainContracts::bare_call(
            ALICE.into(),
            contract_id.clone(),
            0,
            GAS_LIMIT,
            None,
            SELECTOR_GET.to_vec(),
            true,
            Determinism::Deterministic,
        );
        let res = outcome.result.unwrap();
        // check for revert
        assert!(res.did_revert() == false);
        // decode the return value
        let flag = Result::<bool, ()>::decode(&mut res.data.as_ref()).unwrap();
        assert_eq!(flag, Ok(true));
    });

    // Alice transfers the NFT to Bob on ParaB
    ParaA::execute_with(|| {
        let nft_multiasset: MultiAssets = vec![MultiAsset {
            id: Concrete(MultiLocation {
                parents: 1,
                interior: X1(AccountId32 {
                    network: None,
                    id: contract_id.clone().into(),
                }),
            }),
            fun: NonFungible(item.into()),
        }]
        .into();
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
            Box::new((nft_multiasset).into()),
            0,
        ));
    });
    // check for flip status, it should be false
    ParaB::execute_with(|| {
        let outcome = ParachainContracts::bare_call(
            ALICE.into(),
            contract_id.clone(),
            0,
            GAS_LIMIT,
            None,
            SELECTOR_GET.to_vec(),
            true,
            Determinism::Deterministic,
        );
        let res = outcome.result.unwrap();
        // check for revert
        assert!(res.did_revert() == false);
        // decode the return value, it should be false
        let flag = Result::<bool, ()>::decode(&mut res.data.as_ref()).unwrap();
        assert_eq!(flag, Ok(false));
    });
}
