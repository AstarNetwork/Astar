// Copyright 2020-2021 Zenlink
// Licensed under GPL-3.0.

use frame_support::{assert_noop, assert_ok};

use super::{mock::*, AssetId, Error, MultiAssetsHandler};

const DOT_ASSET_ID: AssetId = AssetId { chain_id: 200, asset_type: LOCAL, asset_index: 2 };

const BTC_ASSET_ID: AssetId = AssetId { chain_id: 300, asset_type: RESERVED, asset_index: 3 };

const ETH_ASSET_ID: AssetId = AssetId { chain_id: 400, asset_type: NATIVE, asset_index: 0 };

const ALICE: u128 = 1;
const BOB: u128 = 2;
const CHARLIE: u128 = 3;

const PAIR_DOT_BTC: (AssetId, AssetId) = (
    AssetId { chain_id: 200, asset_type: LOCAL, asset_index: 2 },
    AssetId { chain_id: 300, asset_type: RESERVED, asset_index: 3 },
);
const PAIR_DOT_BTC_ACCOUNT: u128 = 64962681870856338328114322245433978733;
const LOCAL_LP_DOT_BTC: AssetId = AssetId { chain_id: 0, asset_type: LIQUIDITY, asset_index: 0 };

const PAIR_BTC_ETH: (AssetId, AssetId) = (
    AssetId { chain_id: 300, asset_type: RESERVED, asset_index: 3 },
    AssetId { chain_id: 400, asset_type: NATIVE, asset_index: 0 },
);
const PAIR_BTC_ETH_ACCOUNT: u128 = 290936349497416120426004117727903772525;
const LOCAL_LP_BTC_ETH: AssetId = AssetId { chain_id: 0, asset_type: LIQUIDITY, asset_index: 1 };

#[test]
fn local_lp_mint_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(DexPallet::lp_pairs(), vec![]);
        assert!(DexPallet::lp_metadata(PAIR_DOT_BTC).is_none());
        assert!(DexPallet::lp_metadata(PAIR_BTC_ETH).is_none());
        assert!(!DexPallet::lp_is_exists(LOCAL_LP_DOT_BTC));
        assert!(!DexPallet::lp_is_exists(LOCAL_LP_BTC_ETH));

        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(BTC_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(ETH_ASSET_ID, &ALICE, 0));

        assert!(<Test as Config>::MultiAssetsHandler::is_exists(DOT_ASSET_ID));
        assert!(<Test as Config>::MultiAssetsHandler::is_exists(BTC_ASSET_ID));
        assert!(<Test as Config>::MultiAssetsHandler::is_exists(ETH_ASSET_ID));

        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), DOT_ASSET_ID, BTC_ASSET_ID));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), BTC_ASSET_ID, ETH_ASSET_ID));

        assert_eq!(DexPallet::lp_pairs(), vec![PAIR_DOT_BTC, PAIR_BTC_ETH]);
        assert!(DexPallet::lp_is_exists(LOCAL_LP_DOT_BTC));
        assert!(DexPallet::lp_is_exists(LOCAL_LP_BTC_ETH));

        let meta_dot_btc = DexPallet::lp_metadata(PAIR_DOT_BTC).unwrap();
        assert_eq!(meta_dot_btc.0, PAIR_DOT_BTC_ACCOUNT);
        assert_eq!(meta_dot_btc.1, 0);
        let meta_btc_eth = DexPallet::lp_metadata(PAIR_BTC_ETH).unwrap();
        assert_eq!(meta_btc_eth.0, PAIR_BTC_ETH_ACCOUNT);
        assert_eq!(meta_btc_eth.1, 0);

        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 0);
        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_BTC_ETH), 0);
    });
}

#[test]
fn local_lp_mint_more_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(BTC_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(ETH_ASSET_ID, &ALICE, 0));

        assert_noop!(
            DexPallet::lp_mint(LOCAL_LP_DOT_BTC, &ALICE, 0),
            Error::<Test>::AssetNotExists,
        );
        assert_noop!(
            DexPallet::lp_burn(LOCAL_LP_BTC_ETH, &ALICE, 0),
            Error::<Test>::AssetNotExists,
        );

        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), DOT_ASSET_ID, BTC_ASSET_ID));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), BTC_ASSET_ID, ETH_ASSET_ID));

        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 0);
        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_BTC_ETH), 0);

        assert_ok!(DexPallet::lp_mint(LOCAL_LP_DOT_BTC, &ALICE, 100));
        assert_ok!(DexPallet::lp_mint(LOCAL_LP_BTC_ETH, &BOB, 200));

        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 100);
        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_BTC_ETH), 200);

        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 100);
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_BTC_ETH, &BOB), 200);
    });
}

