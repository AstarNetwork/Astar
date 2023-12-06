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

#![cfg_attr(not(feature = "std"), no_std)]

use astar_primitives::BlockNumber;
use pallet_dapp_staking_v3::EraNumber;

sp_api::decl_runtime_apis! {

    /// dApp Staking Api.
    ///
    /// Used to provide information otherwise not available via RPC.
    pub trait DappStakingApi {

        /// For how many standard era lengths does the voting subperiod last.
        fn eras_per_voting_subperiod() -> EraNumber;

        /// How many standard eras are there in the build&earn subperiod.
        fn eras_per_build_and_earn_subperiod() -> EraNumber;

        /// How many blocks are there per standard era.
        fn blocks_per_era() -> BlockNumber;
    }
}
