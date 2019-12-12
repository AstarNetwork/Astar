//! Tests for the plasm-session module.

#![cfg(test)]

use crate::mock::*;
use support::assert_ok;

#[test]
fn set_validators_fails_for_user() {
	new_test_ext().execute_with(|| {
		let res = PlasmSession::force_no_eras(Origin::signed(0));
		assert_eq!(res, Err("RequireRootOrigin"));

		let res = PlasmSession::force_new_era(Origin::signed(0));
		assert_eq!(res, Err("RequireRootOrigin"));

		let res = PlasmSession::set_invulnerables(Origin::signed(0), vec![0,0]);
		assert_eq!(res, Err("RequireRootOrigin"));

		let res = PlasmSession::force_unstake(Origin::signed(0), 0);
		assert_eq!(res, Err("RequireRootOrigin"));

		let res = PlasmSession::force_new_era_always(Origin::signed(0));
		assert_eq!(res, Err("RequireRootOrigin"));
	})
}
