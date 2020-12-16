//! Tests for the plasm-lockdrop module.

#![cfg(test)]

use super::*;
use crate::mock::*;

use frame_support::{assert_err, assert_ok};

pub const ACCOUNT_01: AccountId = 1;
pub const ACCOUNT_02: AccountId = 2;
pub const ACCOUNT_03: AccountId = 3;
pub const ACCOUNT_04: AccountId = 4;

fn assert_timestamp(expected: Moment) {
    assert_eq!(Timestamp::now(), expected);
}

fn set_timestamp(time: Moment) {
    Timestamp::set_timestamp(time);
}

#[test]
fn normal_test() {
    new_test_ext().execute_with(|| {
        set_timestamp(50);
        assert_timestamp(50);
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_01));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_02));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_03));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_04));

        // Added ACCOUNT_01
        assert_ok!(RegisterIntroducers::register(Origin::signed(ACCOUNT_01)));
        assert!(RegisterIntroducers::is_registered(&ACCOUNT_01));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_02));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_03));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_04));
    });
}

#[test]
fn boundary_test() {
    new_test_ext().execute_with(|| {
        set_timestamp(100);
        assert_timestamp(100);
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_01));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_02));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_03));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_04));

        // Added ACCOUNT_01
        assert_ok!(RegisterIntroducers::register(Origin::signed(ACCOUNT_02)));
        assert_ok!(RegisterIntroducers::register(Origin::signed(ACCOUNT_04)));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_01));
        assert!(RegisterIntroducers::is_registered(&ACCOUNT_02));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_03));
        assert!(RegisterIntroducers::is_registered(&ACCOUNT_04));
    });
}

#[test]
fn over_test() {
    new_test_ext().execute_with(|| {
        set_timestamp(101);
        assert_timestamp(101);
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_01));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_02));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_03));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_04));

        // Added ACCOUNT_01
        assert_err!(
            RegisterIntroducers::register(Origin::signed(ACCOUNT_01)),
            Error::<Test>::Expired
        );
        assert_err!(
            RegisterIntroducers::register(Origin::signed(ACCOUNT_02)),
            Error::<Test>::Expired
        );
        assert_err!(
            RegisterIntroducers::register(Origin::signed(ACCOUNT_03)),
            Error::<Test>::Expired
        );
        assert_err!(
            RegisterIntroducers::register(Origin::signed(ACCOUNT_04)),
            Error::<Test>::Expired
        );
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_01));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_02));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_03));
        assert!(!RegisterIntroducers::is_registered(&ACCOUNT_04));
    });
}
