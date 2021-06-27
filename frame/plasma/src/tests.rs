//! Tests for the ovm module.

#![cfg(test)]

use super::{
    AddressInclusionProof, BalanceOf, Checkpoint, Config, InclusionProof, IntervalInclusionProof,
    IntervalTreeNode, IntervalTreeNodeOf, PlappsAddressFor, PropertyOf, Range, RangeOf, RawEvent,
};
use crate::mock::*;
use codec::Encode;
use frame_support::assert_ok;
use frame_system::{EventRecord, Phase};
use hex_literal::hex;
use sp_runtime::traits::Zero;

lazy_static::lazy_static! {
    pub static ref AGGREGATOR_ID: AccountId = to_account_from_seed(&hex![
            "1000000000000000000000000000000000000000000000000000000000005553"
    ]);

    pub static ref ERC20_ID: AccountId = to_account_from_seed(&hex![
            "1000000000000000000000000000000000000000000000000000000000005553"
    ]);
    pub static ref STATE_UPDATE_ID: AccountId = to_account_from_seed(&hex![
            "1000000000000000000000000000000000000000000000000000000000005553"
    ]);
    pub static ref EXIT_ID: AccountId = to_account_from_seed(&hex![
            "1000000000000000000000000000000000000000000000000000000000005553"
    ]);
    pub static ref EXIT_DEPOSIT_ID: AccountId = to_account_from_seed(&hex![
            "1000000000000000000000000000000000000000000000000000000000005553"
    ]);
}

