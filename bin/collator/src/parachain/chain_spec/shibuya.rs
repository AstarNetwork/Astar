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

//! Shibuya chain specifications.

use super::Extensions;
use astar_primitives::parachain::SHIBUYA_ID;
use sc_service::ChainType;
use shibuya_runtime::wasm_binary_unwrap;

/// Specialized `ChainSpec` for Shibuya testnet.
pub type ShibuyaChainSpec = sc_service::GenericChainSpec<Extensions>;

/// Gen Shibuya chain specification.
pub fn get_chain_spec() -> ShibuyaChainSpec {
    let mut properties = serde_json::map::Map::new();
    properties.insert("tokenSymbol".into(), "SBY".into());
    properties.insert("tokenDecimals".into(), 18.into());

    ShibuyaChainSpec::builder(
        wasm_binary_unwrap(),
        Extensions {
            bad_blocks: Default::default(),
            relay_chain: "paseo".into(),
            para_id: SHIBUYA_ID,
        },
    )
    .with_name("Shibuya Testnet")
    .with_id("shibuya")
    .with_chain_type(ChainType::Development)
    .with_properties(properties)
    .with_genesis_config(shibuya_runtime::genesis_config::default_config(SHIBUYA_ID))
    .build()
}
