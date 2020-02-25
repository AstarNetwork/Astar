//! Tests for the plasm-staking module.

#![cfg(test)]

use super::*;
use crate::mock::*;
use balances::BalanceLock;
use sp_runtime::DispatchError;
use support::assert_ok;

#[test]
fn root_calls_fails_for_user() {
    new_test_ext().execute_with(|| {
        let res = PlasmStaking::force_no_eras(Origin::signed(0));
        assert_eq!(res, Err(DispatchError::BadOrigin));

        let res = PlasmStaking::force_new_era(Origin::signed(0));
        assert_eq!(res, Err(DispatchError::BadOrigin));

        let res = PlasmStaking::force_new_era_always(Origin::signed(0));
        assert_eq!(res, Err(DispatchError::BadOrigin));

        let res = PlasmStaking::set_validators(Origin::signed(0), vec![]);
        assert_eq!(res, Err(DispatchError::BadOrigin));
    })
}

#[test]
fn set_validators_works_for_root() {
    new_test_ext().execute_with(|| {
        advance_session();
        assert_eq!(Session::current_index(), 1);
        assert_eq!(Session::validators(), vec![1, 2]);

        assert_ok!(PlasmStaking::set_validators(Origin::ROOT, vec![1, 2, 3]));
        for i in 1..10 {
            assert_eq!(Session::current_index(), i);
            assert_eq!(Session::validators(), vec![1, 2]);
            advance_session();
        }

        advance_session();
        assert_eq!(Session::validators(), vec![1, 2, 3]);

        for i in 11..25 {
            assert_eq!(Session::current_index(), i);
            assert_eq!(Session::validators(), vec![1, 2, 3]);
            advance_session();
        }

        assert_ok!(PlasmStaking::set_validators(Origin::ROOT, vec![1, 2]));
        assert_eq!(PlasmStaking::validators(), vec![1, 2]);

        for i in 25..30 {
            assert_eq!(Session::current_index(), i);
            assert_eq!(Session::validators(), vec![1, 2, 3]);
            advance_session();
        }

        advance_session();
        assert_eq!(Session::current_index(), 31);
        assert_eq!(Session::validators(), vec![1, 2]);
    })
}

#[test]
fn noraml_incremental_era() {
    new_test_ext().execute_with(|| {
        assert_eq!(PlasmStaking::current_era(), 0);
        assert_eq!(PlasmStaking::current_era_start(), 0);
        assert_eq!(PlasmStaking::current_era_start_session_index(), 0);
        assert_eq!(PlasmStaking::force_era(), Forcing::NotForcing);
        assert_eq!(PlasmStaking::storage_version(), 1);
        assert_eq!(Session::validators(), vec![1, 2]);
        assert_eq!(Session::current_index(), 0);

        advance_session();

        assert_eq!(PlasmStaking::current_era(), 0);
        assert_eq!(PlasmStaking::current_era_start(), 0);
        assert_eq!(PlasmStaking::current_era_start_session_index(), 0);
        assert_eq!(PlasmStaking::force_era(), Forcing::NotForcing);
        assert_eq!(PlasmStaking::storage_version(), 1);
        assert_eq!(Session::validators(), vec![1, 2]);
        assert_eq!(Session::current_index(), 1);

        assert_ok!(PlasmStaking::set_validators(
            Origin::ROOT,
            vec![1, 2, 3, 4, 5]
        ));

        assert_eq!(Session::validators(), vec![1, 2]);
        assert_eq!(Session::current_index(), 1);

        // 2~9-th session
        for i in 2..10 {
            advance_session();
            assert_eq!(PlasmStaking::current_era(), 0);
            assert_eq!(PlasmStaking::current_era_start(), 0);
            assert_eq!(PlasmStaking::current_era_start_session_index(), 0);
            assert_eq!(PlasmStaking::force_era(), Forcing::NotForcing);
            assert_eq!(PlasmStaking::storage_version(), 1);
            assert_eq!(Session::validators(), vec![1, 2]);
            assert_eq!(Session::current_index(), i);
        }

        // 10~19-th session
        for i in 10..20 {
            advance_session();
            assert_eq!(PlasmStaking::current_era(), 1);
            assert_eq!(PlasmStaking::current_era_start(), 10 * PER_SESSION);
            assert_eq!(PlasmStaking::current_era_start_session_index(), 10);
            assert_eq!(PlasmStaking::force_era(), Forcing::NotForcing);
            assert_eq!(PlasmStaking::storage_version(), 1);
            assert_eq!(Session::current_index(), i);
            match i {
                10 => assert_eq!(Session::validators(), vec![1, 2]),
                _ => assert_eq!(Session::validators(), vec![1, 2, 3, 4, 5]),
            }
        }

        assert_ok!(PlasmStaking::set_validators(Origin::ROOT, vec![1, 3, 5]));

        // 20~29-th session
        for i in 20..30 {
            advance_session();
            assert_eq!(PlasmStaking::current_era(), 2);
            assert_eq!(PlasmStaking::current_era_start(), 20 * PER_SESSION);
            assert_eq!(PlasmStaking::current_era_start_session_index(), 20);
            assert_eq!(PlasmStaking::force_era(), Forcing::NotForcing);
            assert_eq!(PlasmStaking::storage_version(), 1);
            assert_eq!(Session::current_index(), i);
            match i {
                20 => assert_eq!(Session::validators(), vec![1, 2, 3, 4, 5]),
                _ => assert_eq!(Session::validators(), vec![1, 3, 5]),
            }
        }
    })
}

