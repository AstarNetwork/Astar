//! Support for local development node.

/// Local development service.
mod service;

/// Development chain specs.
mod chain_spec;

pub use chain_spec::*;
pub use service::start_node;
