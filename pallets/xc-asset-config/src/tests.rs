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

use super::{pallet::Error, pallet::Event, *};
use frame_support::{assert_noop, assert_ok, WeakBoundedVec};
use mock::*;
use sp_runtime::traits::BadOrigin;
use xcm::latest::prelude::*;

use xcm::{v3::MultiLocation, VersionedMultiLocation};

#[test]
fn only_root_as_origin() {
    ExternalityBuilder::build().execute_with(|| {
        let asset_location = MultiLocation::here().into_versioned();
        let asset_id = 7;

        assert_noop!(
            XcAssetConfig::register_asset_location(
                RuntimeOrigin::signed(1),
                Box::new(asset_location.clone()),
                asset_id
            ),
            BadOrigin
        );

        assert_noop!(
            XcAssetConfig::set_asset_units_per_second(
                RuntimeOrigin::signed(1),
                Box::new(asset_location.clone()),
                9
            ),
            BadOrigin
        );

        assert_noop!(
            XcAssetConfig::change_existing_asset_location(
                RuntimeOrigin::signed(1),
                Box::new(asset_location.clone()),
                asset_id
            ),
            BadOrigin
        );

        assert_noop!(
            XcAssetConfig::remove_payment_asset(
                RuntimeOrigin::signed(1),
                Box::new(asset_location.clone()),
            ),
            BadOrigin
        );

        assert_noop!(
            XcAssetConfig::remove_asset(RuntimeOrigin::signed(1), asset_id,),
            BadOrigin
        );
    })
}

#[test]
fn register_asset_location_and_units_per_sec_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // Prepare location and Id
        let asset_location = MultiLocation::new(
            1,
            Junctions::X2(Junction::PalletInstance(17), GeneralIndex(7)),
        );
        let asset_id = 13;

        // Register asset and ensure it's ok
        assert_ok!(XcAssetConfig::register_asset_location(
            RuntimeOrigin::root(),
            Box::new(asset_location.clone().into_versioned()),
            asset_id
        ));
        System::assert_last_event(mock::RuntimeEvent::XcAssetConfig(Event::AssetRegistered {
            asset_location: asset_location.clone().into_versioned(),
            asset_id: asset_id,
        }));

        // Assert storage state after registering asset
        assert_eq!(
            AssetIdToLocation::<Test>::get(&asset_id).unwrap(),
            asset_location.clone().into_versioned()
        );
        assert_eq!(
            AssetLocationToId::<Test>::get(asset_location.clone().into_versioned()).unwrap(),
            asset_id
        );
        assert!(!AssetLocationUnitsPerSecond::<Test>::contains_key(
            asset_location.clone().into_versioned()
        ));

        // Register unit per second rate and verify storage
        let units: u128 = 7 * 11 * 13 * 17 * 29;
        assert_ok!(XcAssetConfig::set_asset_units_per_second(
            RuntimeOrigin::root(),
            Box::new(asset_location.clone().into_versioned()),
            units
        ));
        System::assert_last_event(mock::RuntimeEvent::XcAssetConfig(
            Event::UnitsPerSecondChanged {
                asset_location: asset_location.clone().into_versioned(),
                units_per_second: units,
            },
        ));
        assert_eq!(
            AssetLocationUnitsPerSecond::<Test>::get(&asset_location.clone().into_versioned())
                .unwrap(),
            units
        );
    })
}

#[test]
fn asset_is_already_registered() {
    ExternalityBuilder::build().execute_with(|| {
        // Prepare location and Id
        let asset_location = MultiLocation::new(
            1,
            Junctions::X2(Junction::PalletInstance(17), GeneralIndex(7)),
        );
        let asset_id = 13;

        // Register asset and ensure it's ok
        assert_ok!(XcAssetConfig::register_asset_location(
            RuntimeOrigin::root(),
            Box::new(asset_location.clone().into_versioned()),
            asset_id
        ));

        // Now repeat the process and expect an error
        assert_noop!(
            XcAssetConfig::register_asset_location(
                RuntimeOrigin::root(),
                Box::new(asset_location.clone().into_versioned()),
                asset_id
            ),
            Error::<Test>::AssetAlreadyRegistered
        );
    })
}

