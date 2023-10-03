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

//! # XCM Primitives
//!
//! ## Overview
//!
//! Collection of common XCM primitives used by runtimes.
//!
//! - `AssetLocationIdConverter` - conversion between local asset Id and cross-chain asset multilocation
//! - `FixedRateOfForeignAsset` - weight trader for execution payment in foreign asset
//! - `ReserveAssetFilter` - used to check whether asset/origin are a valid reserve location
//! - `XcmFungibleFeeHandler` - used to handle XCM fee execution fees
//!
//! Please refer to implementation below for more info.
//!

use crate::AccountId;

use frame_support::{
    traits::{tokens::fungibles, ContainsPair, Get},
    weights::constants::WEIGHT_REF_TIME_PER_SECOND,
};
use sp_runtime::traits::{Bounded, Convert, Zero};
use sp_std::{borrow::Borrow, marker::PhantomData, vec::Vec};

// Polkadot imports
use xcm::latest::{prelude::*, Weight};
use xcm_builder::TakeRevenue;
use xcm_executor::traits::{MatchesFungibles, WeightTrader};

// ORML imports
use orml_traits::location::{RelativeReserveProvider, Reserve};

use pallet_xc_asset_config::{ExecutionPaymentRate, XcAssetLocation};

#[cfg(test)]
mod tests;

pub const XCM_SIZE_LIMIT: u32 = 2u32.pow(16);

/// Used to convert between cross-chain asset multilocation and local asset Id.
///
/// This implementation relies on `XcAssetConfig` pallet to handle mapping.
/// In case asset location hasn't been mapped, it means the asset isn't supported (yet).
pub struct AssetLocationIdConverter<AssetId, AssetMapper>(PhantomData<(AssetId, AssetMapper)>);
impl<AssetId, AssetMapper> xcm_executor::traits::Convert<MultiLocation, AssetId>
    for AssetLocationIdConverter<AssetId, AssetMapper>
where
    AssetId: Clone + Eq + Bounded,
    AssetMapper: XcAssetLocation<AssetId>,
{
    fn convert_ref(location: impl Borrow<MultiLocation>) -> Result<AssetId, ()> {
        if let Some(asset_id) = AssetMapper::get_asset_id(location.borrow().clone()) {
            Ok(asset_id)
        } else {
            Err(())
        }
    }

    fn reverse_ref(id: impl Borrow<AssetId>) -> Result<MultiLocation, ()> {
        if let Some(multilocation) = AssetMapper::get_xc_asset_location(id.borrow().clone()) {
            Ok(multilocation)
        } else {
            Err(())
        }
    }
}

/// Used as weight trader for foreign assets.
///
/// In case foreigin asset is supported as payment asset, XCM execution time
/// on-chain can be paid by the foreign asset, using the configured rate.
pub struct FixedRateOfForeignAsset<T: ExecutionPaymentRate, R: TakeRevenue> {
    /// Total used weight
    weight: Weight,
    /// Total consumed assets
    consumed: u128,
    /// Asset Id (as MultiLocation) and units per second for payment
    asset_location_and_units_per_second: Option<(MultiLocation, u128)>,
    _pd: PhantomData<(T, R)>,
}

impl<T: ExecutionPaymentRate, R: TakeRevenue> WeightTrader for FixedRateOfForeignAsset<T, R> {
    fn new() -> Self {
        Self {
            weight: Weight::zero(),
            consumed: 0,
            asset_location_and_units_per_second: None,
            _pd: PhantomData,
        }
    }

    fn buy_weight(
        &mut self,
        weight: Weight,
        payment: xcm_executor::Assets,
    ) -> Result<xcm_executor::Assets, XcmError> {
        log::trace!(
            target: "xcm::weight",
            "FixedRateOfForeignAsset::buy_weight weight: {:?}, payment: {:?}",
            weight, payment,
        );

        // Atm in pallet, we only support one asset so this should work
        let payment_asset = payment
            .fungible_assets_iter()
            .next()
            .ok_or(XcmError::TooExpensive)?;

        match payment_asset {
            MultiAsset {
                id: xcm::latest::AssetId::Concrete(asset_location),
                fun: Fungibility::Fungible(_),
            } => {
                if let Some(units_per_second) = T::get_units_per_second(asset_location.clone()) {
                    let amount = units_per_second.saturating_mul(weight.ref_time() as u128) // TODO: change this to u64?
                        / (WEIGHT_REF_TIME_PER_SECOND as u128);
                    if amount == 0 {
                        return Ok(payment);
                    }

                    let unused = payment
                        .checked_sub((asset_location.clone(), amount).into())
                        .map_err(|_| XcmError::TooExpensive)?;

                    self.weight = self.weight.saturating_add(weight);

                    // If there are multiple calls to `BuyExecution` but with different assets, we need to be able to handle that.
                    // Current primitive implementation will just keep total track of consumed asset for the FIRST consumed asset.
                    // Others will just be ignored when refund is concerned.
                    if let Some((old_asset_location, _)) =
                        self.asset_location_and_units_per_second.clone()
                    {
                        if old_asset_location == asset_location {
                            self.consumed = self.consumed.saturating_add(amount);
                        }
                    } else {
                        self.consumed = self.consumed.saturating_add(amount);
                        self.asset_location_and_units_per_second =
                            Some((asset_location, units_per_second));
                    }

                    Ok(unused)
                } else {
                    Err(XcmError::TooExpensive)
                }
            }
            _ => Err(XcmError::TooExpensive),
        }
    }

