// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
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

//! Migrate CodeInfo from faulty alias introduced on contracts's v12 migration

use frame_support::{
    pallet_prelude::*, storage_alias, traits::fungible::Inspect, DefaultNoBound, Identity,
};
use pallet_contracts::{
    migration::{IsFinished, MigrationStep},
    weights::WeightInfo,
    Config, Determinism, Pallet,
};
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "try-runtime")]
use scale_info::prelude::format;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
use sp_std::marker::PhantomData;
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

const LOG_TARGET: &str = "runtime::contracts";

type BalanceOf<T> =
    <<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type CodeHash<T> = <T as frame_system::Config>::Hash;
type CodeVec<T> = BoundedVec<u8, <T as Config>::MaxCodeLen>;

mod old {
    use super::*;

    #[storage_alias]
    pub type CodeInfoOf<T: Config> = StorageMap<Pallet<T>, Twox64Concat, CodeHash<T>, CodeInfo<T>>;
}

#[derive(Encode, Decode, scale_info::TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct CodeInfo<T: Config> {
    owner: AccountIdOf<T>,
    #[codec(compact)]
    deposit: BalanceOf<T>,
    #[codec(compact)]
    refcount: u64,
    determinism: Determinism,
    code_len: u32,
}

#[storage_alias]
pub type CodeInfoOf<T: Config> = StorageMap<Pallet<T>, Identity, CodeHash<T>, CodeInfo<T>>;

#[storage_alias]
pub type PristineCode<T: Config> = StorageMap<Pallet<T>, Identity, CodeHash<T>, CodeVec<T>>;

#[derive(Encode, Decode, MaxEncodedLen, DefaultNoBound)]
pub struct Migration<T: Config> {
    last_code_hash: Option<CodeHash<T>>,
    _phantom: PhantomData<T>,
}

/// Logic as follows,
/// Since we need to modifiy `CodeInfoOf` mapping we cannot use `iter()` or `drain()` on it as
/// that will be undefined behaviour, so we are iterating over keys of `PristineCode` mappings
/// which are code hashes.
///
/// Migration Weights: Reusing v12 migration weights as most heavy operation which is moving
/// code info is same.
impl<T: Config> MigrationStep for Migration<T> {
    const VERSION: u16 = 15;

    fn max_step_weight() -> Weight {
        T::WeightInfo::v12_migration_step(T::MaxCodeLen::get())
    }

    fn step(&mut self) -> (IsFinished, Weight) {
        let mut iter = if let Some(last_key) = self.last_code_hash.take() {
            PristineCode::<T>::iter_keys_from(PristineCode::<T>::hashed_key_for(last_key))
        } else {
            PristineCode::<T>::iter_keys()
        };

        if let Some(code_hash) = iter.next() {
            if let Some(code_info) = old::CodeInfoOf::<T>::take(code_hash) {
                log::debug!(
                    target: LOG_TARGET,
                    "Migrating CodeInfoOf for code_hash {:?}",
                    code_hash
                );

                let code_len = code_info.code_len;

                CodeInfoOf::<T>::insert(code_hash, code_info);

                self.last_code_hash = Some(code_hash);
                (IsFinished::No, T::WeightInfo::v12_migration_step(code_len))
            } else {
                log::warn!(
                    target: LOG_TARGET,
                    "No CodeInfo found for code_hash {:?}, maybe new contract?",
                    code_hash
                );
                // old CodeInfo not found, it's newly deployed contract
                self.last_code_hash = Some(code_hash);
                (IsFinished::No, T::WeightInfo::v12_migration_step(0))
            }
        } else {
            log::debug!(target: LOG_TARGET, "No more CodeInfo to migrate");
            (IsFinished::Yes, T::WeightInfo::v12_migration_step(0))
        }
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade_step() -> Result<Vec<u8>, TryRuntimeError> {
        let len = 100;
        let sample: Vec<_> = old::CodeInfoOf::<T>::iter_keys().take(len).collect();
        log::debug!(
            target: LOG_TARGET,
            "Taking sample of {} CodeInfoOf(s)",
            sample.len()
        );

        Ok(sample.encode())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade_step(state: Vec<u8>) -> Result<(), TryRuntimeError> {
        let state = <Vec<CodeHash<T>> as Decode>::decode(&mut &state[..]).unwrap();

        log::debug!(
            target: LOG_TARGET,
            "Validating state of {} Codeinfo(s)",
            state.len()
        );
        for hash in state {
            ensure!(
                old::CodeInfoOf::<T>::get(&hash).is_none(),
                "Old CodeInfoFor is not none!"
            );
            let _ = CodeInfoOf::<T>::get(&hash)
                .expect(format!("CodeInfo for code_hash {:?} not found!", hash).as_str());
        }
        Ok(())
    }
}
