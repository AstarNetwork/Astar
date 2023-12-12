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

// Copyright 2019-2022 PureStake Inc.
// Copyright 2022      Stake Technologies
// This file is part of AssetsERC20 package, originally developed by Purestake Inc.
// AssetsERC20 package used in Astar Network in terms of GPLv3.
//
// AssetsERC20 is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// AssetsERC20 is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with AssetsERC20.  If not, see <http://www.gnu.org/licenses/>.
use frame_support::assert_ok;
use sp_runtime::traits::Zero;
use std::str::from_utf8;

use crate::mock::*;
use crate::*;

use precompile_utils::testing::*;
use sha3::{Digest, Keccak256};

fn precompiles() -> Erc20AssetsPrecompileSet<Runtime> {
    PrecompilesValue::get()
}

#[test]
fn selector_less_than_four_bytes() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Assets::force_create(
            RuntimeOrigin::root(),
            0u128,
            CryptoAlith.into(),
            true,
            1
        ));
        // This selector is only three bytes long when four are required.
        precompiles()
            .prepare_test(CryptoAlith, LocalAssetId(0u128), vec![1u8, 2u8, 3u8])
            .execute_reverts(|output| output == b"Tried to read selector out of bounds");
    });
}

#[test]
fn no_selector_exists_but_length_is_right() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Assets::force_create(
            RuntimeOrigin::root(),
            0u128,
            CryptoAlith.into(),
            true,
            1
        ));

        precompiles()
            .prepare_test(CryptoAlith, LocalAssetId(0u128), vec![1u8, 2u8, 3u8, 4u8])
            .execute_reverts(|output| output == b"Unknown selector");
    });
}

#[test]
fn selectors() {
    assert!(PrecompileCall::balance_of_selectors().contains(&0x70a08231));
    assert!(PrecompileCall::total_supply_selectors().contains(&0x18160ddd));
    assert!(PrecompileCall::approve_selectors().contains(&0x095ea7b3));
    assert!(PrecompileCall::allowance_selectors().contains(&0xdd62ed3e));
    assert!(PrecompileCall::transfer_selectors().contains(&0xa9059cbb));
    assert!(PrecompileCall::transfer_from_selectors().contains(&0x23b872dd));
    assert!(PrecompileCall::name_selectors().contains(&0x06fdde03));
    assert!(PrecompileCall::symbol_selectors().contains(&0x95d89b41));
    assert!(PrecompileCall::decimals_selectors().contains(&0x313ce567));

    assert!(PrecompileCall::mint_selectors().contains(&0x40c10f19));
    assert!(PrecompileCall::burn_selectors().contains(&0x9dc29fac));

    assert_eq!(
        crate::SELECTOR_LOG_TRANSFER,
        &Keccak256::digest(b"Transfer(address,address,uint256)")[..]
    );

    assert_eq!(
        crate::SELECTOR_LOG_APPROVAL,
        &Keccak256::digest(b"Approval(address,address,uint256)")[..]
    );
}

#[test]
fn modifiers() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));
            let mut tester =
                PrecompilesModifierTester::new(precompiles(), CryptoAlith, LocalAssetId(0u128));

            tester.test_view_modifier(PrecompileCall::balance_of_selectors());
            tester.test_view_modifier(PrecompileCall::total_supply_selectors());
            tester.test_default_modifier(PrecompileCall::approve_selectors());
            tester.test_view_modifier(PrecompileCall::allowance_selectors());
            tester.test_default_modifier(PrecompileCall::transfer_selectors());
            tester.test_default_modifier(PrecompileCall::transfer_from_selectors());
            tester.test_view_modifier(PrecompileCall::name_selectors());
            tester.test_view_modifier(PrecompileCall::symbol_selectors());
            tester.test_view_modifier(PrecompileCall::decimals_selectors());

            tester.test_default_modifier(PrecompileCall::mint_selectors());
            tester.test_default_modifier(PrecompileCall::burn_selectors());
        });
}

