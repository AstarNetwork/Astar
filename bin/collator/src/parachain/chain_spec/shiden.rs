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

//! Shiden chain specifications.

use super::Extensions;
use astar_primitives::parachain::SHIDEN_ID;
use sc_service::ChainType;
use shiden_runtime::wasm_binary_unwrap;

/// Specialized `ChainSpec` for Shiden Network.
pub type ShidenChainSpec =
    sc_service::GenericChainSpec<shiden_runtime::RuntimeGenesisConfig, Extensions>;

/// Gen Shiden chain specification.
pub fn get_chain_spec() -> ShidenChainSpec {
    let mut properties = serde_json::map::Map::new();
    properties.insert("tokenSymbol".into(), "SDN".into());
    properties.insert("tokenDecimals".into(), 18.into());

    ShidenChainSpec::builder(
        wasm_binary_unwrap(),
        Extensions {
            bad_blocks: Default::default(),
            relay_chain: "tokyo".into(),
            para_id: SHIDEN_ID,
        },
    )
    .with_name("Shiden Testnet")
    .with_id("shiden")
    .with_chain_type(ChainType::Development)
    .with_properties(properties)
    .with_genesis_config(shiden_runtime::genesis_config::default_config(SHIDEN_ID))
    .build()
}
