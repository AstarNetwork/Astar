//! Tests for the dapps-staking module.

#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok, traits::OnFinalize};
use pallet_balances::{BalanceLock, Reasons};
use pallet_plasm_rewards::traits::ComputeTotalPayout;
use sp_runtime::DispatchError;

#[test]
fn bond_scenario_test() {
    new_test_ext().execute_with(|| {
        // bond ALICE -> BOB
        assert_ok!(DappsStaking::bond(
            Origin::signed(ALICE_STASH),
            ALICE_CTRL,
            1000,
            RewardDestination::Stash,
        ));
        assert_eq!(DappsStaking::bonded(ALICE_STASH), Some(ALICE_CTRL));
        assert_eq!(DappsStaking::bonded(ALICE_CTRL), None);
        assert_eq!(DappsStaking::payee(ALICE_STASH), RewardDestination::Stash);
        assert_eq!(
            DappsStaking::ledger(ALICE_CTRL),
            Some(StakingLedger {
                stash: ALICE_STASH,
                total: 1000,
                active: 1000,
                unlocking: vec![],
                last_reward: Some(0),
            })
        );
        assert_eq!(DappsStaking::ledger(ALICE_STASH), None);
        assert_eq!(
            Balances::locks(ALICE_STASH),
            vec![BalanceLock {
                id: STAKING_ID,
                amount: 1000,
                reasons: Reasons::All,
            },]
        )
    })
}

#[test]
fn bond_failed_test() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            DappsStaking::bond(
                Origin::signed(ALICE_STASH),
                ALICE_CTRL,
                9,
                RewardDestination::Stash,
            ),
            Err(DispatchError::Other(
                "can not bond with value less than minimum balance"
            ))
        );

        success_first_bond(ALICE_STASH, ALICE_CTRL, 10, RewardDestination::Stash);

        assert_eq!(
            DappsStaking::bond(
                Origin::signed(ALICE_STASH),
                ALICE_CTRL,
                100,
                RewardDestination::Stash,
            ),
            Err(DispatchError::Other("stash already bonded"))
        );

        assert_eq!(
            DappsStaking::bond(
                Origin::signed(BOB_STASH),
                ALICE_CTRL,
                100,
                RewardDestination::Stash,
            ),
            Err(DispatchError::Other("controller already paired"))
        );
    });
}

fn success_first_bond(
    stash: AccountId,
    ctrl: AccountId,
    balance: Balance,
    dest: RewardDestination<AccountId>,
) {
    // bond ALICE -> BOB
    assert_ok!(DappsStaking::bond(
        Origin::signed(stash),
        ctrl,
        balance,
        dest,
    ));
    assert_eq!(DappsStaking::bonded(stash), Some(ctrl));
    assert_eq!(DappsStaking::payee(stash), dest);
    assert_eq!(
        DappsStaking::ledger(ctrl),
        Some(StakingLedger {
            stash: stash,
            total: balance,
            active: balance,
            unlocking: vec![],
            last_reward: Some(0),
        })
    );
    assert_eq!(
        Balances::locks(stash),
        vec![BalanceLock {
            id: STAKING_ID,
            amount: balance,
            reasons: Reasons::All,
        },]
    )
}

#[test]
fn bond_extra_scenario_test() {
    new_test_ext().execute_with(|| {
        // success first bond BOB_STASH -> BOB_CTRL
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);

        assert_ok!(DappsStaking::bond_extra(Origin::signed(BOB_STASH), 1000));
        assert_eq!(DappsStaking::bonded(BOB_STASH), Some(BOB_CTRL));
        assert_eq!(DappsStaking::payee(BOB_STASH), RewardDestination::Stash);
        assert_eq!(
            DappsStaking::ledger(BOB_CTRL),
            Some(StakingLedger {
                stash: BOB_STASH,
                total: 2000,
                active: 2000,
                unlocking: vec![],
                last_reward: Some(0),
            })
        );
        assert_eq!(
            Balances::locks(BOB_STASH),
            vec![BalanceLock {
                id: STAKING_ID,
                amount: 2000,
                reasons: Reasons::All,
            },]
        );
    })
}

#[test]
fn bond_extra_failed_test() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            DappsStaking::bond_extra(Origin::signed(BOB_STASH), 1000),
            Error::<Test>::NotStash,
        );
        <Bonded<Test>>::insert(BOB_STASH, BOB_CTRL);
        assert_noop!(
            DappsStaking::bond_extra(Origin::signed(BOB_STASH), 1000),
            Error::<Test>::NotController,
        );
    })
}

