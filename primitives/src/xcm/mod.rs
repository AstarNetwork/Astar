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
    traits::{
        tokens::{fungibles, imbalance::UnsafeManualAccounting},
        ContainsPair, Get,
    },
    weights::constants::WEIGHT_REF_TIME_PER_SECOND,
};
use sp_runtime::traits::{Bounded, MaybeEquivalence, Zero};
use sp_std::marker::PhantomData;

// Polkadot imports
use xcm::latest::{prelude::*, Weight};
use xcm_builder::TakeRevenue;
use xcm_executor::{
    traits::{MatchesFungibles, TransferType, WeightTrader, XcmAssetTransfers},
    AssetsInHolding,
};

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
    /// Assets taken from the payment so far, handed to `R` on drop
    taken: AssetsInHolding,
    /// Asset Id (as Location) and units per second for payment
    asset_location_and_units_per_second: Option<(Location, u128)>,
    _pd: PhantomData<(T, R)>,
}

impl<T: ExecutionPaymentRate, R: TakeRevenue> WeightTrader for FixedRateOfForeignAsset<T, R> {
    fn new() -> Self {
        Self {
            weight: Weight::zero(),
            taken: AssetsInHolding::new(),
            asset_location_and_units_per_second: None,
            _pd: PhantomData,
        }
    }

    fn buy_weight(
        &mut self,
        weight: Weight,
        mut payment: AssetsInHolding,
        _: &XcmContext,
    ) -> Result<AssetsInHolding, (AssetsInHolding, XcmError)> {
        log::trace!(
            target: "xcm::weight",
            "FixedRateOfForeignAsset::buy_weight weight: {:?}, payment: {:?}",
            weight, payment,
        );

        // Atm in pallet, we only support one asset so this should work
        let Some(Asset {
            id: AssetId(asset_location),
            fun: Fungibility::Fungible(_),
        }) = payment.fungible_assets_iter().next()
        else {
            return Err((payment, XcmError::TooExpensive));
        };

        let Some(units_per_second) = T::get_units_per_second(asset_location.clone()) else {
            return Err((payment, XcmError::TooExpensive));
        };

        let amount = execution_fee(weight, units_per_second);
        if amount == 0 {
            return Ok(payment);
        }

        // This trader tracks a single fee asset.
        if let Some((tracked_asset_location, _)) = &self.asset_location_and_units_per_second {
            if *tracked_asset_location != asset_location {
                return Err((payment, XcmError::NotWithdrawable));
            }
        }

        let to_charge: Asset = (asset_location.clone(), amount).into();
        let Ok(taken) = payment.try_take(to_charge.into()) else {
            return Err((payment, XcmError::TooExpensive));
        };

        self.weight = self.weight.saturating_add(weight);
        self.taken.subsume_assets(taken);
        self.asset_location_and_units_per_second = Some((asset_location, units_per_second));

        Ok(payment)
    }

    fn refund_weight(&mut self, weight: Weight, context: &XcmContext) -> Option<AssetsInHolding> {
        log::trace!(target: "xcm::weight", "FixedRateOfForeignAsset::refund_weight weight: {:?}", weight);

        let (asset_location, _) = self.asset_location_and_units_per_second.clone()?;
        let weight = weight.min(self.weight);
        let quote = self
            .quote_weight(weight, AssetId(asset_location), context)
            .ok()?;

        self.weight = self.weight.saturating_sub(weight);
        // Never hand back more of the asset than was actually taken for it.
        let refunded = self.taken.saturating_take(quote.into());

        if refunded.is_empty() {
            None
        } else {
            Some(refunded)
        }
    }

    fn quote_weight(
        &mut self,
        weight: Weight,
        given: AssetId,
        _: &XcmContext,
    ) -> Result<Asset, XcmError> {
        log::trace!(
            target: "xcm::weight",
            "FixedRateOfForeignAsset::quote_weight weight: {:?}, given: {:?}",
            weight, given,
        );

        if weight.is_zero() {
            return Err(XcmError::NoDeal);
        }
        let units_per_second =
            T::get_units_per_second(given.0.clone()).ok_or(XcmError::NotHoldingFees)?;

        Ok((given, execution_fee(weight, units_per_second)).into())
    }
}

impl<T: ExecutionPaymentRate, R: TakeRevenue> Drop for FixedRateOfForeignAsset<T, R> {
    fn drop(&mut self) {
        if !self.taken.is_empty() {
            R::take_revenue(core::mem::replace(&mut self.taken, AssetsInHolding::new()));
        }
    }
}

/// Fee for `weight` at the given rate, in the smallest unit of the fee asset.
fn execution_fee(weight: Weight, units_per_second: u128) -> u128 {
    units_per_second.saturating_mul(weight.ref_time() as u128)
        / (WEIGHT_REF_TIME_PER_SECOND as u128)
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
        Assets: fungibles::Inspect<AccountId, AssetId: 'static, Balance: 'static>
            + fungibles::Balanced<AccountId, OnDropCredit: 'static, OnDropDebt: 'static>,
        Matcher: MatchesFungibles<Assets::AssetId, Assets::Balance>,
        FeeDestination: Get<AccountId>,
    > TakeRevenue for XcmFungibleFeeHandler<AccountId, Matcher, Assets, FeeDestination>
{
    fn take_revenue(mut revenue: AssetsInHolding) {
        // Move the payer's credits to the fee destination; an unresolved credit is burned.
        for (id, imbalance) in core::mem::take(&mut revenue.fungible) {
            let asset = Asset {
                id,
                fun: Fungibility::Fungible(imbalance.amount()),
            };
            match Matcher::matches_fungibles(&asset) {
                Ok((asset_id, amount)) => {
                    if amount.is_zero() {
                        continue;
                    }
                    let mut credit = fungibles::Credit::<AccountId, Assets>::zero(asset_id.clone());
                    credit.saturating_subsume(imbalance);
                    if let Err(unspent) = Assets::resolve(&FeeDestination::get(), credit) {
                        log::error!(
                            target: "xcm::weight",
                            "XcmFeeHandler::take_revenue failed to deposit {:?} of asset Id {:?}, it has been burned.",
                            unspent.peek(), asset_id,
                        );
                    } else {
                        log::trace!(
                            target: "xcm::weight",
                            "XcmFeeHandler::take_revenue took {:?} of asset Id {:?}",
                            amount, asset_id,
                        );
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
