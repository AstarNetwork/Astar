// Copyright 2020-2021 Zenlink
// Licensed under GPL-3.0.

use frame_support::{assert_noop, assert_ok};

use super::mock::*;
use crate::{AssetId, Error, MultiAssetsHandler};
use sp_runtime::traits::IntegerSquareRoot;

const DOT_ASSET_ID: AssetId = AssetId {
	chain_id: 200,
	asset_type: LOCAL,
	asset_index: 2,
};

const BTC_ASSET_ID: AssetId = AssetId {
	chain_id: 300,
	asset_type: RESERVED,
	asset_index: 3,
};

const PAIR_DOT_BTC: u128 = 64962681870856338328114322245433978733;

const ALICE: u128 = 1;
const BOB: u128 = 2;
const CHARLIE: u128 = 3;
const DOT_UNIT: u128 = 1000_000_000_000_000;
const BTC_UNIT: u128 = 1000_000_00;
const LP_DOT_BTC: AssetId = AssetId {
	chain_id: 0,
	asset_type: 2,
	asset_index: 0,
};

#[test]
fn fee_meta_getter_should_work() {
	new_test_ext().execute_with(|| {
		let (fee_admin, fee_receiver, fee_point) = DexPallet::fee_meta();

		assert_eq!(fee_admin, ALICE);
		assert_eq!(fee_receiver, None);
		assert_eq!(fee_point, 5);
	})
}

#[test]
fn fee_meta_setter_should_not_work() {
	new_test_ext().execute_with(|| {
		let (fee_admin, fee_receiver, fee_point) = DexPallet::fee_meta();

		assert_eq!(fee_admin, ALICE);
		assert_eq!(fee_receiver, None);
		assert_eq!(fee_point, 5);

		assert_noop!(
			DexPallet::set_fee_admin(Origin::signed(BOB), BOB),
			Error::<Test>::RequireProtocolAdmin
		);

		assert_noop!(
			DexPallet::set_fee_receiver(Origin::signed(BOB), Some(BOB)),
			Error::<Test>::RequireProtocolAdmin
		);

		assert_noop!(
			DexPallet::set_fee_point(Origin::signed(BOB), 0),
			Error::<Test>::RequireProtocolAdmin
		);

		assert_noop!(
			DexPallet::set_fee_point(Origin::signed(ALICE), 31u8),
			Error::<Test>::InvalidFeePoint
		);
	})
}

#[test]
fn fee_meta_setter_should_work() {
	new_test_ext().execute_with(|| {
		let (fee_admin, fee_receiver, fee_point) = DexPallet::fee_meta();

		assert_eq!(fee_admin, ALICE);
		assert_eq!(fee_receiver, None);
		assert_eq!(fee_point, 5);

		assert_ok!(DexPallet::set_fee_admin(Origin::signed(ALICE), BOB));
		assert_ok!(DexPallet::set_fee_receiver(Origin::signed(BOB), Some(BOB)));
		assert_ok!(DexPallet::set_fee_point(Origin::signed(BOB), 0));

		let (fee_admin, fee_receiver, fee_point) = DexPallet::fee_meta();
		assert_eq!(fee_admin, BOB);
		assert_eq!(fee_receiver, Some(BOB));
		assert_eq!(fee_point, 0);
	})
}