#[test]
fn unbond_scenario_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);

        assert_ok!(DappsStaking::unbond(Origin::signed(BOB_CTRL), 300));
        assert_eq!(
            DappsStaking::ledger(BOB_CTRL),
            Some(StakingLedger {
                stash: BOB_STASH,
                total: 1000,
                active: 700,
                unlocking: vec![UnlockChunk {
                    value: 300,
                    era: 3, // current_era(0) + bonding_duration(3)
                }],
                last_reward: Some(0),
            })
        );
        assert_eq!(
            Balances::locks(BOB_STASH),
            vec![BalanceLock {
                id: STAKING_ID,
                amount: 1000,
                reasons: Reasons::All,
            },]
        );

        advance_era();

        assert_ok!(DappsStaking::unbond(Origin::signed(BOB_CTRL), 200));
        assert_eq!(
            DappsStaking::ledger(BOB_CTRL),
            Some(StakingLedger {
                stash: BOB_STASH,
                total: 1000,
                active: 500,
                unlocking: vec![
                    UnlockChunk {
                        value: 300,
                        era: 3, // current_era(0) + bonding_duration(3)
                    },
                    UnlockChunk {
                        value: 200,
                        era: 4, // current_era(1) + bonding_duration(3)
                    }
                ],
                last_reward: Some(0),
            })
        );
        assert_eq!(
            Balances::locks(BOB_STASH),
            vec![BalanceLock {
                id: STAKING_ID,
                amount: 1000,
                reasons: Reasons::All,
            },]
        );
    })
}

fn success_unbond(ctrl: AccountId, balance: Balance) {
    let now_ledger = DappsStaking::ledger(ctrl).unwrap();
    let now_unlock_chunk = now_ledger.unlocking;
    let now_len = now_unlock_chunk.len();
    let current_era = PlasmRewards::current_era().unwrap();

    assert_ok!(DappsStaking::unbond(Origin::signed(ctrl), balance));

    let after_ledger = DappsStaking::ledger(ctrl).unwrap();
    let after_unlock_chunks = after_ledger.unlocking;
    assert_eq!(now_unlock_chunk, after_unlock_chunks.split_at(now_len).0);
    assert_eq!(
        after_unlock_chunks[now_len],
        UnlockChunk {
            value: balance,
            era: current_era + 3, // current_era(0) + bonding_duration(3)
        }
    );
    assert_eq!(now_ledger.total, after_ledger.total);
    assert_eq!(now_ledger.active, after_ledger.active + balance);
}

#[test]
fn unbond_failed_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);
        assert_noop!(
            DappsStaking::unbond(Origin::signed(BOB_STASH), 300),
            Error::<Test>::NotController,
        );
        for _ in 0..32 {
            success_unbond(BOB_CTRL, 10);
        }
        assert_noop!(
            DappsStaking::unbond(Origin::signed(BOB_CTRL), 300),
            Error::<Test>::NoMoreChunks,
        );
    })
}

#[test]
fn withdraw_unbonded_scenario_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);
        success_unbond(BOB_CTRL, 300);

        // era 0 -> 1
        advance_era();

        success_unbond(BOB_CTRL, 700);

        // era 1 -> 2
        advance_era();

        assert_ok!(DappsStaking::withdraw_unbonded(Origin::signed(BOB_CTRL)));
        assert_eq!(
            DappsStaking::ledger(BOB_CTRL),
            Some(StakingLedger {
                stash: BOB_STASH,
                total: 1000,
                active: 0,
                unlocking: vec![
                    UnlockChunk { value: 300, era: 3 },
                    UnlockChunk { value: 700, era: 4 },
                ],
                last_reward: Some(0),
            })
        );
        assert_eq!(
            Balances::locks(BOB_STASH),
            vec![BalanceLock {
                id: STAKING_ID,
                amount: 1000,
                reasons: Reasons::All,
            },]
        );

        // era 2 -> 3
        advance_era();

        assert_ok!(DappsStaking::withdraw_unbonded(Origin::signed(BOB_CTRL)));
        assert_eq!(
            DappsStaking::ledger(BOB_CTRL),
            Some(StakingLedger {
                stash: BOB_STASH,
                total: 700,
                active: 0,
                unlocking: vec![UnlockChunk { value: 700, era: 4 },],
                last_reward: Some(0),
            })
        );
        assert_eq!(
            Balances::locks(BOB_STASH),
            vec![BalanceLock {
                id: STAKING_ID,
                amount: 700,
                reasons: Reasons::All,
            },]
        );

        // era 3 -> 4
        advance_era();

        assert_ok!(DappsStaking::withdraw_unbonded(Origin::signed(BOB_CTRL)));
        assert_eq!(DappsStaking::ledger(BOB_CTRL), None);
        assert_eq!(Balances::locks(BOB_STASH), vec![]);
    })
}

