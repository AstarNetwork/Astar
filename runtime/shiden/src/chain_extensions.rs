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

use super::Runtime;

/// Registered WASM contracts chain extensions.
pub use pallet_chain_extension_assets::AssetsExtension;
use pallet_contracts::chain_extension::RegisteredChainExtension;

// Following impls defines chain extension IDs.
impl RegisteredChainExtension<Runtime> for AssetsExtension<Runtime> {
    const ID: u16 = 02;
}

pub type ShidenChainExtensions<Runtime> = AssetsExtension<Runtime>;
