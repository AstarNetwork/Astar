// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

#![cfg(all(test, not(feature = "runtime-benchmarks")))]

use crate::{
    mock::{
        migrations_in_progress, new_test_ext, run_to_block, AllPalletsWithSystem, MaxServiceWeight,
        Purge, Remove, System,
    },
    TrieId, CONTRACT_INFO_OF, DELETION_QUEUE, TRIE_ID_LEN,
};
use frame_support::{
    migrations::{SteppedMigration, SteppedMigrationError},
    storage::unhashed,
    traits::OnRuntimeUpgrade,
    weights::{Weight, WeightMeter},
    StorageHasher, Twox64Concat,
};
use parity_scale_codec::Encode;
use sp_io::hashing::twox_128;

fn pallet_prefix() -> Vec<u8> {
    twox_128(b"Contracts").to_vec()
}

fn map_prefix(item: &[u8]) -> Vec<u8> {
    [twox_128(b"Contracts"), twox_128(item)].concat()
}

fn trie_id(seed: u8) -> Vec<u8> {
    vec![seed; TRIE_ID_LEN]
}

/// Mimics a `ContractInfoOf` entry: a `trie_id` followed by the remaining `ContractInfo` fields.
fn put_contract_info(who: u64, seed: u8) {
    let key = [
        map_prefix(CONTRACT_INFO_OF),
        Twox64Concat::hash(&who.encode()),
    ]
    .concat();
    let mut value = TrieId::try_from(trie_id(seed)).unwrap().encode();
    value.extend(core::iter::repeat(0xAB).take(96));
    unhashed::put_raw(&key, &value);
}

/// Mimics a `DeletionQueue` entry, i.e. the trie of an already terminated contract.
fn put_deletion_queue(nonce: u32, seed: u8) {
    let key = [
        map_prefix(DELETION_QUEUE),
        Twox64Concat::hash(&nonce.encode()),
    ]
    .concat();
    unhashed::put_raw(&key, &TrieId::try_from(trie_id(seed)).unwrap().encode());
}

/// Writes an arbitrary key under the pallet prefix, e.g. a `PristineCode` blob.
fn put_plain_entry(item: &[u8], nonce: u32, value_len: usize) {
    let key = [map_prefix(item), twox_128(&nonce.to_le_bytes()).to_vec()].concat();
    unhashed::put_raw(&key, &vec![0xCD; value_len]);
}

fn populate_child_trie(seed: u8, entries: u32) {
    for i in 0..entries {
        sp_io::default_child_storage::set(&trie_id(seed), &i.to_le_bytes(), &[i as u8; 64]);
    }
}

fn child_trie_is_empty(seed: u8) -> bool {
    sp_io::default_child_storage::next_key(&trie_id(seed), &[]).is_none()
}

fn prefix_is_empty(prefix: &[u8]) -> bool {
    sp_io::storage::next_key(prefix)
        .filter(|key| key.starts_with(prefix))
        .is_none()
}

/// Registers an account the way `pallet-contracts` leaves a live contract account: a provider for
/// its existential deposit, plus the consumer reference taken at instantiation.
fn new_contract_account(who: u64) {
    System::inc_providers(&who);
    System::inc_consumers(&who).expect("the account has a provider");
}

/// Onboards the multi block migrations the way a runtime upgrade does, then runs blocks until
/// `pallet-migrations` reports it is done. Returns how many blocks that took.
fn run_migrations_to_completion() -> u64 {
    AllPalletsWithSystem::on_runtime_upgrade();

    let start = System::block_number();
    let mut current = start;
    while migrations_in_progress() {
        current += 1;
        run_to_block(current);
        assert!(current - start < 1_000, "migrations did not converge");
    }
    current - start
}

/// Drives a single migration to completion outside `pallet-migrations`, one meter per step.
fn run_steps<M: SteppedMigration>(limit: Weight) -> u32 {
    let mut cursor = None;
    for step in 1..1_000 {
        let mut meter = WeightMeter::with_limit(limit);
        match M::step(cursor, &mut meter).expect("migration step succeeds") {
            Some(next) => cursor = Some(next),
            None => return step,
        }
    }
    panic!("migration did not converge")
}

#[test]
fn purge_then_remove_clears_the_whole_pallet() {
    new_test_ext().execute_with(|| {
        // A budget tight enough that neither migration can finish within a single block.
        MaxServiceWeight::set(Weight::from_parts(50_000_000_000, 200_000));
        // Two live contracts and one already terminated, each with its own child trie, plus the
        // top level keys (code blobs, ...) the removal has to sweep afterwards.
        new_contract_account(1);
        new_contract_account(2);
        put_contract_info(1, 1);
        put_contract_info(2, 2);
        put_deletion_queue(0, 3);
        for seed in [1, 2, 3] {
            populate_child_trie(seed, 200);
        }
        for i in 0..20 {
            put_plain_entry(b"PristineCode", i, 8 * 1024);
        }

        let blocks = run_migrations_to_completion();

        assert!(
            blocks > 1,
            "should have spanned several blocks, took {blocks}"
        );
        for seed in [1, 2, 3] {
            assert!(
                child_trie_is_empty(seed),
                "child trie {seed} still populated"
            );
        }
        assert!(prefix_is_empty(&pallet_prefix()));
    });
}

