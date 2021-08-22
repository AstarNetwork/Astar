//! Opaque primitives for different parachain runtimes.

pub type BlockNumber = u32;
pub type Header = sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>;
pub type Block = sp_runtime::generic::Block<Header, sp_runtime::OpaqueExtrinsic>;
pub type Hash = sp_core::H256;
pub type Balance = u128;
pub type Nonce = u32;
pub type AccountId = sp_runtime::AccountId32;
