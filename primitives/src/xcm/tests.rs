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
use frame_support::assert_ok;
use sp_runtime::traits::Zero;
use xcm_executor::traits::Convert;

type AssetId = u128;

// Primitive, perhaps I improve it later
const PARENT: MultiLocation = MultiLocation::parent();
const PARACHAIN: MultiLocation = MultiLocation {
    parents: 1,
    interior: Junctions::X1(Parachain(10)),
};
const GENERAL_INDEX: MultiLocation = MultiLocation {
    parents: 2,
    interior: Junctions::X1(GeneralIndex(20)),
};
const RELAY_ASSET: AssetId = AssetId::MAX;

/// Helper struct used for testing `AssetLocationIdConverter`
struct AssetLocationMapper;
impl XcAssetLocation<AssetId> for AssetLocationMapper {
    fn get_xc_asset_location(asset_id: AssetId) -> Option<MultiLocation> {
        match asset_id {
            RELAY_ASSET => Some(PARENT),
            20 => Some(PARACHAIN),
            30 => Some(GENERAL_INDEX),
            _ => None,
        }
    }

    fn get_asset_id(asset_location: MultiLocation) -> Option<AssetId> {
        match asset_location {
            a if a == PARENT => Some(RELAY_ASSET),
            a if a == PARACHAIN => Some(20),
            a if a == GENERAL_INDEX => Some(30),
            _ => None,
        }
    }
}

/// Helper struct used for testing `FixedRateOfForeignAsset`
struct ExecutionPayment;
impl ExecutionPaymentRate for ExecutionPayment {
    fn get_units_per_second(asset_location: MultiLocation) -> Option<u128> {
        match asset_location {
            a if a == PARENT => Some(1_000_000),
            a if a == PARACHAIN => Some(2_000_000),
            a if a == GENERAL_INDEX => Some(3_000_000),
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
    // Test cases where the MultiLocation is valid
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert_ref(PARENT),
        Ok(u128::MAX)
    );
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert_ref(PARACHAIN),
        Ok(20)
    );
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert_ref(GENERAL_INDEX),
        Ok(30)
    );

    // Test case where MultiLocation isn't supported
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert_ref(MultiLocation::here()),
        Err(())
    );
}

#[test]
fn asset_id_to_location() {
    // Test cases where the AssetId is valid
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::reverse_ref(u128::MAX),
        Ok(PARENT)
    );
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::reverse_ref(20),
        Ok(PARACHAIN)
    );
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::reverse_ref(30),
        Ok(GENERAL_INDEX)
    );

    // Test case where the AssetId isn't supported
    assert_eq!(
        AssetLocationIdConverter::<AssetId, AssetLocationMapper>::reverse_ref(0),
        Err(())
    );
}

