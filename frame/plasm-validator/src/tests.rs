//! Tests for the dapps-staking module.

#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::assert_ok;
use pallet_plasm_rewards::traits::ComputeTotalPayout;
use sp_runtime::DispatchError;

#[test]
fn set_validators_works_for_root() {
    new_test_ext().execute_with(|| {
        advance_session();
        assert_eq!(Session::current_index(), 1);
        assert_eq!(
            Session::validators(),
            vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C, VALIDATOR_D]
        );

        assert_ok!(PlasmValidator::set_validators(
            Origin::ROOT,
            vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C]
        ));
        assert_eq!(
            PlasmValidator::validators(),
            vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C]
        );
        for i in 1..10 {
            assert_eq!(Session::current_index(), i);
            assert_eq!(
                Session::validators(),
                vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C, VALIDATOR_D]
            );
            advance_session();
        }

        advance_session();
        assert_eq!(
            Session::validators(),
            vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C]
        );

        for i in 11..25 {
            assert_eq!(Session::current_index(), i);
            assert_eq!(
                Session::validators(),
                vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C]
            );
            advance_session();
        }

        assert_ok!(PlasmValidator::set_validators(
            Origin::ROOT,
            vec![VALIDATOR_A, VALIDATOR_B]
        ));
        assert_eq!(PlasmValidator::validators(), vec![VALIDATOR_A, VALIDATOR_B]);

        for i in 25..30 {
            assert_eq!(Session::current_index(), i);
            assert_eq!(
                Session::validators(),
                vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C]
            );
            advance_session();
        }

        advance_session();
        assert_eq!(Session::current_index(), 31);
        assert_eq!(Session::validators(), vec![VALIDATOR_A, VALIDATOR_B]);
    })
}

#[test]
fn root_calls_fails_for_user() {
    new_test_ext().execute_with(|| {
        let res = PlasmValidator::set_validators(Origin::signed(0), vec![]);
        assert_eq!(res, Err(DispatchError::BadOrigin));
    })
}

const SIX_HOURS: u64 = 6 * 60 * 60 * 1000;

#[test]
fn reward_to_validator_test() {
    new_test_ext().execute_with(|| {
        advance_session();
        assert_ok!(PlasmValidator::set_validators(
            Origin::ROOT,
            vec![
                VALIDATOR_A,
                VALIDATOR_B,
                VALIDATOR_C,
                VALIDATOR_D,
                VALIDATOR_E
            ]
        ));
        advance_era();
        assert_eq!(PlasmRewards::current_era().unwrap(), 1);
        assert_eq!(
            PlasmValidator::elected_validators(1),
            Some(vec![
                VALIDATOR_A,
                VALIDATOR_B,
                VALIDATOR_C,
                VALIDATOR_D,
                VALIDATOR_E
            ])
        );
        assert_eq!(
            Session::validators(),
            vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C, VALIDATOR_D,]
        );
        advance_session();
        assert_eq!(
            PlasmValidator::elected_validators(1),
            Some(vec![
                VALIDATOR_A,
                VALIDATOR_B,
                VALIDATOR_C,
                VALIDATOR_D,
                VALIDATOR_E
            ])
        );
        assert_eq!(
            Session::validators(),
            vec![
                VALIDATOR_A,
                VALIDATOR_B,
                VALIDATOR_C,
                VALIDATOR_D,
                VALIDATOR_E
            ]
        );

        let pre_total_issuarance = Balances::total_issuance();

        let (a, _) = <mock::Test as pallet_plasm_rewards::Trait>::ComputeTotalPayout::compute(
            pre_total_issuarance,
            SIX_HOURS,
            0,
            0,
        );
        println!("pre_total:{}, a:{}", pre_total_issuarance, a);
        let positive_imbalance = PlasmValidator::reward_to_validators(&0, &a);
        assert_eq!(
            Balances::free_balance(&VALIDATOR_A),
            1_000_000_000_000_000_000 + a / 4
        );
        assert_eq!(
            Balances::free_balance(&VALIDATOR_B),
            1_000_000_000_000_000_000 + a / 4
        );
        assert_eq!(
            Balances::free_balance(&VALIDATOR_C),
            1_000_000_000_000_000_000 + a / 4
        );
        assert_eq!(
            Balances::free_balance(&VALIDATOR_D),
            1_000_000_000_000_000_000 + a / 4
        );
        assert_eq!(positive_imbalance, a);
        assert_eq!(Balances::total_issuance(), pre_total_issuarance + a);
    })
}

// The test will delete, when change the compute algorithm.
#[test]
fn first_reward_to_validator_test() {
    new_test_ext().execute_with(|| {
        advance_session();
        assert_ok!(PlasmValidator::set_validators(
            Origin::ROOT,
            vec![VALIDATOR_A, VALIDATOR_B,]
        ));
        advance_era();
        assert_eq!(PlasmRewards::current_era().unwrap(), 1);
        assert_eq!(
            <PlasmValidator as ComputeEraWithParam<EraIndex>>::compute(&1),
            2
        );

        assert_ok!(PlasmValidator::set_validators(
            Origin::ROOT,
            vec![
                VALIDATOR_A,
                VALIDATOR_B,
                VALIDATOR_C,
                VALIDATOR_D,
                VALIDATOR_E
            ]
        ));
        advance_era();
        assert_eq!(
            <PlasmValidator as ComputeEraWithParam<EraIndex>>::compute(&2),
            5
        );
    })
}