#[test]
fn withdraw_unbonded_failed_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);
        success_unbond(BOB_CTRL, 300);
        assert_noop!(
            DappsStaking::withdraw_unbonded(Origin::signed(BOB_STASH)),
            Error::<Test>::NotController,
        );
    })
}

#[test]
fn nominate_contracts_scenario_test() {
    new_test_ext().execute_with(|| {
        valid_instatiate();
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);
        assert_ok!(DappsStaking::nominate_contracts(
            Origin::signed(BOB_CTRL),
            vec![(OPERATED_CONTRACT_A, 1000)]
        ));
        assert_eq!(
            DappsStaking::dapps_nominations(BOB_STASH),
            Some(Nominations {
                targets: vec![(OPERATED_CONTRACT_A, 1000)],
                submitted_in: 0,
                suppressed: false,
            })
        );
    })
}

fn success_nominate_contracts(ctrl: AccountId, targets: Vec<(AccountId, Balance)>) {
    assert_ok!(DappsStaking::nominate_contracts(
        Origin::signed(ctrl),
        targets.clone()
    ));
    let stash = DappsStaking::ledger(&ctrl).unwrap().stash;
    let current_era = PlasmRewards::current_era().unwrap();
    assert_eq!(
        DappsStaking::dapps_nominations(stash),
        Some(Nominations {
            targets: targets,
            submitted_in: current_era,
            suppressed: false,
        })
    );
}

#[test]
fn nominate_contracts_failed_test() {
    new_test_ext().execute_with(|| {
        valid_instatiate();
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);
        assert_noop!(
            DappsStaking::nominate_contracts(
                Origin::signed(BOB_STASH),
                vec![(OPERATED_CONTRACT_A, 1_000)]
            ),
            Error::<Test>::NotController,
        );
        assert_noop!(
            DappsStaking::nominate_contracts(Origin::signed(BOB_CTRL), vec![]),
            Error::<Test>::EmptyNominateTargets,
        );
        assert_noop!(
            DappsStaking::nominate_contracts(Origin::signed(BOB_CTRL), vec![(BOB_CONTRACT, 1_000)]),
            Error::<Test>::NotOperatedContracts,
        );
        assert_noop!(
            DappsStaking::nominate_contracts(
                Origin::signed(BOB_CTRL),
                vec![(OPERATED_CONTRACT_A, 5_000)]
            ),
            Error::<Test>::NotEnoughStaking,
        );
    })
}

#[test]
fn chill_scenario_test() {
    new_test_ext().execute_with(|| {
        valid_instatiate();
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);
        success_nominate_contracts(BOB_CTRL, vec![(OPERATED_CONTRACT_A, 1000)]);
        assert_ok!(DappsStaking::chill(Origin::signed(BOB_CTRL)));
        assert_eq!(DappsStaking::dapps_nominations(BOB_STASH), None);
    })
}

#[test]
fn chill_failed_test() {
    new_test_ext().execute_with(|| {
        valid_instatiate();
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);
        success_nominate_contracts(BOB_CTRL, vec![(OPERATED_CONTRACT_A, 1000)]);
        assert_noop!(
            DappsStaking::chill(Origin::signed(BOB_STASH)),
            Error::<Test>::NotController,
        );
    })
}

#[test]
fn set_payee_scenario_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(ALICE_STASH, ALICE_CTRL, 1000, RewardDestination::Stash);
        assert_ok!(DappsStaking::set_payee(
            Origin::signed(ALICE_CTRL),
            RewardDestination::Controller
        ));
        assert_eq!(
            DappsStaking::payee(ALICE_STASH),
            RewardDestination::Controller
        );
    })
}