#[test]
fn fixed_rate_of_foreign_asset_buy_is_ok() {
    let mut fixed_rate_trader = FixedRateOfForeignAsset::<ExecutionPayment, ()>::new();

    // The amount we have designated for payment (doesn't mean it will be used though)
    let total_payment = 10_000;
    let payment_multi_asset = MultiAsset {
        id: xcm::latest::AssetId::Concrete(PARENT),
        fun: Fungibility::Fungible(total_payment),
    };
    let weight: Weight = Weight::from_parts(1_000_000_000, 0);

    // Calculate the expected execution fee for the execution weight
    let expected_execution_fee = execution_fee(
        weight,
        ExecutionPayment::get_units_per_second(PARENT).unwrap(),
    );
    assert!(expected_execution_fee > 0); // sanity check

    // 1. Buy weight and expect it to be successful
    let result = fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into());
    if let Ok(assets) = result {
        // We expect only one unused payment asset and specific amount
        assert_eq!(assets.len(), 1);
        assert_ok!(assets.ensure_contains(
            &MultiAsset::from((PARENT, total_payment - expected_execution_fee)).into()
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

    let result = fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into());
    if let Ok(assets) = result {
        // We expect only one unused payment asset and specific amount
        assert_eq!(assets.len(), 1);
        assert_ok!(assets.ensure_contains(
            &MultiAsset::from((PARENT, total_payment - expected_execution_fee)).into()
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
    let payment_multi_asset = MultiAsset {
        id: xcm::latest::AssetId::Concrete(PARACHAIN),
        fun: Fungibility::Fungible(total_payment),
    };

    let weight: Weight = Weight::from_parts(1_750_000_000, 0);
    let expected_execution_fee = execution_fee(
        weight,
        ExecutionPayment::get_units_per_second(PARACHAIN).unwrap(),
    );
    assert!(expected_execution_fee > 0); // sanity check

    let result = fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into());
    if let Ok(assets) = result {
        // We expect only one unused payment asset and specific amount
        assert_eq!(assets.len(), 1);
        assert_ok!(assets.ensure_contains(
            &MultiAsset::from((PARACHAIN, total_payment - expected_execution_fee)).into()
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
    let payment_multi_asset = MultiAsset {
        id: xcm::latest::AssetId::Concrete(PARENT),
        fun: Fungibility::Fungible(total_payment),
    };
    let weight: Weight = Weight::from_parts(3_000_000_000, 0);

    // Calculate the expected execution fee for the execution weight
    let expected_execution_fee = execution_fee(
        weight,
        ExecutionPayment::get_units_per_second(PARENT).unwrap(),
    );
    // sanity check, should be more for UT to make sense
    assert!(expected_execution_fee > total_payment);

    // Expect failure because we lack the required funds
    assert_eq!(
        fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into()),
        Err(XcmError::TooExpensive)
    );

    // Try to pay with unsupported funds, expect failure
    let payment_multi_asset = MultiAsset {
        id: xcm::latest::AssetId::Concrete(MultiLocation::here()),
        fun: Fungibility::Fungible(total_payment),
    };
    assert_eq!(
        fixed_rate_trader.buy_weight(Weight::zero(), payment_multi_asset.clone().into()),
        Err(XcmError::TooExpensive)
    );
}

#[test]
fn fixed_rate_of_foreign_asset_refund_is_ok() {
    let mut fixed_rate_trader = FixedRateOfForeignAsset::<ExecutionPayment, ()>::new();

    // The amount we have designated for payment (doesn't mean it will be used though)
    let total_payment = 10_000;
    let payment_multi_asset = MultiAsset {
        id: xcm::latest::AssetId::Concrete(PARENT),
        fun: Fungibility::Fungible(total_payment),
    };
    let weight: Weight = Weight::from_parts(1_000_000_000, 0);

    // Calculate the expected execution fee for the execution weight and buy it
    let expected_execution_fee = execution_fee(
        weight,
        ExecutionPayment::get_units_per_second(PARENT).unwrap(),
    );
    assert!(expected_execution_fee > 0); // sanity check
    assert_ok!(fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into()));

    // Refund quarter and expect it to pass
    let weight_to_refund = weight / 4;
    let assets_to_refund = expected_execution_fee / 4;
    let (old_weight, old_consumed) = (fixed_rate_trader.weight, fixed_rate_trader.consumed);

    let result = fixed_rate_trader.refund_weight(weight_to_refund);
    if let Some(asset_location) = result {
        assert_eq!(asset_location, (PARENT, assets_to_refund).into());

        assert_eq!(fixed_rate_trader.weight, old_weight - weight_to_refund);
        assert_eq!(fixed_rate_trader.consumed, old_consumed - assets_to_refund);
    }

    // Refund more than remains and expect it to pass (saturated)
    let assets_to_refund = fixed_rate_trader.consumed;

    let result = fixed_rate_trader.refund_weight(weight + Weight::from_parts(10000, 0));
    if let Some(asset_location) = result {
        assert_eq!(asset_location, (PARENT, assets_to_refund).into());

        assert!(fixed_rate_trader.weight.is_zero());
        assert!(fixed_rate_trader.consumed.is_zero());
    }
}

#[test]
fn reserve_asset_filter_for_sibling_parachain_is_ok() {
    let asset_xc_location = MultiLocation {
        parents: 1,
        interior: X2(Parachain(20), GeneralIndex(30)),
    };
    let multi_asset = MultiAsset {
        id: xcm::latest::AssetId::Concrete(asset_xc_location),
        fun: Fungibility::Fungible(123456),
    };
    let origin = MultiLocation {
        parents: 1,
        interior: X1(Parachain(20)),
    };

    assert!(ReserveAssetFilter::contains(&multi_asset, &origin));
}

#[test]
fn reserve_asset_filter_for_relay_chain_is_ok() {
    let asset_xc_location = MultiLocation {
        parents: 1,
        interior: Here,
    };
    let multi_asset = MultiAsset {
        id: xcm::latest::AssetId::Concrete(asset_xc_location),
        fun: Fungibility::Fungible(123456),
    };
    let origin = MultiLocation {
        parents: 1,
        interior: Here,
    };

    assert!(ReserveAssetFilter::contains(&multi_asset, &origin));
}

#[test]
fn reserve_asset_filter_with_origin_mismatch() {
    let asset_xc_location = MultiLocation {
        parents: 1,
        interior: X2(Parachain(20), GeneralIndex(30)),
    };
    let multi_asset = MultiAsset {
        id: xcm::latest::AssetId::Concrete(asset_xc_location),
        fun: Fungibility::Fungible(123456),
    };
    let origin = MultiLocation {
        parents: 1,
        interior: Here,
    };

    assert!(!ReserveAssetFilter::contains(&multi_asset, &origin));
}

#[test]
fn reserve_asset_filter_for_unsupported_asset_multi_location() {
    // 1st case
    let asset_xc_location = MultiLocation {
        parents: 0,
        interior: X2(Parachain(20), GeneralIndex(30)),
    };
    let multi_asset = MultiAsset {
        id: xcm::latest::AssetId::Concrete(asset_xc_location),
        fun: Fungibility::Fungible(123456),
    };
    let origin = MultiLocation {
        parents: 0,
        interior: Here,
    };

    assert!(!ReserveAssetFilter::contains(&multi_asset, &origin));

    // 2nd case
    let asset_xc_location = MultiLocation {
        parents: 1,
        interior: X2(GeneralIndex(50), GeneralIndex(30)),
    };
    let multi_asset = MultiAsset {
        id: xcm::latest::AssetId::Concrete(asset_xc_location),
        fun: Fungibility::Fungible(123456),
    };
    let origin = MultiLocation {
        parents: 1,
        interior: X1(GeneralIndex(50)),
    };

    assert!(!ReserveAssetFilter::contains(&multi_asset, &origin));
}

// TODO: can be deleted after uplift to `polkadot-v0.9.44` or beyond.
#[test]
fn hashed_description_sanity_check() {
    let acc_key_20_mul = MultiLocation {
        parents: 1,
        interior: X2(
            Parachain(1),
            AccountKey20 {
                network: None,
                key: [7u8; 20],
            },
        ),
    };
    // Ensure derived value is same as it would be using `polkadot-v0.9.44` code.
    let derived_account =
        HashedDescription::<[u8; 32], DescribeFamily<DescribeAllTerminal>>::convert(acc_key_20_mul);
    assert_eq!(
        derived_account,
        Ok([
            61_u8, 117, 247, 231, 100, 219, 128, 176, 180, 200, 187, 102, 93, 107, 187, 145, 25,
            146, 50, 248, 244, 153, 83, 95, 207, 165, 90, 10, 220, 39, 23, 49
        ])
    );

    let acc_id_32_mul = MultiLocation {
        parents: 1,
        interior: X2(
            Parachain(50),
            AccountId32 {
                network: None,
                id: [3; 32].into(),
            },
        ),
    };
    // Ensure derived value is same as it would be using `polkadot-v0.9.44` code.
    let derived_account =
        HashedDescription::<[u8; 32], DescribeFamily<DescribeAllTerminal>>::convert(acc_id_32_mul);
    assert_eq!(
        derived_account,
        Ok([
            123, 171, 79, 159, 78, 47, 62, 233, 108, 149, 131, 249, 23, 192, 178, 52, 235, 133,
            147, 145, 152, 89, 129, 92, 63, 79, 211, 235, 213, 152, 201, 205
        ])
    );
}
