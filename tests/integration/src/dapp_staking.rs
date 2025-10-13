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

#![cfg(test)]

use crate::setup::*;
use sp_runtime::Perquintill;

use pallet_collator_selection::{CandidateInfo, Candidates};
use pallet_dapp_staking::*;

#[test]
fn dapp_staking_triggers_inflation_recalculation() {
    new_test_ext().execute_with(|| {
        let init_inflation_config = pallet_inflation::ActiveInflationConfig::<Runtime>::get();
        let recalculation_era = init_inflation_config.recalculation_era;

        // It's not feasible to run through all the blocks needed to trigger all the eras.
        // Instead, we force the era to change on a block by block basis.
        while ActiveProtocolState::<Runtime>::get().era() < recalculation_era - 1 {
            assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era,));
            run_for_blocks(1);
            assert_eq!(
                init_inflation_config,
                pallet_inflation::ActiveInflationConfig::<Runtime>::get(),
                "Must not change until recalculation"
            );
        }
        assert_eq!(
            ActiveProtocolState::<Runtime>::get().subperiod(),
            Subperiod::BuildAndEarn,
            "Sanity check."
        );

        // Again, hacky approach to speed things up.
        // This doesn't influence anything in the protocol essentially.
        ActiveProtocolState::<Runtime>::mutate(|state| {
            state.set_next_era_start(System::block_number() + 5);
        });

        // Another sanity check, move block by block and ensure protocol works as expected.
        let target_block = ActiveProtocolState::<Runtime>::get().next_era_start();
        run_to_block(target_block - 2);
        assert_eq!(
            init_inflation_config,
            pallet_inflation::ActiveInflationConfig::<Runtime>::get(),
            "Sanity check."
        );

        // So far inflation config remained unchanged.
        // Now we expect the trigger which will update it.
        run_for_blocks(1);
        assert_eq!(
            init_inflation_config,
            pallet_inflation::ActiveInflationConfig::<Runtime>::get(),
            "Still the same, should be updated ONLY after the block has been finalized."
        );

        run_for_blocks(1);
        let new_inflation_config = pallet_inflation::ActiveInflationConfig::<Runtime>::get();
        assert_ne!(
            init_inflation_config, new_inflation_config,
            "Must be updated after the block has been finalized."
        );
    });
}

#[test]
fn lock_not_possible_for_collator_candidate_account() {
    new_test_ext().execute_with(|| {
        // Hacky approach but it works
        let candidate_info = CandidateInfo {
            who: ALICE.clone(),
            deposit: pallet_collator_selection::CandidacyBond::<Runtime>::get(),
        };
        Candidates::<Runtime>::mutate(|candidates| {
            candidates.push(candidate_info);
        });

        // Now try to participate in dApp staking with Alice and expect an error
        let minimum_lock_amount =
            <Runtime as pallet_dapp_staking::Config>::MinimumLockedAmount::get();
        assert_noop!(
            DappStaking::lock(RuntimeOrigin::signed(ALICE.clone()), minimum_lock_amount,),
            pallet_dapp_staking::Error::<Runtime>::AccountNotAvailableForDappStaking
        );
    });
}

// Not the ideal place for such test, can be moved later.
#[test]
fn collator_selection_candidacy_application_not_possible_for_dapp_staking_participant() {
    new_test_ext().execute_with(|| {
        // Lock some amount with Alice
        let minimum_lock_amount =
            <Runtime as pallet_dapp_staking::Config>::MinimumLockedAmount::get();
        assert_ok!(DappStaking::lock(
            RuntimeOrigin::signed(ALICE.clone()),
            minimum_lock_amount,
        ));

        // Ensure it's not possible to become a candidate for collator selection while having locked funds in dApp staking
        assert_ok!(CollatorSelection::set_desired_candidates(
            RuntimeOrigin::root(),
            1_000_000,
        ));
        assert_noop!(
            CollatorSelection::apply_for_candidacy(RuntimeOrigin::signed(ALICE.clone())),
            pallet_collator_selection::Error::<Runtime>::NotAllowedCandidate
        );
    });
}

#[test]
fn collator_selection_candidacy_approval_not_possible_for_dapp_staking_participant() {
    new_test_ext().execute_with(|| {
        assert_ok!(CollatorSelection::set_desired_candidates(
            RuntimeOrigin::root(),
            1_000_000,
        ));

        // First apply for candidacy with Alice
        assert_ok!(CollatorSelection::apply_for_candidacy(
            RuntimeOrigin::signed(ALICE.clone())
        ));

        // Lock some amount with Alice
        let minimum_lock_amount =
            <Runtime as pallet_dapp_staking::Config>::MinimumLockedAmount::get();
        assert_ok!(DappStaking::lock(
            RuntimeOrigin::signed(ALICE.clone()),
            minimum_lock_amount,
        ));

        // Ensure it's not possible to become a candidate for collator selection while having locked funds in dApp staking
        assert_noop!(
            CollatorSelection::approve_application(RuntimeOrigin::root(), ALICE.clone()),
            pallet_collator_selection::Error::<Runtime>::NotAllowedCandidate
        );
    });
}

#[test]
fn no_inflation_rewards_with_zero_decay() {
    new_test_ext().execute_with(|| {
        let mut config = pallet_inflation::ActiveInflationConfig::<Runtime>::get();
        config.decay_rate = Perquintill::zero();
        pallet_inflation::ActiveInflationConfig::<Runtime>::put(config.clone());

        let issuance_before = Balances::total_issuance();

        // Advance eras on a block by block basis until subperiod is Voting again for bonus reward payouts
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_for_blocks(1);
        assert_eq!(
            ActiveProtocolState::<Runtime>::get().subperiod(),
            Subperiod::BuildAndEarn,
            "Sanity check."
        );
        while ActiveProtocolState::<Runtime>::get().subperiod() != Subperiod::Voting {
            assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
            run_for_blocks(1);
        }

        let decay_factor = pallet_inflation::ActiveInflationConfig::<Runtime>::get().decay_factor;
        assert_eq!(
            decay_factor,
            Perquintill::zero(),
            "Decay factor must be zero"
        );

        let issuance_after = Balances::total_issuance();
        assert_eq!(
            issuance_before, issuance_after,
            "No rewards should be minted"
        );
    });
}
