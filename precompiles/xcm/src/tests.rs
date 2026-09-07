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

use crate::mock::*;
use crate::*;

use astar_primitives::xcm::ASSET_HUB_PARA_ID;
use parity_scale_codec::Encode;
use precompile_utils::testing::*;
use sp_core::{H160, H256};

fn precompiles() -> TestPrecompileSet<Runtime> {
    PrecompilesValue::get()
}

/// `AccountId32` beneficiary, as the precompile builds it from a raw `bytes32`.
fn beneficiary_32(byte: u8) -> Location {
    Location::new(
        0,
        [AccountId32 {
            network: None,
            id: [byte; 32],
        }],
    )
}

/// `AccountKey20` beneficiary, as the precompile builds it from a raw `address`.
fn beneficiary_key_20(byte: u8) -> Location {
    Location::new(
        0,
        [AccountKey20 {
            network: None,
            key: [byte; 20],
        }],
    )
}

/// The single XCM the mock router recorded, panicking if there isn't exactly one.
fn only_sent_xcm() -> (Location, Xcm<()>) {
    let mut sent = take_sent_xcm();
    assert_eq!(sent.len(), 1, "expected exactly one XCM to be sent");
    sent.pop().expect("length checked above")
}

/// Asserts the message ends by depositing everything to `beneficiary`.
fn assert_deposits_to(message: &Xcm<()>, beneficiary: &Location) {
    let deposit = message
        .0
        .iter()
        .rev()
        .find_map(|instruction| match instruction {
            DepositAsset {
                assets,
                beneficiary,
            } => Some((assets, beneficiary)),
            _ => None,
        })
        .expect("message must deposit the assets somewhere");

    assert_eq!(
        deposit.1, beneficiary,
        "assets must be deposited to the requested beneficiary"
    );
    assert!(
        matches!(deposit.0, Wild(AllCounted(_))),
        "precompile always deposits every asset that survived the transfer, got {:?}",
        deposit.0
    );
}

mod assets_withdraw {
    use super::*;

    #[test]
    fn wrong_assets_len_reverts() {
        ExtBuilder.build().execute_with(|| {
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
        });
    }

    #[test]
    fn out_of_bounds_fee_index_reverts() {
        ExtBuilder.build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_native_v1 {
                        assets: vec![Address::from(Runtime::asset_id_to_address(2u128))].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: false,
                        parachain_id: 10.into(),
                        fee_index: 2.into(),
                    },
                )
                .expect_no_logs()
                .execute_reverts(|output| {
                    output == b"fee_index is out of bounds of the assets list"
                });
        });
    }

    #[test]
    fn sanity_checks_for_parameters() {
        ExtBuilder.build().execute_with(|| {
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

            // more than `MAX_ASSETS_FOR_TRANSFER` assets can not be sent
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
    fn sibling_parachain_asset_back_to_its_reserve_works() {
        ExtBuilder.build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_native_v1 {
                        // asset 2 is registered at `(1, Parachain(10))`, so parachain 10 is its
                        // reserve - a plain destination-reserve withdraw.
                        assets: vec![Address::from(Runtime::asset_id_to_address(2u128))].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: false,
                        parachain_id: 10.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let (dest, message) = only_sent_xcm();
            assert_eq!(dest, Location::new(1, [Parachain(10)]));
            assert_deposits_to(&message, &beneficiary_32(0xF1));
        });
    }

    #[test]
    fn relay_token_is_routed_through_asset_hub() {
        ExtBuilder.build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_native_v1 {
                        // asset 1 is the relay token, registered at `(1, Here)`. Its reserve is
                        // Asset Hub, so a transfer to parachain 10 must go through it.
                        assets: vec![Address::from(Runtime::asset_id_to_address(1u128))].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: false,
                        parachain_id: 10.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let (dest, _message) = only_sent_xcm();
            assert_eq!(
                dest,
                Location::new(1, [Parachain(ASSET_HUB_PARA_ID)]),
                "relay token must be withdrawn via its Asset Hub reserve, not sent to the \
                 destination directly"
            );
        });
    }

    #[test]
    fn relay_token_to_asset_hub_works() {
        ExtBuilder.build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_native_v1 {
                        assets: vec![Address::from(Runtime::asset_id_to_address(1u128))].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: false,
                        parachain_id: ASSET_HUB_PARA_ID.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let (dest, message) = only_sent_xcm();
            assert_eq!(dest, Location::new(1, [Parachain(ASSET_HUB_PARA_ID)]));
            assert_deposits_to(&message, &beneficiary_32(0xF1));
        });
    }

    #[test]
    fn dot_to_relay_reverts() {
        ExtBuilder.build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_native_v1 {
                        assets: vec![Address::from(Runtime::asset_id_to_address(1u128))].into(), // DOT
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: true,
                        parachain_id: 0.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_reverts(|output| {
                    output == b"DOT cannot be sent directly to the relay. Route via AssetHub (parachain 1000)."
                });
        });
    }

    /// The asset's own chain is neither the origin nor the destination, so it has to act as a
    /// remote reserve.
    #[test]
    fn asset_of_a_third_chain_uses_a_remote_reserve() {
        ExtBuilder.build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_native_v1 {
                        // asset 3 lives at `(1, Parachain(10), GeneralIndex(20))`; parachain 10 is
                        // its only reserve, and it is not reachable from parachain 20.
                        assets: vec![Address::from(Runtime::asset_id_to_address(3u128))].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: false,
                        parachain_id: 20.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let (dest, _) = only_sent_xcm();
            assert_eq!(
                dest,
                Location::new(1, [Parachain(10)]),
                "asset must be withdrawn via its own chain acting as remote reserve"
            );
        });
    }
}

