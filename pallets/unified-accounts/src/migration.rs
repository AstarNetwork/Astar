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

use super::{Config, Pallet, Weight};
use astar_primitives::evm::EvmAddress;
use frame_support::{
    pallet_prelude::OptionQuery,
    storage_alias,
    traits::{Get, OnRuntimeUpgrade},
    Blake2_128Concat,
};

#[storage_alias]
type EvmToNative<T: Config> = StorageMap<
    Pallet<T>,
    Blake2_128Concat,
    <T as frame_system::Config>::AccountId,
    EvmAddress,
    OptionQuery,
>;

#[storage_alias]
type NativeToEvm<T: Config> = StorageMap<
    Pallet<T>,
    Blake2_128Concat,
    EvmAddress,
    <T as frame_system::Config>::AccountId,
    OptionQuery,
>;

/// Remove all corrupted mappings.
pub struct ClearCorruptedUnifiedMappings<T>(core::marker::PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for ClearCorruptedUnifiedMappings<T> {
    fn on_runtime_upgrade() -> Weight {
        let healthy_count = crate::EvmToNative::<T>::iter().count() as u64 * 2;
        log::info!("Total healthy entries: {healthy_count}");

        let mut count = 0;
        // translate will fail to decode valid entries and therefore will skip it,
        // so this will remove only corrupt entries
        EvmToNative::<T>::translate(|key, value: EvmAddress| {
            log::debug!("Remove corrupt key: {key:?} with value: {value:?}");
            count += 1;
            None
        });
        NativeToEvm::<T>::translate(|key, value: T::AccountId| {
            log::debug!("Remove corrupt key: {key:?} with value: {value:?}");
            count += 1;
            None
        });
        log::info!("Removed {count} corrupt entries");
        T::DbWeight::get().reads_writes(healthy_count + count, count)
    }
}
