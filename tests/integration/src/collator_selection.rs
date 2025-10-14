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

use crate::{propose_referendum_and_pass, propose_vote_and_close, setup::*};

use frame_support::{assert_ok, dispatch::GetDispatchInfo, traits::Currency};
use pallet_collator_selection::DesiredCandidates;
use parity_scale_codec::Encode;
use sp_runtime::{
    traits::{BlakeTwo256, Dispatchable, Hash},
    SaturatedConversion,
};

fn add_candidates(count: u32) {
    for i in 100..(count + 100) {
        let who = AccountId32::new([i.saturated_into::<u8>(); 32]);
        Balances::make_free_balance_be(&who, INITIAL_AMOUNT);

        assert_ok!(RuntimeCall::Session(pallet_session::Call::set_keys {
            keys: SessionKeys {
                aura: sr25519::Pair::from_seed_slice(who.encode().as_slice())
                    .unwrap()
                    .public()
                    .into()
            },
            proof: vec![]
        })
        .dispatch(RuntimeOrigin::signed(who.clone())));

        assert_ok!(RuntimeCall::CollatorSelection(
            pallet_collator_selection::Call::apply_for_candidacy {}
        )
        .dispatch(RuntimeOrigin::signed(who.clone())));
        assert_ok!(RuntimeCall::CollatorSelection(
            pallet_collator_selection::Call::approve_application { who: who.clone() }
        )
        .dispatch(RuntimeOrigin::root()));
    }
}

fn test_approve_and_close_with<F>(mut execute_privileged: F)
where
    F: FnMut(RuntimeCall),
{
    let good_candidate = &CAT;
    let bad_candidate = &BOB;

    // 1. both candidates apply for candidacy
    assert_ok!(RuntimeCall::CollatorSelection(
        pallet_collator_selection::Call::apply_for_candidacy {}
    )
    .dispatch(RuntimeOrigin::signed(good_candidate.clone())));

    assert_ok!(RuntimeCall::CollatorSelection(
        pallet_collator_selection::Call::apply_for_candidacy {}
    )
    .dispatch(RuntimeOrigin::signed(bad_candidate.clone())));

    // 2. council approves good candidate
    execute_privileged(RuntimeCall::CollatorSelection(
        pallet_collator_selection::Call::approve_application {
            who: good_candidate.clone(),
        },
    ));
    // 4. council rejects bad candidate
    execute_privileged(RuntimeCall::CollatorSelection(
        pallet_collator_selection::Call::close_application {
            who: bad_candidate.clone(),
        },
    ));
}

fn test_kick_with<F>(mut execute_privileged: F)
where
    F: FnMut(RuntimeCall),
{
    let min_candidates = MinCandidates::get();

    // 1. set desired candidates
    DesiredCandidates::<Runtime>::put((min_candidates + 1) as u32);
    // 2. mutate state to add min candidates
    add_candidates(min_candidates.try_into().unwrap());

    // 3. register candidate, BOB
    assert_ok!(RuntimeCall::CollatorSelection(
        pallet_collator_selection::Call::apply_for_candidacy {}
    )
    .dispatch(RuntimeOrigin::signed(BOB)));

    execute_privileged(RuntimeCall::CollatorSelection(
        pallet_collator_selection::Call::approve_application { who: BOB },
    ));

    // 4. kick BOB
    execute_privileged(RuntimeCall::CollatorSelection(
        pallet_collator_selection::Call::kick_candidate { who: BOB },
    ));
}

#[test]
fn council_can_approve_and_close_collator_applications() {
    new_test_ext().execute_with(|| {
        let mut proposal_index =
            pallet_collective::ProposalCount::<Runtime, MainCouncilCollectiveInst>::get();

        test_approve_and_close_with(|call| {
            propose_vote_and_close!(Council, call, proposal_index);
            proposal_index += 1;
        });
    });
}

#[test]
fn referendum_can_approve_and_close_collator_applications() {
    new_test_ext().execute_with(|| {
        test_approve_and_close_with(|call| {
            propose_referendum_and_pass!(call);
        });
    });
}

#[test]
fn council_can_kick_candidates() {
    new_test_ext().execute_with(|| {
        let mut proposal_index =
            pallet_collective::ProposalCount::<Runtime, MainCouncilCollectiveInst>::get();

        test_kick_with(|call| {
            propose_vote_and_close!(Council, call, proposal_index);
            proposal_index += 1;
        });
    });
}

#[test]
fn referendum_can_kick_candidates() {
    new_test_ext().execute_with(|| {
        test_kick_with(|call| {
            propose_referendum_and_pass!(call);
        });
    });
}

#[test]
fn referendum_can_force_leave_candidates() {
    new_test_ext().execute_with(|| {
        run_for_blocks(1);

        let session = pallet_session::CurrentIndex::<Runtime>::get();
        let min_candidates = MinCandidates::get();

        // 1. set desired candidates
        DesiredCandidates::<Runtime>::put((min_candidates + 1) as u32);
        // 2. mutate state to add min candidates
        add_candidates(min_candidates);

        // 3. register candidate, BOB
        assert_ok!(RuntimeCall::CollatorSelection(
            pallet_collator_selection::Call::apply_for_candidacy {}
        )
        .dispatch(RuntimeOrigin::signed(BOB)));

        propose_referendum_and_pass!(RuntimeCall::CollatorSelection(
            pallet_collator_selection::Call::approve_application { who: BOB },
        ));

        // 4. BOB forced to leave via referendum
        propose_referendum_and_pass!(RuntimeCall::Utility(pallet_utility::Call::dispatch_as {
            as_origin: Box::new(frame_system::RawOrigin::Signed(BOB).into()),
            call: Box::new(RuntimeCall::CollatorSelection(
                pallet_collator_selection::Call::leave_intent {}
            )),
        }));

        // 5. BOB can withdraw bond after session change as usual
        // run_for_block is slow to execute, so manually bump session index
        pallet_session::CurrentIndex::<Runtime>::put(session + 1);
        assert_ok!(RuntimeCall::CollatorSelection(
            pallet_collator_selection::Call::withdraw_bond {}
        )
        .dispatch(RuntimeOrigin::signed(BOB)));
    });
}
