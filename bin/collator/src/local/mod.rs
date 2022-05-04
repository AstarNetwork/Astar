//! Support for local development node.

/// Local development service.
mod service;

/// Development chain specs.
mod chain_spec;

pub use chain_spec::*;
pub use service::{new_partial, start_node, Executor, RuntimeApi};
