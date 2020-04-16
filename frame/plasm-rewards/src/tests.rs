//! Tests for the dapps-staking module.

#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::assert_ok;
use sp_runtime::DispatchError;

#[test]
fn root_calls_fails_for_user() {
    new_test_ext().execute_with(|| {
        let res = PlasmRewards::force_no_eras(Origin::signed(0));
        assert_eq!(res, Err(DispatchError::BadOrigin));

        let res = PlasmRewards::force_new_era(Origin::signed(0));
        assert_eq!(res, Err(DispatchError::BadOrigin));

        let res = PlasmRewards::force_new_era_always(Origin::signed(0));
        assert_eq!(res, Err(DispatchError::BadOrigin));
    })
}

#[test]
fn normal_incremental_era() {
    new_test_ext().execute_with(|| {
        assert_eq!(PlasmRewards::current_era().unwrap(), 0);
        assert_eq!(
            PlasmRewards::active_era().unwrap(),
            ActiveEraInfo::<u64> {
                index: 0,
                start: None,
            }
        );
        assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
        assert_eq!(PlasmRewards::eras_start_session_index(0).unwrap(), 0);
        assert_eq!(PlasmRewards::for_dapps_era_reward(0), None);
        assert_eq!(PlasmRewards::for_security_era_reward(0), None);
        assert_eq!(Session::validators(), vec![1, 2, 3, 100]);
        assert_eq!(Session::current_index(), 0);

        advance_session();

        assert_eq!(PlasmRewards::current_era().unwrap(), 0);
        assert_eq!(
            PlasmRewards::active_era().unwrap(),
            ActiveEraInfo::<u64> {
                index: 0,
                start: Some(PER_SESSION),
            }
        );
        assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
        assert_eq!(PlasmRewards::eras_start_session_index(0).unwrap(), 0);
        assert_eq!(PlasmRewards::for_dapps_era_reward(0), None);
        assert_eq!(PlasmRewards::for_security_era_reward(0), None);
        assert_eq!(Session::validators(), vec![1, 2, 3, 100]);
        assert_eq!(Session::current_index(), 1);

        // 2~9-th session
        for i in 2..10 {
            advance_session();
            match i {
                9 => assert_eq!(PlasmRewards::current_era().unwrap(), 1),
                _ => assert_eq!(PlasmRewards::current_era().unwrap(), 0),
            }
            assert_eq!(
                PlasmRewards::active_era().unwrap(),
                ActiveEraInfo::<u64> {
                    index: 0,
                    start: Some(PER_SESSION),
                }
            );
            assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
            assert_eq!(PlasmRewards::eras_start_session_index(0).unwrap(), 0);
            assert_eq!(PlasmRewards::for_dapps_era_reward(0), None);
            assert_eq!(PlasmRewards::for_security_era_reward(0), None);
            assert_eq!(Session::validators(), vec![1, 2, 3, 100]);
            assert_eq!(Session::current_index(), i);
        }

        // 10~19-th session
        for i in 10..20 {
            advance_session();
            match i {
                19 => assert_eq!(PlasmRewards::current_era().unwrap(), 2),
                _ => assert_eq!(PlasmRewards::current_era().unwrap(), 1),
            }
            assert_eq!(
                PlasmRewards::active_era().unwrap(),
                ActiveEraInfo::<u64> {
                    index: 1,
                    start: Some(10 * PER_SESSION),
                }
            );
            assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
            assert_eq!(PlasmRewards::eras_start_session_index(1).unwrap(), 10);
            assert_eq!(PlasmRewards::for_security_era_reward(0).unwrap(), 0);
            assert_eq!(PlasmRewards::for_dapps_era_reward(0).unwrap(), 0);
            assert_eq!(Session::current_index(), i);
            assert_eq!(Session::validators(), vec![1, 2, 3, 101]);
        }

        // 20~29-th session
        for i in 20..30 {
            advance_session();
            match i {
                29 => assert_eq!(PlasmRewards::current_era().unwrap(), 3),
                _ => assert_eq!(PlasmRewards::current_era().unwrap(), 2),
            }
            assert_eq!(
                PlasmRewards::active_era().unwrap(),
                ActiveEraInfo::<u64> {
                    index: 2,
                    start: Some(20 * PER_SESSION),
                }
            );
            assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
            assert_eq!(PlasmRewards::eras_start_session_index(2).unwrap(), 20);
            assert_eq!(
                PlasmRewards::for_security_era_reward(1).unwrap(),
                3168333332066
            );
            assert_eq!(PlasmRewards::for_dapps_era_reward(1).unwrap(), 633666667934);
            assert_eq!(Session::current_index(), i);
            assert_eq!(Session::validators(), vec![1, 2, 3, 102]);
        }
    })
}

