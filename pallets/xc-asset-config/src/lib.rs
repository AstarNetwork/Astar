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

//! # Cross-chain Asset Config Pallet
//!
//! ## Overview
//!
//! This pallet provides mappings between local asset Id and remove asset location.
//! E.g. a multilocation like `{parents: 0, interior: X1::(Junction::Parachain(1000))}` could ba mapped to local asset Id `789`.
//!
//! The pallet ensures that the latest MultiLocation version is always used. Developers must ensure to properly migrate legacy versions
//! to newest when they become available.
//!
//! Additionally, it stores information whether a foreign asset is supported as a payment currency for execution on local network.
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//! - `register_asset_location` - used to register mapping between local asset Id and remote asset location
//! - `set_asset_units_per_second` - registers asset as payment currency and sets the desired payment per second of execution time
//! - `change_existing_asset_location` - changes the remote location of an existing local asset Id
//! - `remove_payment_asset` - removes asset from the set of supported payment assets
//! - `remove_asset` - removes all information related to this asset
//!
//! User is encouraged to refer to specific function implementations for more comprehensive documentation.
//!
//! ### Other
//!
//! `AssetLocationGetter` interface for mapping asset Id to asset location and vice versa
//! - `get_xc_asset_location`
//! - `get_asset_id`
//!
//! `ExecutionPaymentRate` interface for fetching `units per second` if asset is supported payment asset
//! - `get_units_per_second`
//!

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet;
pub use pallet::*;

#[cfg(any(test, feature = "runtime-benchmarks"))]
mod benchmarking;

#[cfg(test)]
pub mod mock;
#[cfg(test)]
pub mod tests;

pub mod migrations;

pub mod weights;
pub use weights::WeightInfo;

#[pallet]
pub mod pallet {

    use crate::weights::WeightInfo;
    use frame_support::{pallet_prelude::*, traits::EnsureOrigin};
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::HasCompact;
    use sp_std::boxed::Box;
    use xcm::{v3::MultiLocation, VersionedMultiLocation};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    /// Callback definition trait for cross-chain asset registration/deregistration notifications.
    pub trait XcAssetChanged<T: Config> {
        /// Will be called by pallet when new asset Id has been registered
        fn xc_asset_registered(asset_id: T::AssetId);

        /// Will be called by pallet when asset Id has been unregistered
        fn xc_asset_unregistered(asset_id: T::AssetId);
    }

    /// Implementation that does nothing
    impl<T: Config> XcAssetChanged<T> for () {
        fn xc_asset_registered(_: T::AssetId) {}
        fn xc_asset_unregistered(_: T::AssetId) {}
    }

    /// Defines conversion between asset Id and cross-chain asset location
    pub trait XcAssetLocation<AssetId> {
        /// Get asset type from assetId
        fn get_xc_asset_location(asset_id: AssetId) -> Option<MultiLocation>;

        /// Get local asset Id from asset location
        fn get_asset_id(xc_asset_location: MultiLocation) -> Option<AssetId>;
    }

    /// Used to fetch `units per second` if cross-chain asset is applicable for local execution payment.
    pub trait ExecutionPaymentRate {
        /// returns units per second from asset type or `None` if asset type isn't a supported payment asset.
        fn get_units_per_second(asset_location: MultiLocation) -> Option<u128>;
    }

    impl<T: Config> XcAssetLocation<T::AssetId> for Pallet<T> {
        fn get_xc_asset_location(asset_id: T::AssetId) -> Option<MultiLocation> {
            AssetIdToLocation::<T>::get(asset_id).and_then(|x| x.try_into().ok())
        }

        fn get_asset_id(asset_location: MultiLocation) -> Option<T::AssetId> {
            AssetLocationToId::<T>::get(asset_location.into_versioned())
        }
    }

    impl<T: Config> ExecutionPaymentRate for Pallet<T> {
        fn get_units_per_second(asset_location: MultiLocation) -> Option<u128> {
            AssetLocationUnitsPerSecond::<T>::get(asset_location.into_versioned())
        }
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The Asset Id. This will be used to create the asset and to associate it with
        /// a AssetLocation
        type AssetId: Member + Parameter + Default + Copy + HasCompact + MaxEncodedLen;