#[test]
fn live_contract_accounts_get_their_consumer_ref_back() {
    // `pallet-contracts` takes a consumer ref at instantiation and only returns it on
    // `seal_terminate`. `ContractInfoOf` is the last record of which accounts are contracts, so
    // dropping the pallet without this would leave every one of them unreapable forever.
    new_test_ext().execute_with(|| {
        new_contract_account(1);
        put_contract_info(1, 1);
        populate_child_trie(1, 10);
        assert_eq!(System::consumers(&1), 1);

        run_migrations_to_completion();

        assert_eq!(System::consumers(&1), 0);
        // Only the reference `pallet-contracts` owned is handed back; the account survives.
        assert_eq!(System::providers(&1), 1);
    });
}

#[test]
fn only_the_contracts_own_consumer_ref_is_handed_back() {
    new_test_ext().execute_with(|| {
        new_contract_account(1);
        // Some other pallet also holds a reference on the same account.
        System::inc_consumers(&1).expect("the account has a provider");
        put_contract_info(1, 1);

        run_migrations_to_completion();

        assert_eq!(System::consumers(&1), 1);
    });
}

#[test]
fn terminated_contracts_keep_their_consumer_count() {
    // `DeletionQueue` entries belong to contracts that already ran `seal_terminate`, which gave
    // the reference back. Decrementing again would underflow someone else's.
    new_test_ext().execute_with(|| {
        new_contract_account(1);
        put_deletion_queue(0, 3);
        populate_child_trie(3, 10);

        run_migrations_to_completion();

        assert!(child_trie_is_empty(3));
        assert_eq!(System::consumers(&1), 1);
    });
}

#[test]
fn purge_drops_entries_without_a_usable_trie_id() {
    // A 16 byte blob decodes fine as a `BoundedVec<u8, 128>` and a 400 byte one does not decode at
    // all: both are the signature of a wrong field offset, and neither addresses a child trie.
    new_test_ext().execute_with(|| {
        for (nonce, garbage) in [(0u32, vec![0x11u8; 16]), (1, vec![0xFFu8; 400])] {
            let key = [
                map_prefix(CONTRACT_INFO_OF),
                Twox64Concat::hash(&nonce.encode()),
            ]
            .concat();
            unhashed::put_raw(&key, &garbage.encode());
        }

        run_migrations_to_completion();

        assert!(prefix_is_empty(&map_prefix(CONTRACT_INFO_OF)));
    });
}

#[test]
fn removing_the_pallet_first_would_orphan_the_child_trie() {
    // Documents *why* the ordering in the runtimes matters: the `trie_id` only lives inside the
    // pallet prefix, so wiping it first makes the child trie unreachable for good.
    new_test_ext().execute_with(|| {
        put_contract_info(1, 1);
        populate_child_trie(1, 10);

        run_steps::<Remove>(MaxServiceWeight::get());

        assert!(prefix_is_empty(&pallet_prefix()));
        assert!(
            !child_trie_is_empty(1),
            "orphaned child trie - exactly what PurgeContractsChildTries prevents"
        );
    });
}

#[test]
fn a_step_that_can_afford_nothing_reports_insufficient_weight() {
    new_test_ext().execute_with(|| {
        put_contract_info(1, 1);
        put_plain_entry(b"PristineCode", 0, 1024);

        let tiny = Weight::from_parts(1, 1);
        for outcome in [
            Purge::step(None, &mut WeightMeter::with_limit(tiny)),
            Remove::step(None, &mut WeightMeter::with_limit(tiny)).map(|cursor| cursor.map(|_| ())),
        ] {
            assert!(matches!(
                outcome,
                Err(SteppedMigrationError::InsufficientWeight { .. })
            ));
        }

        // `Err` rolls the step back, so nothing may have been removed.
        assert!(!prefix_is_empty(&pallet_prefix()));
    });
}

#[test]
fn is_a_no_op_on_empty_storage() {
    new_test_ext().execute_with(|| {
        assert_eq!(run_migrations_to_completion(), 1);
        assert!(!migrations_in_progress());
    });
}

/// The migrations address storage by name rather than through pallet types, so a typo would
/// silently make them no-ops. These values were read off Astar and Shiden mainnet state, where
/// the pallet prefix resolves to 781 and 1244 live keys respectively.
#[test]
fn storage_prefixes_match_mainnet() {
    let hex = |bytes: &[u8]| {
        bytes.iter().fold(String::from("0x"), |mut acc, b| {
            acc.push_str(&format!("{b:02x}"));
            acc
        })
    };

    assert_eq!(
        hex(&pallet_prefix()),
        "0x4342193e496fab7ec59d615ed0dc5530",
        "Contracts"
    );
    assert_eq!(
        hex(&map_prefix(CONTRACT_INFO_OF)),
        "0x4342193e496fab7ec59d615ed0dc5530060e99e5378e562537cf3bc983e17b91",
        "Contracts::ContractInfoOf"
    );
    assert_eq!(
        hex(&map_prefix(DELETION_QUEUE)),
        "0x4342193e496fab7ec59d615ed0dc553029162111ad19ef145155ee552aef2d11",
        "Contracts::DeletionQueue"
    );
    assert_eq!(
        hex(&twox_128(b"RandomnessCollectiveFlip")),
        "0xbd2a529379475088d3e29a918cd47872",
        "RandomnessCollectiveFlip"
    );
}

#[cfg(feature = "try-runtime")]
#[test]
fn post_upgrade_catches_an_orphaned_child_trie() {
    new_test_ext().execute_with(|| {
        put_contract_info(1, 1);
        populate_child_trie(1, 10);

        let state = Purge::pre_upgrade().expect("snapshot succeeds");
        // Simulates the failure mode: the pointers are gone but the trie was never emptied.
        run_steps::<Remove>(MaxServiceWeight::get());

        assert!(Purge::post_upgrade(state).is_err());
    });
}