#[test]
fn force_new_era_incremental_era() {
    new_test_ext().execute_with(|| {
        assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
        assert_ok!(PlasmRewards::force_new_era(Origin::ROOT));
        assert_eq!(PlasmRewards::force_era(), Forcing::ForceNew);

        advance_session();
        assert_eq!(PlasmRewards::current_era().unwrap(), 1);
        assert_eq!(
            PlasmRewards::active_era().unwrap(),
            ActiveEraInfo::<u64> {
                index: 0,
                start: Some(PER_SESSION),
            }
        );
        assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
        assert_eq!(PlasmRewards::eras_start_session_index(0).unwrap(), 0);
        assert_eq!(PlasmRewards::for_dapps_era_reward(0), None);
        assert_eq!(PlasmRewards::for_security_era_reward(0), None);
        assert_eq!(Session::validators(), vec![1, 2, 3, 100]);
        assert_eq!(Session::current_index(), 1);

        // 2-11-th sesson
        for i in 2..12 {
            advance_session();
            match i {
                11 => assert_eq!(PlasmRewards::current_era().unwrap(), 2),
                _ => assert_eq!(PlasmRewards::current_era().unwrap(), 1),
            }
            assert_eq!(
                PlasmRewards::active_era().unwrap(),
                ActiveEraInfo::<u64> {
                    index: 1,
                    start: Some(2 * PER_SESSION),
                }
            );
            assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
            assert_eq!(PlasmRewards::eras_start_session_index(1).unwrap(), 2);
            assert_eq!(PlasmRewards::for_dapps_era_reward(0).unwrap(), 0);
            assert_eq!(PlasmRewards::for_security_era_reward(0).unwrap(), 0);
            assert_eq!(Session::validators(), vec![1, 2, 3, 101]);
            assert_eq!(Session::current_index(), i);
        }

        advance_session();
        assert_eq!(PlasmRewards::current_era().unwrap(), 2);
        assert_eq!(
            PlasmRewards::active_era().unwrap(),
            ActiveEraInfo::<u64> {
                index: 2,
                start: Some(12 * PER_SESSION),
            }
        );
        assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
        assert_eq!(PlasmRewards::eras_start_session_index(2).unwrap(), 12);
        assert_eq!(
            PlasmRewards::for_security_era_reward(1).unwrap(),
            3168333332066,
        );
        assert_eq!(PlasmRewards::for_dapps_era_reward(1).unwrap(), 633666667934);
        assert_eq!(Session::validators(), vec![1, 2, 3, 102]);
        assert_eq!(Session::current_index(), 12);
    })
}

#[test]
fn force_new_era_always_incremental_era() {
    new_test_ext().execute_with(|| {
        assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
        assert_ok!(PlasmRewards::force_new_era_always(Origin::ROOT));
        assert_eq!(PlasmRewards::force_era(), Forcing::ForceAlways);

        for i in 1..10 {
            advance_session();
            assert_eq!(PlasmRewards::current_era().unwrap(), i);
            assert_eq!(
                PlasmRewards::active_era().unwrap(),
                ActiveEraInfo::<u64> {
                    index: i - 1,
                    start: Some(i as u64 * PER_SESSION),
                }
            );
            assert_eq!(PlasmRewards::force_era(), Forcing::ForceAlways);
            match i {
                1 => assert_eq!(PlasmRewards::eras_start_session_index(0).unwrap(), 0),
                _ => assert_eq!(PlasmRewards::eras_start_session_index(i - 1).unwrap(), i),
            }
            match i {
                1 => {
                    assert_eq!(PlasmRewards::for_dapps_era_reward(0), None);
                    assert_eq!(PlasmRewards::for_security_era_reward(0), None);
                }
                2 => {
                    assert_eq!(PlasmRewards::for_dapps_era_reward(0).unwrap(), 0);
                    assert_eq!(PlasmRewards::for_security_era_reward(0).unwrap(), 0);
                }
                _ => {
                    assert_eq!(
                        PlasmRewards::for_security_era_reward(1).unwrap(),
                        315833333207,
                    );
                    assert_eq!(PlasmRewards::for_dapps_era_reward(1).unwrap(), 63166666793);
                }
            }
            assert_eq!(Session::validators(), vec![1, 2, 3, 100 + (i as u64 - 1)]);
            assert_eq!(Session::current_index(), i);
        }
    })
}
