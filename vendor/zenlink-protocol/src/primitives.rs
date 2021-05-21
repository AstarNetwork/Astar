// Copyright 2020-2021 Zenlink
// Licensed under GPL-3.0.

use super::*;

pub type AssetBalance = u128;

/// Native currency
pub const NATIVE: u8 = 0;
/// Swap module asset
pub const LIQUIDITY: u8 = 1;
/// Other asset type on this chain
pub const LOCAL: u8 = 2;
/// Reserved for future
pub const RESERVED: u8 = 3;

/// AssetId use to locate assets in framed base chain.
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Default))]
pub struct AssetId {
    pub chain_id: u32,
    pub asset_type: u8,
    pub asset_index: u32,
}

impl AssetId {
    pub fn is_support(&self) -> bool {
        matches!(self.asset_type, NATIVE | LIQUIDITY | LOCAL | RESERVED)
    }

    pub fn is_native(&self, self_chain_id: u32) -> bool {
        self.chain_id == self_chain_id && self.asset_type == NATIVE && self.asset_index == 0
    }

    pub fn is_foreign(&self, self_chain_id: u32) -> bool {
        self.chain_id != self_chain_id
    }
}