    fn refund_weight(&mut self, weight: Weight) -> Option<MultiAsset> {
        log::trace!(target: "xcm::weight", "FixedRateOfForeignAsset::refund_weight weight: {:?}", weight);

        if let Some((asset_location, units_per_second)) =
            self.asset_location_and_units_per_second.clone()
        {
            let weight = weight.min(self.weight);
            let amount = units_per_second.saturating_mul(weight.ref_time() as u128)
                / (WEIGHT_REF_TIME_PER_SECOND as u128);

            self.weight = self.weight.saturating_sub(weight);
            self.consumed = self.consumed.saturating_sub(amount);

            if amount > 0 {
                Some((asset_location, amount).into())
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<T: ExecutionPaymentRate, R: TakeRevenue> Drop for FixedRateOfForeignAsset<T, R> {
    fn drop(&mut self) {
        if let Some((asset_location, _)) = self.asset_location_and_units_per_second.clone() {
            if self.consumed > 0 {
                R::take_revenue((asset_location, self.consumed).into());
            }
        }
    }
}

/// Used to determine whether the cross-chain asset is coming from a trusted reserve or not
///
/// Basically, we trust any cross-chain asset from any location to act as a reserve since
/// in order to support the xc-asset, we need to first register it in the `XcAssetConfig` pallet.
///
pub struct ReserveAssetFilter;
impl ContainsPair<MultiAsset, MultiLocation> for ReserveAssetFilter {
    fn contains(asset: &MultiAsset, origin: &MultiLocation) -> bool {
        // We assume that relay chain and sibling parachain assets are trusted reserves for their assets
        let reserve_location = if let Concrete(location) = &asset.id {
            match (location.parents, location.first_interior()) {
                // sibling parachain
                (1, Some(Parachain(id))) => Some(MultiLocation::new(1, X1(Parachain(*id)))),
                // relay chain
                (1, _) => Some(MultiLocation::parent()),
                _ => None,
            }
        } else {
            None
        };

        if let Some(ref reserve) = reserve_location {
            origin == reserve
        } else {
            false
        }
    }
}

/// Used to deposit XCM fees into a destination account.
///
/// Only handles fungible assets for now.
/// If for any reason taking of the fee fails, it will be burned and and error trace will be printed.
///
pub struct XcmFungibleFeeHandler<AccountId, Matcher, Assets, FeeDestination>(
    sp_std::marker::PhantomData<(AccountId, Matcher, Assets, FeeDestination)>,
);
impl<
        AccountId,
        Assets: fungibles::Mutate<AccountId>,
        Matcher: MatchesFungibles<Assets::AssetId, Assets::Balance>,
        FeeDestination: Get<AccountId>,
    > TakeRevenue for XcmFungibleFeeHandler<AccountId, Matcher, Assets, FeeDestination>
{
    fn take_revenue(revenue: MultiAsset) {
        match Matcher::matches_fungibles(&revenue) {
            Ok((asset_id, amount)) => {
                if amount > Zero::zero() {
                    if let Err(error) =
                        Assets::mint_into(asset_id.clone(), &FeeDestination::get(), amount)
                    {
                        log::error!(
                            target: "xcm::weight",
                            "XcmFeeHandler::take_revenue failed when minting asset: {:?}", error,
                        );
                    } else {
                        log::trace!(
                            target: "xcm::weight",
                            "XcmFeeHandler::take_revenue took {:?} of asset Id {:?}",
                            amount, asset_id,
                        );
                    }
                }
            }
            Err(_) => {
                log::error!(
                    target: "xcm::weight",
                    "XcmFeeHandler:take_revenue failed to match fungible asset, it has been burned."
                );
            }
        }
    }
}

// TODO: remove this after uplift to `polkadot-v0.9.44` or beyond, and replace it with code in XCM builder.

use parity_scale_codec::{Compact, Encode};
use sp_io::hashing::blake2_256;
use sp_std::prelude::*;
use xcm_executor::traits::Convert as XcmConvert;

/// Means of converting a location into a stable and unique descriptive identifier.
pub trait DescribeLocation {
    /// Create a description of the given `location` if possible. No two locations should have the
    /// same descriptor.
    fn describe_location(location: &MultiLocation) -> Option<Vec<u8>>;
}

#[impl_trait_for_tuples::impl_for_tuples(30)]
impl DescribeLocation for Tuple {
    fn describe_location(l: &MultiLocation) -> Option<Vec<u8>> {
        for_tuples!( #(
			match Tuple::describe_location(l) {
				Some(result) => return Some(result),
				None => {},
			}
		)* );
        None
    }
}

pub struct DescribeTerminus;
impl DescribeLocation for DescribeTerminus {
    fn describe_location(l: &MultiLocation) -> Option<Vec<u8>> {
        match (l.parents, &l.interior) {
            (0, Here) => Some(Vec::new()),
            _ => return None,
        }
    }
}

pub struct DescribePalletTerminal;
impl DescribeLocation for DescribePalletTerminal {
    fn describe_location(l: &MultiLocation) -> Option<Vec<u8>> {
        match (l.parents, &l.interior) {
            (0, X1(PalletInstance(i))) => {
                Some((b"Pallet", Compact::<u32>::from(*i as u32)).encode())
            }
            _ => return None,
        }
    }
}

pub struct DescribeAccountId32Terminal;
impl DescribeLocation for DescribeAccountId32Terminal {
    fn describe_location(l: &MultiLocation) -> Option<Vec<u8>> {
        match (l.parents, &l.interior) {
            (0, X1(AccountId32 { id, .. })) => Some((b"AccountId32", id).encode()),
            _ => return None,
        }
    }
}

pub struct DescribeAccountKey20Terminal;
impl DescribeLocation for DescribeAccountKey20Terminal {
    fn describe_location(l: &MultiLocation) -> Option<Vec<u8>> {
        match (l.parents, &l.interior) {
            (0, X1(AccountKey20 { key, .. })) => Some((b"AccountKey20", key).encode()),
            _ => return None,
        }
    }
}

pub type DescribeAccountIdTerminal = (DescribeAccountId32Terminal, DescribeAccountKey20Terminal);

pub type DescribeAllTerminal = (
    DescribeTerminus,
    DescribePalletTerminal,
    DescribeAccountId32Terminal,
    DescribeAccountKey20Terminal,
);

pub struct DescribeFamily<DescribeInterior>(PhantomData<DescribeInterior>);
impl<Suffix: DescribeLocation> DescribeLocation for DescribeFamily<Suffix> {
    fn describe_location(l: &MultiLocation) -> Option<Vec<u8>> {
        match (l.parents, l.interior.first()) {
            (0, Some(Parachain(index))) => {
                let tail = l.interior.split_first().0;
                let interior = Suffix::describe_location(&tail.into())?;
                Some((b"ChildChain", Compact::<u32>::from(*index), interior).encode())
            }
            (1, Some(Parachain(index))) => {
                let tail = l.interior.split_first().0;
                let interior = Suffix::describe_location(&tail.into())?;
                Some((b"SiblingChain", Compact::<u32>::from(*index), interior).encode())
            }
            (1, _) => {
                let tail = l.interior.into();
                let interior = Suffix::describe_location(&tail)?;
                Some((b"ParentChain", interior).encode())
            }
            _ => return None,
        }
    }
}

pub struct HashedDescription<AccountId, Describe>(PhantomData<(AccountId, Describe)>);
impl<AccountId: From<[u8; 32]> + Clone, Describe: DescribeLocation>
    XcmConvert<MultiLocation, AccountId> for HashedDescription<AccountId, Describe>
{
    fn convert(value: MultiLocation) -> Result<AccountId, MultiLocation> {
        if let Some(description) = Describe::describe_location(&value) {
            Ok(blake2_256(&description).into())
        } else {
            Err(value)
        }
    }
}

/// Convert `AccountId` to `MultiLocation`.
pub struct AccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
    fn convert(account: AccountId) -> MultiLocation {
        X1(AccountId32 {
            network: None,
            id: account.into(),
        })
        .into()
    }
}

/// `MultiAsset` reserve location provider. It's based on `RelativeReserveProvider` and in
/// addition will convert self absolute location to relative location.
pub struct AbsoluteAndRelativeReserveProvider<AbsoluteLocation>(PhantomData<AbsoluteLocation>);
impl<AbsoluteLocation: Get<MultiLocation>> Reserve
    for AbsoluteAndRelativeReserveProvider<AbsoluteLocation>
{
    fn reserve(asset: &MultiAsset) -> Option<MultiLocation> {
        RelativeReserveProvider::reserve(asset).map(|reserve_location| {
            if reserve_location == AbsoluteLocation::get() {
                MultiLocation::here()
            } else {
                reserve_location
            }
        })
    }
}
