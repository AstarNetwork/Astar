//! Tests for the ovm module.

#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::assert_ok;
use frame_system::{self as system, EventRecord, Phase};

const VALID_PREDICATE: &str = r#"valid."#;
const AGGREGATOR_ID: u64 = 101;
const ERC20_ID: u64 = 101;
const STATE_UPDATE_ID: u64 = 101;
const EXIT_ID: u64 = 101;
const EXIT_DEPOSIT_ID: u64 = 101;

#[test]
fn deploy_sucess() {
    new_test_ext().execute_with(|| {
        advance_block();
        assert_ok!(Plasma::deploy(
            Origin::signed(ALICE_STASH),
            AGGREGATOR_ID,
            ERC20_ID,
            STATE_UPDATE_ID,
            EXIT_ID,
            EXIT_DEPOSIT_ID
        ));

        let plapps_id = 10001;

        assert_eq!(Plasma::aggregator_address(&plapps_id), AGGREGATOR_ID);
        assert_eq!(Plasma::erc20(&plapps_id), ERC20_ID);
        assert_eq!(Plasma::state_update_predicate(&plapps_id), STATE_UPDATE_ID);
        assert_eq!(Plasma::exit_predicate(&plapps_id), EXIT_ID);
        assert_eq!(Plasma::exit_deposit_predicate(&plapps_id), EXIT_DEPOSIT_ID);

        assert_eq!(
            System::events(),
            vec![EventRecord {
                phase: Phase::Initialization,
                event: MetaEvent::plasma(RawEvent::Deploy(ALICE_STASH, 10001)),
                topics: vec![],
            }]
        );
    })
}