mod assets_reserve_transfer {
    use super::*;

    /// Exactly the call `AstarReceiver.xcmTransferNativeAsset` makes: the native token, addressed
    /// by the zero address, to an `AccountId32` on Bifrost.
    #[test]
    fn native_asset_to_sibling_works() {
        ExtBuilder.build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_reserve_transfer_native_v1 {
                        assets: vec![Address::from(NATIVE_ADDRESS)].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: false,
                        parachain_id: 2030.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let (dest, message) = only_sent_xcm();
            assert_eq!(dest, Location::new(1, [Parachain(2030)]));
            assert_deposits_to(&message, &beneficiary_32(0xF1));
        });
    }

    /// The two selectors agree on XC20 assets - they were the same xtokens dispatch.
    #[test]
    fn matches_assets_withdraw_for_xc20() {
        ExtBuilder.build().execute_with(|| {
            let xc20 = || PrecompileCall::assets_withdraw_native_v1 {
                assets: vec![Address::from(Runtime::asset_id_to_address(2u128))].into(),
                amounts: vec![42000u64.into()].into(),
                recipient_account_id: H256::repeat_byte(0xF1),
                is_relay: false,
                parachain_id: 10.into(),
                fee_index: 0.into(),
            };

            precompiles()
                .prepare_test(TestAccount::Alice, PRECOMPILE_ADDRESS, xc20())
                .expect_no_logs()
                .execute_returns(true);
            let withdrawn = only_sent_xcm();

            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_reserve_transfer_native_v1 {
                        assets: vec![Address::from(Runtime::asset_id_to_address(2u128))].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: false,
                        parachain_id: 10.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            assert_eq!(withdrawn, only_sent_xcm());
        });
    }

    /// ...and they differ on exactly one thing: `assets_withdraw` has never read the zero address
    /// as the native token, so an uninitialised address stays a revert there rather than becoming
    /// a native-balance transfer.
    #[test]
    fn zero_address_is_native_here_but_not_in_assets_withdraw() {
        ExtBuilder.build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_native_v1 {
                        assets: vec![Address::from(NATIVE_ADDRESS)].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: H256::repeat_byte(0xF1),
                        is_relay: false,
                        parachain_id: 2030.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_reverts(|output| {
                    String::from_utf8_lossy(output).contains("Assets resolution failure.")
                });

            assert!(take_sent_xcm().is_empty());
        });
    }
}

