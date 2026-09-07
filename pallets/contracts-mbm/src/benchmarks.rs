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

#![cfg(feature = "runtime-benchmarks")]

use crate::{
    purge::{clear_child_trie_metered, CONTRACT_INFO_OF},
    Config, Pallet, PurgeContractsChildTries, RemovePalletStepped,
};
use frame_benchmarking::v2::*;
use frame_support::{
    migrations::SteppedMigration,
    storage::unhashed,
    traits::Get,
    weights::WeightMeter,
    {StorageHasher, Twox64Concat},
};
use parity_scale_codec::Encode;
use sp_io::hashing::twox_128;
use sp_std::vec;

/// Upper bound of the value size component.
///
/// `pallet_contracts::Config::MaxCodeLen` was 123 KiB on every Astar runtime, and `PristineCode`
/// blobs are the largest values these migrations ever touch.
const MAX_VALUE_SIZE: u32 = 128 * 1024;

/// Pallet the benchmarks operate on. Only the length of the name matters for the measurement.
pub struct BenchPallet;
impl Get<&'static str> for BenchPallet {
    fn get() -> &'static str {
        "Contracts"
    }
}

type Weights<T> = crate::weights::SubstrateWeight<T>;

#[benchmarks]
mod benches {
    use super::*;

    /// Cost of seeking to, measuring and removing one top level key holding an `x` byte value.
    ///
    /// Runs the real `step` against a prefix holding exactly one key, so the seek, the length
    /// probe and the terminal `next_key` that ends the loop are all measured.
    #[benchmark]
    fn remove_key(x: Linear<0, MAX_VALUE_SIZE>) {
        let prefix = twox_128(BenchPallet::get().as_bytes()).to_vec();
        let key = [prefix.clone(), twox_128(b"PristineCode").to_vec()].concat();
        unhashed::put_raw(&key, &vec![0u8; x as usize]);

        let mut meter = WeightMeter::new();

        #[block]
        {
            RemovePalletStepped::<BenchPallet, Weights<T>>::step(None, &mut meter)
                .expect("migration step succeeds");
        }

        assert!(unhashed::get_raw(&key).is_none());
    }

    /// Cost of seeking to, measuring and removing one child trie key holding an `x` byte value.
    #[benchmark]
    fn remove_child_key(x: Linear<0, MAX_VALUE_SIZE>) {
        let trie_id = vec![1u8; crate::TRIE_ID_LEN];
        let key = twox_128(b"bench-key").to_vec();
        sp_io::default_child_storage::set(&trie_id, &key, &vec![0u8; x as usize]);

        let mut meter = WeightMeter::new();

        #[block]
        {
            clear_child_trie_metered::<Weights<T>>(&trie_id, &mut meter, 1);
        }

        assert!(sp_io::default_child_storage::next_key(&trie_id, &[]).is_none());
    }

    /// Cost of handing back the consumer reference `pallet-contracts` took on one contract
    /// account, including recovering that account id from its `ContractInfoOf` key.
    #[benchmark]
    fn release_contract_consumer() {
        let contract: T::AccountId = account("contract", 0, 0);
        frame_system::Pallet::<T>::inc_providers(&contract);
        frame_system::Pallet::<T>::inc_consumers(&contract).expect("account has a provider");

        let key = [
            PurgeContractsChildTries::<T, BenchPallet, Weights<T>>::map_prefix(CONTRACT_INFO_OF),
            Twox64Concat::hash(&contract.encode()),
        ]
        .concat();

        #[block]
        {
            PurgeContractsChildTries::<T, BenchPallet, Weights<T>>::release_consumer(&key);
        }

        assert_eq!(frame_system::Pallet::<T>::consumers(&contract), 0);
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Runtime);
}
