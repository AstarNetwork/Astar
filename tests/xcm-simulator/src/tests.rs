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

use frame_support::assert_ok;
use parity_scale_codec::Encode;
use xcm::latest::prelude::*;
use xcm_simulator::TestExt;

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
                origin_type: OriginKind::SovereignAccount,
                require_weight_at_most: 1_000_000_000 as u64,
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
                origin_type: OriginKind::SovereignAccount,
                require_weight_at_most: 1_000_000_000 as u64,
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
fn para_to_para_reserve_transfer() {
    MockNet::reset();

    let sibling_asset_id = 123 as u128;
    let para_a_multiloc = Box::new(MultiLocation::new(1, X1(Parachain(1))).versioned());

    // On parachain B create an asset which representes a derivative of parachain A native asset.
    // This asset is allowed as XCM execution fee payment asset.
    ParaB::execute_with(|| {
        assert_ok!(parachain::Assets::force_create(
            parachain::RuntimeOrigin::root(),
            sibling_asset_id,
            sibling_para_account_id(1),
            true,
            1
        ));
        assert_ok!(ParachainXcAssetConfig::register_asset_location(
            parachain::RuntimeOrigin::root(),
            para_a_multiloc.clone(),
            sibling_asset_id
        ));
        assert_ok!(ParachainXcAssetConfig::set_asset_units_per_second(
            parachain::RuntimeOrigin::root(),
            para_a_multiloc,
            1_000_000_000_000, // each unit of weight charged exactly 1
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
                    network: Any,
                    id: ALICE.into()
                })
                .into()
                .into()
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
    ParaB::execute_with(|| {
        // Ensure Alice received assets on ParaB (sent amount minus expenses)
        let four_instructions_execution_cost = 4 * parachain::UnitWeightCost::get() as u128;
        assert_eq!(
            parachain::Assets::balance(sibling_asset_id, ALICE),
            withdraw_amount - four_instructions_execution_cost
        );
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
            sibling_para_account_id(1),
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
                WithdrawAsset((Here, 100_000_000_000).into()),
                BuyExecution {
                    fees: (Here, 100_000_000_000).into(),
                    weight_limit: Unlimited
                },
                Transact {
                    origin_type: OriginKind::SovereignAccount,
                    require_weight_at_most: 1_000_000_000 as u64,
                    call: proxy_call.encode().into(),
                }
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
                WithdrawAsset((Here, 100_000_000_000).into()),
                BuyExecution {
                    fees: (Here, 100_000_000_000).into(),
                    weight_limit: Unlimited
                },
                Transact {
                    origin_type: OriginKind::SovereignAccount,
                    require_weight_at_most: 1_000_000_000 as u64,
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