#[test]
fn force_new_era_incremental_era() {
    new_test_ext().execute_with(|| {
        assert_eq!(PlasmStaking::force_era(), Forcing::NotForcing);
        assert_ok!(PlasmStaking::force_new_era(Origin::ROOT));
        assert_eq!(PlasmStaking::force_era(), Forcing::ForceNew);

        assert_ok!(PlasmStaking::set_validators(
            Origin::ROOT,
            vec![1, 2, 3, 4, 5]
        ));

        advance_session();
        assert_eq!(PlasmStaking::current_era(), 1);
        assert_eq!(PlasmStaking::current_era_start(), PER_SESSION);
        assert_eq!(PlasmStaking::current_era_start_session_index(), 1);
        assert_eq!(PlasmStaking::force_era(), Forcing::NotForcing);
        assert_eq!(PlasmStaking::storage_version(), 1);
        assert_eq!(Session::validators(), vec![1, 2]);
        assert_eq!(Session::current_index(), 1);

        // 2-11-th sesson
        for i in 2..11 {
            advance_session();
            assert_eq!(PlasmStaking::current_era(), 1);
            assert_eq!(PlasmStaking::current_era_start(), PER_SESSION);
            assert_eq!(PlasmStaking::current_era_start_session_index(), 1);
            assert_eq!(PlasmStaking::force_era(), Forcing::NotForcing);
            assert_eq!(PlasmStaking::storage_version(), 1);
            assert_eq!(Session::validators(), vec![1, 2, 3, 4, 5]);
            assert_eq!(Session::current_index(), i);
        }

        advance_session();
        assert_eq!(PlasmStaking::current_era(), 2);
        assert_eq!(PlasmStaking::current_era_start(), 11 * PER_SESSION);
        assert_eq!(PlasmStaking::current_era_start_session_index(), 11);
        assert_eq!(PlasmStaking::force_era(), Forcing::NotForcing);
        assert_eq!(PlasmStaking::storage_version(), 1);
        assert_eq!(Session::validators(), vec![1, 2, 3, 4, 5]);
        assert_eq!(Session::current_index(), 11);
    })
}

#[test]
fn force_new_era_always_incremental_era() {
    new_test_ext().execute_with(|| {
        assert_eq!(PlasmStaking::force_era(), Forcing::NotForcing);
        assert_ok!(PlasmStaking::force_new_era_always(Origin::ROOT));
        assert_eq!(PlasmStaking::force_era(), Forcing::ForceAlways);

        assert_ok!(PlasmStaking::set_validators(
            Origin::ROOT,
            vec![1, 2, 3, 4, 5]
        ));

        advance_session();
        assert_eq!(PlasmStaking::current_era(), 1);
        assert_eq!(PlasmStaking::current_era_start(), PER_SESSION);
        assert_eq!(PlasmStaking::current_era_start_session_index(), 1);
        assert_eq!(PlasmStaking::force_era(), Forcing::ForceAlways);
        assert_eq!(PlasmStaking::storage_version(), 1);
        assert_eq!(Session::validators(), vec![1, 2]);
        assert_eq!(Session::current_index(), 1);

        advance_session();
        assert_eq!(PlasmStaking::current_era(), 2);
        assert_eq!(PlasmStaking::current_era_start(), 2 * PER_SESSION);
        assert_eq!(PlasmStaking::current_era_start_session_index(), 2);
        assert_eq!(PlasmStaking::force_era(), Forcing::ForceAlways);
        assert_eq!(PlasmStaking::storage_version(), 1);
        assert_eq!(Session::validators(), vec![1, 2, 3, 4, 5]);
        assert_eq!(Session::current_index(), 2);
    })
}

