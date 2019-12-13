//! Tests for the plasm-session module.

#![cfg(test)]

use crate::mock::*;
use super::*;
use support::assert_ok;

#[test]
fn set_validators_fails_for_user() {
	new_test_ext().execute_with(|| {
		let res = PlasmSession::force_no_eras(Origin::signed(0));
		assert_eq!(res, Err("RequireRootOrigin"));

		let res = PlasmSession::force_new_era(Origin::signed(0));
		assert_eq!(res, Err("RequireRootOrigin"));

		let res = PlasmSession::set_invulnerables(Origin::signed(0), vec![0, 0]);
		assert_eq!(res, Err("RequireRootOrigin"));

		let res = PlasmSession::force_new_era_always(Origin::signed(0));
		assert_eq!(res, Err("RequireRootOrigin"));
	})
}

#[test]
fn incremental_era() {
	new_test_ext().execute_with(|| {
		assert_eq!(PlasmSession::current_era(), 0);
		assert_eq!(PlasmSession::current_era_start(), 0);
		assert_eq!(PlasmSession::current_era_start_session_index(), 0);
		assert_eq!(PlasmSession::force_era(), Forcing::NotForcing);
		assert_eq!(PlasmSession::storage_version(), 1);
		assert_eq!(Session::validators(), vec![1, 2]);
		assert_eq!(Session::current_index(), 0);

		advance_session();

		assert_eq!(PlasmSession::current_era(), 0);
		assert_eq!(PlasmSession::current_era_start(), 0);
		assert_eq!(PlasmSession::current_era_start_session_index(), 0);
		assert_eq!(PlasmSession::force_era(), Forcing::NotForcing);
		assert_eq!(PlasmSession::storage_version(), 1);
		assert_eq!(Session::validators(), vec![1, 2]);
		assert_eq!(Session::current_index(), 1);

		assert_ok!(SessionManager::set_validators(Origin::ROOT, vec![1,2,3,4,5]));

		assert_eq!(Session::validators(), vec![1, 2]);
		assert_eq!(Session::current_index(), 1);

		// 2~9-th session
		for i in (2..10) {
			advance_session();
			assert_eq!(PlasmSession::current_era(), 0);
			assert_eq!(PlasmSession::current_era_start(), 0);
			assert_eq!(PlasmSession::current_era_start_session_index(), 0);
			assert_eq!(PlasmSession::force_era(), Forcing::NotForcing);
			assert_eq!(PlasmSession::storage_version(), 1);
			assert_eq!(Session::validators(), vec![1, 2]);
			assert_eq!(Session::current_index(), i);
		}

		// 10~19-th session
		for i in (10..20) {
			advance_session();
			println!("{}", i);
			assert_eq!(PlasmSession::current_era(), 1);
			assert_eq!(PlasmSession::current_era_start(), 100);
			assert_eq!(PlasmSession::current_era_start_session_index(), 10);
			assert_eq!(PlasmSession::force_era(), Forcing::NotForcing);
			assert_eq!(PlasmSession::storage_version(), 1);
			assert_eq!(Session::current_index(), i);
			match i {
				10 => assert_eq!(Session::validators(), vec![1, 2]),
				_ => assert_eq!(Session::validators(), vec![1, 2, 3, 4, 5]),
			}
		}

		assert_ok!(SessionManager::set_validators(Origin::ROOT, vec![1,3,5]));

		// 20~29-th session
		for i in (20..30) {
			advance_session();
			assert_eq!(PlasmSession::current_era(), 2);
			assert_eq!(PlasmSession::current_era_start(), 200);
			assert_eq!(PlasmSession::current_era_start_session_index(), 20);
			assert_eq!(PlasmSession::force_era(), Forcing::NotForcing);
			assert_eq!(PlasmSession::storage_version(), 1);
			assert_eq!(Session::current_index(), i);
			match i {
				20 => assert_eq!(Session::validators(), vec![1, 2, 3, 4, 5]),
				_ => assert_eq!(Session::validators(), vec![1, 3, 5]),
			}
		}
	})
}