fn success_deploy(
    sender: AccountId,
    aggregator_id: AccountId,
    erc20: AccountId,
    state_update_predicate: AccountId,
    exit_predicate: AccountId,
    exit_deposit_predicate: AccountId,
) -> AccountId {
    assert_ok!(Plasma::deploy(
        Origin::signed(sender.clone()),
        aggregator_id.clone(),
        erc20.clone(),
        state_update_predicate.clone(),
        exit_predicate.clone(),
        exit_deposit_predicate.clone()
    ));

    let plapps_id = <Test as Config>::DeterminePlappsAddress::plapps_address_for(
        &super::Module::<Test>::generate_plapps_hash(
            &aggregator_id,
            &erc20,
            &state_update_predicate,
            &exit_predicate,
            &exit_deposit_predicate,
        ),
        &sender,
    );

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
            event: Event::pallet_plasma(RawEvent::Deploy(sender, plapps_id.clone())),
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
            (*ALICE_STASH).clone(),
            (*AGGREGATOR_ID).clone(),
            (*ERC20_ID).clone(),
            (*STATE_UPDATE_ID).clone(),
            (*EXIT_ID).clone(),
            (*EXIT_DEPOSIT_ID).clone(),
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
        Origin::signed(sender.clone()),
        plapps_id.clone(),
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
            event: Event::pallet_plasma(RawEvent::BlockSubmitted(
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
            (*ALICE_STASH).clone(),
            (*AGGREGATOR_ID).clone(),
            (*ERC20_ID).clone(),
            (*STATE_UPDATE_ID).clone(),
            (*EXIT_ID).clone(),
            (*EXIT_DEPOSIT_ID).clone(),
        );

        advance_block();

        // 1-indexed.
        success_submit_root(
            (*AGGREGATOR_ID).clone(),
            plapps_id.clone(),
            1,
            H256::default(),
        );
        advance_block();
        success_submit_root(
            (*AGGREGATOR_ID).clone(),
            plapps_id.clone(),
            2,
            H256::default(),
        );
        advance_block();
        success_submit_root(
            (*AGGREGATOR_ID).clone(),
            plapps_id.clone(),
            3,
            H256::default(),
        );
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
        let token_address: AccountId = AccountId::new(hex![
            "0000000000000000000000000000000000000000000000000000000000000000"
        ]);
        let leaf_0: IntervalTreeNodeOf<Test> = IntervalTreeNodeOf::<Test> {
            start: 0,
            data: Keccak256::hash("leaf0".as_bytes()),
        };
        let leaf_1: IntervalTreeNodeOf<Test> = IntervalTreeNodeOf::<Test> {
            start: 7,
            data: Keccak256::hash("leaf1".as_bytes()),
        };
        let leaf_2: IntervalTreeNodeOf<Test> = IntervalTreeNodeOf::<Test> {
            start: 15,
            data: Keccak256::hash("leaf2".as_bytes()),
        };
        let leaf_3: IntervalTreeNodeOf<Test> = IntervalTreeNodeOf::<Test> {
            start: 5000,
            data: Keccak256::hash("leaf3".as_bytes()),
        };

        // interval tree root:
        // level0: [leaf_0, leaf_1, leaf_2, leaf_3];
        // level1: [compute_parent(leaf_0, leaf_1), compute_parent(leaf_2, leaf_3) ];
        // level2: [compute_parent(compute_parent(leaf_0, leaf_1), compute_parent(leaf_2, leaf_3))]
        // root = leve2[0].data

        let level_1 = vec![
            compute_parent(&leaf_0, &leaf_1),
            compute_parent(&leaf_2, &leaf_3),
        ];
        let level_2 = compute_parent(&level_1[0], &level_1[1]);
        let expected_root = level_2.data;
        println!(
            "expected level0: [{:?}, {:?}, {:?}, {:?}]",
            leaf_0, leaf_1, leaf_2, leaf_3
        );
        println!("expected level1: {:?}", level_1);
        println!("expected level2: {:?}", level_2);
        println!("expected root hash: {:?}", expected_root);

        let block_number: BlockNumber = 1;
        let root: H256 = H256::from(hex![
            "81b72772d1c85121dbedfb08fb8785ddd460c346b4d6225d3ede8fc00d0c487b"
        ]);

        // valid inclusion proof by leaf 0
        //                                    address_root          :(v address_inclusion_proof)
        //                                   /            \
        //                     interval_root              *[]*
        //                     /            \                       :(v interval_inclusion_proof)
        //        interval_node(0+1)        *interval_node(2+3)*
        //        /             \           /               \
        //  (leaf0)          *leaf1*   laef2                leaf3
        let valid_inclusion_proof: InclusionProof<AccountId, Balance, H256> =
            InclusionProof::<AccountId, Balance, H256> {
                address_inclusion_proof: AddressInclusionProof {
                    leaf_position: 0,
                    leaf_index: token_address.clone(),
                    siblings: vec![],
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
                                "7d69a61d7938bb29cd1e9658c46c0a5191b2e11c1a581d61f56ae8393533a9f5"
                            ]),
                        },
                    ],
                },
            };

        advance_block();
        // previous tests.

        let plapps_id = success_deploy(
            (*ALICE_STASH).clone(),
            (*AGGREGATOR_ID).clone(),
            (*ERC20_ID).clone(),
            (*STATE_UPDATE_ID).clone(),
            (*EXIT_ID).clone(),
            (*EXIT_DEPOSIT_ID).clone(),
        );

        advance_block();
        // 1-indexed.
        success_submit_root(
            (*AGGREGATOR_ID).clone(),
            plapps_id.clone(),
            block_number,
            root,
        );

        // suceed to verify inclusion of the most left leaf
        let result = Plasma::verify_inclusion(
            &plapps_id,
            &leaf_0.data,
            &token_address,
            &Range::<Balance> { start: 0, end: 5 },
            &valid_inclusion_proof,
            &block_number,
        );
        assert_eq!(result, Ok(true));
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
        Origin::signed(sender.clone().clone()),
        plapps_id.clone(),
        amount.clone(),
        initial_state.clone(),
    ));

    assert_eq!(
        Plasma::deposited_ranges(plapps_id.clone(), new_range.end),
        new_range,
    );
    assert_eq!(
        Plasma::total_deposited(&plapps_id),
        total_deposited + amount
    );
    assert!(System::events().ends_with(
        vec![
            EventRecord {
                phase: Phase::ApplyExtrinsic(0),
                event: Event::pallet_balances(pallet_balances::Event::Transfer(
                    sender,
                    plapps_id.clone(),
                    amount,
                )),
                topics: vec![],
            },
            EventRecord {
                phase: Phase::ApplyExtrinsic(0),
                event: Event::pallet_plasma(RawEvent::DepositedRangeExtended(
                    plapps_id.clone(),
                    new_range
                )),
                topics: vec![],
            },
            EventRecord {
                phase: Phase::ApplyExtrinsic(0),
                event: Event::pallet_plasma(RawEvent::CheckpointFinalized(
                    plapps_id.clone(),
                    checkpoint_id.clone(),
                    checkpoint.clone(),
                )),
                topics: vec![],
            }
        ]
        .as_slice()
    ));
    assert_eq!(Plasma::checkpoints(plapps_id.clone(), &checkpoint_id), true);
}