#[test]
fn set_payee_failed_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(ALICE_STASH, ALICE_CTRL, 1000, RewardDestination::Stash);
        assert_noop!(
            DappsStaking::set_payee(Origin::signed(ALICE_STASH), RewardDestination::Controller),
            Error::<Test>::NotController,
        );
    })
}

#[test]
fn set_controller_scenario_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(ALICE_STASH, ALICE_CTRL, 1000, RewardDestination::Stash);
        assert_ok!(DappsStaking::set_controller(
            Origin::signed(ALICE_STASH),
            BOB_CTRL
        ));
        assert_eq!(DappsStaking::bonded(ALICE_STASH), Some(BOB_CTRL));
        assert_eq!(
            DappsStaking::ledger(BOB_CTRL),
            Some(StakingLedger {
                stash: ALICE_STASH,
                total: 1000,
                active: 1000,
                unlocking: vec![],
                last_reward: Some(0),
            })
        );
        assert_eq!(DappsStaking::ledger(ALICE_CTRL), None);
    })
}

#[test]
fn set_controller_failed_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(ALICE_STASH, ALICE_CTRL, 1000, RewardDestination::Stash);
        assert_noop!(
            DappsStaking::set_controller(Origin::signed(ALICE_CTRL), BOB_CTRL),
            Error::<Test>::NotStash,
        );
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);
        assert_noop!(
            DappsStaking::set_controller(Origin::signed(ALICE_STASH), BOB_CTRL),
            "controller already paired",
        );
    })
}

const SIX_HOURS: u64 = 6 * 60 * 60 * 1000;

#[test]
fn reward_to_operators_test() {
    new_test_ext().execute_with(|| {
        valid_instatiate();
        assert_ok!(Operator::change_operator(
            Origin::signed(OPERATOR_A),
            vec![OPERATED_CONTRACT_A],
            ALICE_STASH
        ));
        success_first_bond(BOB_STASH, BOB_CTRL, 1_000, RewardDestination::Stash);
        success_first_bond(
            ALICE_STASH,
            ALICE_CTRL,
            1_000,
            RewardDestination::Controller,
        );
        success_nominate_contracts(BOB_CTRL, vec![(OPERATED_CONTRACT_A, 1_000)]);
        success_nominate_contracts(ALICE_CTRL, vec![(OPERATED_CONTRACT_A, 1_000)]);
        success_nominate_contracts(ALICE_CTRL, vec![(OPERATED_CONTRACT_B, 1_000)]);

        let current_era = PlasmRewards::current_era().unwrap();
        assert_eq!(DappsStaking::eras_total_stake(current_era), 0);
        assert_eq!(DappsStaking::eras_total_stake(current_era + 1), 3_000);

        advance_era();

        let pre_total_issuarance = Balances::total_issuance();
        let (_, b) = <Test as pallet_plasm_rewards::Trait>::ComputeTotalPayout::compute(
            pre_total_issuarance,
            SIX_HOURS,
            0,
            0,
        );

        advance_session();

        let current_era = PlasmRewards::current_era().unwrap();
        ErasVotes::<Test>::insert(
            current_era - 1,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 3, good: 12 },
        );
        ErasVotes::<Test>::insert(
            current_era,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 3, good: 12 },
        );
        ErasVotes::<Test>::insert(
            current_era - 1,
            OPERATED_CONTRACT_B,
            VoteCounts { bad: 3, good: 12 },
        );
        ErasVotes::<Test>::insert(
            current_era,
            OPERATED_CONTRACT_B,
            VoteCounts { bad: 3, good: 12 },
        );
        let positive_imbalance = DappsStaking::reward_nominator(&current_era, b, &BOB_STASH);
        assert_eq!(Balances::free_balance(&BOB_STASH), 2_000 + 274); // +nomiante reward
        assert_eq!(Balances::free_balance(&BOB_CTRL), 20 + 0); // +0
        assert_eq!(positive_imbalance, 274);
        assert_eq!(Balances::total_issuance(), pre_total_issuarance + 274);

        let positive_imbalance = DappsStaking::reward_operator(&current_era, b, &ALICE_STASH);
        assert_eq!(Balances::free_balance(&ALICE_STASH), 1_000 + 183); // +operator reward
        assert_eq!(positive_imbalance, 183);
        assert_eq!(Balances::total_issuance(), pre_total_issuarance + 457);

        let positive_imbalance = DappsStaking::reward_nominator(&current_era, b, &ALICE_STASH);
        assert_eq!(Balances::free_balance(&ALICE_CTRL), 10 + 274); // +nominate reward
        assert_eq!(positive_imbalance, 274);
        assert_eq!(Balances::total_issuance(), pre_total_issuarance + 731);
    })
}

