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

use super::{Runtime, Xvm};

/// Registered WASM contracts chain extensions.
pub use pallet_chain_extension_assets::AssetsExtension;
use pallet_contracts::chain_extension::RegisteredChainExtension;

pub use pallet_chain_extension_dapps_staking::DappsStakingExtension;
pub use pallet_chain_extension_xvm::XvmExtension;

// Following impls defines chain extension IDs.

impl RegisteredChainExtension<Runtime> for DappsStakingExtension<Runtime> {
    const ID: u16 = 00;
}

impl RegisteredChainExtension<Runtime> for XvmExtension<Runtime, Xvm> {
    const ID: u16 = 01;
}

impl<W: pallet_chain_extension_assets::weights::WeightInfo> RegisteredChainExtension<Runtime>
    for AssetsExtension<Runtime, W>
{
    const ID: u16 = 02;
}
