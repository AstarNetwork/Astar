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

use super::*;
use frame_support::{
    pallet_prelude::*,
    traits::{OnRuntimeUpgrade},
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_std::marker::PhantomData;

#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

/// One-time migration that removes outdated LastAuthoredBlock entries.
/// It keeps entries only for accounts that are currently:
///   - active candidates
///   - invulnerables
///
/// All other accounts are removed.
pub struct LastAuthoredBlockCleanup<T: Config>(PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for LastAuthoredBlockCleanup<T> {
    fn on_runtime_upgrade() -> Weight {
        log::info!("Running CollatorSelection::LastAuthoredBlockCleanup...");

        // Snapshot active identifiers for faster membership checks
        let invulnerables = Invulnerables::<T>::get();
        let candidate_accounts: sp_std::collections::btree_set::BTreeSet<T::AccountId> =
            Candidates::<T>::get()
                .into_iter()
                .map(|c| c.who)
                .collect();

        let mut removed = 0u64;
        let mut read = 0u64;
        let mut write = 0u64;

        // Use translate to selectively keep or drop keys
        LastAuthoredBlock::<T>::translate::<BlockNumberFor<T>, _>(|account, old_value| {
            read += 1;

            let is_invulnerable = invulnerables.contains(&account);
            let is_candidate = candidate_accounts.contains(&account);

            if is_invulnerable || is_candidate {
                // keep entry untouched
                Some(old_value)
            } else {
                // remove this entry
                removed += 1;
                write += 1;
                None
            }
        });

        log::info!(
            "LastAuthoredBlockCleanup completed: removed {} stale collator entries. Reads: {}. Writes: {}.",
            removed, read, write
        );

        let db = <T as frame_system::Config>::DbWeight::get();
        db.reads_writes(read, write)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
        let old_count = LastAuthoredBlock::<T>::iter().count() as u64;
        Ok(old_count.encode())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(data: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
        let old_count: u64 = Decode::decode(&mut &data[..])
            .map_err(|_| sp_runtime::TryRuntimeError::Other("Failed to decode pre-upgrade count"))?;

        let new_count = LastAuthoredBlock::<T>::iter().count() as u64;

        assert!(
            new_count < old_count,
            "LastAuthoredBlockCleanup: new count > old count (should only decrease)"
        );

        Ok(())
    }
}