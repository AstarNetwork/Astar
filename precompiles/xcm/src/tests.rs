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

use assert_matches::assert_matches;

use crate::mock::*;
use crate::*;

use precompile_utils::testing::*;
use precompile_utils::EvmDataWriter;
use sp_core::H160;

fn precompiles() -> TestPrecompileSet<Runtime> {
    PrecompilesValue::get()
}

#[test]
fn wrong_assets_len_or_fee_index_reverts() {
    ExtBuilder::default().build().execute_with(|| {
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsWithdrawNative)
                    .write(vec![Address::from(H160::repeat_byte(0xF1))])
                    .write(Vec::<U256>::new())
                    .write(H256::repeat_byte(0xF1))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(0_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_reverts(|output| output == b"Assets resolution failure.");

        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsWithdrawNative)
                    .write(vec![Address::from(Runtime::asset_id_to_address(1u128))])
                    .write(vec![U256::from(42000u64)])
                    .write(H256::repeat_byte(0xF1))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(2_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_reverts(|output| output == b"Bad fee index.");
    });
}

#[test]
fn assets_withdraw_works() {
    ExtBuilder::default().build().execute_with(|| {
        // SS58
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsWithdrawNative)
                    .write(vec![Address::from(Runtime::asset_id_to_address(1u128))])
                    .write(vec![U256::from(42000u64)])
                    .write(H256::repeat_byte(0xF1))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(0_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(true).build());

        // H160
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsWithdrawEvm)
                    .write(vec![Address::from(Runtime::asset_id_to_address(1u128))])
                    .write(vec![U256::from(42000u64)])
                    .write(Address::from(H160::repeat_byte(0xDE)))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(0_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(true).build());
    });
}

#[test]
fn remote_transact_works() {
    ExtBuilder::default().build().execute_with(|| {
        // SS58
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::RemoteTransact)
                    .write(U256::from(0_u64))
                    .write(true)
                    .write(Address::from(Runtime::asset_id_to_address(1_u128)))
                    .write(U256::from(367))
                    .write(vec![0xff_u8, 0xaa, 0x77, 0x00])
                    .write(U256::from(3_000_000_000u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(true).build());
    });
}

#[test]
fn reserve_transfer_assets_works() {
    ExtBuilder::default().build().execute_with(|| {
        // SS58
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsReserveTransferNative)
                    .write(vec![Address::from(Runtime::asset_id_to_address(1u128))])
                    .write(vec![U256::from(42000u64)])
                    .write(H256::repeat_byte(0xF1))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(0_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(true).build());

        // H160
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsReserveTransferEvm)
                    .write(vec![Address::from(Runtime::asset_id_to_address(1u128))])
                    .write(vec![U256::from(42000u64)])
                    .write(Address::from(H160::repeat_byte(0xDE)))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(0_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(true).build());
    });

    for (location, Xcm(instructions)) in take_sent_xcm() {
        assert_eq!(
            location,
            MultiLocation {
                parents: 1,
                interior: Here
            }
        );

        let non_native_asset = MultiAsset {
            fun: Fungible(42000),
            id: xcm::v3::AssetId::from(MultiLocation {
                parents: 0,
                interior: Here,
            }),
        };

        assert_matches!(
            instructions.as_slice(),
            [
                ReserveAssetDeposited(assets),
                ClearOrigin,
                BuyExecution {
                    fees,
                    ..
                },
                DepositAsset {
                    beneficiary: MultiLocation {
                        parents: 0,
                        interior: X1(_),
                    },
                    ..
                }
            ]

            if fees.contains(&non_native_asset) && assets.contains(&non_native_asset)
        );
    }
}

#[test]
fn reserve_transfer_currency_works() {
    ExtBuilder::default().build().execute_with(|| {
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsReserveTransferNative)
                    .write(vec![Address::from(H160::zero())]) // zero address by convention
                    .write(vec![U256::from(42000u64)])
                    .write(H256::repeat_byte(0xF1))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(0_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(true).build());

        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsReserveTransferEvm)
                    .write(vec![Address::from(H160::zero())]) // zero address by convention
                    .write(vec![U256::from(42000u64)])
                    .write(Address::from(H160::repeat_byte(0xDE)))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(0_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(true).build());
    });

    for (location, Xcm(instructions)) in take_sent_xcm() {
        assert_eq!(
            location,
            MultiLocation {
                parents: 1,
                interior: Here
            }
        );

        let native_asset = MultiAsset {
            fun: Fungible(42000),
            id: xcm::v3::AssetId::from(MultiLocation {
                parents: 0,
                interior: X1(OnlyChild),
            }),
        };

        assert_matches!(
            instructions.as_slice(),
            [
                ReserveAssetDeposited(assets),
                ClearOrigin,
                BuyExecution {
                    fees,
                    ..
                },
                DepositAsset {
                    beneficiary: MultiLocation {
                        parents: 0,
                        interior: X1(_),
                    },
                    ..
                }
            ]

            if fees.contains(&native_asset) && assets.contains(&native_asset)
        );
    }
}
