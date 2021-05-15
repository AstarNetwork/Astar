//! Plasm CLI library.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

/// Genesis token distributions.
mod balances;
/// Chain specifications.
pub mod chain_spec;

#[macro_use]
mod service;
#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "cli")]
mod command;

#[cfg(feature = "cli")]
pub use cli::*;
#[cfg(feature = "cli")]
pub use command::*;
