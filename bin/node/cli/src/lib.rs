//! Plasm CLI library.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

/// Plasm chain specification.
pub mod chain_spec;

#[macro_use]
mod service;
#[cfg(feature = "browser")]
mod browser;
#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "cli")]
mod command;

#[cfg(feature = "browser")]
pub use browser::*;
#[cfg(feature = "cli")]
pub use cli::*;
#[cfg(feature = "cli")]
pub use command::*;

/// The chain specification option.
#[derive(Clone, Debug, PartialEq)]
pub enum ChainSpec {
    /// Whatever the current runtime is, with just Alice as an auth.
    Development,
    /// Whatever the current runtime is, with simple Alice/Bob auths.
    LocalTestnet,
    /// Whatever the current runtime is with the "global testnet" defaults.
    PlasmTestnet,
}

/// Get a chain config from a spec setting.
impl ChainSpec {
    pub(crate) fn load(self) -> Result<chain_spec::ChainSpec, String> {
        Ok(match self {
            ChainSpec::Development => chain_spec::development_config(),
            ChainSpec::LocalTestnet => chain_spec::local_testnet_config(),
            ChainSpec::PlasmTestnet => chain_spec::plasm_testnet_config(),
        })
    }

    pub(crate) fn from(s: &str) -> Option<Self> {
        match s {
            "dev" => Some(ChainSpec::Development),
            "local" => Some(ChainSpec::LocalTestnet),
            "" | "testnet" => Some(ChainSpec::PlasmTestnet),
            _ => None,
        }
    }
}

fn load_spec(id: &str) -> Result<Box<dyn sc_chain_spec::ChainSpec>, String> {
    Ok(match ChainSpec::from(id) {
        Some(spec) => Box::new(spec.load()?),
        None => Box::new(ChainSpec::PlasmTestnet.load()?),
    })
}
