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

use crate::mock::*;
use crate::*;
use xcm::latest::{
    AssetId, Fungibility, Junction, Junctions, MultiAsset, MultiAssets, MultiLocation,
};

use orml_xtokens::Event as XtokensEvent;
use parity_scale_codec::Encode;
use precompile_utils::testing::*;
use sp_core::{H160, H256};
use sp_runtime::traits::Convert;
use xcm::VersionedXcm;

fn precompiles() -> TestPrecompileSet<Runtime> {
    PrecompilesValue::get()
}

mod xcm_old_interface_test {
    use super::*;
    #[test]
    fn wrong_assets_len_or_fee_index_reverts() {
        ExtBuilder::default().build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_native_v1 {
                        assets: vec![Address::from(H160::repeat_byte(0xF1))].into(),
                        amounts: vec![].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: true,
                        parachain_id: 0.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_reverts(|output| output == b"Assets resolution failure.");

            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_native_v1 {
                        assets: vec![Address::from(Runtime::asset_id_to_address(1u128))].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: true,
                        parachain_id: 0.into(),
                        fee_index: 2.into(),
                    },
                )
                .expect_no_logs()
                .execute_reverts(|output| {
                    let error_string = String::from_utf8_lossy(output);
                    error_string.contains("AssetIndexNonExistent")
                });
        });
    }

    #[test]
    fn sanity_checks_for_parameters() {
        ExtBuilder::default().build().execute_with(|| {
            // parachain id resolution failure
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_native_v1 {
                        assets: vec![Address::from(Runtime::asset_id_to_address(1u128))].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: false,
                        parachain_id: u64::MAX.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_reverts(|output| {
                    output == b"error converting parachain_id, maybe value too large"
                });

            // more than 2 assets can not be sent
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_native_v1 {
                        assets: vec![
                            Address::from(H160::repeat_byte(0xF1)),
                            Address::from(H160::repeat_byte(0xF2)),
                            Address::from(H160::repeat_byte(0xF3)),
                        ]
                        .into(),
                        amounts: vec![
                            U256::from(42000u64),
                            U256::from(42000u64),
                            U256::from(42000u64),
                        ]
                        .into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: false,
                        parachain_id: 1.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_reverts(|output| {
                    let error_string = String::from_utf8_lossy(output);
                    error_string.contains("assets: Value is too large for length")
                });
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
                    PrecompileCall::assets_withdraw_native_v1 {
                        assets: vec![Address::from(Runtime::asset_id_to_address(1u128))].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: true,
                        parachain_id: 0.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            // H160
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_evm_v1 {
                        assets: vec![Address::from(Runtime::asset_id_to_address(1u128))].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: Address(H160::repeat_byte(0xDE)),
                        is_relay: false,
                        parachain_id: 0.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            // Checking for non-relay destination case
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_evm_v1 {
                        assets: vec![Address::from(Runtime::asset_id_to_address(1u128))].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: Address(H160::repeat_byte(0xDE)),
                        is_relay: false,
                        parachain_id: 123.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);
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
                    PrecompileCall::remote_transact_v1 {
                        para_id: 0.into(),
                        is_relay: true,
                        fee_asset_addr: Address::from(Runtime::asset_id_to_address(1_u128)),
                        fee_amount: 367.into(),
                        remote_call: vec![0xff_u8, 0xaa, 0x77, 0x00].into(),
                        transact_weight: 3_000_000_000u64.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);
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
                    PrecompileCall::assets_reserve_transfer_native_v1 {
                        assets: vec![Address::from(Runtime::asset_id_to_address(1u128))].into(),
                        amounts: vec![U256::from(42000u64)].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: true,
                        parachain_id: 0.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            // H160
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_reserve_transfer_evm_v1 {
                        assets: vec![Address::from(Runtime::asset_id_to_address(1u128))].into(),
                        amounts: vec![U256::from(42000u64)].into(),
                        recipient_account_id: Address::from(H160::repeat_byte(0xDE)),
                        is_relay: true,
                        parachain_id: 0.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

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
                assert!(matches!(
                    instructions.as_slice(),
                    [
                        WithdrawAsset(assets),
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
                ));
            }
        });
    }

    #[test]
    fn reserve_transfer_currency_works() {
        ExtBuilder::default().build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_reserve_transfer_native_v1 {
                        assets: vec![Address::from(H160::zero())].into(),
                        amounts: vec![U256::from(42000u64)].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: true,
                        parachain_id: 0.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_reserve_transfer_evm_v1 {
                        assets: vec![Address::from(H160::zero())].into(),
                        amounts: vec![U256::from(42000u64)].into(),
                        recipient_account_id: Address::from(H160::repeat_byte(0xDE)),
                        is_relay: true,
                        parachain_id: 0.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

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
                        interior: X1(Parachain(123)),
                    }),
                };

                assert!(matches!(
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
                ));
            }
        });
    }

    #[test]
    fn test_send_clear_origin() {
        ExtBuilder::default().build().execute_with(|| {
            let dest: MultiLocation = MultiLocation {
                parents: 1,
                interior: Junctions::X1(Junction::AccountId32 {
                    network: None,
                    id: H256::repeat_byte(0xF1).into(),
                }),
            };
            let xcm_to_send = VersionedXcm::<()>::V3(Xcm(vec![ClearOrigin])).encode();
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::send_xcm {
                        dest,
                        xcm_call: xcm_to_send.as_slice().into(),
                    },
                )
                // Fixed: TestWeightInfo
                .expect_cost(100000000)
                .expect_no_logs()
                .execute_returns(true);

            let sent_messages = take_sent_xcm();
            let (_, sent_message) = sent_messages.first().unwrap();
            // Lets make sure the message is as expected
            assert!(sent_message.0.contains(&ClearOrigin));
        })
    }
}

