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

use super::*;
use frame_support::{
    pallet_prelude::*,
    traits::{Get, UncheckedOnRuntimeUpgrade},
};
use sp_std::{marker::PhantomData, vec::Vec};

/// Exports for versioned migration `type`s for this pallet.
pub mod versioned {
    use super::*;

    /// Migration storage V3 to V4 wrapped in a [`frame_support::migrations::VersionedMigration`], ensuring
    /// the migration is only performed when on-chain version is 3.
    pub type V3ToV4<T> = frame_support::migrations::VersionedMigration<
        3,
        4,
        unchecked_migration::UncheckedMigrationXcmVersion<{ xcm::v5::VERSION }, T>,
        Pallet<T>,
        <T as frame_system::Config>::DbWeight,
    >;
}

mod unchecked_migration {
    use super::*;
    use xcm::IntoVersion;

    /// Migration for XCM versioned locations, generic over XCM version.
    pub struct UncheckedMigrationXcmVersion<const XCM_VERSION: u32, T: Config>(PhantomData<T>);
    impl<const XCM_VERSION: u32, T: Config> UncheckedOnRuntimeUpgrade
        for UncheckedMigrationXcmVersion<XCM_VERSION, T>
    {
        #[allow(deprecated)]
        fn on_runtime_upgrade() -> Weight {
            let mut consumed_weight = Weight::zero();

            // 1st map
            AssetIdToLocation::<T>::translate::<xcm::VersionedLocation, _>(
                |asset_id, multi_location| {
                    consumed_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

                    multi_location
                        .into_version(XCM_VERSION)
                        .map_err(|_| {
                            log::error!(
                            "Failed to convert AssetIdToLocation value for asset Id: {asset_id:?}",
                        );
                        })
                        .ok()
                },
            );

            // 2rd map
            let location_to_id_entries: Vec<_> = AssetLocationToId::<T>::drain().collect();
            for (multi_location, asset_id) in location_to_id_entries {
                consumed_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

                if let Ok(new_location) = multi_location.into_version(XCM_VERSION) {
                    AssetLocationToId::<T>::insert(new_location, asset_id);
                } else {
                    log::error!(
                        "Failed to convert AssetLocationToId value for asset Id: {asset_id:?}",
                    );
                }
            }

            // 3rd map
            let location_to_price_entries: Vec<_> =
                AssetLocationUnitsPerSecond::<T>::drain().collect();
            for (multi_location, price) in location_to_price_entries {
                consumed_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

                if let Ok(new_location) = multi_location.into_version(XCM_VERSION) {
                    AssetLocationUnitsPerSecond::<T>::insert(new_location, price);
                } else {
                    log::error!("Failed to convert AssetLocationUnitsPerSecond value failed!");
                }
            }

            consumed_weight
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
            let mut count = AssetIdToLocation::<T>::iter().collect::<Vec<_>>().len();
            count += AssetLocationToId::<T>::iter().collect::<Vec<_>>().len();
            count += AssetLocationUnitsPerSecond::<T>::iter()
                .collect::<Vec<_>>()
                .len();

            Ok((count as u32).encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
            let old_count: u32 = Decode::decode(&mut state.as_ref())
                .map_err(|_| "Cannot decode data from pre_upgrade")?;

            let mut count = AssetIdToLocation::<T>::iter().collect::<Vec<_>>().len();
            count += AssetLocationToId::<T>::iter().collect::<Vec<_>>().len();
            count += AssetLocationUnitsPerSecond::<T>::iter()
                .collect::<Vec<_>>()
                .len();

            assert_eq!(old_count, count as u32);
            Ok(())
        }
    }
}