#[test]
fn bond_scenario_test() {
    new_test_ext().execute_with(|| {
        // bond ALICE -> BOB
        assert_ok!(PlasmStaking::bond(
            Origin::signed(ALICE_STASH),
            ALICE_CTRL,
            1000,
            RewardDestination::Stash,
        ));
        assert_eq!(PlasmStaking::bonded(ALICE_STASH), Some(ALICE_CTRL));
        assert_eq!(PlasmStaking::bonded(ALICE_CTRL), None);
        assert_eq!(PlasmStaking::payee(ALICE_STASH), RewardDestination::Stash);
        assert_eq!(
            PlasmStaking::ledger(ALICE_CTRL),
            Some(StakingLedger {
                stash: ALICE_STASH,
                total: 1000,
                active: 1000,
                unlocking: vec![],
            })
        );
        assert_eq!(PlasmStaking::ledger(ALICE_STASH), None);
        assert_eq!(
            Balances::locks(ALICE_STASH),
            vec![BalanceLock {
                id: STAKING_ID,
                amount: 1000,
                reasons: balances::Reasons::All,
            },]
        )
    })
}

#[test]
fn bond_failed_test() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            PlasmStaking::bond(
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
            PlasmStaking::bond(
                Origin::signed(ALICE_STASH),
                ALICE_CTRL,
                100,
                RewardDestination::Stash,
            ),
            Err(DispatchError::Other("stash already bonded"))
        );

        assert_eq!(
            PlasmStaking::bond(
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
    dest: RewardDestination,
) {
    // bond ALICE -> BOB
    assert_ok!(PlasmStaking::bond(
        Origin::signed(stash),
        ctrl,
        balance,
        dest,
    ));
    assert_eq!(PlasmStaking::bonded(stash), Some(ctrl));
    assert_eq!(PlasmStaking::payee(stash), dest);
    assert_eq!(
        PlasmStaking::ledger(ctrl),
        Some(StakingLedger {
            stash: stash,
            total: balance,
            active: balance,
            unlocking: vec![],
        })
    );
    assert_eq!(
        Balances::locks(stash),
        vec![BalanceLock {
            id: STAKING_ID,
            amount: balance,
            reasons: balances::Reasons::All,
        },]
    )
}

#[test]
fn bond_extra_scenario_test() {
    new_test_ext().execute_with(|| {
        // success first bond BOB_STASH -> BOB_CTRL
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);

        assert_ok!(PlasmStaking::bond_extra(Origin::signed(BOB_STASH), 1000));
        assert_eq!(PlasmStaking::bonded(BOB_STASH), Some(BOB_CTRL));
        assert_eq!(PlasmStaking::payee(BOB_STASH), RewardDestination::Stash);
        assert_eq!(
            PlasmStaking::ledger(BOB_CTRL),
            Some(StakingLedger {
                stash: BOB_STASH,
                total: 2000,
                active: 2000,
                unlocking: vec![],
            })
        );
        assert_eq!(
            Balances::locks(BOB_STASH),
            vec![BalanceLock {
                id: STAKING_ID,
                amount: 2000,
                reasons: balances::Reasons::All,
            },]
        );
    })
}

#[test]
fn bond_extra_failed_test() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            PlasmStaking::bond_extra(Origin::signed(BOB_STASH), 1000),
            Err(DispatchError::Other("not a stash"))
        );
        <Bonded<Test>>::insert(BOB_STASH, BOB_CTRL);
        assert_eq!(
            PlasmStaking::bond_extra(Origin::signed(BOB_STASH), 1000),
            Err(DispatchError::Other("not a controller"))
        );
    })
}

#[test]
fn unbond_scenario_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);

        assert_ok!(PlasmStaking::unbond(Origin::signed(BOB_CTRL), 300));
        assert_eq!(
            PlasmStaking::ledger(BOB_CTRL),
            Some(StakingLedger {
                stash: BOB_STASH,
                total: 1000,
                active: 700,
                unlocking: vec![UnlockChunk {
                    value: 300,
                    era: 3, // current_era(0) + bonding_duration(3)
                }],
            })
        );
        assert_eq!(
            Balances::locks(BOB_STASH),
            vec![BalanceLock {
                id: STAKING_ID,
                amount: 1000,
                reasons: balances::Reasons::All,
            },]
        );

        advance_era();

        assert_ok!(PlasmStaking::unbond(Origin::signed(BOB_CTRL), 200));
        assert_eq!(
            PlasmStaking::ledger(BOB_CTRL),
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
            })
        );
        assert_eq!(
            Balances::locks(BOB_STASH),
            vec![BalanceLock {
                id: STAKING_ID,
                amount: 1000,
                reasons: balances::Reasons::All,
            },]
        );
    })
}