fn success_extend_deposited_ranges(sender: AccountId, plapps_id: AccountId, amount: Balance) {
    let total_deposited = Plasma::total_deposited(&plapps_id);
    let new_range = simulation_extend_ranges(&plapps_id, &amount);
    Plasma::bare_extend_deposited_ranges(&plapps_id, amount);
    assert_eq!(
        Plasma::deposited_ranges(plapps_id.clone(), new_range.end),
        new_range,
    );
    assert_eq!(
        Plasma::total_deposited(&plapps_id),
        total_deposited + amount
    );
    assert_eq!(
        System::events(),
        vec![EventRecord {
            phase: Phase::ApplyExtrinsic(0),
            event: Event::pallet_plasma(RawEvent::DepositedRangeExtended(plapps_id, new_range)),
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
    Plasma::bare_remove_deposited_range(&plapps_id, &range, &deposited_range_id);
    assert_eq!(
        System::events(),
        vec![EventRecord {
            phase: Phase::ApplyExtrinsic(0),
            event: Event::pallet_plasma(RawEvent::DepositedRangeRemoved(plapps_id.clone(), range)),
            topics: vec![],
        },]
    );
}

#[test]
fn scenario_test() {
    new_test_ext().execute_with(|| {
        advance_block();
        let plapps_id = success_deploy(
            (*ALICE_STASH).clone(),
            (*AGGREGATOR_ID).clone(),
            (*ERC20_ID).clone(),
            (*STATE_UPDATE_ID).clone(),
            (*EXIT_ID).clone(),
            (*EXIT_DEPOSIT_ID).clone(),
        );

        advance_block();
        success_deposit(
            (*ALICE_STASH).clone(),
            plapps_id.clone(),
            10,
            PropertyOf::<Test> {
                predicate_address: (*STATE_UPDATE_ID).clone(),
                inputs: vec![hex!["01"].to_vec()],
            },
        );
        println!("success deposit: 0");

        advance_block();
        success_deposit(
            (*BOB_STASH).clone(),
            plapps_id.clone(),
            30,
            PropertyOf::<Test> {
                predicate_address: (*STATE_UPDATE_ID).clone(),
                inputs: vec![hex!["01"].to_vec()],
            },
        );
        println!("success deposit: 1");

        advance_block();
        success_deposit(
            (*CHARLIE_STASH).clone(),
            plapps_id.clone(),
            80,
            PropertyOf::<Test> {
                predicate_address: (*STATE_UPDATE_ID).clone(),
                inputs: vec![hex!["01"].to_vec()],
            },
        );
        println!("success deposit: 2");

        advance_block();
        success_extend_deposited_ranges((*ALICE_STASH).clone(), plapps_id.clone(), 100);

        advance_block();
        success_remove_deposited_range(
            (*ALICE_STASH).clone(),
            plapps_id.clone(),
            Range {
                start: 120,
                end: 200,
            },
            220,
        );

        advance_block();
        success_remove_deposited_range(
            (*ALICE_STASH).clone(),
            plapps_id.clone(),
            Range {
                start: 200,
                end: 220,
            },
            220,
        );
    });
}

#[test]
fn scenario_with_ovm_test() {

    // 1. ovm::put_code.
    // 2. ovm::instantiate.
    // 3. plasma::deploy.
    // 4. plasma::submit_root
    // 5. plasma::deposit
    // 6.
}
