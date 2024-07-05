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
use frame_support::assert_ok;
use once_cell::unsync::Lazy;
use sp_runtime::traits::{MaybeEquivalence, Zero};

type AssetId = u128;

// Primitive, perhaps I improve it later
const PARENT: Location = Location::parent();
const PARACHAIN: Lazy<Location> = Lazy::new(|| Location {
    parents: 1,
    interior: [Parachain(10)].into(),
});
const GENERAL_INDEX: Lazy<Location> = Lazy::new(|| Location {
    parents: 2,
    interior: [GeneralIndex(20)].into(),
});
const RELAY_ASSET: AssetId = AssetId::MAX;

/// Helper struct used for testing `AssetLocationIdConverter`
struct AssetLocationMapper;
impl XcAssetLocation<AssetId> for AssetLocationMapper {
    fn get_xc_asset_location(asset_id: AssetId) -> Option<Location> {
        match asset_id {
            RELAY_ASSET => Some(PARENT),
            20 => Some((*PARACHAIN).clone()),
            30 => Some((*GENERAL_INDEX).clone()),
            _ => None,
        }
    }

    fn get_asset_id(asset_location: Location) -> Option<AssetId> {
        match asset_location {
            a if a == PARENT => Some(RELAY_ASSET),
            a if a == (*PARACHAIN).clone() => Some(20),
            a if a == (*GENERAL_INDEX).clone() => Some(30),
            _ => None,
        }
    }
}

/// Helper struct used for testing `FixedRateOfForeignAsset`
struct ExecutionPayment;
impl ExecutionPaymentRate for ExecutionPayment {
    fn get_units_per_second(asset_location: Location) -> Option<u128> {
        match asset_location {
            a if a == PARENT => Some(1_000_000),
            a if a == *PARACHAIN => Some(2_000_000),
            a if a == *GENERAL_INDEX => Some(3_000_000),
            _ => None,
        }
    }
}

/// Execution fee for the specified weight, using provided `units_per_second`
fn execution_fee(weight: Weight, units_per_second: u128) -> u128 {
    units_per_second * (weight.ref_time() as u128) / (WEIGHT_REF_TIME_PER_SECOND as u128)
}

#[test]
fn asset_location_to_id() {
    // Test cases where the Location is valid
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert(&PARENT),
        Some(u128::MAX)
    );
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert(&*PARACHAIN),
        Some(20)
    );
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert(&*GENERAL_INDEX),
        Some(30)
    );

    // Test case where Location isn't supported
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert(&Location::here()),
        None
    );
}

#[test]
fn asset_id_to_location() {
    // Test cases where the AssetId is valid
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert_back(&u128::MAX),
        Some(PARENT)
    );
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert_back(&20),
        Some((*PARACHAIN).clone())
    );
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert_back(&30),
        Some((*GENERAL_INDEX).clone())
    );

    // Test case where the AssetId isn't supported
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert_back(&0),
        None
    );
}

