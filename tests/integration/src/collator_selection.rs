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

use crate::{governance::propose_vote_and_close, setup::*};

use frame_support::{assert_ok, dispatch::GetDispatchInfo};
use pallet_collator_selection::{
    CandidacyBond, CandidateInfo, Candidates, DesiredCandidates, PendingApplications,
};
use parity_scale_codec::Encode;
use sp_runtime::traits::{BlakeTwo256, Dispatchable, Hash};

#[test]
fn council_can_approve_and_close_collator_applications() {
    new_test_ext().execute_with(|| {
        let good_candidate = &CAT;
        let bad_candidate = &BOB;
        let bond = CandidacyBond::<Runtime>::get();

        // 1. both candidates apply for candidacy
        assert_ok!(RuntimeCall::CollatorSelection(
            pallet_collator_selection::Call::apply_for_candidacy {}
        )
        .dispatch(RuntimeOrigin::signed(good_candidate.clone())));

        assert_ok!(RuntimeCall::CollatorSelection(
            pallet_collator_selection::Call::apply_for_candidacy {}
        )
        .dispatch(RuntimeOrigin::signed(bad_candidate.clone())));

        // 2. verify both applications are pending
        assert!(PendingApplications::<Runtime>::contains_key(good_candidate));
        assert!(PendingApplications::<Runtime>::contains_key(bad_candidate));
        assert_eq!(Balances::reserved_balance(good_candidate), bond);
        assert_eq!(Balances::reserved_balance(bad_candidate), bond);

        // 3. council approves good candidate
        let approve_call =
            RuntimeCall::CollatorSelection(pallet_collator_selection::Call::approve_application {
                who: good_candidate.clone(),
            });
        propose_vote_and_close!(Council, approve_call, 0);

        // 4. council rejects bad candidate
        let reject_call =
            RuntimeCall::CollatorSelection(pallet_collator_selection::Call::close_application {
                who: bad_candidate.clone(),
            });
        propose_vote_and_close!(Council, reject_call, 1);

        // 5. verify final state, no pending applications remain
        assert_eq!(PendingApplications::<Runtime>::iter().count(), 0);

        // only good candidate is in candidates list
        let candidates = Candidates::<Runtime>::get();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].who, *good_candidate);

        // Bond states: good candidate still reserved, bad candidate refunded
        assert_eq!(Balances::reserved_balance(good_candidate), bond);
        assert_eq!(Balances::reserved_balance(bad_candidate), 0);
    });
}

#[test]
fn council_can_kick_candidates() {
    new_test_ext().execute_with(|| {
        let bond = CandidacyBond::<Runtime>::get();

        // 1. set desrired candidates to 1 & mutate state to add some candidates
        // so we can kick from them later
        DesiredCandidates::<Runtime>::put(6u32);
        Candidates::<Runtime>::put(
            (0..MinCandidates::get())
                .map(|i| {
                    let i: u8 = i.try_into().unwrap();
                    CandidateInfo {
                        who: AccountId32::new([i + 10_u8; 32]),
                        deposit: bond,
                    }
                })
                .collect::<Vec<_>>(),
        );

        // 2. register two candidates, BOB and CAT
        //
        // 2.1 both candidates apply for candidacy
        assert_ok!(RuntimeCall::CollatorSelection(
            pallet_collator_selection::Call::apply_for_candidacy {}
        )
        .dispatch(RuntimeOrigin::signed(BOB)));
        assert_ok!(RuntimeCall::CollatorSelection(
            pallet_collator_selection::Call::apply_for_candidacy {}
        )
        .dispatch(RuntimeOrigin::signed(CAT)));

        // 2.2. approves candidate
        propose_vote_and_close!(
            Council,
            RuntimeCall::CollatorSelection(pallet_collator_selection::Call::approve_application {
                who: BOB,
            }),
            0
        );
        propose_vote_and_close!(
            Council,
            RuntimeCall::CollatorSelection(pallet_collator_selection::Call::approve_application {
                who: CAT,
            }),
            1
        );

        // 3. verify we have two candidates
        assert_eq!(Candidates::<Runtime>::get().len(), 6);

        // 4. kick BOB
        propose_vote_and_close!(
            Council,
            RuntimeCall::CollatorSelection(pallet_collator_selection::Call::kick_candidate {
                who: BOB
            }),
            2
        );

        // 5. verify final state, only CAT remains
        assert_eq!(Candidates::<Runtime>::get().len(), 5);
        assert_eq!(Balances::reserved_balance(BOB), 0);
        assert_eq!(
            Balances::free_balance(BOB),
            INITIAL_AMOUNT - (SlashRatio::get() * bond)
        );
    });
}