#[test]
fn change_asset_location_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // Prepare location, Id and units
        let asset_location = MultiLocation::new(1, Junctions::X1(Junction::Parachain(2007)));
        let asset_id = 17;
        let units: u128 = 3 * 11 * 13 * 17;

        // Register asset and ups
        assert_ok!(XcAssetConfig::register_asset_location(
            RuntimeOrigin::root(),
            Box::new(asset_location.clone().into_versioned()),
            asset_id
        ));
        assert_ok!(XcAssetConfig::set_asset_units_per_second(
            RuntimeOrigin::root(),
            Box::new(asset_location.clone().into_versioned()),
            units
        ));

        // Change the asset location and assert change was successful
        let new_asset_location = MultiLocation::new(2, Junctions::X1(Junction::PalletInstance(3)));
        assert_ne!(new_asset_location, asset_location); // sanity check

        assert_ok!(XcAssetConfig::change_existing_asset_location(
            RuntimeOrigin::root(),
            Box::new(new_asset_location.clone().into_versioned()),
            asset_id
        ));
        System::assert_last_event(mock::RuntimeEvent::XcAssetConfig(
            Event::AssetLocationChanged {
                previous_asset_location: asset_location.clone().into_versioned(),
                asset_id: asset_id,
                new_asset_location: new_asset_location.clone().into_versioned(),
            },
        ));

        // Assert storage state
        assert_eq!(
            AssetIdToLocation::<Test>::get(&asset_id).unwrap(),
            new_asset_location.clone().into_versioned()
        );
        assert_eq!(
            AssetLocationToId::<Test>::get(new_asset_location.clone().into_versioned()).unwrap(),
            asset_id
        );

        // This should have been deleted
        assert!(!AssetLocationUnitsPerSecond::<Test>::contains_key(
            asset_location.clone().into_versioned()
        ));
        assert_eq!(
            AssetLocationUnitsPerSecond::<Test>::get(new_asset_location.clone().into_versioned())
                .unwrap(),
            units
        );
    })
}

#[test]
fn remove_payment_asset_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // Prepare location, Id and units
        let asset_location = MultiLocation::new(1, Junctions::X1(Junction::Parachain(2007)));
        let asset_id = 17;
        let units: u128 = 3 * 11 * 13 * 17;

        // Register asset and ups
        assert_ok!(XcAssetConfig::register_asset_location(
            RuntimeOrigin::root(),
            Box::new(asset_location.clone().into_versioned()),
            asset_id
        ));
        assert_ok!(XcAssetConfig::set_asset_units_per_second(
            RuntimeOrigin::root(),
            Box::new(asset_location.clone().into_versioned()),
            units
        ));

        // Now we remove supported asset
        assert_ok!(XcAssetConfig::remove_payment_asset(
            RuntimeOrigin::root(),
            Box::new(asset_location.clone().into_versioned()),
        ));
        System::assert_last_event(mock::RuntimeEvent::XcAssetConfig(
            Event::SupportedAssetRemoved {
                asset_location: asset_location.clone().into_versioned(),
            },
        ));
        assert!(!AssetLocationUnitsPerSecond::<Test>::contains_key(
            asset_location.clone().into_versioned()
        ));

        // Repeated calls don't do anything
        assert_ok!(XcAssetConfig::remove_payment_asset(
            RuntimeOrigin::root(),
            Box::new(asset_location.clone().into_versioned()),
        ));
    })
}

#[test]
fn remove_asset_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // Prepare location, Id and units
        let asset_location = MultiLocation::new(1, Junctions::X1(Junction::Parachain(2007)));
        let asset_id = 17;
        let units: u128 = 3 * 11 * 13 * 17;

        // Register asset and ups
        assert_ok!(XcAssetConfig::register_asset_location(
            RuntimeOrigin::root(),
            Box::new(asset_location.clone().into_versioned()),
            asset_id
        ));
        assert_ok!(XcAssetConfig::set_asset_units_per_second(
            RuntimeOrigin::root(),
            Box::new(asset_location.clone().into_versioned()),
            units
        ));

        // Remove asset entirely and assert op is ok
        assert_ok!(XcAssetConfig::remove_asset(RuntimeOrigin::root(), asset_id,));
        System::assert_last_event(mock::RuntimeEvent::XcAssetConfig(Event::AssetRemoved {
            asset_location: asset_location.clone().into_versioned(),
            asset_id: asset_id,
        }));

        // Assert that storage is empty after successful removal
        assert!(!AssetIdToLocation::<Test>::contains_key(asset_id));
        assert!(!AssetLocationToId::<Test>::contains_key(
            asset_location.clone().into_versioned()
        ));
        assert!(!AssetLocationUnitsPerSecond::<Test>::contains_key(
            asset_location.clone().into_versioned()
        ));
    })
}