#[test]
fn local_lp_querying_assets_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(BTC_ASSET_ID, &ALICE, 0));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), DOT_ASSET_ID, BTC_ASSET_ID));
        assert_ok!(DexPallet::lp_mint(LOCAL_LP_DOT_BTC, &ALICE, 100));

        assert_ok!(DexPallet::lp_transfer(LOCAL_LP_DOT_BTC, &ALICE, &BOB, 50));

        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 50);
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &BOB), 50);

        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 100);
    });
}

#[test]
fn local_lp_querying_total_supply_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(BTC_ASSET_ID, &ALICE, 0));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), DOT_ASSET_ID, BTC_ASSET_ID));
        assert_ok!(DexPallet::lp_mint(LOCAL_LP_DOT_BTC, &ALICE, 100));

        assert_ok!(DexPallet::lp_transfer(LOCAL_LP_DOT_BTC, &ALICE, &BOB, 50));

        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 50);
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &BOB), 50);

        assert_ok!(DexPallet::lp_transfer(LOCAL_LP_DOT_BTC, &BOB, &CHARLIE, 31));

        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 50);
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &BOB), 19);
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &CHARLIE), 31);

        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 100);
    });
}

#[test]
fn transferring_amount_above_available_balance_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(BTC_ASSET_ID, &ALICE, 0));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), DOT_ASSET_ID, BTC_ASSET_ID));
        assert_ok!(DexPallet::lp_mint(LOCAL_LP_DOT_BTC, &ALICE, 100));
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 100);

        assert_ok!(DexPallet::transfer(Origin::signed(ALICE), LOCAL_LP_DOT_BTC, BOB, 50));

        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 50);
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &BOB), 50);
        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 100);
    });
}

#[test]
fn transferring_zero_unit_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(BTC_ASSET_ID, &ALICE, 0));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), DOT_ASSET_ID, BTC_ASSET_ID));
        assert_ok!(DexPallet::lp_mint(LOCAL_LP_DOT_BTC, &ALICE, 100));
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 100);

        assert_ok!(<Test as Config>::MultiAssetsHandler::transfer(
            LOCAL_LP_DOT_BTC,
            &ALICE,
            &BOB,
            0
        ));

        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 100);
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &BOB), 0);
        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 100);
    });
}

#[test]
fn transferring_more_units_than_total_supply_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(BTC_ASSET_ID, &ALICE, 0));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), DOT_ASSET_ID, BTC_ASSET_ID));
        assert_ok!(DexPallet::lp_mint(LOCAL_LP_DOT_BTC, &ALICE, 100));
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 100);

        assert_noop!(
            DexPallet::transfer(Origin::signed(ALICE), LOCAL_LP_DOT_BTC, BOB, 101),
            Error::<Test>::InsufficientAssetBalance
        );
    });
}

#[test]
fn local_lp_burn_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(BTC_ASSET_ID, &ALICE, 0));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), DOT_ASSET_ID, BTC_ASSET_ID));
        assert_ok!(DexPallet::lp_mint(LOCAL_LP_DOT_BTC, &ALICE, 100));
        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 100);

        assert_ok!(DexPallet::lp_burn(LOCAL_LP_DOT_BTC, &ALICE, 100));
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 0);
        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 0);
    });
}

#[test]
fn local_lp_burn_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(BTC_ASSET_ID, &ALICE, 0));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), DOT_ASSET_ID, BTC_ASSET_ID));
        assert_ok!(DexPallet::lp_mint(LOCAL_LP_DOT_BTC, &ALICE, 0));
        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 0);

        assert_noop!(
            DexPallet::lp_burn(LOCAL_LP_DOT_BTC, &ALICE, 1),
            Error::<Test>::InsufficientLiquidity,
        );

        assert_ok!(DexPallet::lp_burn(LOCAL_LP_DOT_BTC, &ALICE, 0));
    });
}

