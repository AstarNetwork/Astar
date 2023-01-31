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

use codec::Encode;
use frame_support::assert_ok;
use xcm::latest::prelude::*;
use xcm_simulator::TestExt;

// // Helper function for forming buy execution message
// fn buy_execution<C>(fees: impl Into<MultiAsset>) -> Instruction<C> {
//     BuyExecution {
//         fees: fees.into(),
//         weight_limit: Unlimited,
//     }
// }

#[test]
fn basic_dmp() {
    MockNet::reset();

    let remark = parachain::RuntimeCall::System(
        frame_system::Call::<parachain::Runtime>::remark_with_event {
            remark: vec![1, 2, 3],
        },
    );
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

    // Create asset and register it as cross-chain & payable
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
            1_000_000_000_000, // each unit of weight charged exactly 1 TODO: make this common & document it
        ));
    });

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

    ParaB::execute_with(|| {
        // Ensure Alice received assets on ParaB (sent amount minus expenses)
        let four_instructions_execution_cost = 4 * parachain::UnitWeightCost::get() as u128;
        assert_eq!(
            parachain::Assets::balance(sibling_asset_id, ALICE),
            withdraw_amount - four_instructions_execution_cost
        );
    });
}