#[test]
fn get_total_supply() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000), (Bob.into(), 2500)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));
            assert_ok!(Assets::mint(
                RuntimeOrigin::signed(CryptoAlith.into()),
                0u128,
                CryptoAlith.into(),
                1000
            ));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::total_supply {},
                )
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(U256::from(1000u64));
        });
}

#[test]
fn get_balances_known_user() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));
            assert_ok!(Assets::mint(
                RuntimeOrigin::signed(CryptoAlith.into()),
                0u128,
                CryptoAlith.into(),
                1000
            ));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::balance_of {
                        who: Address(CryptoAlith.into()),
                    },
                )
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(U256::from(1000u64));
        });
}

#[test]
fn get_balances_unknown_user() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::balance_of {
                        who: Address(Bob.into()),
                    },
                )
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(U256::from(0u64));
        });
}

#[test]
fn mint_is_ok() {
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = 0;
        assert_ok!(Assets::force_create(
            RuntimeOrigin::root(),
            asset_id,
            CryptoAlith.into(),
            true,
            1,
        ));

        // Sanity check, Bob should be without assets
        assert!(Assets::balance(asset_id, &Bob.into()).is_zero());

        // Mint some assets for Bob
        let mint_amount = 7 * 11 * 19;
        precompiles()
            .prepare_test(
                CryptoAlith,
                LocalAssetId(asset_id),
                PrecompileCall::mint {
                    to: Address(Bob.into()),
                    value: mint_amount.into(),
                },
            )
            .expect_cost(28770756) // 1 weight => 1 gas in mock
            .expect_log(log3(
                LocalAssetId(0u128),
                SELECTOR_LOG_TRANSFER,
                Zero,
                Bob,
                solidity::encode_event_data(U256::from(mint_amount)),
            ))
            .execute_returns(true);

        // Ensure Bob's asset balance was increased
        assert_eq!(Assets::balance(asset_id, &Bob.into()), mint_amount);
    });
}

#[test]
fn mint_non_admin_is_not_ok() {
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = 0;
        assert_ok!(Assets::force_create(
            RuntimeOrigin::root(),
            asset_id,
            CryptoAlith.into(),
            true,
            1,
        ));

        precompiles()
            .prepare_test(
                Bob,
                LocalAssetId(asset_id),
                PrecompileCall::mint {
                    to: Address(Bob.into()),
                    value: 42.into(),
                },
            )
            .expect_no_logs()
            .execute_reverts(|output| from_utf8(&output).unwrap().contains("NoPermission"));

        precompiles()
            .prepare_test(
                CryptoAlith,
                LocalAssetId(0u128),
                PrecompileCall::mint {
                    to: Address(CryptoAlith.into()),
                    value: U256::from(1) << 128,
                },
            )
            .execute_reverts(|output| {
                from_utf8(&output)
                    .unwrap()
                    .contains("value: Value is too large for balance type")
            });
    });
}

#[test]
fn burn_is_ok() {
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = 0;
        assert_ok!(Assets::force_create(
            RuntimeOrigin::root(),
            asset_id,
            CryptoAlith.into(),
            true,
            1,
        ));

        // Issue some initial assets for Bob
        let init_amount = 123;
        assert_ok!(Assets::mint(
            RuntimeOrigin::signed(CryptoAlith.into()),
            asset_id,
            Bob.into(),
            init_amount,
        ));
        assert_eq!(Assets::balance(asset_id, &Bob.into()), init_amount);

        // Burn some assets from Bob
        let burn_amount = 19;
        precompiles()
            .prepare_test(
                CryptoAlith,
                LocalAssetId(asset_id),
                PrecompileCall::burn {
                    from: Address(Bob.into()),
                    value: burn_amount.into(),
                },
            )
            .expect_cost(34903756) // 1 weight => 1 gas in mock
            .expect_log(log3(
                LocalAssetId(0u128),
                SELECTOR_LOG_TRANSFER,
                Bob,
                Zero,
                solidity::encode_event_data(U256::from(burn_amount)),
            ))
            .execute_returns(true);

        // Ensure Bob's asset balance was decreased
        assert_eq!(
            Assets::balance(asset_id, &Bob.into()),
            init_amount - burn_amount
        );
    });
}

