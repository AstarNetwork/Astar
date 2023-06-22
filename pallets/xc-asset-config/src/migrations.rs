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

use super::*;
use frame_support::traits::OnRuntimeUpgrade;
use frame_support::{dispatch::GetStorageVersion, log, pallet_prelude::*, traits::Get};
use sp_std::{marker::PhantomData, vec::Vec};
use xcm::IntoVersion;

pub struct MigrationXcmV3<T: Config>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for MigrationXcmV3<T> {
    fn on_runtime_upgrade() -> Weight {
        let version = Pallet::<T>::on_chain_storage_version();
        let mut consumed_weight = Weight::zero();
        if version >= 2 {
            return consumed_weight;
        }

        // 1st map //
        let id_to_location_entries: Vec<_> = AssetIdToLocation::<T>::iter().collect();

        for (asset_id, legacy_location) in id_to_location_entries {
            consumed_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

            if let Ok(new_location) = legacy_location.into_version(3) {
                AssetIdToLocation::<T>::insert(asset_id, new_location);
            } else {
                // Won't happen, can be verified with try-runtime before upgrade
                log::warn!(
                    "Failed to convert AssetIdToLocation value for asset Id: {:?}",
                    asset_id
                );
            }
        }

        // 2nd map //
        let location_to_id_entries: Vec<_> = AssetLocationToId::<T>::drain().collect();

        for (legacy_location, asset_id) in location_to_id_entries {
            consumed_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 2));

            if let Ok(new_location) = legacy_location.into_version(3) {
                AssetLocationToId::<T>::insert(new_location, asset_id);
            } else {
                // Shouldn't happen, can be verified with try-runtime before upgrade
                log::warn!(
                    "Failed to convert AssetLocationToId value for asset Id: {:?}",
                    asset_id
                );
            }
        }

        // 3rd map //
        let location_to_price_entries: Vec<_> = AssetLocationUnitsPerSecond::<T>::drain().collect();

        for (legacy_location, price) in location_to_price_entries {
            consumed_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 2));

            if let Ok(new_location) = legacy_location.into_version(3) {
                AssetLocationUnitsPerSecond::<T>::insert(new_location, price);
            } else {
                // Shouldn't happen, can be verified with try-runtime before upgrade
                log::warn!("Failed to convert AssetLocationUnitsPerSecond value!");
            }
        }

        StorageVersion::new(2).put::<Pallet<T>>();
        consumed_weight.saturating_accrue(T::DbWeight::get().reads(1));

        consumed_weight
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
        assert!(Pallet::<T>::on_chain_storage_version() < 2);
        let id_to_location_entries: Vec<_> = AssetIdToLocation::<T>::iter().collect();

        Ok(id_to_location_entries.encode())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
        assert_eq!(Pallet::<T>::on_chain_storage_version(), 2);

        use xcm::VersionedMultiLocation;
        let legacy_id_to_location_entries: Vec<(T::AssetId, VersionedMultiLocation)> =
            Decode::decode(&mut state.as_ref())
                .map_err(|_| "Cannot decode data from pre_upgrade")?;

        let new_id_to_location_entries: Vec<_> = AssetIdToLocation::<T>::iter().collect();
        assert_eq!(
            legacy_id_to_location_entries.len(),
            new_id_to_location_entries.len()
        );

        for (ref id, ref _legacy_location) in legacy_id_to_location_entries {
            let new_location = AssetIdToLocation::<T>::get(id);
            assert!(new_location.is_some());
            let new_location = new_location.expect("Assert above ensures it's `Some`.");

            assert_eq!(AssetLocationToId::<T>::get(&new_location), Some(*id));
            assert!(AssetLocationUnitsPerSecond::<T>::contains_key(
                &new_location
            ));
        }

        Ok(())
    }
}
