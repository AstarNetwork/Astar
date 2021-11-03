//! Astar collator library.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

/// Development node support.
pub mod local;
/// Parachain node support.
pub mod parachain;

mod cli;
mod command;
mod primitives;
mod rpc;

pub use cli::*;
pub use command::*;
