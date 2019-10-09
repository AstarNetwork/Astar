//! A collection of plasm-specific RPC methods.
//!
//! Since `substrate` srml functionality makes no assumptions
//! about the modules used inside the runtime, so do
//! RPC methods defined in `substrate-rpc` crate.
//! It means that `srml/rpc` can't have any methods that
//! need some strong assumptions about the particular runtime.
//!
//! The RPCs available in this crate however can make some assumptions
//! about how the runtime is constructed and what `SRML` modules
//! are part of it. Therefore all plasm-runtime-specific RPCs can
//! be placed here.

#![warn(missing_docs)]

pub mod accounts;