mod transfer {
    use super::*;

    fn weight() -> WeightV2 {
        WeightV2::from(3_000_000_000u64, 1024)
    }

    #[test]
    fn sibling_parachain_asset_works() {
        ExtBuilder.build().execute_with(|| {
            let destination = Location::new(
                1,
                [
                    Parachain(10),
                    AccountId32 {
                        network: None,
                        id: [1u8; 32],
                    },
                ],
            );

            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer {
                        currency_address: Address::from(Runtime::asset_id_to_address(2u128)),
                        amount_of_tokens: 42000u64.into(),
                        destination,
                        weight: weight(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let (dest, message) = only_sent_xcm();
            assert_eq!(
                dest,
                Location::new(1, [Parachain(10)]),
                "the beneficiary junction must be stripped from the destination"
            );
            assert_deposits_to(&message, &beneficiary_32(1));
        });
    }

    #[test]
    fn native_asset_works() {
        ExtBuilder.build().execute_with(|| {
            let destination = Location::new(
                1,
                [
                    Parachain(10),
                    AccountId32 {
                        network: None,
                        id: [1u8; 32],
                    },
                ],
            );

            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer {
                        currency_address: Address::from(NATIVE_ADDRESS),
                        amount_of_tokens: 42000u64.into(),
                        destination,
                        weight: weight(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let (dest, message) = only_sent_xcm();
            assert_eq!(dest, Location::new(1, [Parachain(10)]));
            assert_deposits_to(&message, &beneficiary_32(1));
        });
    }

    #[test]
    fn relay_token_to_asset_hub_works() {
        ExtBuilder.build().execute_with(|| {
            let destination = Location::new(
                1,
                [
                    Parachain(ASSET_HUB_PARA_ID),
                    AccountId32 {
                        network: None,
                        id: [1u8; 32],
                    },
                ],
            );

            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer {
                        currency_address: Address::from(Runtime::asset_id_to_address(1u128)),
                        amount_of_tokens: 42000u64.into(),
                        destination,
                        weight: weight(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let (dest, message) = only_sent_xcm();
            assert_eq!(dest, Location::new(1, [Parachain(ASSET_HUB_PARA_ID)]));
            assert_deposits_to(&message, &beneficiary_32(1));
        });
    }

    #[test]
    fn relay_token_to_relay_reverts() {
        ExtBuilder.build().execute_with(|| {
            let destination = Location::new(
                1,
                [AccountId32 {
                    network: None,
                    id: [1u8; 32],
                }],
            );

            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer {
                        currency_address: Address::from(Runtime::asset_id_to_address(1u128)),
                        amount_of_tokens: 42000u64.into(),
                        destination,
                        weight: weight(),
                    },
                )
                .expect_no_logs()
                .execute_reverts(|output| {
                    output == b"DOT cannot be sent directly to the relay. Route via AssetHub (parachain 1000)."
                });
        });
    }

    #[test]
    fn unknown_currency_address_reverts() {
        ExtBuilder.build().execute_with(|| {
            let destination = Location::new(
                1,
                [
                    Parachain(10),
                    AccountId32 {
                        network: None,
                        id: [1u8; 32],
                    },
                ],
            );

            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer {
                        currency_address: Address::from(H160::repeat_byte(0xF1)),
                        amount_of_tokens: 42000u64.into(),
                        destination,
                        weight: weight(),
                    },
                )
                .expect_no_logs()
                .execute_reverts(|output| output == b"Failed to resolve asset id from address");
        });
    }

    #[test]
    fn destination_without_chain_part_reverts() {
        ExtBuilder.build().execute_with(|| {
            // No parent and no chain junction - there is no chain to send this to.
            let destination = Location::new(
                0,
                [AccountId32 {
                    network: None,
                    id: [1u8; 32],
                }],
            );

            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::transfer {
                        currency_address: Address::from(NATIVE_ADDRESS),
                        amount_of_tokens: 42000u64.into(),
                        destination,
                        weight: weight(),
                    },
                )
                .expect_no_logs()
                .execute_reverts(|output| {
                    output == b"error splitting destination into chain and beneficiary"
                });
        });
    }
}

/// The six selectors that were backed by `orml-xtokens` and have no pallet-xcm equivalent.
///
/// They stay registered so the precompile's ABI is unchanged, but every one of them must revert
/// with the shared deprecation notice rather than silently doing something unexpected.
mod evm_beneficiary {
    use super::*;

    /// The `address` overloads deposit to an `AccountKey20` on the destination.
    #[test]
    fn assets_withdraw_deposits_to_account_key_20() {
        ExtBuilder.build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_withdraw_evm_v1 {
                        assets: vec![Address::from(Runtime::asset_id_to_address(2u128))].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: Address(H160::repeat_byte(0xDE)),
                        is_relay: false,
                        parachain_id: 10.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let (dest, message) = only_sent_xcm();
            assert_eq!(dest, Location::new(1, [Parachain(10)]));
            assert_deposits_to(&message, &beneficiary_key_20(0xDE));
        });
    }

    /// `assets_reserve_transfer` is the same alias relationship on the `address` overloads.
    #[test]
    fn assets_reserve_transfer_is_an_alias() {
        ExtBuilder.build().execute_with(|| {
            precompiles()
                .prepare_test(
                    TestAccount::Alice,
                    PRECOMPILE_ADDRESS,
                    PrecompileCall::assets_reserve_transfer_evm_v1 {
                        assets: vec![Address::from(Runtime::asset_id_to_address(2u128))].into(),
                        amounts: vec![42000u64.into()].into(),
                        recipient_account_id: Address(H160::repeat_byte(0xDE)),
                        is_relay: false,
                        parachain_id: 10.into(),
                        fee_index: 0.into(),
                    },
                )
                .expect_no_logs()
                .execute_returns(true);

            let (dest, message) = only_sent_xcm();
            assert_eq!(dest, Location::new(1, [Parachain(10)]));
            assert_deposits_to(&message, &beneficiary_key_20(0xDE));
        });
    }
}

mod remote_transact {
    use super::*;

    const CALL: [u8; 4] = [0xff, 0xaa, 0x77, 0x00];
    const TRANSACT_WEIGHT: u64 = 3_000_000_000;

    fn call(is_relay: bool, para_id: u64) -> PrecompileCall {
        PrecompileCall::remote_transact_v1 {
            para_id: para_id.into(),
            is_relay,
            fee_asset_addr: Address::from(Runtime::asset_id_to_address(2u128)),
            fee_amount: 367.into(),
            remote_call: CALL.to_vec().into(),
            transact_weight: TRANSACT_WEIGHT,
        }
    }

    /// The caller is descended into the origin exactly as `pallet_xcm::send` used to do for a
    /// signed origin, so the account the destination derives does not move.
    #[test]
    fn sibling_transact_descends_the_caller() {
        ExtBuilder.build().execute_with(|| {
            precompiles()
                .prepare_test(TestAccount::Alice, PRECOMPILE_ADDRESS, call(false, 10))
                .expect_no_logs()
                .execute_returns(true);

            let (dest, message) = only_sent_xcm();
            assert_eq!(dest, Location::new(1, [Parachain(10)]));

            assert_eq!(
                message.0.first(),
                Some(&DescendOrigin(
                    AccountId32 {
                        network: Some(NetworkId::Polkadot),
                        id: [0xAA; 32],
                    }
                    .into()
                )),
                "the message must open with the caller's `AccountId32`, network included"
            );

            assert!(
                matches!(message.0.get(1), Some(WithdrawAsset(_))),
                "fees are withdrawn on the destination, got {:?}",
                message.0.get(1)
            );
            assert!(
                matches!(message.0.get(2), Some(BuyExecution { .. })),
                "execution is bought with them, got {:?}",
                message.0.get(2)
            );
            assert_eq!(
                message.0.get(3),
                Some(&Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: Some(Weight::from_parts(
                        TRANSACT_WEIGHT,
                        DEFAULT_PROOF_SIZE
                    )),
                    call: CALL.to_vec().into(),
                })
            );
        });
    }

    /// UMP is `Root`-only for a reason - the EVM must not be able to reach it at all.
    #[test]
    fn relay_destination_reverts() {
        ExtBuilder.build().execute_with(|| {
            precompiles()
                .prepare_test(TestAccount::Alice, PRECOMPILE_ADDRESS, call(true, 0))
                .expect_no_logs()
                .execute_reverts(|output| {
                    String::from_utf8_lossy(output)
                        .contains("remote_transact to the relay chain is not supported")
                });

            assert!(
                take_sent_xcm().is_empty(),
                "a relay-bound remote_transact must not send any XCM"
            );
        });
    }
}

mod deprecated {
    use super::*;

