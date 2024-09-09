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

//! Astar chain specifications.

use super::Extensions;
use astar_primitives::parachain::ASTAR_ID;
use astar_runtime::wasm_binary_unwrap;
use sc_service::ChainType;

/// Specialized `ChainSpec` for Astar Network.
pub type AstarChainSpec = sc_service::GenericChainSpec<Extensions>;

/// Get Astar chain specification.
pub fn get_chain_spec() -> AstarChainSpec {
    let mut properties = serde_json::map::Map::new();
    properties.insert("tokenSymbol".into(), "ASTR".into());
    properties.insert("tokenDecimals".into(), 18.into());

    AstarChainSpec::builder(
        wasm_binary_unwrap(),
        Extensions {
            bad_blocks: Default::default(),
            relay_chain: "tokyo".into(),
            para_id: ASTAR_ID,
        },
    )
    .with_name("Astar Testnet")
    .with_id("astar")
    .with_chain_type(ChainType::Development)
    .with_properties(properties)
    .with_genesis_config(astar_runtime::genesis_config::default_config(ASTAR_ID))
    .build()
}