#[test]
fn new_session_scenario_test() {
    new_test_ext().execute_with(|| {
        advance_session();
        valid_instatiate();
        assert_ok!(Operator::change_operator(
            Origin::signed(OPERATOR_A),
            vec![OPERATED_CONTRACT_A],
            ALICE_STASH
        ));
        success_first_bond(BOB_STASH, BOB_CTRL, 1_000, RewardDestination::Stash);
        success_first_bond(
            ALICE_STASH,
            ALICE_CTRL,
            1_000,
            RewardDestination::Controller,
        );
        success_nominate_contracts(BOB_CTRL, vec![(OPERATED_CONTRACT_A, 1_000)]);
        success_nominate_contracts(ALICE_CTRL, vec![(OPERATED_CONTRACT_A, 1_000)]);
        success_nominate_contracts(ALICE_CTRL, vec![(OPERATED_CONTRACT_B, 1_000)]);

        let current_era = PlasmRewards::current_era().unwrap();
        assert_eq!(DappsStaking::eras_total_stake(current_era), 0);
        assert_eq!(DappsStaking::eras_total_stake(current_era + 1), 3_000);
        let target_era = current_era + 1;

        advance_era();
        DappsStaking::on_finalize(0);
        advance_session();

        let pre_total_issuarance = Balances::total_issuance();
        assert_eq!(Balances::free_balance(&BOB_STASH), 2_000);
        assert_eq!(Balances::free_balance(&BOB_CTRL), 20);
        assert_eq!(Balances::free_balance(&ALICE_STASH), 1_000);
        assert_eq!(Balances::free_balance(&ALICE_CTRL), 10);
        assert_eq!(pre_total_issuarance, 4_003_030);

        advance_era();
        DappsStaking::on_finalize(0);
        advance_session();

        ErasVotes::<Test>::insert(
            target_era - 1,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 3, good: 12 },
        );
        ErasVotes::<Test>::insert(
            target_era,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 3, good: 12 },
        );
        ErasVotes::<Test>::insert(
            target_era - 1,
            OPERATED_CONTRACT_B,
            VoteCounts { bad: 3, good: 12 },
        );
        ErasVotes::<Test>::insert(
            target_era,
            OPERATED_CONTRACT_B,
            VoteCounts { bad: 3, good: 12 },
        );

        assert_ok!(DappsStaking::claim_for_nominator(
            Origin::signed(BOB_STASH),
            target_era
        ));

        assert_eq!(Balances::free_balance(&BOB_STASH), 2_000 + 8); // +nomiante reward
        assert_eq!(Balances::free_balance(&BOB_CTRL), 20 + 0); // +0

        assert_ok!(DappsStaking::claim_for_operator(
            Origin::signed(ALICE_STASH),
            target_era
        ));
        assert_ok!(DappsStaking::claim_for_nominator(
            Origin::signed(ALICE_STASH),
            target_era
        ));

        assert_eq!(Balances::free_balance(&ALICE_STASH), 1_000 + 5); // +operator reward
        assert_eq!(Balances::free_balance(&ALICE_CTRL), 10 + 8); // +nominate reward
        assert_eq!(Balances::total_issuance(), 4_003_030 + 21);
    })
}

