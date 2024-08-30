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

use frame_support::{
    dispatch::GetDispatchInfo,
    traits::{Currency, StorePreimage},
};
use parity_scale_codec::Encode;
use sp_runtime::traits::{BlakeTwo256, Hash};

#[test]
fn external_proposals_work() {
    new_test_ext().execute_with(|| {
        let remark_call = RuntimeCall::System(frame_system::Call::remark {
            remark: b"1337".to_vec(),
        });
        let remark_call_bounded = Preimage::bound(remark_call).unwrap();

        let external_propose_call =
            RuntimeCall::Democracy(pallet_democracy::Call::external_propose_majority {
                proposal: remark_call_bounded.clone(),
            });
        let external_propose_call_hash = BlakeTwo256::hash_of(&external_propose_call);

        // Main council should be able to make external proposals
        assert_ok!(Council::propose(
            RuntimeOrigin::signed(ALICE.clone()),
            2,
            Box::new(external_propose_call.clone()),
            external_propose_call.encode().len() as u32
        ));

        // Vote 'aye'
        for signer in &[BOB, CAT] {
            assert_ok!(Council::vote(
                RuntimeOrigin::signed(signer.clone()),
                external_propose_call_hash,
                0,
                true
            ));
        }

        // Close the proposal & execute it
        assert_ok!(Council::close(
            RuntimeOrigin::signed(ALICE.clone()),
            external_propose_call_hash,
            0,
            external_propose_call.get_dispatch_info().weight,
            external_propose_call.encode().len() as u32,
        ));

        let next_external_proposal = pallet_democracy::NextExternal::<Runtime>::get().unwrap();
        assert_eq!(
            next_external_proposal.0, remark_call_bounded,
            "Call should have been put as the next external proposal."
        );

        // Fast-track the proposal
        let (voting_period, delay) = (13, 17);
        let fast_track_call = RuntimeCall::Democracy(pallet_democracy::Call::fast_track {
            proposal_hash: next_external_proposal.0.hash(),
            voting_period,
            delay,
        });
        let fast_track_call_hash = BlakeTwo256::hash_of(&fast_track_call);

        // Tech committee should be able to fast-track external proposals
        assert_ok!(TechnicalCommittee::propose(
            RuntimeOrigin::signed(ALICE.clone()),
            2,
            Box::new(fast_track_call.clone()),
            fast_track_call.encode().len() as u32
        ));

        for signer in &[ALICE, BOB, CAT] {
            assert_ok!(TechnicalCommittee::vote(
                RuntimeOrigin::signed(signer.clone()),
                fast_track_call_hash,
                0,
                true
            ));
        }

        assert_ok!(TechnicalCommittee::close(
            RuntimeOrigin::signed(ALICE.clone()),
            fast_track_call_hash,
            0,
            fast_track_call.get_dispatch_info().weight,
            fast_track_call.encode().len() as u32,
        ));

        // Basic check that a new (first) referendum was created
        let referendum_index = 0;
        let created_referendum =
            pallet_democracy::ReferendumInfoOf::<Runtime>::get(referendum_index).unwrap();
        matches!(
            created_referendum,
            pallet_democracy::ReferendumInfo::Ongoing(_)
        );
    })
}

#[test]
fn community_council_can_execute_dapp_staking_calls() {
    new_test_ext().execute_with(|| {
        // Fund the proxy account
        let proxy_account = <Runtime as pallet_collective_proxy::Config>::ProxyAccountId::get();
        let lock_amount = 10_000_000_000_000_000_000_000;
        Balances::make_free_balance_be(&proxy_account, lock_amount);

        // Prepare the wrapped dApp staking lock call
        let lock_call = RuntimeCall::DappStaking(pallet_dapp_staking::Call::lock {
            amount: lock_amount,
        });
        let collective_proxy_call =
            RuntimeCall::CollectiveProxy(pallet_collective_proxy::Call::execute_call {
                call: Box::new(lock_call),
            });
        let collective_proxy_call_hash = BlakeTwo256::hash_of(&collective_proxy_call);

        // Community council should be able to execute dApp staking calls
        assert_ok!(CommunityCouncil::propose(
            RuntimeOrigin::signed(ALICE.clone()),
            2,
            Box::new(collective_proxy_call.clone()),
            collective_proxy_call.encode().len() as u32
        ));

        for signer in &[BOB, CAT] {
            assert_ok!(CommunityCouncil::vote(
                RuntimeOrigin::signed(signer.clone()),
                collective_proxy_call_hash,
                0,
                true
            ));
        }

        assert_ok!(CommunityCouncil::close(
            RuntimeOrigin::signed(ALICE.clone()),
            collective_proxy_call_hash,
            0,
            collective_proxy_call.get_dispatch_info().weight,
            collective_proxy_call.encode().len() as u32,
        ));

        // Check that the lock was successful
        assert_eq!(
            pallet_dapp_staking::Ledger::<Runtime>::get(&proxy_account).locked(),
            lock_amount
        );
    })
}