#[test]
fn turn_on_protocol_fee_only_add_liquidity_no_fee_should_work() {
	new_test_ext().execute_with(|| {
		// 1. turn on the protocol fee
		// use default rate: 1/6

		let sorted_pair = DexPallet::sort_asset_id(DOT_ASSET_ID, BTC_ASSET_ID);

		assert_ok!(DexPallet::set_fee_receiver(Origin::signed(ALICE), Some(BOB)));
		assert_eq!(DexPallet::k_last(sorted_pair), 0);

		// 2. first add_liquidity

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::create_pair(
			Origin::signed(ALICE),
			DOT_ASSET_ID,
			BTC_ASSET_ID
		));

		let total_supply_dot: u128 = 1 * DOT_UNIT;
		let total_supply_btc: u128 = 1 * BTC_UNIT;

		assert_ok!(DexPallet::add_liquidity(
			Origin::signed(ALICE),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			total_supply_dot,
			total_supply_btc,
			0,
			0,
			100
		));

		let lp_of_alice_0 = 316227766016;
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &ALICE), lp_of_alice_0);
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &BOB), 0);
		assert_eq!(DexPallet::k_last(sorted_pair), DOT_UNIT * BTC_UNIT);

		// 3. second add_liquidity

		let total_supply_dot = 50 * DOT_UNIT;
		let total_supply_btc = 50 * BTC_UNIT;

		assert_ok!(DexPallet::add_liquidity(
			Origin::signed(ALICE),
			BTC_ASSET_ID,
			DOT_ASSET_ID,
			total_supply_btc,
			total_supply_dot,
			0,
			0,
			100
		));

		let lp_of_alice_1 = 16127616066816u128;
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &ALICE), lp_of_alice_1);
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &BOB), 0);
		assert_eq!(DexPallet::k_last(sorted_pair), 51 * DOT_UNIT * 51 * BTC_UNIT);

		let balance_dot = <Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &PAIR_DOT_BTC);
		let balance_btc = <Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &PAIR_DOT_BTC);

		//println!("balance_DOT {}, balance_BTC {}", balance_dot, balance_btc);
		assert_eq!(balance_dot, 51000000000000000);
		assert_eq!(balance_btc, 5100000000);
		assert_eq!((balance_dot / DOT_UNIT), (balance_btc / BTC_UNIT));

		// 4. third add_liquidity
		assert_ok!(DexPallet::add_liquidity(
			Origin::signed(ALICE),
			BTC_ASSET_ID,
			DOT_ASSET_ID,
			total_supply_btc,
			total_supply_dot,
			0,
			0,
			100
		));

		let lp_total = DexPallet::lp_total_supply(LP_DOT_BTC);
		let lp_of_alice_2 = 31939004367616u128;
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &ALICE), lp_of_alice_2);
		let lp_of_bob = 0u128;
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &BOB), lp_of_bob);
		assert_eq!(lp_total, lp_of_alice_2 + lp_of_bob);

		assert_eq!(DexPallet::k_last(sorted_pair), 101 * DOT_UNIT * 101 * BTC_UNIT);
	});
}

#[test]
fn turn_on_protocol_fee_remove_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		// 1. turn on the protocol fee
		// use default rate: 1/6

		let sorted_pair = DexPallet::sort_asset_id(DOT_ASSET_ID, BTC_ASSET_ID);

		assert_ok!(DexPallet::set_fee_receiver(Origin::signed(ALICE), Some(BOB)));
		assert_eq!(DexPallet::k_last(sorted_pair), 0);

		// 2. first add_liquidity

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::create_pair(
			Origin::signed(ALICE),
			DOT_ASSET_ID,
			BTC_ASSET_ID
		));

		let total_supply_dot: u128 = 1 * DOT_UNIT;
		let total_supply_btc: u128 = 1 * BTC_UNIT;

		assert_ok!(DexPallet::add_liquidity(
			Origin::signed(ALICE),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			total_supply_dot,
			total_supply_btc,
			0,
			0,
			100
		));

		let lp_of_alice_0 = 316227766016;
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &ALICE), 316227766016);
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &BOB), 0);
		assert_eq!(DexPallet::k_last(sorted_pair), DOT_UNIT * BTC_UNIT);

		// 3. second add_liquidity

		let total_supply_dot = 50 * DOT_UNIT;
		let total_supply_btc = 50 * BTC_UNIT;

		assert_ok!(DexPallet::add_liquidity(
			Origin::signed(ALICE),
			BTC_ASSET_ID,
			DOT_ASSET_ID,
			total_supply_btc,
			total_supply_dot,
			0,
			0,
			100
		));

		let lp_of_alice_1 = 16127616066816u128;
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &ALICE), lp_of_alice_1);
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &BOB), 0);
		assert_eq!(DexPallet::k_last(sorted_pair), 51 * DOT_UNIT * 51 * BTC_UNIT);

		let balance_dot = <Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &PAIR_DOT_BTC);
		let balance_btc = <Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &PAIR_DOT_BTC);

		//println!("balance_DOT {}, balance_BTC {}", balance_dot, balance_btc);
		assert_eq!(balance_dot, 51000000000000000);
		assert_eq!(balance_btc, 5100000000);
		assert_eq!((balance_dot / DOT_UNIT), (balance_btc / BTC_UNIT));

		// 4. remove_liquidity
		assert_ok!(DexPallet::remove_liquidity(
			Origin::signed(ALICE),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			lp_of_alice_0,
			0u128,
			0u128,
			ALICE,
			100
		));

		let lp_total = DexPallet::lp_total_supply(LP_DOT_BTC);
		assert_eq!(
			DexPallet::lp_balance_of(LP_DOT_BTC, &ALICE),
			lp_of_alice_1 - lp_of_alice_0
		);
		let lp_of_bob = 0u128;
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &BOB), lp_of_bob);
		assert_eq!(lp_total, lp_of_alice_1 - lp_of_alice_0 + lp_of_bob);
		assert_eq!(DexPallet::k_last(sorted_pair), 50 * DOT_UNIT * 50 * BTC_UNIT);
	});
}

