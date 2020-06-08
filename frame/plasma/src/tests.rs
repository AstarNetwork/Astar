//! Tests for the ovm module.

#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::assert_ok;
use frame_system::{EventRecord, Phase};
use hex_literal::hex;

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

#[test]
fn verify_inclusion_test() {
    /*
     * Tree for test case
     *         root
     *         / \
     *    root0   root1
     *    / \
     *  / \ / \   /   \
     * 0  1 2  3 1-0 1-1
     */
    new_test_ext().execute_with(|| {
        let token_address: AccountId = 1;
        let leaf_0: H256 = BlakeTwo256::hash("leaf0".as_bytes());
        let leaf_1: H256 = BlakeTwo256::hash("leaf1".as_bytes());
        let leaf_2: H256 = BlakeTwo256::hash("leaf2".as_bytes());
        let leaf_3: H256 = BlakeTwo256::hash("leaf3".as_bytes());
        let block_number: BlockNumber = 1;
        let root: H256 = H256::from(hex![
            "1aa3429d5aa7bf693f3879fdfe0f1a979a4b49eaeca9638fea07ad7ee5f0b64f"
        ]);
        let valid_inclusion_proof: InclusionProof<AccountId, Balance, H256> =
            InclusionProof::<AccountId, Balance, H256> {
                address_inclusion_proof: AddressInclusionProof {
                    leaf_position: 0,
                    leaf_index: 0,
                    siblings: vec![AddressTreeNode {
                        token_address: 1,
                        data: H256::from(hex![
                            "dd779be20b84ced84b7cbbdc8dc98d901ecd198642313d35d32775d75d916d3a"
                        ]),
                    }],
                },
                interval_inclusion_proof: IntervalInclusionProof {
                    leaf_position: 0,
                    leaf_index: 0,
                    siblings: vec![
                        IntervalTreeNode {
                            start: 7,
                            data: H256::from(hex![
                                "036491cc10808eeb0ff717314df6f19ba2e232d04d5f039f6fa382cae41641da"
                            ]),
                        },
                        IntervalTreeNode {
                            start: 5000,
                            data: H256::from(hex![
                                "ef583c07cae62e3a002a9ad558064ae80db17162801132f9327e8bb6da16ea8a"
                            ]),
                        },
                    ],
                },
            };

        advance_block();
        // previous tests.

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
        success_submit_root(AGGREGATOR_ID, plapps_id.clone(), block_number, root);

        // suceed to verify inclusion of the most left leaf
        let result = Plasma::verify_inclusion(
            plapps_id,
            leaf_0.clone(),
            token_address,
            Range::<Balance> { start: 0, end: 5 },
            valid_inclusion_proof,
            block_number,
        );
        // TODO: shuld be passed true.
        assert_eq!(result, Ok(false));
    })
}

fn simulation_extend_ranges(plapps_id: &AccountId, amount: &Balance) -> RangeOf<Test> {
    let total_deposited = Plasma::total_deposited(plapps_id);
    let old_range = Plasma::deposited_ranges(plapps_id, &total_deposited);
    let new_start = if old_range.start == BalanceOf::<Test>::zero()
        && old_range.end == BalanceOf::<Test>::zero()
    {
        total_deposited
    } else {
        old_range.start
    };

    let new_end = total_deposited.saturating_add(amount.clone());
    Range {
        start: new_start,
        end: new_end,
    }
}

