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

use crate::setup::*;

use frame_support::traits::OnRuntimeUpgrade;
use xcm::{
    v5::{
        Asset as XcmAsset, AssetFilter, Assets as XcmAssets, ExecuteXcm,
        Instruction::{ClaimAsset, DepositAsset, WithdrawAsset},
        Junction, Location, Outcome, Weight, WildAsset, Xcm,
    },
    VersionedLocation,
};
use xcm_executor::XcmExecutor;

const FOREIGN_ASSET_ID: u128 = 1001;
const UNHELD_ASSET_ID: u128 = 1002;
const AMOUNT: u128 = 1_000_000;
const WEIGHT_LIMIT: Weight = Weight::from_parts(100_000_000_000, 1024 * 1024);

fn account_location(account: AccountId32) -> Location {
    Junction::AccountId32 {
        network: None,
        id: account.into(),
    }
    .into()
}

fn register_foreign_asset(asset_id: u128, location: Location) {
    assert_ok!(Assets::force_create(
        RuntimeOrigin::root(),
        asset_id.into(),
        MultiAddress::Id(ALICE),
        true,
        1
    ));
    assert_ok!(XcAssetConfig::register_asset_location(
        RuntimeOrigin::root(),
        Box::new(VersionedLocation::V5(location)),
        asset_id.into(),
    ));
}

// Calls the executor directly, since an extrinsic's own rollback would hide a leak.
fn execute_as_alice(instructions: Vec<xcm::v5::Instruction<RuntimeCall>>) -> Outcome {
    let mut hash = [0u8; 32];
    XcmExecutor::<xcm_config::XcmConfig>::prepare_and_execute(
        account_location(ALICE),
        Xcm(instructions),
        &mut hash,
        WEIGHT_LIMIT,
        WEIGHT_LIMIT,
    )
}

// Sorted so `held` is withdrawn before `unheld` fails, otherwise nothing would need rolling back.
fn withdraw_held_then_unheld(held: Location, unheld: Location) -> Outcome {
    let assets: XcmAssets = vec![
        XcmAsset::from((held.clone(), AMOUNT)),
        XcmAsset::from((unheld, AMOUNT)),
    ]
    .into();
    assert_eq!(assets.inner()[0].id.0, held);

    execute_as_alice(vec![WithdrawAsset(assets)])
}

// Proves `held` is withdrawable, so the failed withdraw really did reach the second asset.
fn withdraw_and_deposit_to_bob(held: Location) -> Outcome {
    execute_as_alice(vec![
        WithdrawAsset(XcmAsset::from((held, AMOUNT)).into()),
        DepositAsset {
            assets: AssetFilter::Wild(WildAsset::AllCounted(1)),
            beneficiary: account_location(BOB),
        },
    ])
}

// Nothing deposits the withdrawn asset, so it ends up trapped.
fn trap_native(amount: u128) -> Outcome {
    execute_as_alice(vec![WithdrawAsset(
        XcmAsset::from((Location::here(), amount)).into(),
    )])
}

fn claim_native_to_bob(amount: u128) -> Outcome {
    execute_as_alice(vec![
        ClaimAsset {
            assets: XcmAsset::from((Location::here(), amount)).into(),
            ticket: Location::here(),
        },
        DepositAsset {
            assets: AssetFilter::Wild(WildAsset::AllCounted(1)),
            beneficiary: account_location(BOB),
        },
    ])
}

// Remove together with the migration in `Unreleased`.
#[test]
fn unreleased_migration_resets_asset_traps() {
    let mut ext = new_test_ext();
    let bob_balance = ext.execute_with(|| {
        let bob_balance = Balances::free_balance(&BOB);

        // Trapped twice, so one claim shows a trap is claimable before the migration.
        assert!(matches!(trap_native(AMOUNT), Outcome::Complete { .. }));
        assert!(matches!(trap_native(AMOUNT), Outcome::Complete { .. }));
        assert!(matches!(
            claim_native_to_bob(AMOUNT),
            Outcome::Complete { .. }
        ));
        assert_eq!(Balances::free_balance(&BOB), bob_balance + AMOUNT);

        bob_balance
    });

    // An upgrade finds traps in committed storage, not in the overlay.
    ext.commit_all().unwrap();

    ext.execute_with(|| {
        <Unreleased as OnRuntimeUpgrade>::on_runtime_upgrade();

        assert!(matches!(
            claim_native_to_bob(AMOUNT),
            Outcome::Incomplete { .. }
        ));
        assert_eq!(Balances::free_balance(&BOB), bob_balance + AMOUNT);
    });
}

#[test]
fn failed_multi_asset_withdraw_keeps_native_issuance() {
    new_test_ext().execute_with(|| {
        let native = Location::here();
        let unheld = Location::parent();
        register_foreign_asset(UNHELD_ASSET_ID, unheld.clone());

        let issuance = Balances::total_issuance();
        let alice_balance = Balances::free_balance(&ALICE);
        let bob_balance = Balances::free_balance(&BOB);

        let outcome = withdraw_held_then_unheld(native.clone(), unheld);
        assert!(matches!(outcome, Outcome::Incomplete { .. }), "{outcome:?}");
        assert_eq!(Balances::free_balance(&ALICE), alice_balance);
        assert_eq!(Balances::total_issuance(), issuance);

        let outcome = withdraw_and_deposit_to_bob(native);
        assert!(matches!(outcome, Outcome::Complete { .. }), "{outcome:?}");
        assert_eq!(Balances::free_balance(&ALICE), alice_balance - AMOUNT);
        assert_eq!(Balances::free_balance(&BOB), bob_balance + AMOUNT);
        assert_eq!(Balances::total_issuance(), issuance);
    });
}

#[test]
fn failed_multi_asset_withdraw_keeps_foreign_supply() {
    new_test_ext().execute_with(|| {
        let foreign = Location::parent();
        let unheld = Location::new(1, [Junction::Parachain(2000)]);
        register_foreign_asset(FOREIGN_ASSET_ID, foreign.clone());
        register_foreign_asset(UNHELD_ASSET_ID, unheld.clone());
        assert_ok!(Assets::mint(
            RuntimeOrigin::signed(ALICE),
            FOREIGN_ASSET_ID.into(),
            MultiAddress::Id(ALICE),
            AMOUNT * 2
        ));

        let outcome = withdraw_held_then_unheld(foreign.clone(), unheld);
        assert!(matches!(outcome, Outcome::Incomplete { .. }), "{outcome:?}");
        assert_eq!(Assets::balance(FOREIGN_ASSET_ID, ALICE), AMOUNT * 2);
        assert_eq!(Assets::total_supply(FOREIGN_ASSET_ID), AMOUNT * 2);

        let outcome = withdraw_and_deposit_to_bob(foreign);
        assert!(matches!(outcome, Outcome::Complete { .. }), "{outcome:?}");
        assert_eq!(Assets::balance(FOREIGN_ASSET_ID, ALICE), AMOUNT);
        assert_eq!(Assets::balance(FOREIGN_ASSET_ID, BOB), AMOUNT);
        assert_eq!(Assets::total_supply(FOREIGN_ASSET_ID), AMOUNT * 2);
    });
}
