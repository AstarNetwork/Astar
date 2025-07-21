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

use crate::test::mock::*;
use crate::{AccountLedger, CurrentEraInfo, EraInfo, Ledger, UnlockingChunk};
use frame_support::traits::OnRuntimeUpgrade;

#[test]
fn lazy_migrations() {
    ExtBuilder::default().build_and_execute(|| {
        Ledger::<Test>::set(
            &1,
            AccountLedger {
                locked: 1000,
                unlocking: vec![
                    UnlockingChunk {
                        amount: 100,
                        unlock_block: 5,
                    },
                    UnlockingChunk {
                        amount: 100,
                        unlock_block: 20,
                    },
                ]
                .try_into()
                .unwrap(),
                staked: Default::default(),
                staked_future: None,
                contract_stake_count: 0,
            },
        );
        CurrentEraInfo::<Test>::put(EraInfo {
            total_locked: 1000,
            unlocking: 200,
            current_stake_amount: Default::default(),
            next_stake_amount: Default::default(),
        });

        // go to block before migration
        run_to_block(9);

        // onboard MBMs
        AllPalletsWithSystem::on_runtime_upgrade();
        run_to_block(10);

        assert_eq!(
            Ledger::<Test>::get(&1),
            AccountLedger {
                locked: 1000,
                unlocking: vec![
                    UnlockingChunk {
                        amount: 100,
                        unlock_block: 5, // already unlocked
                    },
                    UnlockingChunk {
                        amount: 100,
                        unlock_block: 30, // double remaining blocks
                    },
                ]
                .try_into()
                .unwrap(),
                staked: Default::default(),
                staked_future: None,
                contract_stake_count: 0,
            }
        );
    })
}
