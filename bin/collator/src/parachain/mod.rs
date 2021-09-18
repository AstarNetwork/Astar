//! Support for Astar ecosystem parachains.

/// Shell to Aura consensus upgrades.
mod shell_upgrade;

/// Parachain specified service.
pub mod service;

/// Parachain specs.
mod chain_spec;

pub use chain_spec::*;
pub use service::{
    build_import_queue, new_partial, shibuya, shiden, start_shibuya_node, start_shiden_node,
};
