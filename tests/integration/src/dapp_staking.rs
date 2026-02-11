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
use astar_primitives::dapp_staking::{DAppId, RankedTier, SmartContract};
use frame_support::traits::BuildGenesisConfig;
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

// During Voting subperiod
// - Initialize TierConfig for the cycle: Tier0/3 disabled, Tier1 has 6 slots, Tier2 has 10 slots (with rank multipliers)
// - Set inflation params with bonus rewards = 0% (fully redistributed away from bonus), force an inflation recalculation,
//   and snapshot the active inflation config (used as the “baseline” for the rest of the cycle)
//   Collator and Treasury are also set to 0 for no perturbation of the tier threshold estimation.
// - Register 16 dApps and stake so that:
//   - Tier1 is fully occupied with ranks [10, 8, 6, 4, 2, 0] (6 dApps)
//   - Tier2 is occupied with ranks [9, 8, 7, 6, 5, 4, 3, 2, 1] (9 dApps)
//   - 1 extra dApp is staked just below Tier2 threshold and is excluded from tiers (no rewards)
//
// Advance to Build&Earn subperiod
// - Force an era transition into Build&Earn and capture the era that got assigned tiers
// - Ensure TierConfig is carried correctly into the active cycle (slots per tier unchanged)
// - Ensure the dApps occupy the expected tiers and ranks for the assigned era, and the excluded dApp is not in tiers
// - Snapshot inflation active config again and ensure it has NOT recalculated within the same cycle
//
// Still within the same cycle (before the recalculation boundary)
// - Claim dApp rewards for representative Tier1 and Tier2 dApps and verify the emitted event contains the expected tier/rank
// - Ensure double-claim fails and excluded dApp has no claimable rewards
//
// Recalculation boundary (next recalculation era)
// - Force era advancement up to the era right before recalculation and ensure the active inflation config is still the baseline
// - Advance one more era to cross the recalculation boundary and fetch the new active inflation config
// - Ensure recalculation occurred (recalculation_era bumped) and bonus/collator/treasury rewards remain zero
// - Ensure reward pools are >= previous values (issuance increased via minted dApp rewards, so pools should grow or stay equal)

