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

#![cfg_attr(not(feature = "std"), no_std)]

use astar_primitives::dapp_staking::{DAppId, EraNumber, PeriodNumber, RankedTier, TierId};
use astar_primitives::BlockNumber;
pub use sp_std::collections::btree_map::BTreeMap;

sp_api::decl_runtime_apis! {

    /// dApp Staking Api.
    ///
    /// Used to provide information otherwise not available via RPC.
    #[api_version(2)]
    pub trait DappStakingApi {

        /// How many periods are there in one cycle.
        fn periods_per_cycle() -> PeriodNumber;

        /// For how many standard era lengths does the voting subperiod last.
        fn eras_per_voting_subperiod() -> EraNumber;

        /// How many standard eras are there in the build&earn subperiod.
        fn eras_per_build_and_earn_subperiod() -> EraNumber;

        /// How many blocks are there per standard era.
        fn blocks_per_era() -> BlockNumber;

        /// Get dApp tier assignment for the given dApp.
        #[changed_in(2)]
        fn get_dapp_tier_assignment() -> BTreeMap<DAppId, TierId>;

        /// Get dApp ranked tier assignment for the given dApp.
        fn get_dapp_tier_assignment() -> BTreeMap<DAppId, RankedTier>;
    }
}