#[test]
fn not_registered_asset_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // Prepare location, Id and units
        let asset_location = MultiLocation::parent();
        let asset_id = 17;
        let units: u128 = 3 * 11 * 13 * 17;

        assert_noop!(
            XcAssetConfig::set_asset_units_per_second(
                RuntimeOrigin::root(),
                Box::new(asset_location.clone().into_versioned()),
                units
            ),
            Error::<Test>::AssetDoesNotExist
        );

        assert_noop!(
            XcAssetConfig::change_existing_asset_location(
                RuntimeOrigin::root(),
                Box::new(asset_location.clone().into_versioned()),
                asset_id
            ),
            Error::<Test>::AssetDoesNotExist
        );

        assert_noop!(
            XcAssetConfig::remove_asset(RuntimeOrigin::root(), asset_id,),
            Error::<Test>::AssetDoesNotExist
        );
    })
}

#[test]
fn public_interfaces_are_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // Prepare location, Id and units
        let asset_location = MultiLocation::parent();
        let asset_id = 17;
        let units: u128 = 3 * 11 * 13 * 17;

        // Initially, expect `None` to be returned for all
        assert!(XcAssetConfig::get_xc_asset_location(asset_id).is_none());
        assert!(XcAssetConfig::get_asset_id(asset_location.clone()).is_none());
        assert!(XcAssetConfig::get_units_per_second(asset_location.clone()).is_none());

        // Register asset and expect values to be returned but UPS should still be `None`
        assert_ok!(XcAssetConfig::register_asset_location(
            RuntimeOrigin::root(),
            Box::new(asset_location.clone().into_versioned()),
            asset_id
        ));
        assert_eq!(
            XcAssetConfig::get_xc_asset_location(asset_id),
            Some(asset_location.clone())
        );
        assert_eq!(
            XcAssetConfig::get_asset_id(asset_location.clone()),
            Some(asset_id)
        );
        assert!(XcAssetConfig::get_units_per_second(asset_location.clone()).is_none());

        // Register ups and expect value value to be returned
        assert_ok!(XcAssetConfig::set_asset_units_per_second(
            RuntimeOrigin::root(),
            Box::new(asset_location.clone().into_versioned()),
            units
        ));
        assert_eq!(
            XcAssetConfig::get_units_per_second(asset_location.clone()),
            Some(units)
        );
    })
}

#[test]
fn different_xcm_versions_are_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // Prepare location and Id
        let legacy_asset_location = xcm::v2::MultiLocation::parent();
        let new_asset_location = xcm::v3::MultiLocation::parent();
        let asset_id = 17;

        // Register asset using legacy multilocation
        assert_ok!(XcAssetConfig::register_asset_location(
            RuntimeOrigin::root(),
            Box::new(VersionedMultiLocation::V2(legacy_asset_location.clone())),
            asset_id
        ));

        // Ensure that the new format is properly returned
        assert_eq!(
            XcAssetConfig::get_xc_asset_location(asset_id),
            Some(new_asset_location.clone())
        );
    })
}

#[test]
fn incompatible_versioned_multilocations_are_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // MultiLocation that cannot be converted from v2 to v3
        let incompatible_asset_location = xcm::v2::MultiLocation {
            parents: 1,
            interior: xcm::v2::Junctions::X1(xcm::v2::Junction::GeneralKey(
                WeakBoundedVec::<_, _>::force_from([123_u8; 33].to_vec(), None),
            )),
        };
        let asset_id = 123;

        assert_noop!(
            XcAssetConfig::register_asset_location(
                RuntimeOrigin::root(),
                Box::new(VersionedMultiLocation::V2(
                    incompatible_asset_location.clone()
                )),
                asset_id
            ),
            Error::<Test>::MultiLocationNotSupported
        );

        assert_noop!(
            XcAssetConfig::set_asset_units_per_second(
                RuntimeOrigin::root(),
                Box::new(VersionedMultiLocation::V2(
                    incompatible_asset_location.clone()
                )),
                12345,
            ),
            Error::<Test>::MultiLocationNotSupported
        );

        assert_noop!(
            XcAssetConfig::change_existing_asset_location(
                RuntimeOrigin::root(),
                Box::new(VersionedMultiLocation::V2(
                    incompatible_asset_location.clone()
                )),
                12345,
            ),
            Error::<Test>::MultiLocationNotSupported
        );

        assert_noop!(
            XcAssetConfig::remove_payment_asset(
                RuntimeOrigin::root(),
                Box::new(VersionedMultiLocation::V2(
                    incompatible_asset_location.clone()
                )),
            ),
            Error::<Test>::MultiLocationNotSupported
        );
    })
}
