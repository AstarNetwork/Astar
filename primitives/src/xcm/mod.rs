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
//! - `split_location_into_chain_part_and_beneficiary` - splits a combined `Location` into
//!   the destination chain part and the beneficiary part, as required by `pallet_xcm`
//! - `resolve_transfer_type` - picks the reserve model `pallet_xcm` should use for a transfer
//!
//! Please refer to implementation below for more info.
//!

use frame_support::{
    traits::{tokens::fungibles, ContainsPair, Get},
    weights::constants::WEIGHT_REF_TIME_PER_SECOND,
};
use sp_runtime::traits::{Bounded, MaybeEquivalence, Zero};
use sp_std::marker::PhantomData;

// Polkadot imports
use xcm::latest::{prelude::*, Weight};
use xcm_builder::TakeRevenue;
use xcm_executor::traits::{MatchesFungibles, TransferType, WeightTrader, XcmAssetTransfers};

use pallet_xc_asset_config::{ExecutionPaymentRate, XcAssetLocation};

#[cfg(test)]
mod tests;

pub const XCM_SIZE_LIMIT: u32 = 2u32.pow(16);
pub const MAX_ASSETS: u32 = 64;
pub const ASSET_HUB_PARA_ID: u32 = 1000;

/// Used to convert between cross-chain asset multilocation and local asset Id.
///
/// This implementation relies on `XcAssetConfig` pallet to handle mapping.
/// In case asset location hasn't been mapped, it means the asset isn't supported (yet).
pub struct AssetLocationIdConverter<AssetId, AssetMapper>(PhantomData<(AssetId, AssetMapper)>);
impl<AssetId, AssetMapper> MaybeEquivalence<Location, AssetId>
    for AssetLocationIdConverter<AssetId, AssetMapper>
where
    AssetId: Clone + Eq + Bounded,
    AssetMapper: XcAssetLocation<AssetId>,
{
    fn convert(location: &Location) -> Option<AssetId> {
        AssetMapper::get_asset_id(location.clone())
    }

    fn convert_back(id: &AssetId) -> Option<Location> {
        AssetMapper::get_xc_asset_location(id.clone())
    }
}