#[test]
fn ignore_nomination_test() {
    new_test_ext().execute_with(|| {
        advance_session();
        valid_instatiate();
        assert_ok!(Operator::change_operator(
            Origin::signed(OPERATOR_A),
            vec![OPERATED_CONTRACT_A],
            ALICE_STASH
        ));
        success_first_bond(BOB_STASH, BOB_CTRL, 1_000, RewardDestination::Stash);
        success_first_bond(
            ALICE_STASH,
            ALICE_CTRL,
            1_000,
            RewardDestination::Controller,
        );
        success_nominate_contracts(BOB_CTRL, vec![(OPERATED_CONTRACT_A, 1_000)]);
        success_nominate_contracts(ALICE_CTRL, vec![(OPERATED_CONTRACT_A, 1_000)]);
        success_nominate_contracts(ALICE_CTRL, vec![(OPERATED_CONTRACT_B, 1)]);

        let current_era = PlasmRewards::current_era().unwrap();
        assert_eq!(DappsStaking::eras_total_stake(current_era), 0);
        assert_eq!(DappsStaking::eras_total_stake(current_era + 1), 2_001);
        let target_era = current_era + 1;

        advance_era();
        DappsStaking::on_finalize(0);
        advance_session();

        let pre_total_issuarance = Balances::total_issuance();
        assert_eq!(Balances::free_balance(&BOB_STASH), 2_000);
        assert_eq!(Balances::free_balance(&BOB_CTRL), 20);
        assert_eq!(Balances::free_balance(&ALICE_STASH), 1_000);
        assert_eq!(Balances::free_balance(&ALICE_CTRL), 10);
        assert_eq!(pre_total_issuarance, 4_003_030);

        advance_era();
        DappsStaking::on_finalize(0);
        advance_session();

        ErasVotes::<Test>::insert(
            target_era - 1,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 3, good: 12 },
        );
        ErasVotes::<Test>::insert(
            target_era,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 3, good: 12 },
        );
        ErasVotes::<Test>::insert(
            target_era - 1,
            OPERATED_CONTRACT_B,
            VoteCounts { bad: 3, good: 12 },
        );
        ErasVotes::<Test>::insert(
            target_era,
            OPERATED_CONTRACT_B,
            VoteCounts { bad: 3, good: 12 },
        );

        assert_ok!(DappsStaking::claim_for_nominator(
            Origin::signed(BOB_STASH),
            target_era
        ));

        assert_eq!(Balances::free_balance(&BOB_STASH), 2_000 + 8); // +nomiante reward
        assert_eq!(Balances::free_balance(&BOB_CTRL), 20 + 0); // +0

        assert_ok!(DappsStaking::claim_for_operator(
            Origin::signed(ALICE_STASH),
            target_era
        ));
        assert_ok!(DappsStaking::claim_for_nominator(
            Origin::signed(ALICE_STASH),
            target_era
        ));

        assert_eq!(Balances::free_balance(&ALICE_STASH), 1_000 + 7); // +operator reward
        assert_eq!(Balances::free_balance(&ALICE_CTRL), 10 + 8); // +nominate reward
        assert_eq!(Balances::total_issuance(), 4_003_030 + 23);
    })
}

#[test]
fn vote_contracts_test() {
    new_test_ext().execute_with(|| {
        advance_session();
        valid_instatiate();
        let current_era = PlasmRewards::current_era().unwrap();

        assert_ok!(DappsStaking::vote_contracts(
            Origin::signed(ALICE_CTRL),
            vec![(OPERATED_CONTRACT_A, Vote::Good)]
        ));
        assert_eq!(
            DappsStaking::accounts_vote(ALICE_CTRL, OPERATED_CONTRACT_A),
            VoteCounts { bad: 0, good: 1 }
        );
        assert_eq!(
            DappsStaking::eras_votes(current_era + 1, OPERATED_CONTRACT_A),
            VoteCounts { bad: 0, good: 1 }
        );

        assert_ok!(DappsStaking::vote_contracts(
            Origin::signed(ALICE_CTRL),
            vec![(OPERATED_CONTRACT_A, Vote::Bad)]
        ));
        assert_eq!(
            DappsStaking::accounts_vote(ALICE_CTRL, OPERATED_CONTRACT_A),
            VoteCounts { bad: 1, good: 0 }
        );
        assert_eq!(
            DappsStaking::eras_votes(current_era + 1, OPERATED_CONTRACT_A),
            VoteCounts { bad: 1, good: 0 }
        );

        assert_ok!(DappsStaking::vote_contracts(
            Origin::signed(BOB_CTRL),
            vec![(OPERATED_CONTRACT_A, Vote::Good)]
        ));
        assert_eq!(
            DappsStaking::accounts_vote(BOB_CTRL, OPERATED_CONTRACT_A),
            VoteCounts { bad: 0, good: 1 }
        );
        assert_eq!(
            DappsStaking::eras_votes(current_era + 1, OPERATED_CONTRACT_A),
            VoteCounts { bad: 1, good: 1 }
        );

        assert_eq!(
            DappsStaking::has_votes_requirement(&OPERATED_CONTRACT_A, &(current_era + 1)),
            false
        );
        assert_eq!(
            DappsStaking::has_votes_requirement(&OPERATED_CONTRACT_B, &(current_era + 1)),
            false
        );

        ErasVotes::<Test>::insert(
            current_era + 1,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 3, good: 12 },
        );
        assert_eq!(
            DappsStaking::has_votes_requirement(&OPERATED_CONTRACT_A, &(current_era + 1)),
            true
        );
    })
}

