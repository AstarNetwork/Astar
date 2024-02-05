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

#![cfg(test)]

use crate::setup::*;

use pallet_dapp_staking_v3::*;

#[test]
fn dapp_staking_triggers_inflation_recalculation() {
    new_test_ext().execute_with(|| {
        let init_inflation_config = pallet_inflation::ActiveInflationConfig::<Runtime>::get();

        let recalculation_era = init_inflation_config.recalculation_era;

        // It's not feasible to run through all the blocks needed to trigger all the eras.
        // Instead, we force the era to change on a block by block basis.
        while ActiveProtocolState::<Runtime>::get().era < recalculation_era - 1 {
            assert_ok!(DappStaking::force(RuntimeOrigin::root(), ForcingType::Era,));
            run_for_blocks(1);
            assert_eq!(
                init_inflation_config,
                pallet_inflation::ActiveInflationConfig::<Runtime>::get(),
                "Must not change until recalculation"
            );
        }
        assert_eq!(
            ActiveProtocolState::<Runtime>::get().subperiod(),
            Subperiod::BuildAndEarn,
            "Sanity check."
        );

        // Again, hacky approach to speed things up.
        // This doesn't influence anything in the protocol essentially.
        ActiveProtocolState::<Runtime>::mutate(|state| {
            state.next_era_start = System::block_number() + 5;
        });

        // Another sanity check, move block by block and ensure protocol works as expected.
        let target_block = ActiveProtocolState::<Runtime>::get().next_era_start;
        run_to_block(target_block - 2);
        assert_eq!(
            init_inflation_config,
            pallet_inflation::ActiveInflationConfig::<Runtime>::get(),
            "Sanity check."
        );

        // So far inflation config remained unchanged.
        // Now we expect the trigger which will update it.
        run_for_blocks(1);
        assert_eq!(
            init_inflation_config,
            pallet_inflation::ActiveInflationConfig::<Runtime>::get(),
            "Still the same, should be updated ONLY after the block has been finalized."
        );

        run_for_blocks(1);
        let new_inflation_config = pallet_inflation::ActiveInflationConfig::<Runtime>::get();
        assert_ne!(
            init_inflation_config, new_inflation_config,
            "Must be updated after the block has been finalized."
        );
    });
}