#[test]
fn local_lp_multi_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(BTC_ASSET_ID, &ALICE, 0));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), DOT_ASSET_ID, BTC_ASSET_ID));

        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(LOCAL_LP_DOT_BTC, &ALICE, 100));
        assert_eq!(<Test as Config>::MultiAssetsHandler::balance_of(LOCAL_LP_DOT_BTC, &ALICE), 100);
        assert_eq!(<Test as Config>::MultiAssetsHandler::total_supply(LOCAL_LP_DOT_BTC), 100);

        assert_ok!(<Test as Config>::MultiAssetsHandler::withdraw(LOCAL_LP_DOT_BTC, &ALICE, 100));

        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 0);
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 0);

        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(LOCAL_LP_DOT_BTC, &ALICE, 200));
        assert_eq!(<Test as Config>::MultiAssetsHandler::balance_of(LOCAL_LP_DOT_BTC, &ALICE), 200);
        assert_eq!(<Test as Config>::MultiAssetsHandler::total_supply(LOCAL_LP_DOT_BTC), 200);

        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 200);
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 200);

        assert_ok!(DexPallet::transfer(Origin::signed(ALICE), LOCAL_LP_DOT_BTC, BOB, 50));

        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 150);
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &BOB), 50);
        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 200);

        assert_noop!(
            DexPallet::lp_burn(LOCAL_LP_DOT_BTC, &ALICE, 200),
            Error::<Test>::InsufficientLiquidity,
        );
    });
}

#[test]
fn local_lp_multi_asset_total_supply_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(BTC_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(ETH_ASSET_ID, &ALICE, 0));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), DOT_ASSET_ID, BTC_ASSET_ID));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), BTC_ASSET_ID, ETH_ASSET_ID));

        assert_ok!(DexPallet::lp_mint(LOCAL_LP_DOT_BTC, &ALICE, 100));
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 100);
        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 100);

        assert_ok!(DexPallet::lp_mint(LOCAL_LP_BTC_ETH, &ALICE, 200));
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_BTC_ETH, &ALICE), 200);
        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_BTC_ETH), 200);

        assert_eq!(DexPallet::lp_pairs(), vec![PAIR_DOT_BTC, PAIR_BTC_ETH]);
    });
}

#[test]
fn local_lp_multi_asset_withdraw_to_zenlink_module_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(BTC_ASSET_ID, &ALICE, 0));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), DOT_ASSET_ID, BTC_ASSET_ID));

        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 0);
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 0);

        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(LOCAL_LP_DOT_BTC, &ALICE, 100));
        assert_ok!(<Test as Config>::MultiAssetsHandler::withdraw(LOCAL_LP_DOT_BTC, &ALICE, 50));

        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 50);
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 50);
    });
}

#[test]
fn local_lp_multi_asset_transfer_should_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            <Test as Config>::MultiAssetsHandler::transfer(LOCAL_LP_DOT_BTC, &ALICE, &BOB, 50),
            Error::<Test>::AssetNotExists
        );

        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(DOT_ASSET_ID, &ALICE, 0));
        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(BTC_ASSET_ID, &ALICE, 0));
        assert_ok!(DexPallet::create_pair(Origin::signed(ALICE), DOT_ASSET_ID, BTC_ASSET_ID));

        assert_noop!(
            <Test as Config>::MultiAssetsHandler::transfer(LOCAL_LP_DOT_BTC, &ALICE, &BOB, 50),
            Error::<Test>::InsufficientAssetBalance
        );

        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(LOCAL_LP_DOT_BTC, &ALICE, 100));
        assert_ok!(<Test as Config>::MultiAssetsHandler::transfer(
            LOCAL_LP_DOT_BTC,
            &ALICE,
            &BOB,
            50
        ));

        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &ALICE), 50);
        assert_eq!(DexPallet::lp_balance_of(LOCAL_LP_DOT_BTC, &BOB), 50);
        assert_eq!(DexPallet::lp_total_supply(LOCAL_LP_DOT_BTC), 100);
    });
}

#[test]
fn local_lp_multi_deposit_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            <Test as Config>::MultiAssetsHandler::deposit(LOCAL_LP_DOT_BTC, &ALICE, 0),
            Error::<Test>::AssetNotExists
        );
    });
}

#[test]
fn foreign_lp_multi_deposit_should_work() {
    new_test_ext().execute_with(|| {
        let foreign_lp = AssetId { chain_id: 100, asset_type: LIQUIDITY, asset_index: 0 };

        assert_ok!(<Test as Config>::MultiAssetsHandler::deposit(foreign_lp, &ALICE, 0));

        assert_eq!(DexPallet::lp_pairs(), vec![]);
        assert_eq!(DexPallet::foreign_list(), vec![foreign_lp]);
    });
}
