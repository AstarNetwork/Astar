//! Plasm CLI library.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

mod balances;
/// Plasm chain specification.
pub mod chain_spec;

#[macro_use]
mod service;
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
