// Copyright 2020-2021 Zenlink
// Licensed under GPL-3.0.

use frame_support::{assert_noop, assert_ok};

use super::{mock::*, AssetId, Error, MultiAssetsHandler};

// Native Currency
const DEV_ASSET_ID: AssetId = AssetId { chain_id: 0, asset_type: NATIVE, asset_index: 0 };

// Foreign Liquidity
const LP_ASSET_ID: AssetId = AssetId { chain_id: 100, asset_type: LIQUIDITY, asset_index: 1 };

const DOT_ASSET_ID: AssetId = AssetId { chain_id: 200, asset_type: LOCAL, asset_index: 2 };

const BTC_ASSET_ID: AssetId = AssetId { chain_id: 300, asset_type: RESERVED, asset_index: 3 };

const ETH_ASSET_ID: AssetId = AssetId { chain_id: 400, asset_type: NATIVE, asset_index: 0 };

const ALICE: u128 = 1;
const BOB: u128 = 2;
const CHARLIE: u128 = 3;

#[test]
fn foreign_mint_should_work() {
    new_test_ext().execute_with(|| {
        assert!(!DexPallet::foreign_is_exists(LP_ASSET_ID));
        assert!(!DexPallet::foreign_is_exists(DOT_ASSET_ID));
        assert!(!DexPallet::foreign_is_exists(BTC_ASSET_ID));
        assert!(!DexPallet::foreign_is_exists(ETH_ASSET_ID));

        assert_ok!(DexPallet::foreign_mint(LP_ASSET_ID, &ALICE, 0));
        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &ALICE, 100));
        assert_ok!(DexPallet::foreign_mint(ETH_ASSET_ID, &ALICE, 1000));

        // Native Currency
        assert!(<Test as Config>::MultiAssetsHandler::is_exists(DEV_ASSET_ID));

        assert!(DexPallet::foreign_is_exists(LP_ASSET_ID));
        assert!(DexPallet::foreign_is_exists(DOT_ASSET_ID));
        assert!(DexPallet::foreign_is_exists(BTC_ASSET_ID));
        assert!(DexPallet::foreign_is_exists(ETH_ASSET_ID));

        assert_eq!(DexPallet::foreign_balance_of(LP_ASSET_ID, &ALICE), 0);
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 0);
        assert_eq!(DexPallet::foreign_balance_of(BTC_ASSET_ID, &ALICE), 100);
        assert_eq!(DexPallet::foreign_balance_of(ETH_ASSET_ID, &ALICE), 1000);
    });
}

#[test]
fn foreign_mint_more_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &BOB, 100));
        assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &BOB, 100));
        assert_ok!(DexPallet::foreign_mint(ETH_ASSET_ID, &BOB, 100));

        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &CHARLIE, 1000));
        assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &CHARLIE, 1000));
        assert_ok!(DexPallet::foreign_mint(ETH_ASSET_ID, &CHARLIE, 1000));

        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &BOB), 100);
        assert_eq!(DexPallet::foreign_balance_of(BTC_ASSET_ID, &BOB), 100);
        assert_eq!(DexPallet::foreign_balance_of(ETH_ASSET_ID, &BOB), 100);

        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &CHARLIE), 1000);
        assert_eq!(DexPallet::foreign_balance_of(BTC_ASSET_ID, &CHARLIE), 1000);
        assert_eq!(DexPallet::foreign_balance_of(ETH_ASSET_ID, &CHARLIE), 1000);
    });
}

#[test]
fn querying_assets_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 100));
        assert_ok!(DexPallet::foreign_transfer(DOT_ASSET_ID, &ALICE, &BOB, 50));

        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 50);
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &BOB), 50);

        assert_eq!(DexPallet::foreign_total_supply(DOT_ASSET_ID), 100);
    });
}

#[test]
fn querying_total_supply_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 100));
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 100);

        assert_ok!(DexPallet::foreign_transfer(DOT_ASSET_ID, &ALICE, &BOB, 50));
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 50);
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &BOB), 50);

        assert_ok!(DexPallet::foreign_transfer(DOT_ASSET_ID, &BOB, &CHARLIE, 31));
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 50);
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &BOB), 19);
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &CHARLIE), 31);

        assert_eq!(DexPallet::foreign_total_supply(DOT_ASSET_ID), 100);
    });
}

#[test]
fn transferring_amount_above_available_balance_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 100));
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 100);

        assert_ok!(DexPallet::transfer(Origin::signed(ALICE), DOT_ASSET_ID, BOB, 50));

        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 50);
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &BOB), 50);
    });
}