#[test]
fn burn_non_admin_is_not_ok() {
    ExtBuilder::default().build().execute_with(|| {
        let asset_id = 0;
        assert_ok!(Assets::force_create(
            RuntimeOrigin::root(),
            asset_id,
            CryptoAlith.into(),
            true,
            1,
        ));
        assert_ok!(Assets::mint(
            RuntimeOrigin::signed(CryptoAlith.into()),
            asset_id,
            Bob.into(),
            1000000,
        ));

        precompiles()
            .prepare_test(
                Bob,
                LocalAssetId(asset_id),
                PrecompileCall::burn {
                    from: Address(Bob.into()),
                    value: 42.into(),
                },
            )
            .expect_no_logs()
            .execute_reverts(|output| from_utf8(&output).unwrap().contains("NoPermission"));

        precompiles()
            .prepare_test(
                CryptoAlith,
                LocalAssetId(0u128),
                PrecompileCall::burn {
                    from: Address(CryptoAlith.into()),
                    value: U256::from(1) << 128,
                },
            )
            .execute_reverts(|output| {
                from_utf8(&output)
                    .unwrap()
                    .contains("Value is too large for balance type")
            });
    });
}

#[test]
fn approve() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));
            assert_ok!(Assets::mint(
                RuntimeOrigin::signed(CryptoAlith.into()),
                0u128,
                CryptoAlith.into(),
                1000
            ));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::approve {
                        spender: Address(Bob.into()),
                        value: 500.into(),
                    },
                )
                .expect_log(log3(
                    LocalAssetId(0u128),
                    SELECTOR_LOG_APPROVAL,
                    CryptoAlith,
                    Bob,
                    solidity::encode_event_data(U256::from(500)),
                ))
                .execute_returns(true);
        });
}

#[test]
fn approve_saturating() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));
            assert_ok!(Assets::mint(
                RuntimeOrigin::signed(CryptoAlith.into()),
                0u128,
                CryptoAlith.into(),
                1000
            ));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::approve {
                        spender: Address(Bob.into()),
                        value: U256::MAX,
                    },
                )
                .expect_log(log3(
                    LocalAssetId(0u128),
                    SELECTOR_LOG_APPROVAL,
                    CryptoAlith,
                    Bob,
                    solidity::encode_event_data(U256::MAX),
                ))
                .execute_returns(true);

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::allowance {
                        owner: Address(CryptoAlith.into()),
                        spender: Address(Bob.into()),
                    },
                )
                .expect_cost(0u64)
                .expect_no_logs()
                .execute_returns(U256::from(u128::MAX));
        });
}

#[test]
fn check_allowance_existing() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));
            assert_ok!(Assets::mint(
                RuntimeOrigin::signed(CryptoAlith.into()),
                0u128,
                CryptoAlith.into(),
                1000
            ));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::approve {
                        spender: Address(Bob.into()),
                        value: 500.into(),
                    },
                )
                .execute_some();

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::allowance {
                        owner: Address(CryptoAlith.into()),
                        spender: Address(Bob.into()),
                    },
                )
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(U256::from(500u64));
        });
}

#[test]
fn check_allowance_not_existing() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::allowance {
                        owner: Address(CryptoAlith.into()),
                        spender: Address(Bob.into()),
                    },
                )
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(U256::from(0u64));
        });
}

#[test]
fn transfer() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));
            assert_ok!(Assets::mint(
                RuntimeOrigin::signed(CryptoAlith.into()),
                0u128,
                CryptoAlith.into(),
                1000
            ));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::transfer {
                        to: Address(Bob.into()),
                        value: 400.into(),
                    },
                )
                .expect_log(log3(
                    LocalAssetId(0u128),
                    SELECTOR_LOG_TRANSFER,
                    CryptoAlith,
                    Bob,
                    solidity::encode_event_data(U256::from(400)),
                ))
                .execute_returns(true);

            precompiles()
                .prepare_test(
                    Bob,
                    LocalAssetId(0u128),
                    PrecompileCall::balance_of {
                        who: Address(Bob.into()),
                    },
                )
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(U256::from(400));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::balance_of {
                        who: Address(CryptoAlith.into()),
                    },
                )
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(U256::from(600));
        });
}

