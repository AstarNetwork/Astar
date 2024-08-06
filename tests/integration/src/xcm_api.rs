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

use xcm::{
    v4::{AssetId as XcmAssetId, Junction, Junctions::*, Location, VERSION as V_4},
    VersionedLocation,
};
use xcm_fee_payment_runtime_api::runtime_decl_for_xcm_payment_api::XcmPaymentApi;

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
            let result = Runtime::query_acceptable_payment_assets(2);
            assert!(result.is_err());
        }

        // 1. First check the return values without any foreign asset registered.
        {
            let assets = Runtime::query_acceptable_payment_assets(V_4)
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

            let assets = Runtime::query_acceptable_payment_assets(V_4)
                .expect("Must return at least native currency.");

            assert_eq!(assets.len(), 2);
            assert!(assets.contains(&XcmAssetId(Location::here()).into()));
            assert!(assets.contains(&XcmAssetId(payable_location).into()));
        }
    })
}
