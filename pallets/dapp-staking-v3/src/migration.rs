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
use frame_support::traits::OnRuntimeUpgrade;

#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

/// Exports for versioned migration `type`s for this pallet.
pub mod versioned_migrations {
    use super::*;

    /// Migration V6 to V7 wrapped in a [`frame_support::migrations::VersionedMigration`], ensuring
    /// the migration is only performed when on-chain version is 6.
    pub type V6ToV7<T> = frame_support::migrations::VersionedMigration<
        6,
        7,
        v7::VersionUncheckedMigrateV6ToV7<T>,
        Pallet<T>,
        <T as frame_system::Config>::DbWeight,
    >;
}

/// Translate DAppTiers to include rank rewards.
mod v7 {
    use super::*;
    use crate::migration::v6::DAppTierRewards as DAppTierRewardsV6;

    pub struct VersionUncheckedMigrateV6ToV7<T>(PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for VersionUncheckedMigrateV6ToV7<T> {
        fn on_runtime_upgrade() -> Weight {
            let current = Pallet::<T>::current_storage_version();

            let mut translated = 0usize;
            DAppTiers::<T>::translate::<
                DAppTierRewardsV6<T::MaxNumberOfContracts, T::NumberOfTiers>,
                _,
            >(|_key, old_value| {
                translated.saturating_inc();
                Some(DAppTierRewards {
                    dapps: old_value.dapps,
                    rewards: old_value.rewards,
                    period: old_value.period,
                    rank_rewards: Default::default(),
                })
            });

            current.put::<Pallet<T>>();

            log::info!("Upgraded {translated} dAppTiers to {current:?}");

            T::DbWeight::get().reads_writes(1 + translated as u64, 1 + translated as u64)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
            Ok(Vec::new())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_data: Vec<u8>) -> Result<(), TryRuntimeError> {
            ensure!(
                Pallet::<T>::on_chain_storage_version() >= 7,
                "dapp-staking-v3::migration::v7: wrong storage version"
            );
            Ok(())
        }
    }
}

pub mod v6 {
    use astar_primitives::{
        dapp_staking::{DAppId, PeriodNumber, RankedTier},
        Balance,
    };
    use frame_support::{
        pallet_prelude::{Decode, Get},
        BoundedBTreeMap, BoundedVec,
    };

    /// Information about all of the dApps that got into tiers, and tier rewards
    #[derive(Decode)]
    pub struct DAppTierRewards<MD: Get<u32>, NT: Get<u32>> {
        /// DApps and their corresponding tiers (or `None` if they have been claimed in the meantime)
        pub dapps: BoundedBTreeMap<DAppId, RankedTier, MD>,
        /// Rewards for each tier. First entry refers to the first tier, and so on.
        pub rewards: BoundedVec<Balance, NT>,
        /// Period during which this struct was created.
        #[codec(compact)]
        pub period: PeriodNumber,
    }
}
