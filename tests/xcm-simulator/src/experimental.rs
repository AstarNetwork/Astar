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

use crate::mocks::{msg_queue::mock_msg_queue, parachain, *};

use frame_support::{assert_ok, weights::Weight};
use parity_scale_codec::Encode;
use sp_runtime::traits::Bounded;
use xcm::{prelude::*, v3::Response};
use xcm_simulator::TestExt;

#[test]
fn basic_xcmp_outcome() {
    MockNet::reset();

    // basic remark call
    let remark = parachain::RuntimeCall::System(
        frame_system::Call::<parachain::Runtime>::remark_with_event {
            remark: vec![1, 2, 3],
        },
    );

    // priveleged root call
    let root_call =
        parachain::RuntimeCall::System(frame_system::Call::<parachain::Runtime>::set_storage {
            items: vec![(vec![0], vec![1])],
        });

    let send_transcat = |call: parachain::RuntimeCall, dest: MultiLocation| {
        let mut xcm = Xcm(vec![
            WithdrawAsset((Here, 100_000_000_000_u128).into()),
            BuyExecution {
                fees: (Here, 100_000_000_000_u128).into(),
                weight_limit: Unlimited,
            },
            Transact {
                origin_kind: OriginKind::SovereignAccount,
                require_weight_at_most: Weight::from_parts(1_000_000_000, 1024 * 1024),
                call: call.encode().into(),
            },
            ExpectTransactStatus(MaybeErrorCode::Success),
        ]);

        // this will register the query and add `SetApendix` with `ReportError`.
        ParachainPalletXcm::report_outcome(&mut xcm, dest, Bounded::max_value()).unwrap();
        // We have to swap the appendix instruction with widthraw & buy execution
        // to make barrier(AllowTopLevelPaidExecutionFrom) happy.
        xcm.0.swap(0, 1);
        xcm.0.swap(1, 2);

        // send the XCM to ParaB
        assert_ok!(ParachainPalletXcm::send_xcm(Here, dest, xcm,));
    };

    // send the remark Transct to ParaB expecting success and have outcome back
    // TODO: do not use `pallet_xcm::report_outcome()` directly,
    //       build a mock pallet to wrap it in a dispatch
    ParaA::execute_with(move || {
        send_transcat(remark, (Parent, Parachain(2)).into());
    });

    // check for if remark was executed in ParaB
    ParaB::execute_with(|| {
        use parachain::{RuntimeEvent, System};
        // check remark events
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::System(frame_system::Event::Remarked { .. })
        )));
    });

    // check the outcome we recieved from ParaB
    ParaA::execute_with(|| {
        let xcms = parachain::MsgQueue::received_xcmp();
        // sanity check
        assert!(xcms.len() == 1 && xcms[0].len() == 1);
        assert!(matches!(
            xcms[0].0.as_slice(),
            &[QueryResponse {
                query_id: 0,
                response: Response::ExecutionResult(None),
                ..
            }]
        ));
    });

    //
    // Failure
    //

    // send the root_call Transct to ParaB expecting failure and have outcome back
    // TODO: do not use `pallet_xcm::report_outcome()` directly,
    //       build a mock pallet to wrap it in a dispatch
    ParaA::execute_with(move || {
        send_transcat(root_call, (Parent, Parachain(2)).into());
    });

    // check for if remark was executed in ParaB
    ParaB::execute_with(|| {
        use parachain::{RuntimeEvent, System};
        // check queue failed events
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::MsgQueue(mock_msg_queue::Event::Fail(..))
        )));
    });

    // check the outcome we recieved from ParaB
    ParaA::execute_with(|| {
        let xcms = parachain::MsgQueue::received_xcmp();
        // sanity check
        assert!(xcms.len() == 2 && xcms[1].len() == 1);
        assert!(matches!(
            xcms[1].0.as_slice(),
            &[QueryResponse {
                query_id: 1,
                response: Response::ExecutionResult(Some((4, xcm::v3::Error::ExpectationFalse))),
                ..
            }]
        ));
    });
}
