//!
use super::Runtime;
/// Registered WASM contracts chain extensions.
///
use pallet_contracts::chain_extension::RegisteredChainExtension;

pub use pallet_chain_extension_dapps_staking::DappsStakingExtension;
pub use pallet_chain_extension_xvm::XvmExtension;

// Following impls defines chain extension IDs.

impl RegisteredChainExtension<Runtime> for DappsStakingExtension<Runtime> {
    const ID: u16 = 00;
}

impl RegisteredChainExtension<Runtime> for XvmExtension<Runtime> {
    const ID: u16 = 01;
}
