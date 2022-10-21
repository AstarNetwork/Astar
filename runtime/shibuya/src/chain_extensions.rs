//!
use super::Runtime;
/// Registered WASM contracts chain extensions.
///
use pallet_contracts::chain_extension::RegisteredChainExtension;

pub use pallet_chain_extension_dapps_staking::DappsStakingExtension;

// Following impls defines chain extension IDs.

impl RegisteredChainExtension<Runtime> for DappsStakingExtension<Runtime> {
    const ID: u16 = 00;
}
