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

use crate::mocks::{parachain_c, parachain, relay_chain, *};

use frame_support::{assert_ok, weights::Weight};
use pallet_contracts::Determinism;
use parity_scale_codec::{Decode, Encode};

use xcm::prelude::*;
use xcm_simulator::TestExt;

#[test]
fn basic_dmp() {
    MockNet::reset();

    let remark = parachain_c::RuntimeCall::System(
        frame_system::Call::<parachain_c::Runtime>::remark_with_event {
            remark: vec![1, 2, 3],
        },
    );

    // A remote `Transact` is sent to the parachain A.
    // No need to pay for the execution time since parachain is configured to allow unpaid execution from parents.
    Relay::execute_with(|| {
        assert_ok!(RelayChainPalletXcm::send_xcm(
            Here,
            Parachain(3),
            Xcm(vec![Transact {
                origin_kind: OriginKind::SovereignAccount,
                require_weight_at_most: Weight::from_parts(1_000_000_000, 1024 * 1024),
                call: remark.encode().into(),
            }]),
        ));
    });

    // Execute remote transact and verify that `Remarked` event is emitted.
    ParaC::execute_with(|| {
        use parachain_c::{RuntimeEvent, System};
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
    ParaC::execute_with(|| {
        assert_ok!(NftParachainPalletXcm::send_xcm(
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

    let remark = parachain_c::RuntimeCall::System(
        frame_system::Call::<parachain_c::Runtime>::remark_with_event {
            remark: vec![1, 2, 3],
        },
    );
    ParaA::execute_with(|| {
        assert_ok!(ParachainPalletXcm::send_xcm(
            Here,
            (Parent, Parachain(3)),
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

    ParaC::execute_with(|| {
        use parachain_c::{RuntimeEvent, System};
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::System(frame_system::Event::Remarked { .. })
        )));
    });

    ParaC::execute_with(|| {
        assert_ok!(NftParachainPalletXcm::send_xcm(
            Here,
            (Parent, Parachain(1)),
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

    ParaA::execute_with(|| {
        use parachain::{RuntimeEvent, System};
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::System(frame_system::Event::Remarked { .. })
        )));
    });
}


/// Scenario:
/// User transfers an NFT from ParaA to ParaC.
/// NFT is first minted on ParaA pallet-uniques.
/// On ParaC, a derivative NFT is minted on smart contract.
#[test]
fn transfer_nft_to_smart_contract() {
    MockNet::reset();
    let uniques_pallet_instance = 13u8;
    let collection_ml: MultiLocation = MultiLocation {
        parents: 0,
        interior: X2(PalletInstance(uniques_pallet_instance), GeneralIndex(1u128)),
    };
    let item = 42;

    // let para_a_collection_location = MultiLocation {
    //     parents: 1,
    //     // interior: Here,
    //     // interior: X1(Parachain(1)),
    //     interior: X3(Parachain(1), PalletInstance(uniques_pallet_instance), GeneralIndex(collection.into())),
    // };
    
    // let sibling_asset_id = 123 as u128;
    // let para_a_multiloc = (Parent, Parachain(1));

    // Deploy and initialize flipper contract with `true` in ParaC
    const SELECTOR_CONSTRUCTOR: [u8; 4] = [0x9b, 0xae, 0x9d, 0x5e];
    const SELECTOR_GET: [u8; 4] = [0x2f, 0x86, 0x5b, 0xd9];
    // const SELECTOR_FLIP: [u8; 4] = [0x63, 0x3a, 0xa5, 0x51];
    const GAS_LIMIT: Weight = Weight::from_parts(100_000_000_000, 3 * 1024 * 1024);
    let mut contract_id = [0u8; 32].into();
    ParaC::execute_with(|| {
        (contract_id, _) = deploy_contract::<parachain_c::Runtime>(
            "flipper",
            ALICE.into(),
            0,
            GAS_LIMIT,
            None,
            // selector + true
            [SELECTOR_CONSTRUCTOR.to_vec(), vec![0x01]].concat(),
        );

        println!("####### ParaC deployed Contract ID: {:?}", contract_id);
        // check for flip status
        let outcome = NftParachainContracts::bare_call(
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

        // Register ParaA nft item as asset on ParaC
    
        _ = pallet_xc_asset_config::Pallet::<parachain_c::Runtime>::register_asset_location(
            parachain_c::RuntimeOrigin::root(),
            Box::new(collection_ml.clone().into_versioned()),
            item,
        );

        _ = pallet_xc_asset_config::Pallet::<parachain_c::Runtime>::set_asset_units_per_second(
            parachain_c::RuntimeOrigin::root(),
            Box::new(collection_ml.into_versioned()),
            1_000_000_000_000,
        );
        println!("####### ParaC registered asset: {:?}", collection_ml);

    });

    // Alice transfers the NFT to Bob on ParaC
    ParaA::execute_with(|| {
        // Mint nft for to be transferred
        use parachain::{RuntimeOrigin, Uniques};
        assert_eq!(
            Uniques::force_create(RuntimeOrigin::root(), collection_ml, ALICE, true),
            Ok(())
        );
        assert_eq!(
            Uniques::mint(RuntimeOrigin::signed(ALICE), collection_ml, Index(item), child_account_id(1)),
            Ok(())
        );
        assert_eq!(Uniques::owner(collection_ml, Index(item)), Some(child_account_id(1)));

        // Alice owns an NFT on the ParaA chain. Pre-minted in mod.rs
        assert_eq!(
            parachain::Uniques::owner(collection_ml, Index(item)),
            Some(child_account_id(1))
        );
        let nft_multiasset: MultiAsset = MultiAsset {
            id: Concrete(collection_ml),
            fun: NonFungible(Index(item)),
        };
        let native_multiasset: MultiAsset = MultiAsset {
            id: Concrete(MultiLocation {
                parents: 0,
                interior: Here,
            }),
            fun: Fungible(1_000_000),
        };
        let all_assets: Vec<MultiAsset> = vec![nft_multiasset.clone(), native_multiasset.clone()];
        assert_ok!(ParachainPalletXcm::reserve_transfer_assets(
            parachain::RuntimeOrigin::signed(ALICE),
            Box::new(MultiLocation::new(1, X1(Parachain(3))).into()),
            Box::new(
                X1(AccountId32 {
                    network: None,
                    id: ALICE.into()
                })
                .into_location()
                .into_versioned()
            ),
            Box::new((all_assets.clone()).into()),
            0,
        ));
        println!("####### ParaA reserve_transfer_assets sent, all_assets: {:?}", all_assets);

    });
    // check for flip status, it should be false
    ParaC::execute_with(|| {
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

// xcm::execute_xcm: origin: MultiLocation { parents: 1, interior: X1(Parachain(1)) },
// message: Xcm([
//     ReserveAssetDeposited(
//                 MultiAssets([MultiAsset { id: Concrete(MultiLocation { parents: 1, interior: X1(Parachain(1)) }),
//                                 fun: Fungible(1000000) },
//                             MultiAsset { id: Concrete(MultiLocation { parents: 1, interior: X1(AccountId32 { network: None, id: [246, 106, 229, 81, 70, 154, 31, 201, 19, 66, 83, 186, 54, 229, 40, 18, 106, 241, 228, 219, 151, 28, 138, 38, 201, 239, 192, 139, 235, 162, 88, 245] }) }),
//                                 fun: NonFungible(Index(42)) }
//                             ])
//                         ),
//     ClearOrigin,
//     BuyExecution { fees: MultiAsset { id: Concrete(MultiLocation { parents: 1, interior: X1(Parachain(1)) }),
//                                       fun: Fungible(1000000) },
//                     weight_limit: Limited(Weight { ref_time: 40, proof_size: 0 })
//                 },
//     DepositAsset { assets: Wild(AllCounted(2)),
//                     beneficiary: MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: [250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250]
//                     }
//     ) } }]),
// weight_limit: Weight { ref_time: 18446744073709551615, proof_size: 18446744073709551615 }