fn success_deposit(
    sender: AccountId,
    plapps_id: AccountId,
    amount: BalanceOf<Test>,
    initial_state: PropertyOf<Test>,
    gas_limit: Gas,
) {
    let total_deposited = Plasma::total_deposited(&plapps_id);
    let deposit_range = RangeOf::<Test> {
        start: total_deposited,
        end: total_deposited.saturating_add(amount.clone()),
    };
    let new_range = simulation_extend_ranges(&plapps_id, &amount);
    let state_update = PropertyOf::<Test> {
        predicate_address: Plasma::state_update_predicate(&plapps_id),
        inputs: vec![
            plapps_id.encode(),
            deposit_range.encode(),
            Plasma::get_latest_plasma_block_number(&plapps_id).encode(),
            initial_state.encode(),
        ],
    };
    let checkpoint = Checkpoint {
        state_update: state_update,
    };
    let checkpoint_id = Plasma::get_checkpoint_id(&checkpoint);

    assert_ok!(Plasma::deposit(
        Origin::signed(sender.clone()),
        plapps_id,
        amount,
        initial_state.clone(),
        gas_limit,
    ));

    assert_eq!(
        Plasma::deposited_ranges(plapps_id, new_range.end),
        new_range,
    );
    assert_eq!(Plasma::total_deposited(plapps_id), total_deposited + amount);
    assert_eq!(
        System::events(),
        vec![
            EventRecord {
                phase: Phase::ApplyExtrinsic(0),
                event: MetaEvent::plasma(RawEvent::DepositedRangeExtended(plapps_id, new_range)),
                topics: vec![],
            },
            EventRecord {
                phase: Phase::ApplyExtrinsic(0),
                event: MetaEvent::plasma(RawEvent::CheckpointFinalized(
                    plapps_id.clone(),
                    checkpoint_id.clone(),
                    checkpoint.clone(),
                )),
                topics: vec![],
            }
        ]
    );
    assert_eq!(Plasma::checkpoints(plapps_id.clone(), &checkpoint_id), true);
}

fn success_extend_deposited_ranges(sender: AccountId, plapps_id: AccountId, amount: Balance) {
    let total_deposited = Plasma::total_deposited(plapps_id);
    let new_range = simulation_extend_ranges(&plapps_id, &amount);
    assert_ok!(Plasma::extend_deposited_ranges(
        Origin::signed(sender),
        plapps_id,
        amount
    ));
    assert_eq!(
        Plasma::deposited_ranges(plapps_id, new_range.end),
        new_range,
    );
    assert_eq!(Plasma::total_deposited(plapps_id), total_deposited + amount);
    assert_eq!(
        System::events(),
        vec![EventRecord {
            phase: Phase::ApplyExtrinsic(0),
            event: MetaEvent::plasma(RawEvent::DepositedRangeExtended(plapps_id, new_range)),
            topics: vec![],
        },]
    );
}

fn success_remove_deposited_range(
    sender: AccountId,
    plapps_id: AccountId,
    range: RangeOf<Test>,
    deposited_range_id: Balance,
) {
    assert_ok!(Plasma::remove_deposited_range(
        Origin::signed(sender),
        plapps_id,
        range.clone(),
        deposited_range_id
    ));
    assert_eq!(
        System::events(),
        vec![EventRecord {
            phase: Phase::ApplyExtrinsic(0),
            event: MetaEvent::plasma(RawEvent::DepositedRangeRemoved(plapps_id, range)),
            topics: vec![],
        },]
    );
}

#[test]
fn scenario_test() {
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
        success_deposit(
            ALICE_STASH,
            plapps_id,
            10,
            PropertyOf::<Test> {
                predicate_address: STATE_UPDATE_ID,
                inputs: vec![hex!["01"].to_vec()],
            },
            1000000,
        );

        advance_block();
        success_deposit(
            BOB_STASH,
            plapps_id,
            30,
            PropertyOf::<Test> {
                predicate_address: STATE_UPDATE_ID,
                inputs: vec![hex!["01"].to_vec()],
            },
            1000000,
        );

        advance_block();
        success_deposit(
            CHARLIE_STASH,
            plapps_id,
            80,
            PropertyOf::<Test> {
                predicate_address: STATE_UPDATE_ID,
                inputs: vec![hex!["01"].to_vec()],
            },
            1000000,
        );

        advance_block();
        success_extend_deposited_ranges(ALICE_STASH, plapps_id, 100);

        advance_block();
        success_remove_deposited_range(
            ALICE_STASH,
            plapps_id,
            Range {
                start: 120,
                end: 200,
            },
            220,
        );

        advance_block();
        success_remove_deposited_range(
            ALICE_STASH,
            plapps_id,
            Range {
                start: 200,
                end: 220,
            },
            220,
        );
    });
}
