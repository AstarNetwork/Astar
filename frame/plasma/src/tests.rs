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

fn success_deploy(
    sender: AccountId,
    aggregator_id: AccountId,
    erc20: AccountId,
    state_update_predicate: AccountId,
    exit_predicate: AccountId,
    exit_deposit_predicate: AccountId,
) -> AccountId {
    assert_ok!(Plasma::deploy(
        Origin::signed(sender),
        aggregator_id,
        erc20,
        state_update_predicate,
        exit_predicate,
        exit_deposit_predicate
    ));

    let plapps_id = DummyPlappsAddressFor::plapps_address_for(&H256::default(), &sender);

    // check initail config ids
    assert_eq!(Plasma::aggregator_address(&plapps_id), aggregator_id);
    assert_eq!(Plasma::erc20(&plapps_id), erc20);
    assert_eq!(
        Plasma::state_update_predicate(&plapps_id),
        state_update_predicate
    );
    assert_eq!(Plasma::exit_predicate(&plapps_id), exit_predicate);
    assert_eq!(
        Plasma::exit_deposit_predicate(&plapps_id),
        exit_deposit_predicate
    );
    assert_eq!(
        System::events(),
        vec![EventRecord {
            phase: Phase::ApplyExtrinsic(0),
            event: MetaEvent::plasma(RawEvent::Deploy(sender, plapps_id)),
            topics: vec![],
        }]
    );
    return plapps_id;
}

#[test]
fn deploy_sucess() {
    new_test_ext().execute_with(|| {
        advance_block();
        success_deploy(
            ALICE_STASH,
            AGGREGATOR_ID,
            ERC20_ID,
            STATE_UPDATE_ID,
            EXIT_ID,
            EXIT_DEPOSIT_ID,
        );
    })
}

fn success_submit_root(
    sender: AccountId,
    plapps_id: AccountId,
    block_number: BlockNumber,
    root: H256,
) -> (AccountId, BlockNumber, H256) {
    assert_ok!(Plasma::submit_root(
        Origin::signed(sender),
        plapps_id,
        block_number,
        root,
    ));

    // check update blocks
    assert_eq!(Plasma::blocks(&plapps_id, &block_number), root);
    // check upadte current block
    assert_eq!(Plasma::current_block(&plapps_id), block_number);
    assert_eq!(
        System::events(),
        vec![EventRecord {
            phase: Phase::ApplyExtrinsic(0),
            event: MetaEvent::plasma(RawEvent::BlockSubmitted(
                plapps_id.clone(),
                block_number.clone(),
                root.clone()
            )),
            topics: vec![],
        }]
    );
    return (plapps_id, block_number, root);
}

#[test]
fn submit_root_success() {
    new_test_ext().execute_with(|| {
        advance_block();
        let plapps_id = success_deploy(
            ALICE_STASH,
            AGGREGATOR_ID,
            ERC20_ID,
            STATE_UPDATE_ID,
            EXIT_ID,
            EXIT_DEPOSIT_ID,
        );

        advance_block();

        // 1-indexed.
        success_submit_root(AGGREGATOR_ID, plapps_id.clone(), 1, H256::default());
        advance_block();
        success_submit_root(AGGREGATOR_ID, plapps_id.clone(), 2, H256::default());
        advance_block();
        success_submit_root(AGGREGATOR_ID, plapps_id.clone(), 3, H256::default());
    })
}