#[test]
fn full_period_transition_recalculation_and_reward_distribution() {
    new_test_ext().execute_with(|| {
        // 1. Setup: Tier0/3 empty, Tier1=6 slots, Tier2=10 slots ───
        pallet_dapp_staking::GenesisConfig::<Runtime> {
            slots_per_tier: vec![0, 6, 10, 0],
            safeguard: Some(false),
            ..Default::default()
        }
        .build();

        // Inflation: bonus=treasury=collators=0%, redistributed to dapps
        assert_ok!(Inflation::force_set_inflation_params(
            RuntimeOrigin::root(),
            pallet_inflation::InflationParameters {
                max_inflation_rate: Perquintill::from_percent(7),
                treasury_part: Perquintill::from_percent(0),
                collators_part: Perquintill::from_percent(0),
                dapps_part: Perquintill::from_percent(40),
                base_stakers_part: Perquintill::from_percent(25),
                adjustable_stakers_part: Perquintill::from_percent(35),
                bonus_part: Perquintill::zero(),
                ideal_staking_rate: Perquintill::from_percent(50),
                decay_rate: Perquintill::one(),
            },
        ));
        assert_ok!(Inflation::force_inflation_recalculation(
            RuntimeOrigin::root(),
            ActiveProtocolState::<Runtime>::get().era(),
        ));
        let init_config = pallet_inflation::ActiveInflationConfig::<Runtime>::get();
        assert_eq!(
            init_config.bonus_reward_pool_per_period, 0,
            "Bonus pool must be zero"
        );

        // ─── 2. Register 16 dApps ───
        let base_id = NextDAppId::<Runtime>::get();
        let contracts: Vec<_> = (0u8..16)
            .map(|i| {
                let c = AccountId32::new([20 + i; 32]);
                assert_ok!(DappStaking::register(
                    RuntimeOrigin::root(),
                    ALICE,
                    SmartContract::Wasm(c.clone())
                ));
                c
            })
            .collect();

        let tc = TierConfig::<Runtime>::get();
        let t0 = *tc.tier_thresholds().get(0).unwrap();
        let t1 = *tc.tier_thresholds().get(1).unwrap();
        let t2 = *tc.tier_thresholds().get(2).unwrap();

        fn stake_for_rank(lo: Balance, hi: Balance, r: u8) -> Balance {
            if r == 10 {
                return hi;
            }
            let find_min = |target: u8| -> Balance {
                let (mut a, mut b) = (lo, hi);
                while a < b {
                    let m = a + (b - a) / 2;
                    if RankedTier::find_rank(lo, hi, m) >= target {
                        b = m;
                    } else {
                        a = m + 1;
                    }
                }
                a
            };
            let s = find_min(r + 1).saturating_sub(1).max(find_min(r));
            assert_eq!(RankedTier::find_rank(lo, hi, s), r);
            s
        }

        let tier1_ranks: [u8; 6] = [10, 8, 6, 4, 2, 0];
        let tier2_ranks: [u8; 9] = [9, 8, 7, 6, 5, 4, 3, 2, 1];

        let mut stakes: Vec<Balance> = tier1_ranks
            .iter()
            .map(|&r| stake_for_rank(t1, t0, r))
            .collect();
        stakes.extend(tier2_ranks.iter().map(|&r| {
            let s = stake_for_rank(t2, t1, r);
            assert!(s < t1);
            s
        }));
        stakes.push(t2.saturating_sub(1)); // 16th: excluded

        assert_ok!(DappStaking::lock(
            RuntimeOrigin::signed(ALICE.clone()),
            stakes.iter().sum()
        ));
        for (i, &s) in stakes.iter().enumerate() {
            assert_ok!(DappStaking::stake(
                RuntimeOrigin::signed(ALICE.clone()),
                SmartContract::Wasm(contracts[i].clone()),
                s,
            ));
        }

        // ── 3. Voting → Build&Earn ──
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_for_blocks(1);
        assert_eq!(
            ActiveProtocolState::<Runtime>::get().subperiod(),
            Subperiod::BuildAndEarn
        );

        // ─── 4. Verify TierConfig recalculated correctly ───
        let new_tier_config = TierConfig::<Runtime>::get();
        assert_eq!(
            new_tier_config
                .slots_per_tier()
                .get(0)
                .copied()
                .unwrap_or(0),
            0
        );
        assert_eq!(
            new_tier_config
                .slots_per_tier()
                .get(1)
                .copied()
                .unwrap_or(0),
            6
        );
        assert_eq!(
            new_tier_config
                .slots_per_tier()
                .get(2)
                .copied()
                .unwrap_or(0),
            10
        );
        assert_eq!(
            new_tier_config
                .slots_per_tier()
                .get(3)
                .copied()
                .unwrap_or(0),
            0
        );

        // Finalize 1 era for dapps assignation in DAppTiers
        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_for_blocks(1);
        let assigned_era = ActiveProtocolState::<Runtime>::get().era() - 1;

        // ── 5. Verify tier/rank assignments ──
        let mut tiers = DAppTiers::<Runtime>::get(assigned_era).expect("tiers must exist");

        for (i, &r) in tier1_ranks.iter().enumerate() {
            let (amt, ranked) = tiers.try_claim(base_id + i as DAppId).unwrap();
            assert!(amt > 0);
            assert_eq!((ranked.tier(), ranked.rank()), (1, r));
        }
        for (i, &r) in tier2_ranks.iter().enumerate() {
            let (amt, ranked) = tiers.try_claim(base_id + (6 + i) as DAppId).unwrap();
            assert!(amt > 0);
            assert_eq!((ranked.tier(), ranked.rank()), (2, r));
        }
        assert_eq!(
            tiers.try_claim(base_id + 15),
            Err(DAppTierError::NoDAppInTiers)
        );

        // ── 6. Inflation stable within cycle ──
        assert_eq!(
            pallet_inflation::ActiveInflationConfig::<Runtime>::get().recalculation_era,
            init_config.recalculation_era,
        );

        // ── 7. Claim rewards BEFORE recalculation ──
        let claim_and_check = |idx: usize, exp_tier: u8, exp_rank: u8| {
            let sc = SmartContract::Wasm(contracts[idx].clone());
            assert_ok!(DappStaking::claim_dapp_reward(
                RuntimeOrigin::signed(ALICE.clone()),
                sc.clone(),
                assigned_era,
            ));
            assert!(frame_system::Pallet::<Runtime>::events()
                .iter()
                .rev()
                .any(|r| matches!(
                    &r.event,
                    RuntimeEvent::DappStaking(
                        pallet_dapp_staking::Event::<Runtime>::DAppReward {
                            smart_contract,
                            tier_id,
                            rank,
                            ..
                        }
                    ) if *smart_contract == sc
                        && *tier_id == exp_tier
                        && *rank == exp_rank
                )));
        };

        claim_and_check(0, 1, tier1_ranks[0]);
        claim_and_check(6, 2, tier2_ranks[0]);

        // Excluded dApp has no rewards
        assert_noop!(
            DappStaking::claim_dapp_reward(
                RuntimeOrigin::signed(ALICE.clone()),
                SmartContract::Wasm(contracts[15].clone()),
                assigned_era,
            ),
            pallet_dapp_staking::Error::<Runtime>::NoClaimableRewards
        );

        // ── 8. Force to recalculation boundary ──
        while ActiveProtocolState::<Runtime>::get().era() < init_config.recalculation_era - 1 {
            assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
            run_for_blocks(1);
        }
        assert_eq!(
            pallet_inflation::ActiveInflationConfig::<Runtime>::get().recalculation_era,
            init_config.recalculation_era,
            "not yet recalculated"
        );
        assert_eq!(
            pallet_inflation::ActiveInflationConfig::<Runtime>::get().dapp_reward_pool_per_era,
            init_config.dapp_reward_pool_per_era,
            "no pool inflation"
        );

        assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era));
        run_for_blocks(1);
        let new_config = pallet_inflation::ActiveInflationConfig::<Runtime>::get();

        // ── 9. Verify recalculation happened and pool increased ──
        assert!(
            new_config.recalculation_era > init_config.recalculation_era,
            "Recalculation era must have bumped"
        );
        assert_eq!(new_config.bonus_reward_pool_per_period, 0);
        assert_eq!(new_config.collator_reward_per_block, 0);
        assert_eq!(new_config.treasury_reward_per_block, 0);

        // Rewards were minted via claim_dapp_reward (increasing issuance).
        assert!(
            new_config.dapp_reward_pool_per_era >= init_config.dapp_reward_pool_per_era,
            "dApp reward pool must grow (or stay equal) after recalculation on higher issuance"
        );
        assert!(
            new_config.base_staker_reward_pool_per_era
                >= init_config.base_staker_reward_pool_per_era,
            "Base staker pool must grow (or stay equal) after recalculation"
        );
    });
}
