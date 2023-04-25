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

use crate::mocks::{parachain, *};

use frame_support::{assert_ok, traits::fungible::Inspect, weights::Weight};
use pallet_contracts::Determinism;
use pallet_xcm_transactor::{
    chain_extension::{Error as XcmCEError, ValidateSendInput},
    QueryConfig, QueryType,
};
use parity_scale_codec::{Decode, Encode};
use sp_runtime::traits::Bounded;
use xcm::prelude::*;
use xcm_simulator::TestExt;

type AccoundIdOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

const GAS_LIMIT: Weight = Weight::from_parts(100_000_000_000, 3 * 1024 * 1024);

const SELECTOR_CONSTRUCTOR: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];
const SELECTOR_EXECUTE: [u8; 4] = [0x11, 0x11, 0x11, 0x11];
const SELECTOR_SEND: [u8; 4] = [0x22, 0x22, 0x22, 0x22];
const SELECTOR_QUERY: [u8; 4] = [0x33, 0x33, 0x33, 0x33];
const SELECTOR_HANDLE_RESPONSE: [u8; 4] = [0x55, 0x55, 0x55, 0x55];
const SELECTOR_GET: [u8; 4] = [0x66, 0x66, 0x66, 0x66];

#[test]
fn xcm_remote_contract_callback() {
    MockNet::reset();

    // deploy and initialize xcm flipper contract with `false` in ParaA
    let mut contract_id = [0u8; 32].into();
    ParaA::execute_with(|| {
        (contract_id, _) = deploy_contract::<parachain::Runtime>(
            "xcm_flip",
            ALICE.into(),
            0,
            GAS_LIMIT,
            None,
            // selector + true
            SELECTOR_CONSTRUCTOR.to_vec(),
        );

        // check for flip status
        let outcome = ParachainContracts::bare_call(
            ALICE.into(),
            contract_id.clone(),
            0,
            GAS_LIMIT,
            None,
            SELECTOR_GET.to_vec(),
            true,
            Determinism::Deterministic,
        );
        let res = outcome.result.unwrap();
        // check for revert
        assert!(!res.did_revert());
        // decode the return value
        let flag = Result::<bool, ()>::decode(&mut res.data.as_ref()).unwrap();
        assert_eq!(flag, Ok(false));
    });

    // transfer funds to contract derieve account
    ParaB::execute_with(|| {
        use parachain::System;

        let account = sibling_para_account_account_id(1, contract_id.clone());
        assert_ok!(ParachainBalances::transfer(
            parachain::RuntimeOrigin::signed(ALICE),
            account,
            INITIAL_BALANCE / 2,
        ));

        System::reset_events();
    });

    // check the execute
    ParaA::execute_with(|| {
        let transfer_amount = 100_000;
        // transfer some native to contract
        assert_ok!(ParachainBalances::transfer(
            parachain::RuntimeOrigin::signed(ALICE),
            contract_id.clone(),
            transfer_amount,
        ));

        let xcm: Xcm<()> = Xcm(vec![
            WithdrawAsset((Here, transfer_amount).into()),
            BuyExecution {
                fees: (Here, transfer_amount).into(),
                weight_limit: Unlimited,
            },
            DepositAsset {
                assets: All.into(),
                beneficiary: AccountId32 {
                    network: None,
                    id: ALICE.into(),
                }
                .into(),
            },
        ]);

        // run execute in contract
        let alice_balance_before = ParachainBalances::balance(&ALICE.into());
        let (res, _, _) =
            call_contract_method::<parachain::Runtime, Result<Result<Weight, XcmCEError>, ()>>(
                ALICE.into(),
                contract_id.clone(),
                0,
                GAS_LIMIT,
                None,
                [SELECTOR_EXECUTE.to_vec(), VersionedXcm::V3(xcm).encode()].concat(),
                true,
            );

        assert_eq!(res, Ok(Ok(Weight::from_parts(30, 0))));
        assert!(
            // TODO: since bare_call doesn't charge, use call
            ParachainBalances::balance(&ALICE.into()) == alice_balance_before + transfer_amount
        );
    });

    //
    // Check send & query
    //
    ParaA::execute_with(|| {
        use parachain::{Runtime, RuntimeCall};

        let remark_call = RuntimeCall::System(frame_system::Call::remark_with_event {
            remark: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0],
        });

        let config = QueryConfig::<AccoundIdOf<Runtime>, BlockNumberOf<Runtime>> {
            query_type: QueryType::WASMContractCallback {
                contract_id: contract_id.clone(),
                selector: SELECTOR_HANDLE_RESPONSE,
            },
            timeout: Bounded::max_value(),
        };
        let dest: VersionedMultiLocation = (Parent, Parachain(2)).into();

        let (res, _, _) =
            call_contract_method::<parachain::Runtime, Result<Result<QueryId, XcmCEError>, ()>>(
                ALICE.into(),
                contract_id.clone(),
                0,
                GAS_LIMIT,
                None,
                [
                    SELECTOR_QUERY.to_vec(),
                    (config.clone(), dest.clone()).encode(),
                ]
                .concat(),
                true,
            );
        assert_eq!(res, Ok(Ok(0)));
        let query_id = res.unwrap().unwrap();

        let xcm: Xcm<()> = Xcm(vec![
            WithdrawAsset((Here, INITIAL_BALANCE / 2).into()),
            BuyExecution {
                fees: (Here, INITIAL_BALANCE / 2).into(),
                weight_limit: Unlimited,
            },
            SetAppendix(Xcm(vec![ReportTransactStatus(QueryResponseInfo {
                destination: (Parent, Parachain(1)).into(),
                query_id,
                max_weight: GAS_LIMIT,
            })])),
            Transact {
                origin_kind: OriginKind::SovereignAccount,
                require_weight_at_most: Weight::from_parts(100_000_000_000_000, 1024 * 1024 * 1024),
                call: remark_call.encode().into(),
            },
        ]);

        // send xcm
        let (_res, _, _) = call_contract_method::<
            parachain::Runtime,
            Result<Result<VersionedMultiAssets, XcmCEError>, ()>,
        >(
            ALICE.into(),
            contract_id.clone(),
            0,
            GAS_LIMIT,
            None,
            [
                SELECTOR_SEND.to_vec(),
                ValidateSendInput {
                    dest,
                    xcm: VersionedXcm::V3(xcm),
                }
                .encode(),
            ]
            .concat(),
            true,
        );

        // dbg!(res);
    });

    // check if remark was executed in ParaB
    ParaB::execute_with(|| {
        use parachain::{RuntimeEvent, System};
        // check remark events
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::System(frame_system::Event::Remarked { .. })
        )));

        // clear the events
        System::reset_events();
    });

    // check for callback, if callback success then flip=true
    ParaA::execute_with(|| {
        // check for flip status
        let (res, _, _) = call_contract_method::<parachain::Runtime, Result<bool, ()>>(
            ALICE.into(),
            contract_id.clone(),
            0,
            GAS_LIMIT,
            None,
            SELECTOR_GET.to_vec(),
            true,
        );
        assert_eq!(res, Ok(true));
    });

    // ParaA::execute_with(|| {
    //     use parachain::XcmTransact;
    //     // dispatch call to flip contract
    //     // let call = parachain::RuntimeCall::Contracts(pallet_contracts::Call::call {
    //     //     dest: contract_id.clone(),
    //     //     value: 0,
    //     //     gas_limit: Weight::from_parts(100_000_000_000, 1024 * 1024),
    //     //     storage_deposit_limit: None,
    //     //     data: SELECTOR_FLIP.to_vec(),
    //     // });
    //     let call = parachain::RuntimeCall::System(frame_system::Call::remark_with_event {
    //         remark: vec![1, 2, 3, 4],
    //     });

    //     let query_id = XcmTransact::new_query(
    //         QueryConfig {
    //             query_type: QueryType::WASMContractCallback {
    //                 contract_id: contract_id.clone(),
    //                 selector: SELECTOR_XCM_FLIP,
    //             },
    //             timeout: Bounded::max_value(),
    //         },
    //         AccountId32 {
    //             id: ALICE.into(),
    //             network: Some(Kusama),
    //         }
    //         .into(),
    //         // Here,
    //         (Parent, Parachain(2)),
    //     )
    //     .unwrap();

    //     let xcm: Xcm<()> = Xcm(vec![
    //         WithdrawAsset((Here, INITIAL_BALANCE).into()),
    //         BuyExecution {
    //             fees: (Here, INITIAL_BALANCE).into(),
    //             weight_limit: Unlimited,
    //         },
    //         SetAppendix(Xcm(vec![ReportTransactStatus(QueryResponseInfo {
    //             destination: (Parent, Parachain(1)).into(),
    //             query_id,
    //             max_weight: GAS_LIMIT,
    //         })])),
    //         Transact {
    //             origin_kind: OriginKind::SovereignAccount,
    //             require_weight_at_most: Weight::from_parts(100_000_000_000_000, 1024 * 1024 * 1024),
    //             call: call.encode().into(),
    //         },
    //     ]);

    //     // send the XCM to ParaA
    //     assert_ok!(ParachainPalletXcm::send(
    //         parachain::RuntimeOrigin::signed(ALICE),
    //         // parachain::RuntimeOrigin::root(),
    //         Box::new((Parent, Parachain(2)).into()),
    //         Box::new(VersionedXcm::V3(xcm)),
    //     ));

    //     use parachain::System;
    //     System::reset_events();
    //     // println!("{:?}", System::events());
    // });

    // ParaB::execute_with(|| {

    //     // let outcome = ParachainContracts::bare_call(
    //     //     ALICE.into(),
    //     //     contract_id.clone(),
    //     //     0,
    //     //     GAS_LIMIT,
    //     //     None,
    //     //     SELECTOR_GET.to_vec(),
    //     //     true,
    //     //     Determinism::Deterministic,
    //     // );
    //     // let res = outcome.result.unwrap();
    //     // // check for revert
    //     // assert!(res.did_revert() == false);
    //     // // decode the return value, it should be false
    //     // let flag = Result::<bool, ()>::decode(&mut res.data.as_ref()).unwrap();
    //     // assert_eq!(flag, Ok(false));
    // });

    // ParaA::execute_with(|| {
    //     // use parachain::System;
    //     // println!("{:?}", System::events());

    //     let outcome = ParachainContracts::bare_call(
    //         ALICE.into(),
    //         contract_id.clone(),
    //         0,
    //         GAS_LIMIT,
    //         None,
    //         SELECTOR_GET.to_vec(),
    //         true,
    //         Determinism::Deterministic,
    //     );
    //     let res = outcome.result.unwrap();
    //     // check for revert
    //     assert!(res.did_revert() == false);
    //     // decode the return value, it should be false
    //     let flag = Result::<bool, ()>::decode(&mut res.data.as_ref()).unwrap();

    //     // println!("{:?}", ParachainPalletXcm::query(0));
    //     // println!(
    //     //     "expecting response = {:?}",
    //     //     ParachainPalletXcm::expecting_response(
    //     //         &(Parent, Parachain(2)).into(),
    //     //         0,
    //     //         Some(&Here.into())
    //     //     )
    //     // );

    //     // std::thread::sleep(std::time::Duration::from_secs(1));
    //     assert_eq!(flag, Ok(true));
    // });

    // ParaA::execute_with(|| {
    //     // use parachain::System;
    //     // println!("{:?}", System::events());
    //     let xcm: VersionedXcm<()> = VersionedXcm::V3(Xcm(vec![
    //         WithdrawAsset((Here, 10).into()),
    //         BuyExecution {
    //             fees: (Here, 10).into(),
    //             weight_limit: Unlimited,
    //         },
    //         SetAppendix(Xcm(vec![ReportError(QueryResponseInfo {
    //             destination: (Parent, Parachain(1)).into(),
    //             query_id: 1,
    //             max_weight: GAS_LIMIT,
    //         })])),
    //     ]));
    //     let outcome = ParachainContracts::bare_call(
    //         ALICE.into(),
    //         contract_id.clone(),
    //         0,
    //         GAS_LIMIT,
    //         None,
    //         [SELECTOR_TEST.to_vec(), xcm.encode()].concat(),
    //         true,
    //         Determinism::Deterministic,
    //     );
    //     println!("outcome={outcome:?}");
    //     println!("{:?}", String::from_utf8(outcome.debug_message));
    //     let res = outcome.result.unwrap();
    //     // check for revert
    //     assert!(res.did_revert() == false);
    //     // decode the return value, it should be false
    //     let out = Result::<Result<Weight, XcmCEError>, ()>::decode(&mut res.data.as_ref()).unwrap();
    //     println!("out={out:?}");
    //     // println!("{:?}", ParachainPalletXcm::query(0));
    //     // println!(
    //     //     "expecting response = {:?}",
    //     //     ParachainPalletXcm::expecting_response(
    //     //         &(Parent, Parachain(2)).into(),
    //     //         0,
    //     //         Some(&Here.into())
    //     //     )
    //     // );

    //     // std::thread::sleep(std::time::Duration::from_secs(1));
    //     // assert_eq!(flag, Ok(true));
    // });
}
