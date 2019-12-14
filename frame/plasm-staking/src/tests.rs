//! Tests for the plasm-staking module.

#![cfg(test)]

use crate::mock::*;
use super::*;
use support::assert_ok;

#[test]
fn set_validators_fails_for_user() {
	new_test_ext().execute_with(|| {
		let res = PlasmStaking::force_no_eras(Origin::signed(0));
		assert_eq!(res, Err("RequireRootOrigin"));

		let res = PlasmStaking::force_new_era(Origin::signed(0));
		assert_eq!(res, Err("RequireRootOrigin"));

		let res = PlasmStaking::force_new_era_always(Origin::signed(0));
		assert_eq!(res, Err("RequireRootOrigin"));
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

		assert_ok!(ValidatorManager::set_validators(Origin::ROOT, vec![1,2,3,4,5]));

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
			println!("{}", i);
			assert_eq!(PlasmStaking::current_era(), 1);
			assert_eq!(PlasmStaking::current_era_start(), 100);
			assert_eq!(PlasmStaking::current_era_start_session_index(), 10);
			assert_eq!(PlasmStaking::force_era(), Forcing::NotForcing);
			assert_eq!(PlasmStaking::storage_version(), 1);
			assert_eq!(Session::current_index(), i);
			match i {
				10 => assert_eq!(Session::validators(), vec![1, 2]),
				_ => assert_eq!(Session::validators(), vec![1, 2, 3, 4, 5]),
			}
		}

		assert_ok!(ValidatorManager::set_validators(Origin::ROOT, vec![1,3,5]));

		// 20~29-th session
		for i in 20..30 {
			advance_session();
			assert_eq!(PlasmStaking::current_era(), 2);
			assert_eq!(PlasmStaking::current_era_start(), 200);
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

		assert_ok!(ValidatorManager::set_validators(Origin::ROOT, vec![1,2, 3,4,5]));

		advance_session();
		assert_eq!(PlasmStaking::current_era(), 1);
		assert_eq!(PlasmStaking::current_era_start(), 10);
		assert_eq!(PlasmStaking::current_era_start_session_index(), 1);
		assert_eq!(PlasmStaking::force_era(), Forcing::NotForcing);
		assert_eq!(PlasmStaking::storage_version(), 1);
		assert_eq!(Session::validators(), vec![1, 2]);
		assert_eq!(Session::current_index(), 1);

		// 2-11-th sesson
		for i in 2..11 {
			advance_session();
			assert_eq!(PlasmStaking::current_era(), 1);
			assert_eq!(PlasmStaking::current_era_start(), 10);
			assert_eq!(PlasmStaking::current_era_start_session_index(), 1);
			assert_eq!(PlasmStaking::force_era(), Forcing::NotForcing);
			assert_eq!(PlasmStaking::storage_version(), 1);
			assert_eq!(Session::validators(), vec![1, 2, 3, 4, 5]);
			assert_eq!(Session::current_index(), i);
		}

		advance_session();
		assert_eq!(PlasmStaking::current_era(), 2);
		assert_eq!(PlasmStaking::current_era_start(), 110);
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

		assert_ok!(ValidatorManager::set_validators(Origin::ROOT, vec![1,2, 3,4,5]));

		advance_session();
		assert_eq!(PlasmStaking::current_era(), 1);
		assert_eq!(PlasmStaking::current_era_start(), 10);
		assert_eq!(PlasmStaking::current_era_start_session_index(), 1);
		assert_eq!(PlasmStaking::force_era(), Forcing::ForceAlways);
		assert_eq!(PlasmStaking::storage_version(), 1);
		assert_eq!(Session::validators(), vec![1, 2]);
		assert_eq!(Session::current_index(), 1);

		advance_session();
		assert_eq!(PlasmStaking::current_era(), 2);
		assert_eq!(PlasmStaking::current_era_start(), 20);
		assert_eq!(PlasmStaking::current_era_start_session_index(), 2);
		assert_eq!(PlasmStaking::force_era(), Forcing::ForceAlways);
		assert_eq!(PlasmStaking::storage_version(), 1);
		assert_eq!(Session::validators(), vec![1, 2, 3, 4, 5]);
		assert_eq!(Session::current_index(), 2);
	})
}
