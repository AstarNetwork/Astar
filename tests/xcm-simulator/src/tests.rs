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

use crate::mocks::{parachain, relay_chain, statemint_like, *};

use frame_support::{
    assert_ok,
    traits::{IsType, PalletInfoAccess},
    weights::Weight,
};
use parity_scale_codec::Encode;
use sp_runtime::{
    traits::{Bounded, StaticLookup},
    DispatchResult,
};
use xcm::prelude::*;
use xcm_executor::traits::Convert;
use xcm_simulator::TestExt;

fn register_asset<Runtime, AssetId>(
    origin: Runtime::RuntimeOrigin,
    asset_id: AssetId,
    asset_location: impl Into<MultiLocation> + Clone,
    asset_controller: <Runtime::Lookup as StaticLookup>::Source,
    is_sufficent: Option<bool>,
    initial_balance: Option<Runtime::Balance>,
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
        initial_balance.unwrap_or(Bounded::min_value()),
    )?;

    pallet_xc_asset_config::Pallet::<Runtime>::register_asset_location(
        origin.clone(),
        Box::new(asset_location.clone().into().into_versioned()),
        asset_id.into(),
    )?;

    pallet_xc_asset_config::Pallet::<Runtime>::set_asset_units_per_second(
        origin,
        Box::new(asset_location.into().into_versioned()),
        units_per_second.unwrap_or(1_000_000_000_000), // each unit of weight charged exactly 1
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
fn para_to_para_reserve_transfer() {
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
    ParaB::execute_with(|| {
        // Ensure Alice received assets on ParaB (sent amount minus expenses)
        let four_instructions_execution_cost = parachain::UnitWeightCost::get() * 4;
        assert_eq!(
            parachain::Assets::balance(sibling_asset_id, ALICE),
            withdraw_amount - four_instructions_execution_cost.ref_time() as u128
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

#[test]
fn test_statemint_like() {
    MockNet::reset();

    let dest_para = (Parent, Parachain(1));

    let sov = xcm_builder::SiblingParachainConvertsVia::<
        polkadot_parachain::primitives::Sibling,
        statemint_like::AccountId,
    >::convert_ref(&dest_para.into())
    .unwrap();

    let statemint_asset_a_balances = MultiLocation::new(
        1,
        X3(
            Parachain(4),
            PalletInstance(5),
            xcm::latest::prelude::GeneralIndex(0u128),
        ),
    );
    let source_id = 123;

    ParaA::execute_with(|| {
        assert_ok!(register_asset::<parachain::Runtime, _>(
            parachain::RuntimeOrigin::root(),
            source_id,
            statemint_asset_a_balances,
            sibling_account_id(4),
            Some(true),
            Some(1),
            // free execution
            Some(0)
        ));
    });

    Statemint::execute_with(|| {
        // Set new prefix
        statemint_like::PrefixChanger::set_prefix(
            PalletInstance(<StatemintAssets as PalletInfoAccess>::index() as u8).into(),
        );

        assert_ok!(StatemintAssets::create(
            statemint_like::RuntimeOrigin::signed(ALICE),
            0,
            ALICE,
            1
        ));

        assert_ok!(StatemintAssets::mint(
            statemint_like::RuntimeOrigin::signed(ALICE),
            0,
            ALICE,
            300000000000000
        ));

        // This is needed, since the asset is created as non-sufficient
        assert_ok!(StatemintBalances::transfer(
            statemint_like::RuntimeOrigin::signed(ALICE),
            sov,
            100000000000000
        ));

        // Actually send relay asset to parachain
        let dest: MultiLocation = AccountId32 {
            network: None,
            id: ALICE.into(),
        }
        .into();

        // Send with new prefix
        assert_ok!(StatemintPalletXcm::reserve_transfer_assets(
            statemint_like::RuntimeOrigin::signed(ALICE),
            Box::new(MultiLocation::new(1, X1(Parachain(1))).into()),
            Box::new(VersionedMultiLocation::V3(dest).clone().into()),
            Box::new(
                (
                    X2(
                        xcm::latest::prelude::PalletInstance(
                            <StatemintAssets as PalletInfoAccess>::index() as u8
                        ),
                        xcm::latest::prelude::GeneralIndex(0),
                    ),
                    123
                )
                    .into()
            ),
            0,
        ));
    });

    ParaA::execute_with(|| {
        assert_eq!(ParachainAssets::balance(source_id, &ALICE.into()), 123);
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