#[test]
fn is_rewardable_test() {
    new_test_ext().execute_with(|| {
        let current_era = PlasmRewards::current_era().unwrap();

        ErasVotes::<Test>::insert(
            current_era + 1,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 3, good: 12 },
        );

        assert_eq!(
            DappsStaking::is_rewardable(&OPERATED_CONTRACT_A, &current_era),
            false
        );
        assert_eq!(
            DappsStaking::is_rewardable(&OPERATED_CONTRACT_A, &(current_era + 1)),
            false
        );
        assert_eq!(
            DappsStaking::is_rewardable(&OPERATED_CONTRACT_A, &(current_era + 2)),
            false
        );

        ErasVotes::<Test>::insert(
            current_era + 2,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 3, good: 12 },
        );

        assert_eq!(
            DappsStaking::is_rewardable(&OPERATED_CONTRACT_A, &(current_era + 1)),
            false
        );
        assert_eq!(
            DappsStaking::is_rewardable(&OPERATED_CONTRACT_A, &(current_era + 2)),
            true
        );
    })
}

#[test]
fn is_locked_test() {
    new_test_ext().execute_with(|| {
        let current_era = PlasmRewards::current_era().unwrap();

        ErasVotes::<Test>::insert(
            current_era + 1,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 3, good: 12 },
        );

        assert_eq!(
            DappsStaking::is_locked(&OPERATED_CONTRACT_A, &current_era),
            false
        );
        assert_eq!(
            DappsStaking::is_locked(&OPERATED_CONTRACT_A, &(current_era + 1)),
            false
        );
        assert_eq!(
            DappsStaking::is_locked(&OPERATED_CONTRACT_A, &(current_era + 2)),
            false
        );

        ErasVotes::<Test>::insert(
            current_era + 2,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 3, good: 12 },
        );

        assert_eq!(
            DappsStaking::is_locked(&OPERATED_CONTRACT_A, &(current_era + 1)),
            false
        );
        assert_eq!(
            DappsStaking::is_locked(&OPERATED_CONTRACT_A, &(current_era + 2)),
            false
        );

        ErasVotes::<Test>::insert(
            current_era + 1,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 5, good: 9 },
        );

        assert_eq!(
            DappsStaking::is_locked(&OPERATED_CONTRACT_A, &(current_era + 1)),
            false
        );
        assert_eq!(
            DappsStaking::is_locked(&OPERATED_CONTRACT_A, &(current_era + 2)),
            true
        );
    })
}

#[test]
fn is_slashable_test() {
    new_test_ext().execute_with(|| {
        let current_era = PlasmRewards::current_era().unwrap();

        ErasVotes::<Test>::insert(
            current_era + 1,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 3, good: 12 },
        );

        assert_eq!(
            DappsStaking::is_slashable(&OPERATED_CONTRACT_A, &current_era),
            false
        );
        assert_eq!(
            DappsStaking::is_slashable(&OPERATED_CONTRACT_A, &(current_era + 1)),
            false
        );
        assert_eq!(
            DappsStaking::is_slashable(&OPERATED_CONTRACT_A, &(current_era + 2)),
            false
        );

        ErasVotes::<Test>::insert(
            current_era + 2,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 3, good: 12 },
        );

        assert_eq!(
            DappsStaking::is_slashable(&OPERATED_CONTRACT_A, &(current_era + 1)),
            false
        );
        assert_eq!(
            DappsStaking::is_slashable(&OPERATED_CONTRACT_A, &(current_era + 2)),
            false
        );

        ErasVotes::<Test>::insert(
            current_era + 1,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 10, good: 9 },
        );
        ErasVotes::<Test>::insert(
            current_era + 2,
            OPERATED_CONTRACT_A,
            VoteCounts { bad: 10, good: 9 },
        );

        assert_eq!(
            DappsStaking::is_slashable(&OPERATED_CONTRACT_A, &(current_era + 1)),
            false
        );
        assert_eq!(
            DappsStaking::is_slashable(&OPERATED_CONTRACT_A, &(current_era + 2)),
            true
        );
    })
}