    fn weight() -> WeightV2 {
        WeightV2::from(3_000_000_000u64, 1024)
    }

    fn parachain_destination() -> Location {
        Location::new(
            1,
            [
                Parachain(10),
                AccountId32 {
                    network: None,
                    id: [1u8; 32],
                },
            ],
        )
    }

    fn assert_deprecated(call: PrecompileCall) {
        precompiles()
            .prepare_test(TestAccount::Alice, PRECOMPILE_ADDRESS, call)
            .expect_no_logs()
            .execute_reverts(|output| {
                String::from_utf8_lossy(output).contains("deprecated: xtokens has been removed")
            });

        assert!(
            take_sent_xcm().is_empty(),
            "a deprecated method must not send any XCM"
        );
    }

    #[test]
    fn send_xcm_reverts() {
        ExtBuilder.build().execute_with(|| {
            let message: Xcm<()> = Xcm(vec![ClearOrigin]);
            assert_deprecated(PrecompileCall::send_xcm {
                dest: Location::parent(),
                xcm_call: xcm::VersionedXcm::V5(message).encode().into(),
            });
        });
    }

    #[test]
    fn transfer_with_fee_reverts() {
        ExtBuilder.build().execute_with(|| {
            assert_deprecated(PrecompileCall::transfer_with_fee {
                currency_address: Address::from(Runtime::asset_id_to_address(2u128)),
                amount_of_tokens: 42000u64.into(),
                fee: 100u64.into(),
                destination: parachain_destination(),
                weight: weight(),
            });
        });
    }

