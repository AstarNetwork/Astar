// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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

use cumulus_primitives_core::Unlimited;
use sp_runtime::traits::{BlakeTwo256, Hash, Zero};
use xcm::{
    v5::{
        Asset as XcmAsset, AssetId as XcmAssetId, Fungibility,
        Junction::{self, *},
        Junctions::*,
        Location, Parent, Xcm, VERSION as V_5,
    },
    VersionedAsset, VersionedLocation, VersionedXcm,
};
use xcm_runtime_apis::dry_run::runtime_decl_for_dry_run_api::DryRunApiV2;
use xcm_runtime_apis::fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV1;
use xcm_runtime_apis::trusted_query::runtime_decl_for_trusted_query_api::TrustedQueryApiV1;

/// Register an asset into `pallet-assets` instance, and register as as cross-chain asset.
///
/// If specified, also set _units-per-second_ to make the asset _payable_.
fn prepare_asset(asset_id: u128, location: VersionedLocation, units_per_second: Option<u128>) {
    // 1. Create an asset representation
    assert_ok!(Assets::force_create(
        RuntimeOrigin::root(),
        asset_id.into(),
        MultiAddress::Id(ALICE),
        true,
        1
    ));

    // 2. Register its location & bind it to the registered asset representation
    assert_ok!(XcAssetConfig::register_asset_location(
        RuntimeOrigin::root(),
        Box::new(location.clone()),
        asset_id.into(),
    ));

    // 3. Maybe set the units per second
    if let Some(units_per_second) = units_per_second {
        assert_ok!(XcAssetConfig::set_asset_units_per_second(
            RuntimeOrigin::root(),
            Box::new(location),
            units_per_second.into()
        ));
    }
}

#[test]
fn query_acceptable_payment_assets_is_ok() {
    new_test_ext().execute_with(|| {
        // 0. Sanity check for unsupported version
        {
            assert!(Runtime::query_acceptable_payment_assets(2).is_err());
        }

        // 1. First check the return values without any foreign asset registered.
        {
            let assets = Runtime::query_acceptable_payment_assets(V_5)
                .expect("Must return at least native currency.");
            assert_eq!(assets, vec![XcmAssetId(Location::here()).into()]);
        }

        // 2. Register two foreign assets - one payable, one not.
        // Expect native asset & payable asset to be returned.
        {
            let payable_location = Location::new(1, Here);
            let non_payable_location = Location::new(1, Junction::Parachain(2));

            prepare_asset(1, payable_location.clone().into_versioned(), Some(1000));
            prepare_asset(2, non_payable_location.clone().into_versioned(), None);

            let assets = Runtime::query_acceptable_payment_assets(V_5)
                .expect("Must return at least native currency.");

            assert_eq!(assets.len(), 2);
            assert!(assets.contains(&XcmAssetId(Location::here()).into()));
            assert!(assets.contains(&XcmAssetId(payable_location).into()));
        }
    })
}

#[test]
fn query_weight_to_asset_fee_is_ok() {
    new_test_ext().execute_with(|| {
        // 0. Sanity check for unsupported asset
        {
            let non_payable_location = Location::new(1, Here);
            assert!(Runtime::query_weight_to_asset_fee(
                Weight::from_parts(1000, 1000),
                XcmAssetId(non_payable_location.clone()).into(),
            )
            .is_err());

            prepare_asset(1, non_payable_location.clone().into_versioned(), None);
            assert!(Runtime::query_weight_to_asset_fee(
                Weight::from_parts(1000, 1000),
                XcmAssetId(non_payable_location).into(),
            )
            .is_err());
        }

        // 1. Native asset payment
        {
            let weight = Weight::from_parts(1000, 1000);
            let expected_fee = XcmWeightToFee::weight_to_fee(&weight);
            let fee =
                Runtime::query_weight_to_asset_fee(weight, XcmAssetId(Location::here()).into())
                    .expect("Must return fee for native asset.");

            assert_eq!(
                fee, expected_fee,
                "Fee must match the expected weight-to-fee conversion."
            );
        }

        // 2. Foreign asset payment
        {
            let payable_location = Location::new(2, Here);
            let units_per_second = 1_000_000_000_000;

            prepare_asset(
                2,
                payable_location.clone().into_versioned(),
                Some(units_per_second),
            );

            let weight = Weight::from_parts(1_000_000_000, 1_000_000);
            let expected_fee = XcAssetConfig::weight_to_fee(weight, units_per_second);
            let fee =
                Runtime::query_weight_to_asset_fee(weight, XcmAssetId(payable_location).into())
                    .expect("Must return fee for payable asset.");

            assert_eq!(
                fee, expected_fee,
                "Fee must match the expected weight-to-fee conversion."
            );
        }
    })
}

#[test]
fn query_xcm_weight_is_ok() {
    new_test_ext().execute_with(|| {
        let native_asset: XcmAsset =
            XcmAssetId(Location::here()).into_asset(Fungibility::Fungible(1_000_000_000));

        // Prepare an xcm sequence
        let xcm_sequence = Xcm::<()>::builder_unsafe()
            .withdraw_asset(native_asset.clone())
            .deposit_asset(
                native_asset,
                Junction::AccountId32 {
                    network: None,
                    id: BOB.clone().into(),
                },
            )
            .build();

        let weight =
            Runtime::query_xcm_weight(VersionedXcm::V5(xcm_sequence)).expect("Must return weight.");
        assert!(
            !weight.is_zero(),
            "Weight must be non-zero since we're performing asset withdraw & deposit."
        );
    })
}

