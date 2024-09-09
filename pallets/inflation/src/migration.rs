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
use frame_support::pallet_prelude::Weight;
use frame_support::traits::OnRuntimeUpgrade;

/// Half block reward for collators and treasury
pub struct AdjustBlockRewardMigration<T>(core::marker::PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for AdjustBlockRewardMigration<T> {
    fn on_runtime_upgrade() -> Weight {
        log::info!("🚚 migrated to async backing, adjust reward per block");
        ActiveInflationConfig::<T>::mutate_exists(|maybe| {
            if let Some(config) = maybe {
                config.collator_reward_per_block =
                    config.collator_reward_per_block.saturating_div(2);
                config.treasury_reward_per_block =
                    config.treasury_reward_per_block.saturating_div(2);
            }
        });
        T::DbWeight::get().reads_writes(1, 1)
    }
}