        /// Callback handling for cross-chain asset registration or unregistration.
        type XcAssetChanged: XcAssetChanged<Self>;

        /// The required origin for managing cross-chain asset configuration
        ///
        /// Should most likely be root.
        type ManagerOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

        type WeightInfo: WeightInfo;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Asset is already registered.
        AssetAlreadyRegistered,
        /// Asset does not exist (hasn't been registered).
        AssetDoesNotExist,
        /// Failed to convert to latest versioned MultiLocation
        MultiLocationNotSupported,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Registed mapping between asset type and asset Id.
        AssetRegistered {
            asset_location: VersionedMultiLocation,
            asset_id: T::AssetId,
        },
        /// Changed the amount of units we are charging per execution second for an asset
        UnitsPerSecondChanged {
            asset_location: VersionedMultiLocation,
            units_per_second: u128,
        },
        /// Changed the asset type mapping for a given asset id
        AssetLocationChanged {
            previous_asset_location: VersionedMultiLocation,
            asset_id: T::AssetId,
            new_asset_location: VersionedMultiLocation,
        },
        /// Supported asset type for fee payment removed.
        SupportedAssetRemoved {
            asset_location: VersionedMultiLocation,
        },
        /// Removed all information related to an asset Id
        AssetRemoved {
            asset_location: VersionedMultiLocation,
            asset_id: T::AssetId,
        },
    }

    /// Mapping from an asset id to asset type.
    /// Can be used when receiving transaction specifying an asset directly,
    /// like transferring an asset from this chain to another.
    #[pallet::storage]
    #[pallet::getter(fn asset_id_to_location)]
    pub type AssetIdToLocation<T: Config> =
        StorageMap<_, Twox64Concat, T::AssetId, VersionedMultiLocation>;

    /// Mapping from an asset type to an asset id.
    /// Can be used when receiving a multilocation XCM message to retrieve
    /// the corresponding asset in which tokens should me minted.
    #[pallet::storage]
    #[pallet::getter(fn asset_location_to_id)]
    pub type AssetLocationToId<T: Config> =
        StorageMap<_, Twox64Concat, VersionedMultiLocation, T::AssetId>;

    /// Stores the units per second for local execution for a AssetLocation.
    /// This is used to know how to charge for XCM execution in a particular asset.
    ///
    /// Not all asset types are supported for payment. If value exists here, it means it is supported.
    #[pallet::storage]
    #[pallet::getter(fn asset_location_units_per_second)]
    pub type AssetLocationUnitsPerSecond<T: Config> =
        StorageMap<_, Twox64Concat, VersionedMultiLocation, u128>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register new asset location to asset Id mapping.
        ///
        /// This makes the asset eligible for XCM interaction.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_asset_location())]
        pub fn register_asset_location(
            origin: OriginFor<T>,
            asset_location: Box<VersionedMultiLocation>,
            #[pallet::compact] asset_id: T::AssetId,
        ) -> DispatchResult {
            T::ManagerOrigin::ensure_origin(origin)?;

            // Ensure such an assetId does not exist
            ensure!(
                !AssetIdToLocation::<T>::contains_key(&asset_id),
                Error::<T>::AssetAlreadyRegistered
            );

            let v3_asset_loc = MultiLocation::try_from(*asset_location)
                .map_err(|_| Error::<T>::MultiLocationNotSupported)?;
            let asset_location = VersionedMultiLocation::V3(v3_asset_loc);

            AssetIdToLocation::<T>::insert(&asset_id, asset_location.clone());
            AssetLocationToId::<T>::insert(&asset_location, asset_id);

            T::XcAssetChanged::xc_asset_registered(asset_id);

            Self::deposit_event(Event::AssetRegistered {
                asset_location,
                asset_id,
            });
            Ok(())
        }

