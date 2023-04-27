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

use crate::mocks::{parachain, parachain_c, relay_chain, *};

use frame_support::{assert_ok, traits::IsType, weights::Weight};
use pallet_contracts::Determinism;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::{
    traits::{Bounded, StaticLookup},
    DispatchError,
};
use xcm::prelude::*;
use xcm_simulator::TestExt;

const GAS_LIMIT: Weight = Weight::from_parts(100_000_000_000, 3 * 1024 * 1024);
const SELECTOR_CONSTRUCTOR: [u8; 4] = [0x9b, 0xae, 0x9d, 0x5e];
const SELECTOR_SUPPLY: [u8; 4] = [0x62, 0x84, 0x13, 0xfe];

fn register_nonfungible_native<Runtime, AssetId>(
    origin: Runtime::RuntimeOrigin,
    reserve_nonfungible_location: impl Into<MultiLocation> + Clone,
    payment_asset_location: impl Into<MultiLocation> + Clone,
    payment_asset_id: AssetId,
    asset_controller: <Runtime::Lookup as StaticLookup>::Source,
    min_balance: Option<Runtime::Balance>,
) -> Result<[u8; 32], DispatchError>
where
    Runtime: pallet_xc_asset_config::Config + pallet_assets::Config,
    AssetId: IsType<<Runtime as pallet_xc_asset_config::Config>::AssetId>
        + IsType<<Runtime as pallet_assets::Config>::AssetId>
        + Clone,
{
    pallet_assets::Pallet::<Runtime>::force_create(
        origin.clone(),
        <Runtime as pallet_assets::Config>::AssetIdParameter::from(payment_asset_id.clone().into()),
        asset_controller,
        true,
        min_balance.unwrap_or(Bounded::min_value()),
    )?;

    let name: Vec<u8> = "xcDerivative".into();
    let symbol: Vec<u8> = "XD".into();
    let baseuri: Vec<u8> = "http://baseuri".into();

    let contract_id;
    (contract_id, _) = deploy_contract::<parachain_c::Runtime>(
        "xcm_nft_psp34",
        sibling_account_id(1),
        0,
        GAS_LIMIT,
        None,
        // selector + params
        [
            SELECTOR_CONSTRUCTOR.to_vec(),
            (name.clone(), symbol.clone(), baseuri.clone()).encode(),
        ]
        .concat(),
    );

    let local_contract_ml = MultiLocation {
        parents: 0,
        interior: X1(Junction::AccountId32 {
            network: None,
            id: contract_id.clone().into(),
        }),
    };
    // check for supply status
    ensure_total_supply(contract_id.clone().into(), GAS_LIMIT, 0);

    pallet_xc_asset_config::Pallet::<Runtime>::register_nonfungible_location(
        origin.clone(),
        Box::new(reserve_nonfungible_location.clone().into().into_versioned()),
        Box::new(local_contract_ml.into_versioned()),
    )?;
    println!(
        "Reserve Nonfungible location registered: {:?}",
        reserve_nonfungible_location.clone().into().into_versioned()
    );

    pallet_xc_asset_config::Pallet::<Runtime>::register_asset_location(
        origin.clone(),
        Box::new(payment_asset_location.clone().into().into_versioned()),
        payment_asset_id.into(),
    )?;

    pallet_xc_asset_config::Pallet::<Runtime>::set_asset_units_per_second(
        origin,
        Box::new(payment_asset_location.into().into_versioned()),
        1_000_000_000_000,
    )?;

    Ok(contract_id.into())
}

/// Check expected total supply
fn ensure_total_supply(contract_id: [u8; 32], gas_limit: Weight, expected_supply: u64) {
    let outcome = NftParachainContracts::bare_call(
        ALICE.into(),
        contract_id.into(),
        0,
        gas_limit,
        None,
        SELECTOR_SUPPLY.to_vec(),
        true,
        Determinism::Deterministic,
    );
    let res = outcome.result.unwrap();
    assert!(res.did_revert() == false);
    let supply = Result::<u64, ()>::decode(&mut res.data.as_ref()).unwrap();
    assert_eq!(supply, Ok(expected_supply));
    println!("Total Supply: {:?}", supply);
}

// get contract owner
// fn get_contract_owner(contract_id: [u8; 32]) -> [u8; 32]{
//     const SELECTOR_GET_OWNER: [u8; 4] = [0x4f, 0xa4, 0x3c, 0x8c];
//     let outcome = NftParachainContracts::bare_call(
//         ALICE.into(),
//         contract_id.into(),
//         0,
//         GAS_LIMIT,
//         None,
//         SELECTOR_GET_OWNER.to_vec(),
//         true,
//         Determinism::Deterministic,
//     );
//     let res = outcome.result.unwrap();
//     assert!(res.did_revert() == false);
//     let owner = Result::<[u8; 32], ()>::decode(&mut res.data.as_ref()).unwrap();
//     println!("Contract Owner: {:?}", owner);

