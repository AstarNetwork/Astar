use sc_chain_spec::ChainSpecExtension;
use serde::{Deserialize, Serialize};

pub mod shibuya;
pub mod shiden;

pub use shibuya::ShibuyaChainSpec;
pub use shiden::ShidenChainSpec;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

// TODO: add helper methods here. The problem is that they don't have the same type so how to best handle it? Generics? Introduction of a shared type?