#[test]
fn query_delivery_fees_is_ok() {
    new_test_ext().execute_with(|| {
        let location = Location::new(1, Here).into_versioned();

        // Prepare a dummy xcm sequence
        let xcm_sequence = Xcm::<()>::builder_unsafe()
            .clear_error()
            .unsubscribe_version()
            .build();

        // TODO: this is something we should revisit
        assert!(
            Runtime::query_delivery_fees(location, VersionedXcm::V5(xcm_sequence)).is_err(),
            "At the moment, `PriceForMessageDelivery` is not implemented."
        );
    })
}

#[test]
fn dry_run_call_is_ok() {
    new_test_ext().execute_with(|| {
        let origin = OriginCaller::system(frame_system::RawOrigin::Signed(ALICE.clone()).into());
        // TODO: Improve this test using an XCM call with more side effects and compare local_xcm with recorded one to get ride of `xcm_recorder_configuration_is_ok` test
        let call = RuntimeCall::System(frame_system::Call::remark_with_event {
            remark: vec![0u8; 32],
        });

        let result = Runtime::dry_run_call(origin, call, xcm::prelude::XCM_VERSION)
            .expect("Must return some effects.");
        assert_eq!(result.forwarded_xcms, vec![]);
        assert_eq!(
            result.emitted_events[0],
            RuntimeEvent::System(frame_system::Event::Remarked {
                sender: ALICE.into(),
                hash: BlakeTwo256::hash_of(&[0u8; 32]).into(),
            }),
        );
    })
}

#[test]
fn dry_run_xcm_is_ok() {
    new_test_ext().execute_with(|| {
        let transfer_amount = 10_000 * UNIT;
        let native_asset: XcmAsset =
            XcmAssetId(Location::here()).into_asset(Fungibility::Fungible(transfer_amount.clone()));

        // Prepare an xcm sequence
        let xcm_sequence = Xcm::<()>::builder_unsafe()
            .withdraw_asset(native_asset.clone())
            .clear_origin()
            .buy_execution((Here, 1 * UNIT), Unlimited) // TODO: This can be improved by estimating real execution fees
            .deposit_asset(
                native_asset,
                Junction::AccountId32 {
                    network: None,
                    id: BOB.clone().into(),
                },
            )
            .build();

        // ALICE location origin
        let origin_location = VersionedLocation::V5(
            Junction::AccountId32 {
                id: ALICE.into(),
                network: None,
            }
            .into(),
        );
        let dummy_message =
            Xcm::<RuntimeCall>::from(VersionedXcm::V5(xcm_sequence).try_into().unwrap());
        let versioned_xcm = VersionedXcm::V5(dummy_message);

        let result = Runtime::dry_run_xcm(origin_location, versioned_xcm)
            .expect("Must return some effects.");

        assert_eq!(result.forwarded_xcms, vec![]);

        assert_eq!(
            result.emitted_events[0],
            RuntimeEvent::Balances(pallet_balances::Event::Burned {
                who: ALICE.into(),
                amount: transfer_amount
            }),
        );
    })
}

#[test]
fn xcm_recorder_configuration_is_ok() {
    use xcm_executor::RecordXcm;

    new_test_ext().execute_with(|| {
        let result = <xcm_config::XcmConfig as xcm_executor::Config>::XcmRecorder::should_record();
        assert!(
            !result,
            "XCM recorder should NOT record incoming XCMs by default."
        );

        <xcm_config::XcmConfig as xcm_executor::Config>::XcmRecorder::set_record_xcm(true);
        let result = <xcm_config::XcmConfig as xcm_executor::Config>::XcmRecorder::should_record();
        assert!(
            result,
            "XCM recorder must be ready to record incoming XCMs."
        );
    })
}

#[test]
fn trusted_api_is_reserve_is_ok() {
    new_test_ext().execute_with(|| {
        let para_a: Location = (Parent, Parachain(1)).into();
        let para_a_asset: XcmAsset = (para_a.clone(), 10u128).into();
        let relay_asset: XcmAsset = (Parent, 10u128).into();

        // para_a origin should be trusted reserve for para_a asset
        assert_eq!(
            Runtime::is_trusted_reserve(
                VersionedAsset::V5(para_a_asset),
                VersionedLocation::V5(para_a.clone())
            ),
            Ok(true)
        );

        // relay origin should be NOT be a trusted reserve for para_a asset
        assert_eq!(
            Runtime::is_trusted_reserve(
                VersionedAsset::V5(relay_asset),
                VersionedLocation::V5(para_a)
            ),
            Ok(false)
        );
    })
}

#[test]
fn trusted_api_is_teleport_is_ok() {
    new_test_ext().execute_with(|| {
        let para_a: Location = (Parent, Parachain(1)).into();
        let para_a_asset: XcmAsset = (para_a.clone(), 10u128).into();
        let relay_asset: XcmAsset = (Parent, 10u128).into();

        // We have no trusted teleporters configured for any runtime
        assert_eq!(
            Runtime::is_trusted_teleporter(
                VersionedAsset::V5(para_a_asset),
                VersionedLocation::V5(para_a.clone())
            ),
            Ok(false)
        );
        assert_eq!(
            Runtime::is_trusted_reserve(
                VersionedAsset::V5(relay_asset),
                VersionedLocation::V5(para_a)
            ),
            Ok(false)
        );
    })
}