mod xcm_new_interface_test {
    use super::*;
    #[test]
    fn xtokens_transfer_works() {
        let weight = WeightV2::from(3_000_000_000u64, 1024);

        ExtBuilder::default().build().execute_with(|| {
            let parent_destination = MultiLocation {
                parents: 1,
                interior: Junctions::X1(Junction::AccountId32 {
                    network: None,
                    id: [1u8; 32],
                }),
            };

            let sibling_parachain_location = MultiLocation {
                parents: 1,
                interior: Junctions::X2(
                    Junction::Parachain(10),
                    Junction::AccountId32 {
                        network: None,
                        id: [1u8; 32],
                    },
                ),
            };

            // sending relay token back to relay chain
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer {
                        currency_address: Address::from(Runtime::asset_id_to_address(1u128)),
                        amount_of_tokens: 42000u64.into(),
                        destination: parent_destination,
                        weight: weight.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let expected_asset: MultiAsset = MultiAsset {
                id: AssetId::Concrete(CurrencyIdToMultiLocation::convert(1).unwrap()),
                fun: Fungibility::Fungible(42000),
            };

            let expected: crate::mock::RuntimeEvent =
                mock::RuntimeEvent::Xtokens(XtokensEvent::TransferredMultiAssets {
                    sender: TestAccount::Alice.into(),
                    assets: vec![expected_asset.clone()].into(),
                    fee: expected_asset,
                    dest: parent_destination,
                })
                .into();
            assert!(events().contains(&expected));

            // sending parachain token back to parachain
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer {
                        currency_address: Address::from(Runtime::asset_id_to_address(2u128)),
                        amount_of_tokens: 42000u64.into(),
                        destination: sibling_parachain_location,
                        weight,
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let expected_asset: MultiAsset = MultiAsset {
                id: AssetId::Concrete(CurrencyIdToMultiLocation::convert(2).unwrap()),
                fun: Fungibility::Fungible(42000),
            };

            let expected: crate::mock::RuntimeEvent =
                mock::RuntimeEvent::Xtokens(XtokensEvent::TransferredMultiAssets {
                    sender: TestAccount::Alice.into(),
                    assets: vec![expected_asset.clone()].into(),
                    fee: expected_asset,
                    dest: sibling_parachain_location,
                })
                .into();
            assert!(events().contains(&expected));
        });
    }

    #[test]
    fn xtokens_transfer_works_for_native_asset() {
        let weight = WeightV2::from(3_000_000_000u64, 1024);
        let parent_destination = MultiLocation {
            parents: 1,
            interior: Junctions::X1(Junction::AccountId32 {
                network: None,
                id: [1u8; 32],
            }),
        };

        ExtBuilder::default().build().execute_with(|| {
            // sending native token to relay
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer {
                        currency_address: Address::from(NATIVE_ADDRESS),
                        amount_of_tokens: 42000u64.into(),
                        destination: parent_destination,
                        weight: weight.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let expected_asset: MultiAsset = MultiAsset {
                id: AssetId::Concrete(Here.into()),
                fun: Fungibility::Fungible(42000),
            };

            let expected: crate::mock::RuntimeEvent =
                mock::RuntimeEvent::Xtokens(XtokensEvent::TransferredMultiAssets {
                    sender: TestAccount::Alice.into(),
                    assets: vec![expected_asset.clone()].into(),
                    fee: expected_asset,
                    dest: parent_destination,
                })
                .into();
            assert!(events().contains(&expected));
        });
    }

    #[test]
    fn xtokens_transfer_with_fee_works() {
        let weight = WeightV2::from(3_000_000_000u64, 1024);
        ExtBuilder::default().build().execute_with(|| {
            let parent_destination = MultiLocation {
                parents: 1,
                interior: Junctions::X1(Junction::AccountId32 {
                    network: None,
                    id: [1u8; 32],
                }),
            };

            // sending relay token back to relay chain
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer_with_fee {
                        currency_address: Address::from(Runtime::asset_id_to_address(1u128)),
                        amount_of_tokens: 42000u64.into(),
                        fee: 50.into(),
                        destination: parent_destination,
                        weight,
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let expected_asset: MultiAsset = MultiAsset {
                id: AssetId::Concrete(CurrencyIdToMultiLocation::convert(1).unwrap()),
                fun: Fungibility::Fungible(42000),
            };
            let expected_fee: MultiAsset = MultiAsset {
                id: AssetId::Concrete(CurrencyIdToMultiLocation::convert(1).unwrap()),
                fun: Fungibility::Fungible(50),
            };

            let expected: crate::mock::RuntimeEvent =
                mock::RuntimeEvent::Xtokens(XtokensEvent::TransferredMultiAssets {
                    sender: TestAccount::Alice.into(),
                    assets: vec![expected_asset.clone(), expected_fee.clone()].into(),
                    fee: expected_fee,
                    dest: parent_destination,
                })
                .into();
            assert!(events().contains(&expected));
        });
    }

    #[test]
    fn xtokens_transfer_with_fee_works_for_native_asset() {
        let weight = WeightV2::from(3_000_000_000u64, 1024);
        let parent_destination = MultiLocation {
            parents: 1,
            interior: Junctions::X1(Junction::AccountId32 {
                network: None,
                id: [1u8; 32],
            }),
        };

        ExtBuilder::default().build().execute_with(|| {
            // sending native token to relay
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer_with_fee {
                        currency_address: Address::from(NATIVE_ADDRESS),
                        amount_of_tokens: 42000u64.into(),
                        fee: 50.into(),
                        destination: parent_destination,
                        weight,
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let expected_asset: MultiAsset = MultiAsset {
                id: AssetId::Concrete(Here.into()),
                fun: Fungibility::Fungible(42000),
            };
            let expected_fee: MultiAsset = MultiAsset {
                id: AssetId::Concrete(Here.into()),
                fun: Fungibility::Fungible(50),
            };
            let expected: crate::mock::RuntimeEvent =
                mock::RuntimeEvent::Xtokens(XtokensEvent::TransferredMultiAssets {
                    sender: TestAccount::Alice.into(),
                    assets: vec![expected_asset.clone(), expected_fee.clone()].into(),
                    fee: expected_fee,
                    dest: parent_destination,
                })
                .into();
            assert!(events().contains(&expected));
        });
    }

    #[test]
    fn transfer_multiasset_works() {
        let weight = WeightV2::from(3_000_000_000u64, 1024);
        ExtBuilder::default().build().execute_with(|| {
            let relay_token_location = MultiLocation {
                parents: 1,
                interior: Junctions::Here,
            };
            let relay_destination = MultiLocation {
                parents: 1,
                interior: Junctions::X1(Junction::AccountId32 {
                    network: None,
                    id: [1u8; 32],
                }),
            };
            let para_destination = MultiLocation {
                parents: 1,
                interior: Junctions::X2(
                    Junction::Parachain(10),
                    Junction::AccountId32 {
                        network: None,
                        id: [1u8; 32],
                    },
                ),
            };
            let native_token_location: MultiLocation = (Here).into();

            let amount = 4200u64;
            // relay token to relay
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer_multiasset {
                        asset_location: relay_token_location,
                        amount_of_tokens: amount.into(),
                        destination: relay_destination,
                        weight: weight.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let expected_asset: MultiAsset = MultiAsset {
                id: AssetId::Concrete(relay_token_location),
                fun: Fungibility::Fungible(amount.into()),
            };
            let expected: crate::mock::RuntimeEvent =
                mock::RuntimeEvent::Xtokens(XtokensEvent::TransferredMultiAssets {
                    sender: TestAccount::Alice.into(),
                    assets: vec![expected_asset.clone()].into(),
                    fee: expected_asset,
                    dest: relay_destination,
                })
                .into();

            // Assert that the events vector contains the one expected
            assert!(events().contains(&expected));

            // relay to para
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer_multiasset {
                        asset_location: relay_token_location,
                        amount_of_tokens: amount.into(),
                        destination: para_destination,
                        weight: weight.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let expected_asset: MultiAsset = MultiAsset {
                id: AssetId::Concrete(relay_token_location),
                fun: Fungibility::Fungible(amount.into()),
            };
            let expected: crate::mock::RuntimeEvent =
                mock::RuntimeEvent::Xtokens(XtokensEvent::TransferredMultiAssets {
                    sender: TestAccount::Alice.into(),
                    assets: vec![expected_asset.clone()].into(),
                    fee: expected_asset,
                    dest: para_destination,
                })
                .into();

            // Assert that the events vector contains the one expected
            assert!(events().contains(&expected));

            // native token to para

            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer_multiasset {
                        asset_location: native_token_location, // zero address by convention
                        amount_of_tokens: amount.into(),
                        destination: para_destination,
                        weight: weight.clone(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let expected_asset: MultiAsset = MultiAsset {
                id: AssetId::Concrete(native_token_location),
                fun: Fungibility::Fungible(amount.into()),
            };
            let expected: crate::mock::RuntimeEvent =
                mock::RuntimeEvent::Xtokens(XtokensEvent::TransferredMultiAssets {
                    sender: TestAccount::Alice.into(),
                    assets: vec![expected_asset.clone()].into(),
                    fee: expected_asset,
                    dest: para_destination,
                })
                .into();

            // Assert that the events vector contains the one expected
            assert!(events().contains(&expected));
        });
    }

    #[test]
    fn transfer_multi_currencies_works() {
        let destination = MultiLocation::new(
            1,
            Junctions::X1(Junction::AccountId32 {
                network: None,
                id: [1u8; 32],
            }),
        );

        let weight = WeightV2::from(3_000_000_000u64, 1024);

        //  NOTE: Currently only support `ToReserve` with relay-chain asset as fee. other case
        // like `NonReserve` or `SelfReserve` with relay-chain fee is not support.
        let currencies = vec![
            (
                Address::from(Runtime::asset_id_to_address(2u128)),
                U256::from(500),
            )
                .into(),
            (
                Address::from(Runtime::asset_id_to_address(3u128)),
                U256::from(500),
            )
                .into(),
        ]
        .into();

        ExtBuilder::default().build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer_multi_currencies {
                        currencies,
                        fee_item: 0u32,
                        destination,
                        weight,
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let expected_asset_1: MultiAsset = MultiAsset {
                id: AssetId::Concrete(CurrencyIdToMultiLocation::convert(2u128).unwrap()),
                fun: Fungibility::Fungible(500),
            };
            let expected_asset_2: MultiAsset = MultiAsset {
                id: AssetId::Concrete(CurrencyIdToMultiLocation::convert(3u128).unwrap()),
                fun: Fungibility::Fungible(500),
            };

            let expected: crate::mock::RuntimeEvent =
                mock::RuntimeEvent::Xtokens(XtokensEvent::TransferredMultiAssets {
                    sender: TestAccount::Alice.into(),
                    assets: vec![expected_asset_1.clone(), expected_asset_2].into(),
                    fee: expected_asset_1,
                    dest: destination,
                })
                .into();
            assert!(events().contains(&expected));
        });
    }

    #[test]
    fn transfer_multi_currencies_cannot_insert_more_than_max() {
        let destination = MultiLocation::new(
            1,
            Junctions::X1(Junction::AccountId32 {
                network: None,
                id: [1u8; 32],
            }),
        );
        let weight = WeightV2::from(3_000_000_000u64, 1024);
        // we only allow upto 2 currencies to be transfered
        let currencies = vec![
            (
                Address::from(Runtime::asset_id_to_address(2u128)),
                U256::from(500),
            )
                .into(),
            (
                Address::from(Runtime::asset_id_to_address(3u128)),
                U256::from(500),
            )
                .into(),
            (
                Address::from(Runtime::asset_id_to_address(4u128)),
                U256::from(500),
            )
                .into(),
        ]
        .into();

        ExtBuilder::default().build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer_multi_currencies {
                        currencies,
                        fee_item: 0u32,
                        destination,
                        weight,
                    },
                )
                .expect_no_logs()
                .execute_reverts(|output| output == b"currencies: Value is too large for length");
        });
    }

    #[test]
    fn transfer_multiassets_works() {
        let destination = MultiLocation::new(
            1,
            Junctions::X2(
                Junction::Parachain(2),
                Junction::AccountId32 {
                    network: None,
                    id: [1u8; 32],
                },
            ),
        );
        let weight = WeightV2::from(3_000_000_000u64, 1024);

        let asset_1_location = MultiLocation::new(
            1,
            Junctions::X2(Junction::Parachain(2), Junction::GeneralIndex(0u128)),
        );
        let asset_2_location = MultiLocation::new(
            1,
            Junctions::X2(Junction::Parachain(2), Junction::GeneralIndex(1u128)),
        );

        let assets = vec![
            (asset_1_location.clone(), U256::from(500)).into(),
            (asset_2_location.clone(), U256::from(500)).into(),
        ]
        .into();

        let multiassets = MultiAssets::from_sorted_and_deduplicated(vec![
            (asset_1_location.clone(), 500).into(),
            (asset_2_location, 500).into(),
        ])
        .unwrap();

        ExtBuilder::default().build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer_multi_assets {
                        assets,
                        fee_item: 0u32,
                        destination,
                        weight,
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let expected: crate::mock::RuntimeEvent =
                mock::RuntimeEvent::Xtokens(XtokensEvent::TransferredMultiAssets {
                    sender: TestAccount::Alice.into(),
                    assets: multiassets,
                    fee: (asset_1_location, 500).into(),
                    dest: destination,
                })
                .into();
            assert!(events().contains(&expected));
        });
    }

    #[test]
    fn transfer_multiassets_cannot_insert_more_than_max() {
        // We have definaed MaxAssetsForTransfer = 2,
        // so any number greater than MaxAssetsForTransfer will result in error
        let destination = MultiLocation::new(
            1,
            Junctions::X2(
                Junction::Parachain(2),
                Junction::AccountId32 {
                    network: None,
                    id: [1u8; 32],
                },
            ),
        );
        let weight = WeightV2::from(3_000_000_000u64, 1024);

        let asset_1_location = MultiLocation::new(
            1,
            Junctions::X2(Junction::Parachain(2), Junction::GeneralIndex(0u128)),
        );
        let asset_2_location = MultiLocation::new(
            1,
            Junctions::X2(Junction::Parachain(2), Junction::GeneralIndex(1u128)),
        );
        let asset_3_location = MultiLocation::new(
            1,
            Junctions::X2(Junction::Parachain(2), Junction::GeneralIndex(3u128)),
        );

        let assets = vec![
            (asset_1_location.clone(), U256::from(500)).into(),
            (asset_2_location.clone(), U256::from(500)).into(),
            (asset_3_location.clone(), U256::from(500)).into(),
        ]
        .into();

        ExtBuilder::default().build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer_multi_assets {
                        assets,
                        fee_item: 0u32,
                        destination,
                        weight,
                    },
                )
                .expect_no_logs()
                .execute_reverts(|output| output == b"assets: Value is too large for length");
        });
    }
}