/// Used as weight trader for foreign assets.
///
/// In case foreign asset is supported as payment asset, XCM execution time
/// on-chain can be paid by the foreign asset, using the configured rate.
pub struct FixedRateOfForeignAsset<T: ExecutionPaymentRate, R: TakeRevenue> {
    /// Total used weight
    weight: Weight,
    /// Total consumed assets
    consumed: u128,
    /// Asset Id (as Location) and units per second for payment
    asset_location_and_units_per_second: Option<(Location, u128)>,
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
        payment: xcm_executor::AssetsInHolding,
        _: &XcmContext,
    ) -> Result<xcm_executor::AssetsInHolding, XcmError> {
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
            Asset {
                id: AssetId(asset_location),
                fun: Fungibility::Fungible(_),
            } => {
                if let Some(units_per_second) = T::get_units_per_second(asset_location.clone()) {
                    let amount = units_per_second.saturating_mul(weight.ref_time() as u128) // TODO: change this to u64?
                        / (WEIGHT_REF_TIME_PER_SECOND as u128);
                    if amount == 0 {
                        return Ok(payment);
                    }

                    // This trader tracks a single fee asset.
                    if let Some((tracked_asset_location, _)) =
                        &self.asset_location_and_units_per_second
                    {
                        if *tracked_asset_location != asset_location {
                            return Err(XcmError::NotWithdrawable);
                        }
                    }

                    let unused = payment
                        .checked_sub((asset_location.clone(), amount).into())
                        .map_err(|_| XcmError::TooExpensive)?;

                    self.weight = self.weight.saturating_add(weight);
                    self.consumed = self.consumed.saturating_add(amount);
                    self.asset_location_and_units_per_second =
                        Some((asset_location, units_per_second));

                    Ok(unused)
                } else {
                    Err(XcmError::TooExpensive)
                }
            }
            _ => Err(XcmError::TooExpensive),
        }
    }

    fn refund_weight(&mut self, weight: Weight, _: &XcmContext) -> Option<Asset> {
        log::trace!(target: "xcm::weight", "FixedRateOfForeignAsset::refund_weight weight: {:?}", weight);

        if let Some((asset_location, units_per_second)) =
            self.asset_location_and_units_per_second.clone()
        {
            let weight = weight.min(self.weight);
            // Never hand back more of the asset than was actually taken for it.
            let amount = units_per_second
                .saturating_mul(weight.ref_time() as u128)
                .saturating_div(WEIGHT_REF_TIME_PER_SECOND as u128)
                .min(self.consumed);

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
impl ContainsPair<Asset, Location> for ReserveAssetFilter {
    fn contains(asset: &Asset, origin: &Location) -> bool {
        let AssetId(location) = &asset.id;
        match (location.parents, location.first_interior()) {
            // sibling parachain reserve
            (1, Some(Parachain(id))) => origin == &Location::new(1, [Parachain(*id)]),
            // relay token (DOT/KSM) - only Asset Hub is valid reserve now
            (1, None) => origin == &Location::new(1, [Parachain(ASSET_HUB_PARA_ID)]),
            _ => false,
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
        AccountId: Eq,
        Assets: fungibles::Mutate<AccountId>,
        Matcher: MatchesFungibles<Assets::AssetId, Assets::Balance>,
        FeeDestination: Get<AccountId>,
    > TakeRevenue for XcmFungibleFeeHandler<AccountId, Matcher, Assets, FeeDestination>
{
    fn take_revenue(revenue: Asset) {
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

/// Splits a combined `Location` into its chain part and its beneficiary part.
///
/// Junctions are popped off the tail until a chain identifier (`Parachain`/`GlobalConsensus`)
/// is reached; whatever was popped becomes the beneficiary, relative to the chain part.
///
/// A location with no chain identifier is only valid when it has exactly one parent, in which
/// case the chain part is the relay chain. Returns `None` for anything else.
pub fn split_location_into_chain_part_and_beneficiary(
    mut location: Location,
) -> Option<(Location, Location)> {
    let mut beneficiary_junctions = Junctions::Here;

    while let Some(junction) = location.last() {
        if matches!(
            junction,
            Junction::Parachain(_) | Junction::GlobalConsensus(_)
        ) {
            return Some((location, beneficiary_junctions.into_location()));
        }

        let (prefix, maybe_last) = location.split_last_interior();
        location = prefix;
        if let Some(junction) = maybe_last {
            beneficiary_junctions.push_front(junction).ok()?;
        }
    }

    // No chain identifier found: only the relay chain qualifies.
    if location.parent_count() == 1 {
        Some((Location::parent(), beneficiary_junctions.into_location()))
    } else {
        None
    }
}

/// Resolves which reserve model `pallet_xcm` should use to move `asset` to `dest`.
///
/// Defers to the XCM executor's own determination, which is driven by the runtime's
/// [`ReserveAssetFilter`] and teleport filter - the same logic `pallet_xcm::transfer_assets` runs
/// internally. One case the executor cannot resolve on its own: the relay-native token (`DOT`,
/// `KSM`, ...) is identified by the bare parent location, but its trusted reserve is Asset Hub
/// rather than the relay chain. For any destination other than Asset Hub itself the executor gives
/// up, so we name Asset Hub as a remote reserve explicitly.
///
/// This mirrors the routing that `orml-xtokens` derived from its `AbsoluteAndRelativeReserveProvider`,
/// so EVM callers see the same behaviour after the pallet's removal.
///
/// Returns `None` when no reserve can be determined - the caller should reject the transfer.
pub fn resolve_transfer_type<XcmExecutor: XcmAssetTransfers>(
    asset: &Asset,
    dest: &Location,
) -> Option<TransferType> {
    if let Ok(transfer_type) = XcmExecutor::determine_for(asset, dest) {
        return Some(transfer_type);
    }

    let AssetId(location) = &asset.id;
    if location == &Location::parent() {
        return Some(TransferType::RemoteReserve(
            Location::new(1, [Parachain(ASSET_HUB_PARA_ID)]).into(),
        ));
    }

    None
}