#[test]
fn fixed_rate_of_foreign_asset_buy_is_ok() {
    let mut fixed_rate_trader = FixedRateOfForeignAsset::<ExecutionPayment, ()>::new();

    // The amount we have designated for payment (doesn't mean it will be used though)
    let total_payment = 10_000;
    let payment_multi_asset = Asset {
        id: xcm::latest::AssetId(PARENT),
        fun: Fungibility::Fungible(total_payment),
    };
    let weight: Weight = Weight::from_parts(1_000_000_000, 0);
    let ctx = XcmContext {
        // arbitary ML
        origin: Some(Location::here()),
        message_id: XcmHash::default(),
        topic: None,
    };

    // Calculate the expected execution fee for the execution weight
    let expected_execution_fee = execution_fee(
        weight,
        ExecutionPayment::get_units_per_second(PARENT).unwrap(),
    );
    assert!(expected_execution_fee > 0); // sanity check

    // 1. Buy weight and expect it to be successful
    let result = fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into(), &ctx);
    if let Ok(assets) = result {
        // We expect only one unused payment asset and specific amount
        assert_eq!(assets.len(), 1);
        assert_ok!(assets.ensure_contains(
            &Asset::from((PARENT, total_payment - expected_execution_fee)).into()
        ));

        assert_eq!(fixed_rate_trader.consumed, expected_execution_fee);
        assert_eq!(fixed_rate_trader.weight, weight);
        assert_eq!(
            fixed_rate_trader.asset_location_and_units_per_second,
            Some((
                PARENT,
                ExecutionPayment::get_units_per_second(PARENT).unwrap()
            ))
        );
    } else {
        panic!("Should have been `Ok` wrapped Assets!");
    }

    // 2. Buy more weight, using the same trader and asset type. Verify it works as expected.
    let (old_weight, old_consumed) = (fixed_rate_trader.weight, fixed_rate_trader.consumed);

    let weight: Weight = Weight::from_parts(3_500_000_000, 0);
    let expected_execution_fee = execution_fee(
        weight,
        ExecutionPayment::get_units_per_second(PARENT).unwrap(),
    );
    assert!(expected_execution_fee > 0); // sanity check

    let result = fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into(), &ctx);
    if let Ok(assets) = result {
        // We expect only one unused payment asset and specific amount
        assert_eq!(assets.len(), 1);
        assert_ok!(assets.ensure_contains(
            &Asset::from((PARENT, total_payment - expected_execution_fee)).into()
        ));

        assert_eq!(
            fixed_rate_trader.consumed,
            expected_execution_fee + old_consumed
        );
        assert_eq!(fixed_rate_trader.weight, weight + old_weight);
        assert_eq!(
            fixed_rate_trader.asset_location_and_units_per_second,
            Some((
                PARENT,
                ExecutionPayment::get_units_per_second(PARENT).unwrap()
            ))
        );
    } else {
        panic!("Should have been `Ok` wrapped Assets!");
    }

    // 3. Buy even more weight, but use a different type of asset now while reusing the old trader instance.
    let (old_weight, old_consumed) = (fixed_rate_trader.weight, fixed_rate_trader.consumed);

    // Note that the concrete asset type differs now from previous buys
    let total_payment = 20_000;
    let payment_multi_asset = Asset {
        id: xcm::latest::AssetId((*PARACHAIN).clone()),
        fun: Fungibility::Fungible(total_payment),
    };

    let weight: Weight = Weight::from_parts(1_750_000_000, 0);
    let expected_execution_fee = execution_fee(
        weight,
        ExecutionPayment::get_units_per_second((*PARACHAIN).clone()).unwrap(),
    );
    assert!(expected_execution_fee > 0); // sanity check

    let result = fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into(), &ctx);
    if let Ok(assets) = result {
        // We expect only one unused payment asset and specific amount
        assert_eq!(assets.len(), 1);
        assert_ok!(assets.ensure_contains(
            &Asset::from(((*PARACHAIN).clone(), total_payment - expected_execution_fee)).into()
        ));

        assert_eq!(fixed_rate_trader.weight, weight + old_weight);
        // We don't expect this to change since trader already contains data about previous asset type.
        // Current rule is not to update in this case.
        assert_eq!(fixed_rate_trader.consumed, old_consumed);
        assert_eq!(
            fixed_rate_trader.asset_location_and_units_per_second,
            Some((
                PARENT,
                ExecutionPayment::get_units_per_second(PARENT).unwrap()
            ))
        );
    } else {
        panic!("Should have been `Ok` wrapped Assets!");
    }
}

#[test]
fn fixed_rate_of_foreign_asset_buy_execution_fails() {
    let mut fixed_rate_trader = FixedRateOfForeignAsset::<ExecutionPayment, ()>::new();

    // The amount we have designated for payment (doesn't mean it will be used though)
    let total_payment = 1000;
    let payment_multi_asset = Asset {
        id: xcm::latest::AssetId(PARENT),
        fun: Fungibility::Fungible(total_payment),
    };
    let weight: Weight = Weight::from_parts(3_000_000_000, 0);
    let ctx = XcmContext {
        // arbitary ML
        origin: Some(Location::here()),
        message_id: XcmHash::default(),
        topic: None,
    };

    // Calculate the expected execution fee for the execution weight
    let expected_execution_fee = execution_fee(
        weight,
        ExecutionPayment::get_units_per_second(PARENT).unwrap(),
    );
    // sanity check, should be more for UT to make sense
    assert!(expected_execution_fee > total_payment);

    // Expect failure because we lack the required funds
    assert_eq!(
        fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into(), &ctx),
        Err(XcmError::TooExpensive)
    );

    // Try to pay with unsupported funds, expect failure
    let payment_multi_asset = Asset {
        id: xcm::latest::AssetId(Location::here()),
        fun: Fungibility::Fungible(total_payment),
    };
    assert_eq!(
        fixed_rate_trader.buy_weight(Weight::zero(), payment_multi_asset.clone().into(), &ctx),
        Err(XcmError::TooExpensive)
    );
}

