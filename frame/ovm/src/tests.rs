//! Tests for the ovm module.

#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::assert_ok;
use frame_system::{EventRecord, Phase};

const VALID_PREDICATE: &str = r#"valid."#;

#[test]
fn test_calls() {
    let (valid_predicate, code_hash) = compile_predicate::<Test>(VALID_PREDICATE);
    new_test_ext().execute_with(|| {
        advance_block();
        assert_ok!(Ovm::put_code(
            Origin::signed((*ALICE_STASH).clone()),
            valid_predicate.clone()
        ));
        assert_eq!(Ovm::predicate_codes(&code_hash), Some(valid_predicate),);
        assert_eq!(
            System::events(),
            vec![EventRecord {
                phase: Phase::Initialization,
                event: MetaEvent::ovm(RawEvent::PutPredicate(code_hash)),
                topics: vec![],
            }]
        );
    })
}