#[test]
fn transferring_zero_unit_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 100));
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 100);
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &BOB), 0);

        assert_ok!(DexPallet::transfer(Origin::signed(ALICE), DOT_ASSET_ID, BOB, 0));

        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 100);
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &BOB), 0);
    });
}

#[test]
fn transferring_more_units_than_total_supply_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 100));
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 100);
        assert_noop!(
            DexPallet::transfer(Origin::signed(ALICE), DOT_ASSET_ID, BOB, 101),
            Error::<Test>::InsufficientAssetBalance
        );
    });
}

#[test]
fn foreign_burn_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 100));
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 100);
        assert_eq!(DexPallet::foreign_total_supply(DOT_ASSET_ID), 100);

        assert_ok!(DexPallet::foreign_burn(DOT_ASSET_ID, &ALICE, 100));
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 0);
        assert_eq!(DexPallet::foreign_total_supply(DOT_ASSET_ID), 0);
    });
}

#[test]
fn foreign_burn_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 100));
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 100);
        assert_eq!(DexPallet::foreign_total_supply(DOT_ASSET_ID), 100);

        assert_noop!(
            DexPallet::foreign_burn(DOT_ASSET_ID, &ALICE, 200),
            Error::<Test>::InsufficientAssetBalance,
        );
    });
}

#[test]
fn foreign_mint_transfer_burn_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 100));
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 100);
        assert_eq!(DexPallet::foreign_total_supply(DOT_ASSET_ID), 100);

        assert_ok!(DexPallet::foreign_burn(DOT_ASSET_ID, &ALICE, 100));

        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 200));
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 200);
        assert_eq!(DexPallet::foreign_total_supply(DOT_ASSET_ID), 200);

        assert_ok!(DexPallet::transfer(Origin::signed(ALICE), DOT_ASSET_ID, BOB, 50));

        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 150);
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &BOB), 50);

        assert_noop!(
            DexPallet::foreign_burn(DOT_ASSET_ID, &ALICE, 200),
            Error::<Test>::InsufficientAssetBalance,
        );
    });
}

#[test]
fn foreign_multi_asset_total_supply_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 100));
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 100);
        assert_eq!(DexPallet::foreign_total_supply(DOT_ASSET_ID), 100);

        assert_ok!(DexPallet::foreign_mint(BTC_ASSET_ID, &BOB, 100));
        assert_eq!(DexPallet::foreign_balance_of(BTC_ASSET_ID, &BOB), 100);
        assert_eq!(DexPallet::foreign_total_supply(BTC_ASSET_ID), 100);

        assert_ok!(DexPallet::foreign_mint(ETH_ASSET_ID, &CHARLIE, 100));
        assert_eq!(DexPallet::foreign_balance_of(ETH_ASSET_ID, &CHARLIE), 100);
        assert_eq!(DexPallet::foreign_total_supply(ETH_ASSET_ID), 100);

        assert_eq!(
            DexPallet::foreign_list(),
            vec![
                AssetId { chain_id: 200, asset_type: LOCAL, asset_index: 2 },
                AssetId { chain_id: 300, asset_type: RESERVED, asset_index: 3 },
                AssetId { chain_id: 400, asset_type: NATIVE, asset_index: 0 }
            ]
        );
    });
}

#[test]
fn foreign_multi_asset_withdraw_to_zenlink_module_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(<Test as Config>::MultiAssetsHandler::total_supply(DOT_ASSET_ID), 0);
        assert_eq!(<Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &ALICE), 0);

        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 100));
        assert_ok!(<Test as Config>::MultiAssetsHandler::withdraw(DOT_ASSET_ID, &ALICE, 50));

        assert_eq!(<Test as Config>::MultiAssetsHandler::total_supply(DOT_ASSET_ID), 50);
        assert_eq!(<Test as Config>::MultiAssetsHandler::balance_of(DOT_ASSET_ID, &ALICE), 50);
    });
}

#[test]
fn foreign_multi_asset_transfer_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(DexPallet::foreign_mint(DOT_ASSET_ID, &ALICE, 100));
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 100);
        assert_eq!(DexPallet::foreign_total_supply(DOT_ASSET_ID), 100);

        assert_ok!(<Test as Config>::MultiAssetsHandler::transfer(DOT_ASSET_ID, &ALICE, &BOB, 50));

        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &ALICE), 50);
        assert_eq!(DexPallet::foreign_balance_of(DOT_ASSET_ID, &BOB), 50);
        assert_eq!(DexPallet::foreign_total_supply(DOT_ASSET_ID), 100);
    });
}