#[test]
fn fixed_rate_of_foreign_asset_refund_is_ok() {
    let mut fixed_rate_trader = FixedRateOfForeignAsset::<ExecutionPayment, ()>::new();

    // The amount we have designated for payment (doesn't mean it will be used though)
    let total_payment = 10_000;
    let payment_multi_asset = Asset {
        id: xcm::latest::AssetId(PARENT),
        fun: Fungibility::Fungible(total_payment),
    };
    let weight: Weight = Weight::from_parts(1_000_000_000, 0);
    let ctx = XcmContext {
        // arbitary ML
        origin: Some(Location::here()),
        message_id: XcmHash::default(),
        topic: None,
    };

    // Calculate the expected execution fee for the execution weight and buy it
    let expected_execution_fee = execution_fee(
        weight,
        ExecutionPayment::get_units_per_second(PARENT).unwrap(),
    );
    assert!(expected_execution_fee > 0); // sanity check
    assert_ok!(fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into(), &ctx));

    // Refund quarter and expect it to pass
    let weight_to_refund = weight / 4;
    let assets_to_refund = expected_execution_fee / 4;
    let (old_weight, old_consumed) = (fixed_rate_trader.weight, fixed_rate_trader.consumed);

    let result = fixed_rate_trader.refund_weight(weight_to_refund, &ctx);
    if let Some(asset_location) = result {
        assert_eq!(asset_location, (PARENT, assets_to_refund).into());

        assert_eq!(fixed_rate_trader.weight, old_weight - weight_to_refund);
        assert_eq!(fixed_rate_trader.consumed, old_consumed - assets_to_refund);
    }

    // Refund more than remains and expect it to pass (saturated)
    let assets_to_refund = fixed_rate_trader.consumed;

    let result = fixed_rate_trader.refund_weight(weight + Weight::from_parts(10000, 0), &ctx);
    if let Some(asset_location) = result {
        assert_eq!(asset_location, (PARENT, assets_to_refund).into());

        assert!(fixed_rate_trader.weight.is_zero());
        assert!(fixed_rate_trader.consumed.is_zero());
    }
}

#[test]
fn reserve_asset_filter_for_sibling_parachain_is_ok() {
    let asset_xc_location = Location {
        parents: 1,
        interior: [Parachain(20), GeneralIndex(30)].into(),
    };
    let multi_asset = Asset {
        id: xcm::latest::AssetId(asset_xc_location),
        fun: Fungibility::Fungible(123456),
    };
    let origin = Location {
        parents: 1,
        interior: [Parachain(20)].into(),
    };

    assert!(ReserveAssetFilter::contains(&multi_asset, &origin));
}

#[test]
fn reserve_asset_filter_for_relay_chain_is_ok() {
    let asset_xc_location = Location {
        parents: 1,
        interior: Here,
    };
    let multi_asset = Asset {
        id: xcm::latest::AssetId(asset_xc_location),
        fun: Fungibility::Fungible(123456),
    };
    let origin = Location {
        parents: 1,
        interior: Here,
    };

    assert!(ReserveAssetFilter::contains(&multi_asset, &origin));
}

#[test]
fn reserve_asset_filter_with_origin_mismatch() {
    let asset_xc_location = Location {
        parents: 1,
        interior: [Parachain(20), GeneralIndex(30)].into(),
    };
    let multi_asset = Asset {
        id: xcm::latest::AssetId(asset_xc_location),
        fun: Fungibility::Fungible(123456),
    };
    let origin = Location {
        parents: 1,
        interior: Here,
    };

    assert!(!ReserveAssetFilter::contains(&multi_asset, &origin));
}

#[test]
fn reserve_asset_filter_for_unsupported_asset_multi_location() {
    // 1st case
    let asset_xc_location = Location {
        parents: 0,
        interior: [Parachain(20), GeneralIndex(30)].into(),
    };
    let multi_asset = Asset {
        id: xcm::latest::AssetId(asset_xc_location),
        fun: Fungibility::Fungible(123456),
    };
    let origin = Location {
        parents: 0,
        interior: Here,
    };

    assert!(!ReserveAssetFilter::contains(&multi_asset, &origin));

    // 2nd case
    let asset_xc_location = Location {
        parents: 1,
        interior: [GeneralIndex(50), GeneralIndex(30)].into(),
    };
    let multi_asset = Asset {
        id: xcm::latest::AssetId(asset_xc_location),
        fun: Fungibility::Fungible(123456),
    };
    let origin = Location {
        parents: 1,
        interior: [GeneralIndex(50)].into(),
    };

    assert!(!ReserveAssetFilter::contains(&multi_asset, &origin));
}