    #[test]
    fn transfer_multiasset_reverts() {
        ExtBuilder.build().execute_with(|| {
            assert_deprecated(PrecompileCall::transfer_multiasset {
                asset_location: Location::new(1, [Parachain(10)]),
                amount_of_tokens: 42000u64.into(),
                destination: parachain_destination(),
                weight: weight(),
            });
        });
    }

    #[test]
    fn transfer_multiasset_with_fee_reverts() {
        ExtBuilder.build().execute_with(|| {
            assert_deprecated(PrecompileCall::transfer_multiasset_with_fee {
                asset_location: Location::new(1, [Parachain(10)]),
                amount_of_tokens: 42000u64.into(),
                fee: 100u64.into(),
                destination: parachain_destination(),
                weight: weight(),
            });
        });
    }

    #[test]
    fn transfer_multi_currencies_reverts() {
        ExtBuilder.build().execute_with(|| {
            assert_deprecated(PrecompileCall::transfer_multi_currencies {
                currencies: vec![(
                    Address::from(Runtime::asset_id_to_address(2u128)),
                    42000.into(),
                )
                    .into()]
                .into(),
                fee_item: 0u32,
                destination: parachain_destination(),
                weight: weight(),
            });
        });
    }

    #[test]
    fn transfer_multi_assets_reverts() {
        ExtBuilder.build().execute_with(|| {
            assert_deprecated(PrecompileCall::transfer_multi_assets {
                assets: vec![(Location::new(1, [Parachain(10)]), 42000.into()).into()].into(),
                fee_item: 0u32,
                destination: parachain_destination(),
                weight: weight(),
            });
        });
    }
}