//     owner.unwrap()
// }

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
    let collection_junction = X3(
        Parachain(1),
        PalletInstance(uniques_pallet_instance),
        GeneralIndex(1u128),
    );
    let reserve_collection_ml: MultiLocation = MultiLocation {
        parents: 1,
        interior: collection_junction,
    };
    let item = 42;
    let mut contract_id = [0u8; 32].into();
    ParaC::execute_with(|| {
        let sibling_asset_id = 123 as u128;
        let para_a_multiloc = (Parent, Parachain(1));

        // On parachain C create an asset which represents a derivative of parachain A native asset.
        contract_id = register_nonfungible_native::<parachain_c::Runtime, _>(
            parachain_c::RuntimeOrigin::root(),
            reserve_collection_ml,
            para_a_multiloc.clone(),
            sibling_asset_id,
            sibling_account_id(1),
            Some(1),
        )
        .unwrap();
    });

    // Alice mints and transfers the NFT to Alice on ParaC
    ParaA::execute_with(|| {
        println!("--------------ParaA reserve_transfer_assets  -------------\n");
        let collection_junction = X2(PalletInstance(uniques_pallet_instance), GeneralIndex(1u128));
        let collection_ml: MultiLocation = MultiLocation {
            parents: 0,
            interior: collection_junction,
        };
        // Mint nft on ParaA
        use parachain::{RuntimeOrigin, Uniques};
        assert_ok!(Uniques::force_create(
            RuntimeOrigin::root(),
            collection_ml,
            ALICE,
            true
        ));
        assert_ok!(Uniques::mint(
            RuntimeOrigin::signed(ALICE),
            collection_ml,
            Index(item),
            ALICE
        ));
        // Alice owns an NFT on the ParaA chain
        assert_eq!(Uniques::owner(collection_ml, Index(item)), Some(ALICE));

        // Create MultiAssets needed for the transfer
        let nft_multiasset: MultiAsset = MultiAsset {
            id: Concrete(MultiLocation {
                parents: 0,
                interior: collection_junction,
            }),
            fun: NonFungible(Index(item)),
        };
        let native_multiasset: MultiAsset = MultiAsset {
            id: Concrete(MultiLocation {
                parents: 0,
                interior: Here,
            }),
            fun: Fungible(900_000_000_000),
        };

        let all_assets: Vec<MultiAsset> = vec![native_multiasset.clone(), nft_multiasset.clone()];

        // Alice transfers the NFT to ParaC
        assert_ok!(ParachainPalletXcm::reserve_transfer_assets(
            parachain::RuntimeOrigin::signed(ALICE),
            Box::new(MultiLocation::new(1, X1(Parachain(3))).into()),
            Box::new(
                MultiLocation::new(
                    0,
                    X1(AccountId32 {
                        network: None,
                        id: ALICE.into()
                    })
                )
                .into_versioned()
            ),
            Box::new((all_assets).into()),
            0,
        ));
        // println!("--------------ParaA Events -------------\n");
        // for e in parachain::System::events() {
        //     println!("A {:?}\n", e.event);
        // }
    });

    // There should be increase in total supply
    ParaC::execute_with(|| {
        ensure_total_supply(contract_id.clone().into(), GAS_LIMIT, 1);
    });
}

// xcm::execute_xcm:
// origin: MultiLocation { parents: 1, interior: X1(Parachain(1)) },
// message: Xcm([
//     ReserveAssetDeposited(
//         MultiAssets([
//             MultiAsset { id: Concrete(MultiLocation { parents: 1, interior: X1(Parachain(1)) }),
//             fun: Fungible(900000000000) },
//             MultiAsset { id: Concrete(MultiLocation { parents: 1, interior: X3(Parachain(1), PalletInstance(13), GeneralIndex(1)) }),
//             fun: NonFungible(Index(42)) }])),
//     ClearOrigin,
//     BuyExecution {
//         fees: MultiAsset { id: Concrete(MultiLocation { parents: 1, interior: X1(Parachain(1)) }),
//         fun: Fungible(900000000000) }, weight_limit: Limited(Weight { ref_time: 40, proof_size: 0 }) },
//     DepositAsset {
//         assets: Wild(AllCounted(2)),
//         beneficiary: MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: [250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250] }) } }]),
// weight_limit: Weight { ref_time: 18446744073709551615, proof_size: 18446744073709551615 }
