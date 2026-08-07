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

use crate::{xcm::MAX_ASSETS, Address, AssetId};
use core::marker::PhantomData;
use frame_support::{assert_ok, dispatch::RawOrigin, traits::IsType};
use sp_runtime::traits::StaticLookup;
use sp_std::{boxed::Box, vec::Vec};
use xcm::prelude::*;
/// Benchmark helper for `pallet-assets`.
pub struct AssetsBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl<AssetIdParameter: From<u128>> pallet_assets::BenchmarkHelper<AssetIdParameter, ()>
    for AssetsBenchmarkHelper
{
    fn create_asset_id_parameter(id: u32) -> AssetIdParameter {
        AssetId::from(id).into()
    }
    fn create_reserve_id_parameter(_: u32) -> () {
        ()
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
    pub fn worst_case_holding() -> Assets {
        // Max number of assets - relay asset & native asset
        let fungibles = MAX_ASSETS - 2;
        let fungibles_amount: u128 = 1_000_000_000_000_000_000_000_000;
        let assets = (1..=fungibles)
            .map(|i| Asset {
                id: AssetId(GeneralIndex(i as u128).into()),
                fun: Fungible(fungibles_amount * i as u128),
            })
            // adding relay asset as it is used in buy execution benchmarks
            .chain(core::iter::once(Asset {
                id: AssetId(Location::parent()),
                fun: Fungible(fungibles_amount),
            }))
            .collect::<Vec<_>>();

        // register the assets
        for (i, asset) in assets.iter().enumerate() {
            if let Asset {
                id: AssetId(location),
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

        // Expand with native asset
        assets
            .into_iter()
            .chain(core::iter::once(Asset {
                id: AssetId(Location::here()),
                fun: Fungible(fungibles_amount),
            }))
            .rev()
            .collect::<Vec<Asset>>()
            .into()
    }

    /// Set up a *worst-case* complex asset transfer for the `pallet_xcm`
    /// `transfer_assets` benchmark, whose weight is reused verbatim by
    /// `transfer_assets_using_type_and_then`
    /// (`#[pallet::weight(T::WeightInfo::transfer_assets())]`).
    ///
    /// The worst case for these extrinsics is a transfer where the fee asset
    /// and the transferred asset resolve to *different* transfer types,
    /// forcing pallet-xcm to build the heavier, two-part ("...and_then") program:
    ///   * fees:   the **native** token (`Here`) -> local-reserve transfer,
    ///   * assets: a **foreign** asset reserved on the destination sibling
    ///             -> destination-reserve transfer.
    #[cfg(feature = "runtime-benchmarks")]
    pub fn set_up_complex_asset_transfer() -> Option<(Assets, u32, Location, Box<dyn FnOnce()>)>
    where
        T: pallet_balances::Config,
        <T as pallet_balances::Config>::Balance: IsType<u128>,
    {
        use frame_benchmarking::whitelisted_caller;
        use frame_support::traits::{fungibles::Mutate, Currency};

        // Some sibling parachain we can reach over HRMP (the channel is opened by
        // the benchmark's `DeliveryHelper`). The foreign asset below is reserved
        // on this same chain, making its transfer a *destination-reserve* one.
        let sibling_para_id: u32 = 43_211_235;
        let dest: Location = (Parent, Parachain(sibling_para_id)).into();
        let foreign_location: Location = (Parent, Parachain(sibling_para_id)).into();

        // The whitelisted caller that signs the benchmarked transfer.
        let who: T::AccountId = whitelisted_caller();

        // --- Fee leg: native token (`Here`), a *local-reserve* transfer. ---
        let native_fee_amount: u128 = 1_000_000_000_000_000_000; // 1 unit, ample for fees.
        let native_funding: <T as pallet_balances::Config>::Balance =
            native_fee_amount.saturating_mul(1_000).into();
        let _ = <pallet_balances::Pallet<T> as Currency<T::AccountId>>::make_free_balance_be(
            &who,
            native_funding,
        );

        // --- Transfer leg: a foreign asset reserved on the destination sibling,
        // a *destination-reserve* transfer (a different type than the fee). ---
        let local_asset_id: u128 = u32::MAX as u128; // avoid clashing with real ids
                                                     // Create the local derivative and make it sufficient.
        assert_ok!(pallet_assets::Pallet::<T>::force_create(
            RawOrigin::Root.into(),
            local_asset_id.into(),
            Address::Id([0u8; 32].into()).into(),
            true,
            1u128.into(),
        ));
        // Map the derivative <-> its XCM location and give it a fee rate, so the
        // XCM executor recognizes it and can charge fees in it.
        assert_ok!(
            pallet_xc_asset_config::Pallet::<T>::register_asset_location(
                RawOrigin::Root.into(),
                Box::new(foreign_location.clone().into_versioned()),
                local_asset_id.into(),
            )
        );
        assert_ok!(
            pallet_xc_asset_config::Pallet::<T>::set_asset_units_per_second(
                RawOrigin::Root.into(),
                Box::new(foreign_location.clone().into_versioned()),
                1_000_000_000_000u128,
            )
        );
        // Give the caller some derivative so it can be withdrawn/transferred.
        let transfer_amount: u128 = 10_000_000_000_000;
        let pallet_assets_id: <T as pallet_assets::Config>::AssetId =
            <<T as pallet_assets::Config>::AssetIdParameter as From<u128>>::from(local_asset_id)
                .into();
        assert_ok!(
            <pallet_assets::Pallet<T> as Mutate<T::AccountId>>::mint_into(
                pallet_assets_id,
                &who,
                transfer_amount.into(),
            )
        );

        // The fee asset MUST be at index 0: the benchmark hard-codes fee index `0`.
        let fee_asset = Asset {
            id: AssetId(Location::here()),
            fun: Fungible(native_fee_amount),
        };
        let transfer_asset = Asset {
            id: AssetId(foreign_location),
            fun: Fungible(transfer_amount),
        };
        let assets: Assets = Vec::from([fee_asset, transfer_asset]).into();
        let fee_index: u32 = 0;

        let verify: Box<dyn FnOnce()> = Box::new(move || {
            // Native balance decreased by at least the fee we declared.
            let remaining =
                <pallet_balances::Pallet<T> as Currency<T::AccountId>>::free_balance(&who);
            assert!(remaining <= native_funding - native_fee_amount.into());
        });

        Some((assets, fee_index, dest, verify))
    }
}
