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

//! End-to-end checks of the `pallet-contracts` decommission against the real runtimes rather than
//! a mock: the storage prefixes, the account id encoding and the migration ordering are all the
//! runtime's own.

use crate::setup::*;
use contracts_mbm::{TrieId, CONTRACT_INFO_OF};
use frame_support::{
    migrations::SteppedMigrations, storage::unhashed, weights::WeightMeter, StorageHasher,
    Twox64Concat,
};
use parity_scale_codec::Encode;
use sp_io::hashing::twox_128;

/// Writes the storage `pallet-contracts` left behind for one live contract: a `ContractInfoOf`
/// entry pointing at a populated child trie, plus the consumer reference the pallet took on the
/// contract account at instantiation.
fn plant_live_contract(contract: &AccountId32, trie_id: &[u8]) {
    let key = [
        twox_128(b"Contracts").to_vec(),
        twox_128(CONTRACT_INFO_OF).to_vec(),
        Twox64Concat::hash(&contract.encode()),
    ]
    .concat();
    let mut value = TrieId::try_from(trie_id.to_vec()).unwrap().encode();
    // Stands in for the rest of `ContractInfo`, which the migration never looks at.
    value.extend(core::iter::repeat(0xAB).take(96));
    unhashed::put_raw(&key, &value);

    sp_io::default_child_storage::set(trie_id, b"some-key", &[0xCD; 64]);

    frame_system::Pallet::<Runtime>::inc_providers(contract);
    frame_system::Pallet::<Runtime>::inc_consumers(contract).expect("the account has a provider");
}

/// Runs the runtime's own migration tuple to completion, the way `pallet-migrations` does.
fn run_multi_block_migrations() {
    for index in 0..MultiBlockMigrationsList::len() {
        let mut cursor = None;
        loop {
            let mut meter = WeightMeter::new();
            match MultiBlockMigrationsList::nth_step(index, cursor, &mut meter)
                .expect("the tuple has a migration at this index")
                .expect("the step succeeds")
            {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
    }
}

/// A live contract account carries a consumer reference taken by `pallet-contracts` at
/// instantiation and only released by `seal_terminate`. `ContractInfoOf` is the last on-chain
/// record of which accounts are contracts, so the purge is the last chance to hand it back -
/// without that, every contract account stays unreapable forever.
#[test]
fn purging_a_contract_releases_its_consumer_ref_and_child_trie() {
    new_test_ext().execute_with(|| {
        let contract = AccountId32::new([9_u8; 32]);
        let trie_id = [7_u8; 32];
        plant_live_contract(&contract, &trie_id);

        assert_eq!(frame_system::Pallet::<Runtime>::consumers(&contract), 1);

        run_multi_block_migrations();

        assert_eq!(frame_system::Pallet::<Runtime>::consumers(&contract), 0);
        assert_eq!(frame_system::Pallet::<Runtime>::providers(&contract), 1);
        assert!(sp_io::default_child_storage::next_key(&trie_id, &[]).is_none());
    });
}

/// Nothing may survive under the retired pallet prefixes; the indices themselves stay reserved.
#[test]
fn the_retired_pallet_prefixes_are_fully_purged() {
    new_test_ext().execute_with(|| {
        let contract = AccountId32::new([9_u8; 32]);
        plant_live_contract(&contract, &[7_u8; 32]);
        // A `PristineCode` blob, i.e. a key the child trie purge does not own.
        unhashed::put_raw(
            &[
                twox_128(b"Contracts").to_vec(),
                twox_128(b"PristineCode").to_vec(),
                Twox64Concat::hash(&[1_u8; 32]),
            ]
            .concat(),
            &[0xEF; 1024],
        );
        #[cfg(any(feature = "shiden", feature = "shibuya"))]
        unhashed::put_raw(
            &[
                twox_128(b"RandomnessCollectiveFlip").to_vec(),
                twox_128(b"RandomMaterial").to_vec(),
            ]
            .concat(),
            &[0xEF; 32],
        );

        run_multi_block_migrations();

        for prefix in [
            twox_128(b"Contracts"),
            twox_128(b"RandomnessCollectiveFlip"),
        ] {
            assert!(
                sp_io::storage::next_key(&prefix)
                    .filter(|key| key.starts_with(&prefix))
                    .is_none(),
                "keys left under a retired pallet prefix",
            );
        }
    });
}