fn success_unbond(ctrl: AccountId, balance: Balance) {
    let now_ledger = PlasmStaking::ledger(ctrl).unwrap();
    let now_unlock_chunk = now_ledger.unlocking;
    let now_len = now_unlock_chunk.len();
    let current_era = PlasmStaking::current_era();

    assert_ok!(PlasmStaking::unbond(Origin::signed(ctrl), balance));

    let after_ledger = PlasmStaking::ledger(ctrl).unwrap();
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
        assert_eq!(
            PlasmStaking::unbond(Origin::signed(BOB_STASH), 300),
            Err(DispatchError::Other("not a controller"))
        );
        for _ in 0..32 {
            success_unbond(BOB_CTRL, 10);
        }
        assert_eq!(
            PlasmStaking::unbond(Origin::signed(BOB_CTRL), 300),
            Err(DispatchError::Other("can not schedule more unlock chunks"))
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

        assert_ok!(PlasmStaking::withdraw_unbonded(Origin::signed(BOB_CTRL)));
        assert_eq!(
            PlasmStaking::ledger(BOB_CTRL),
            Some(StakingLedger {
                stash: BOB_STASH,
                total: 1000,
                active: 0,
                unlocking: vec![
                    UnlockChunk { value: 300, era: 3 },
                    UnlockChunk { value: 700, era: 4 },
                ],
            })
        );
        assert_eq!(
            Balances::locks(BOB_STASH),
            vec![BalanceLock {
                id: STAKING_ID,
                amount: 1000,
                reasons: balances::Reasons::All,
            },]
        );

        // era 2 -> 3
        advance_era();

        assert_ok!(PlasmStaking::withdraw_unbonded(Origin::signed(BOB_CTRL)));
        assert_eq!(
            PlasmStaking::ledger(BOB_CTRL),
            Some(StakingLedger {
                stash: BOB_STASH,
                total: 700,
                active: 0,
                unlocking: vec![UnlockChunk { value: 700, era: 4 },],
            })
        );
        assert_eq!(
            Balances::locks(BOB_STASH),
            vec![BalanceLock {
                id: STAKING_ID,
                amount: 700,
                reasons: balances::Reasons::All,
            },]
        );

        // era 3 -> 4
        advance_era();

        assert_ok!(PlasmStaking::withdraw_unbonded(Origin::signed(BOB_CTRL)));
        assert_eq!(PlasmStaking::ledger(BOB_CTRL), None);
        assert_eq!(Balances::locks(BOB_STASH), vec![]);
    })
}

#[test]
fn withdraw_unbonded_failed_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);
        success_unbond(BOB_CTRL, 300);
        assert_eq!(
            PlasmStaking::withdraw_unbonded(Origin::signed(BOB_STASH)),
            Err(DispatchError::Other("not a controller"))
        );
    })
}

#[test]
fn nominate_contracts_scenario_test() {
    new_test_ext().execute_with(|| {
        valid_instatiate();
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);
        assert_ok!(PlasmStaking::nominate_contracts(
            Origin::signed(BOB_CTRL),
            vec![OPERATED_CONTRACT]
        ));
        assert_eq!(
            PlasmStaking::dapps_nominations(BOB_STASH),
            Some(staking::Nominations {
                targets: vec![OPERATED_CONTRACT],
                submitted_in: 0,
                suppressed: false,
            })
        );
    })
}