        /// Change the amount of units we are charging per execution second
        /// for a given AssetLocation.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::set_asset_units_per_second())]
        pub fn set_asset_units_per_second(
            origin: OriginFor<T>,
            asset_location: Box<VersionedMultiLocation>,
            #[pallet::compact] units_per_second: u128,
        ) -> DispatchResult {
            T::ManagerOrigin::ensure_origin(origin)?;

            let v3_asset_loc = MultiLocation::try_from(*asset_location)
                .map_err(|_| Error::<T>::MultiLocationNotSupported)?;
            let asset_location = VersionedMultiLocation::V3(v3_asset_loc);

            ensure!(
                AssetLocationToId::<T>::contains_key(&asset_location),
                Error::<T>::AssetDoesNotExist
            );

            AssetLocationUnitsPerSecond::<T>::insert(&asset_location, units_per_second);

            Self::deposit_event(Event::UnitsPerSecondChanged {
                asset_location,
                units_per_second,
            });
            Ok(())
        }

        /// Change the xcm type mapping for a given asset Id.
        /// The new asset type will inherit old `units per second` value.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::change_existing_asset_location())]
        pub fn change_existing_asset_location(
            origin: OriginFor<T>,
            new_asset_location: Box<VersionedMultiLocation>,
            #[pallet::compact] asset_id: T::AssetId,
        ) -> DispatchResult {
            T::ManagerOrigin::ensure_origin(origin)?;

            let v3_asset_loc = MultiLocation::try_from(*new_asset_location)
                .map_err(|_| Error::<T>::MultiLocationNotSupported)?;
            let new_asset_location = VersionedMultiLocation::V3(v3_asset_loc);

            let previous_asset_location =
                AssetIdToLocation::<T>::get(&asset_id).ok_or(Error::<T>::AssetDoesNotExist)?;

            // Insert new asset type info
            AssetIdToLocation::<T>::insert(&asset_id, new_asset_location.clone());
            AssetLocationToId::<T>::insert(&new_asset_location, asset_id);

            // Remove previous asset type info
            AssetLocationToId::<T>::remove(&previous_asset_location);

            // Change AssetLocationUnitsPerSecond
            if let Some(units) = AssetLocationUnitsPerSecond::<T>::take(&previous_asset_location) {
                AssetLocationUnitsPerSecond::<T>::insert(&new_asset_location, units);
            }

            Self::deposit_event(Event::AssetLocationChanged {
                previous_asset_location,
                asset_id,
                new_asset_location,
            });
            Ok(())
        }

        /// Removes asset from the set of supported payment assets.
        ///
        /// The asset can still be interacted with via XCM but it cannot be used to pay for execution time.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::remove_payment_asset())]
        pub fn remove_payment_asset(
            origin: OriginFor<T>,
            asset_location: Box<VersionedMultiLocation>,
        ) -> DispatchResult {
            T::ManagerOrigin::ensure_origin(origin)?;

            let v3_asset_loc = MultiLocation::try_from(*asset_location)
                .map_err(|_| Error::<T>::MultiLocationNotSupported)?;
            let asset_location = VersionedMultiLocation::V3(v3_asset_loc);

            AssetLocationUnitsPerSecond::<T>::remove(&asset_location);

            Self::deposit_event(Event::SupportedAssetRemoved { asset_location });
            Ok(())
        }

        /// Removes all information related to asset, removing it from XCM support.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::remove_asset())]
        pub fn remove_asset(
            origin: OriginFor<T>,
            #[pallet::compact] asset_id: T::AssetId,
        ) -> DispatchResult {
            T::ManagerOrigin::ensure_origin(origin)?;

            let asset_location =
                AssetIdToLocation::<T>::get(&asset_id).ok_or(Error::<T>::AssetDoesNotExist)?;

            AssetIdToLocation::<T>::remove(&asset_id);
            AssetLocationToId::<T>::remove(&asset_location);
            AssetLocationUnitsPerSecond::<T>::remove(&asset_location);
            T::XcAssetChanged::xc_asset_unregistered(asset_id);

            Self::deposit_event(Event::AssetRemoved {
                asset_id,
                asset_location,
            });
            Ok(())
        }
    }
}