#[test]
fn transfer_not_enough_founds() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));
            assert_ok!(Assets::mint(
                RuntimeOrigin::signed(CryptoAlith.into()),
                0u128,
                CryptoAlith.into(),
                1
            ));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::transfer {
                        to: Address(Charlie.into()),
                        value: 50.into(),
                    },
                )
                .execute_reverts(|output| {
                    from_utf8(&output)
                        .unwrap()
                        .contains("Dispatched call failed with error: Module(ModuleError")
                        && from_utf8(&output).unwrap().contains("BalanceLow")
                });

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::transfer {
                        to: Address(Charlie.into()),
                        value: U256::from(1) << 128,
                    },
                )
                .execute_reverts(|output| {
                    from_utf8(&output)
                        .unwrap()
                        .contains("Value is too large for balance type")
                });
        });
}

#[test]
fn transfer_from() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));
            assert_ok!(Assets::mint(
                RuntimeOrigin::signed(CryptoAlith.into()),
                0u128,
                CryptoAlith.into(),
                1000
            ));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::approve {
                        spender: Address(Bob.into()),
                        value: 500.into(),
                    },
                )
                .execute_some();

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::approve {
                        spender: Address(Bob.into()),
                        value: 500.into(),
                    },
                )
                .execute_some();

            precompiles()
                .prepare_test(
                    Bob, // Bob is the one sending transferFrom!
                    LocalAssetId(0u128),
                    PrecompileCall::transfer_from {
                        from: Address(CryptoAlith.into()),
                        to: Address(Charlie.into()),
                        value: 400.into(),
                    },
                )
                .expect_log(log3(
                    LocalAssetId(0u128),
                    SELECTOR_LOG_TRANSFER,
                    CryptoAlith,
                    Charlie,
                    solidity::encode_event_data(U256::from(400)),
                ))
                .execute_returns(true);

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::balance_of {
                        who: Address(CryptoAlith.into()),
                    },
                )
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(U256::from(600));

            precompiles()
                .prepare_test(
                    Bob,
                    LocalAssetId(0u128),
                    PrecompileCall::balance_of {
                        who: Address(Bob.into()),
                    },
                )
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(U256::from(0));

            precompiles()
                .prepare_test(
                    Charlie,
                    LocalAssetId(0u128),
                    PrecompileCall::balance_of {
                        who: Address(Charlie.into()),
                    },
                )
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(U256::from(400));
        });
}

#[test]
fn transfer_from_non_incremental_approval() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));
            assert_ok!(Assets::mint(
                RuntimeOrigin::signed(CryptoAlith.into()),
                0u128,
                CryptoAlith.into(),
                1000
            ));

            // We first approve 500
            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::approve {
                        spender: Address(Bob.into()),
                        value: 500.into(),
                    },
                )
                .expect_log(log3(
                    LocalAssetId(0u128),
                    SELECTOR_LOG_APPROVAL,
                    CryptoAlith,
                    Bob,
                    solidity::encode_event_data(U256::from(500)),
                ))
                .execute_returns(true);

            // We then approve 300. Non-incremental, so this is
            // the approved new value
            // Additionally, the gas used in this approval is higher because we
            // need to clear the previous one
            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::approve {
                        spender: Address(Bob.into()),
                        value: 300.into(),
                    },
                )
                .expect_log(log3(
                    LocalAssetId(0u128),
                    SELECTOR_LOG_APPROVAL,
                    CryptoAlith,
                    Bob,
                    solidity::encode_event_data(U256::from(300)),
                ))
                .execute_returns(true);

            // This should fail, as now the new approved quantity is 300
            precompiles()
                .prepare_test(
                    Bob, // Bob is the one sending transferFrom!
                    LocalAssetId(0u128),
                    PrecompileCall::transfer_from {
                        from: Address(CryptoAlith.into()),
                        to: Address(Bob.into()),
                        value: 500.into(),
                    },
                )
                .execute_reverts(|output| {
                    output
                        == b"Dispatched call failed with error: Module(ModuleError \
                    { index: 2, error: [10, 0, 0, 0], message: Some(\"Unapproved\") })"
                });
        });
}