fn success_nominate_contracts(ctrl: AccountId, targets: Vec<AccountId>) {
    assert_ok!(PlasmStaking::nominate_contracts(
        Origin::signed(ctrl),
        targets.clone()
    ));
    let stash = PlasmStaking::ledger(&ctrl).unwrap().stash;
    let current_era = PlasmStaking::current_era();
    assert_eq!(
        PlasmStaking::dapps_nominations(stash),
        Some(staking::Nominations {
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
        assert_eq!(
            PlasmStaking::nominate_contracts(Origin::signed(BOB_STASH), vec![OPERATED_CONTRACT]),
            Err(DispatchError::Other("not a controller"))
        );
        assert_eq!(
            PlasmStaking::nominate_contracts(Origin::signed(BOB_CTRL), vec![]),
            Err(DispatchError::Other("targets cannot be empty"))
        );
        assert_eq!(
            PlasmStaking::nominate_contracts(Origin::signed(BOB_CTRL), vec![BOB_CONTRACT]),
            Err(DispatchError::Other("tragets must be operated contracts"))
        );
    })
}

#[test]
fn chill_scenario_test() {
    new_test_ext().execute_with(|| {
        valid_instatiate();
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);
        success_nominate_contracts(BOB_CTRL, vec![OPERATED_CONTRACT]);
        assert_ok!(PlasmStaking::chill(Origin::signed(BOB_CTRL)));
        assert_eq!(PlasmStaking::dapps_nominations(BOB_STASH), None);
    })
}

#[test]
fn chill_failed_test() {
    new_test_ext().execute_with(|| {
        valid_instatiate();
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);
        success_nominate_contracts(BOB_CTRL, vec![OPERATED_CONTRACT]);
        assert_eq!(
            PlasmStaking::chill(Origin::signed(BOB_STASH)),
            Err(DispatchError::Other("not a controller"))
        );
    })
}

#[test]
fn set_payee_scenario_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(ALICE_STASH, ALICE_CTRL, 1000, RewardDestination::Stash);
        assert_ok!(PlasmStaking::set_payee(
            Origin::signed(ALICE_CTRL),
            RewardDestination::Controller
        ));
        assert_eq!(
            PlasmStaking::payee(ALICE_STASH),
            RewardDestination::Controller
        );
    })
}

#[test]
fn set_payee_failed_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(ALICE_STASH, ALICE_CTRL, 1000, RewardDestination::Stash);
        assert_eq!(
            PlasmStaking::set_payee(Origin::signed(ALICE_STASH), RewardDestination::Controller),
            Err(DispatchError::Other("not a controller"))
        );
    })
}

#[test]
fn set_controller_scenario_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(ALICE_STASH, ALICE_CTRL, 1000, RewardDestination::Stash);
        assert_ok!(PlasmStaking::set_controller(
            Origin::signed(ALICE_STASH),
            BOB_CTRL
        ));
        assert_eq!(PlasmStaking::bonded(ALICE_STASH), Some(BOB_CTRL));
        assert_eq!(
            PlasmStaking::ledger(BOB_CTRL),
            Some(StakingLedger {
                stash: ALICE_STASH,
                total: 1000,
                active: 1000,
                unlocking: vec![],
            })
        );
        assert_eq!(PlasmStaking::ledger(ALICE_CTRL), None);
    })
}

#[test]
fn set_controller_failed_test() {
    new_test_ext().execute_with(|| {
        success_first_bond(ALICE_STASH, ALICE_CTRL, 1000, RewardDestination::Stash);
        assert_eq!(
            PlasmStaking::set_controller(Origin::signed(ALICE_CTRL), BOB_CTRL),
            Err(DispatchError::Other("not a stash"))
        );
        success_first_bond(BOB_STASH, BOB_CTRL, 1000, RewardDestination::Stash);
        assert_eq!(
            PlasmStaking::set_controller(Origin::signed(ALICE_STASH), BOB_CTRL),
            Err(DispatchError::Other("controller already paired"))
        );
    })
}

const SIX_HOURS: u64 = 6 * 60 * 60 * 1000;

#[test]
fn reward_to_validator_test() {
    new_test_ext().execute_with(|| {
        advance_session();
        assert_ok!(PlasmStaking::set_validators(
            Origin::ROOT,
            vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C, VALIDATOR_D]
        ));
        advance_era();
        assert_eq!(
            PlasmStaking::current_elected(),
            vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C, VALIDATOR_D]
        );
        advance_session();
        assert_eq!(
            Session::validators(),
            vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C, VALIDATOR_D]
        );

        let pre_total_issuarance = Balances::total_issuance();

        let (a, b) = inflation::compute_total_payout_test(pre_total_issuarance, SIX_HOURS);
        let positive_imbalance = PlasmStaking::reward_to_validators(a / 2, b / 2);
        assert_eq!(Balances::free_balance(&VALIDATOR_A), 1_000_068);
        assert_eq!(Balances::free_balance(&VALIDATOR_B), 1_000_068);
        assert_eq!(Balances::free_balance(&VALIDATOR_C), 1_000_068);
        assert_eq!(Balances::free_balance(&VALIDATOR_D), 1_000_068);
        assert_eq!(positive_imbalance, 272);
        assert_eq!(Balances::total_issuance(), pre_total_issuarance + 272);
    })
}