#[test]
fn turn_on_protocol_fee_swap_have_fee_should_work() {
	new_test_ext().execute_with(|| {
		// 1. turn on the protocol fee

		let sorted_pair = DexPallet::sort_asset_id(DOT_ASSET_ID, BTC_ASSET_ID);

		assert_ok!(DexPallet::set_fee_receiver(Origin::signed(ALICE), Some(BOB)));
		// use default rate: 0.3% * 1 / 6 = 0.0005
		assert_ok!(DexPallet::set_fee_point(Origin::signed(ALICE), 5u8));
		assert_eq!(DexPallet::k_last(sorted_pair), 0);

		// 2. first add_liquidity

		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, u128::MAX));
		assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &CHARLIE, u128::MAX));
		assert_ok!(DexPallet::create_pair(
			Origin::signed(ALICE),
			DOT_ASSET_ID,
			BTC_ASSET_ID
		));

		let total_supply_dot: u128 = 1 * DOT_UNIT;
		let total_supply_btc: u128 = 1 * BTC_UNIT;

		assert_ok!(DexPallet::add_liquidity(
			Origin::signed(ALICE),
			DOT_ASSET_ID,
			BTC_ASSET_ID,
			total_supply_dot,
			total_supply_btc,
			0,
			0,
			100
		));

		let lp_of_alice_0 = 316227766016;
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &ALICE), lp_of_alice_0);
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &BOB), 0);
		assert_eq!(DexPallet::k_last(sorted_pair), DOT_UNIT * BTC_UNIT);

		// 3. swap

		assert_ok!(DexPallet::inner_swap_exact_assets_for_assets(
			&CHARLIE,
			DOT_UNIT,
			1,
			&vec![DOT_ASSET_ID, BTC_ASSET_ID],
			&CHARLIE,
		));

		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &ALICE), lp_of_alice_0);
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &BOB), 0);
		assert_eq!(DexPallet::k_last(sorted_pair), DOT_UNIT * BTC_UNIT);

		let balance_dot = <Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &PAIR_DOT_BTC);
		let balance_btc = <Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &PAIR_DOT_BTC);

		//println!("balance_DOT {}, balance_BTC {}", balance_dot, balance_btc);
		assert_eq!(balance_dot, 2000000000000000);
		assert_eq!(balance_btc, 50075113);

		let k_last = DexPallet::k_last(sorted_pair);
		let reserve_0 = <Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &PAIR_DOT_BTC);
		let reserve_1 = <Test as Config>::MultiAssetsHandler::balance_of(BTC_ASSET_ID, &PAIR_DOT_BTC);
		let root_k = reserve_0.saturating_mul(reserve_1).integer_sqrt();
		let root_k_last = k_last.integer_sqrt();

		assert!(root_k > root_k_last);

		let lp_total = DexPallet::lp_total_supply(LP_DOT_BTC);
		let numerator = lp_total.saturating_mul(root_k.saturating_sub(root_k_last));
		let denominator = root_k.saturating_mul(5).saturating_add(root_k_last);
		let expect_fee = numerator.checked_div(denominator).unwrap_or_default();

		// 4. second add_liquidity
		assert_ok!(DexPallet::add_liquidity(
			Origin::signed(ALICE),
			BTC_ASSET_ID,
			DOT_ASSET_ID,
			total_supply_btc,
			total_supply_dot,
			0,
			0,
			100
		));

		let lp_total = DexPallet::lp_total_supply(LP_DOT_BTC);
		let lp_of_alice_2 = 474361420078u128;
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &ALICE), lp_of_alice_2);

		let lp_of_bob = 39548424u128;
		assert_eq!(expect_fee, lp_of_bob);
		assert_eq!(DexPallet::lp_balance_of(LP_DOT_BTC, &BOB), expect_fee);
		assert_eq!(lp_total, lp_of_alice_2 + lp_of_bob);

		assert_eq!(DexPallet::k_last(sorted_pair), 225338007000000000000000);
	});
}