#[test]
fn transfer_from_above_allowance() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));
            assert_ok!(Assets::mint(
                RuntimeOrigin::signed(CryptoAlith.into()),
                0u128,
                CryptoAlith.into(),
                1000
            ));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::approve {
                        spender: Address(Bob.into()),
                        value: 300.into(),
                    },
                )
                .execute_some();

            precompiles()
                .prepare_test(
                    Bob, // Bob is the one sending transferFrom!
                    LocalAssetId(0u128),
                    PrecompileCall::transfer_from {
                        from: Address(CryptoAlith.into()),
                        to: Address(Bob.into()),
                        value: 400.into(),
                    },
                )
                .execute_reverts(|output| {
                    output
                        == b"Dispatched call failed with error: Module(ModuleError \
                    { index: 2, error: [10, 0, 0, 0], message: Some(\"Unapproved\") })"
                });

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::transfer_from {
                        from: Address(CryptoAlith.into()),
                        to: Address(Bob.into()),
                        value: U256::from(1) << 128,
                    },
                )
                .execute_reverts(|output| {
                    from_utf8(&output)
                        .unwrap()
                        .contains("Value is too large for balance type")
                });
        });
}

#[test]
fn transfer_from_self() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));
            assert_ok!(Assets::mint(
                RuntimeOrigin::signed(CryptoAlith.into()),
                0u128,
                CryptoAlith.into(),
                1000
            ));

            precompiles()
                .prepare_test(
                    CryptoAlith, // Alice sending transferFrom herself, no need for allowance.
                    LocalAssetId(0u128),
                    PrecompileCall::transfer_from {
                        from: Address(CryptoAlith.into()),
                        to: Address(Bob.into()),
                        value: 400.into(),
                    },
                )
                .expect_log(log3(
                    LocalAssetId(0u128),
                    SELECTOR_LOG_TRANSFER,
                    CryptoAlith,
                    Bob,
                    solidity::encode_event_data(U256::from(400)),
                ))
                .execute_returns(true);

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::balance_of {
                        who: Address(CryptoAlith.into()),
                    },
                )
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(U256::from(600));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::balance_of {
                        who: Address(Bob.into()),
                    },
                )
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(U256::from(400));
        });
}

#[test]
fn get_metadata() {
    ExtBuilder::default()
        .with_balances(vec![(CryptoAlith.into(), 1000), (Bob.into(), 2500)])
        .build()
        .execute_with(|| {
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                0u128,
                CryptoAlith.into(),
                true,
                1
            ));
            assert_ok!(Assets::force_set_metadata(
                RuntimeOrigin::root(),
                0u128,
                b"TestToken".to_vec(),
                b"Test".to_vec(),
                12,
                false
            ));

            precompiles()
                .prepare_test(CryptoAlith, LocalAssetId(0u128), PrecompileCall::name {})
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(UnboundedBytes::from("TestToken"));

            precompiles()
                .prepare_test(CryptoAlith, LocalAssetId(0u128), PrecompileCall::symbol {})
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(UnboundedBytes::from("Test"));

            precompiles()
                .prepare_test(
                    CryptoAlith,
                    LocalAssetId(0u128),
                    PrecompileCall::decimals {},
                )
                .expect_cost(0) // TODO: Test db read/write costs
                .expect_no_logs()
                .execute_returns(12u8);
        });
}

#[test]
fn minimum_balance_is_right() {
    ExtBuilder::default().build().execute_with(|| {
        let expected_min_balance = 19;
        assert_ok!(Assets::force_create(
            RuntimeOrigin::root(),
            0u128,
            CryptoAlith.into(),
            true,
            expected_min_balance,
        ));

        precompiles()
            .prepare_test(
                CryptoAlith,
                LocalAssetId(0u128),
                PrecompileCall::minimum_balance {},
            )
            .expect_cost(0) // TODO: Test db read/write costs
            .expect_no_logs()
            .execute_returns(U256::from(expected_min_balance));
    });
}