#[test]
fn reward_to_operators_test() {
    new_test_ext().execute_with(|| {
        valid_instatiate();
        assert_ok!(Operator::change_operator(
            Origin::signed(OPERATOR),
            vec![OPERATED_CONTRACT],
            ALICE_STASH
        ));
        success_first_bond(BOB_STASH, BOB_CTRL, 1_000, RewardDestination::Stash);
        success_first_bond(
            ALICE_STASH,
            ALICE_CTRL,
            1_000,
            RewardDestination::Controller,
        );
        success_nominate_contracts(BOB_CTRL, vec![OPERATED_CONTRACT]);
        success_nominate_contracts(ALICE_CTRL, vec![OPERATED_CONTRACT]);

        advance_era();

        let pre_total_issuarance = Balances::total_issuance();

        let (a, b) = inflation::compute_total_payout_test(pre_total_issuarance, SIX_HOURS);
        let positive_imbalance = PlasmStaking::reward_to_operators(a - a / 2, b - b / 2);
        assert_eq!(Balances::free_balance(&BOB_STASH), 2_000 + 34); // +nomiante reward
        assert_eq!(Balances::free_balance(&BOB_CTRL), 20 + 0); // +0
        assert_eq!(Balances::free_balance(&ALICE_STASH), 1_000 + 274); // +operator reward
        assert_eq!(Balances::free_balance(&ALICE_CTRL), 10 + 34); // +nominate reward
        assert_eq!(positive_imbalance, 342);
        assert_eq!(Balances::total_issuance(), pre_total_issuarance + 342);
    })
}

#[test]
fn new_session_scenario_test() {
    new_test_ext().execute_with(|| {
        advance_session();
        assert_ok!(PlasmStaking::set_validators(
            Origin::ROOT,
            vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C, VALIDATOR_D]
        ));

        valid_instatiate();
        assert_ok!(Operator::change_operator(
            Origin::signed(OPERATOR),
            vec![OPERATED_CONTRACT],
            ALICE_STASH
        ));
        success_first_bond(BOB_STASH, BOB_CTRL, 1_000, RewardDestination::Stash);
        success_first_bond(
            ALICE_STASH,
            ALICE_CTRL,
            1_000,
            RewardDestination::Controller,
        );
        success_nominate_contracts(BOB_CTRL, vec![OPERATED_CONTRACT]);
        success_nominate_contracts(ALICE_CTRL, vec![OPERATED_CONTRACT]);

        advance_era();
        advance_session();
        assert_eq!(
            PlasmStaking::current_elected(),
            vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C, VALIDATOR_D]
        );

        let pre_total_issuarance = Balances::total_issuance();
        assert_eq!(Balances::free_balance(&VALIDATOR_A), 1_000_000);
        assert_eq!(Balances::free_balance(&VALIDATOR_B), 1_000_000);
        assert_eq!(Balances::free_balance(&VALIDATOR_C), 1_000_000);
        assert_eq!(Balances::free_balance(&VALIDATOR_D), 1_000_000);
        assert_eq!(Balances::free_balance(&BOB_STASH), 2_000);
        assert_eq!(Balances::free_balance(&BOB_CTRL), 20);
        assert_eq!(Balances::free_balance(&ALICE_STASH), 1_000);
        assert_eq!(Balances::free_balance(&ALICE_CTRL), 10);
        assert_eq!(pre_total_issuarance, 5_002_318);
        for _ in 0..10 {
            advance_session();
        }
        assert_eq!(Balances::free_balance(&VALIDATOR_A), 1_000_000 + 2); // +validator reward
        assert_eq!(Balances::free_balance(&VALIDATOR_B), 1_000_000 + 2); // +validator reward
        assert_eq!(Balances::free_balance(&VALIDATOR_C), 1_000_000 + 2); // +validator reward
        assert_eq!(Balances::free_balance(&VALIDATOR_D), 1_000_000 + 2); // +validator reward
        assert_eq!(Balances::free_balance(&BOB_STASH), 2_000 + 1); // +nomiante reward
        assert_eq!(Balances::free_balance(&BOB_CTRL), 20 + 0); // +0
        assert_eq!(Balances::free_balance(&ALICE_STASH), 1_000 + 8); // +operator reward
        assert_eq!(Balances::free_balance(&ALICE_CTRL), 10 + 1); // +nominate reward
        assert_eq!(Balances::total_issuance(), 5_002_318 + 18);
    })
}
