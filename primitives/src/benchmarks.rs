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

use crate::{xcm::MAX_ASSETS, Address, AssetId};
use core::marker::PhantomData;
use frame_support::{assert_ok, dispatch::RawOrigin, traits::IsType};
use sp_runtime::traits::StaticLookup;
use sp_std::{boxed::Box, vec::Vec};
use xcm::prelude::*;
/// Benchmark helper for `pallet-assets`.
pub struct AssetsBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl<AssetIdParameter: From<u128>> pallet_assets::BenchmarkHelper<AssetIdParameter>
    for AssetsBenchmarkHelper
{
    fn create_asset_id_parameter(id: u32) -> AssetIdParameter {
        AssetId::from(id).into()
    }
}

pub struct XcmBenchmarkHelper<T>(PhantomData<T>);
impl<T> XcmBenchmarkHelper<T>
where
    T: pallet_assets::Config + pallet_xc_asset_config::Config,
    <T as pallet_assets::Config>::AssetIdParameter: From<u128>,
    <T as pallet_assets::Config>::Balance: IsType<u128>,
    <T as pallet_xc_asset_config::Config>::AssetId: IsType<u128>,
    <<T as frame_system::pallet::Config>::Lookup as StaticLookup>::Source: IsType<Address>,
{
    /// Get the worst case holding for xcm benchmarks
    /// Scenario: Max allowed fungible assets (pallet_assets)
    pub fn worst_case_holding() -> MultiAssets {
        let fungibles = MAX_ASSETS - 1;
        let fungibles_amount: u128 = 100_000;
        let assets = (0..fungibles)
            .map(|i| MultiAsset {
                id: Concrete(GeneralIndex(i as u128).into()),
                fun: Fungible(fungibles_amount * i as u128),
            })
            // adding relay asset as it is used in buy execution benchmarks
            .chain(core::iter::once(MultiAsset {
                id: Concrete(MultiLocation::parent()),
                fun: Fungible(u128::MAX),
            }))
            .collect::<Vec<_>>();

        // register the assets
        for (i, asset) in assets.iter().enumerate() {
            if let MultiAsset {
                id: Concrete(location),
                fun: Fungible(_),
            } = asset
            {
                // create the asset
                assert_ok!(pallet_assets::Pallet::<T>::force_create(
                    RawOrigin::Root.into(),
                    (i as u128).into(),
                    // min balance, no significane in holding
                    Address::Id([0u8; 32].into()).into(),
                    true,
                    // min balance, no significane in holding
                    1u128.into()
                ));

                // register asset in XcAssetConfig
                assert_ok!(
                    pallet_xc_asset_config::Pallet::<T>::register_asset_location(
                        RawOrigin::Root.into(),
                        Box::new(location.clone().into_versioned()),
                        (i as u128).into(),
                    )
                );
                assert_ok!(
                    pallet_xc_asset_config::Pallet::<T>::set_asset_units_per_second(
                        RawOrigin::Root.into(),
                        Box::new(location.clone().into_versioned()),
                        1_000_000_000_000u128,
                    )
                );
            }
        }

        assets.into()
    }
}
