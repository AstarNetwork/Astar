//! # ovmi
//! This library allows **Predicate** to be loaded in binary format and their functions invoked.
//!
//! # Introduction
//! Predicate is a DSL for resolving Layer 2 dispute logic,
//! and the Optimistic Virtual Machine gives simulated environment by Predicate.
//!
//! There are several types of Predicate, each of which has an interface that looks like this.
//! ## Atomic Predicate
//! - decideTrue
//! - decide
//!
//! ## CompiledPredicate
//! - payoutContractAddress
//! - isValidChallenge
//! - getChild
//! - decide
//! - decideTrue
//! - decideWithWitness
//!
//! ## DecidablePredicate
//! - decideWithWitness
//!
//! ## LogicalConnective
//! - isValidChallenge
//!
//! Each of these definitions can be imported and exported.
//!
//! OVM does not manage the state, but it can be called externally.
//!
//! # Loading and Validation
//! Before execution, a module must be validated. This process checks that the module is well-formed and makes only allowed operations.
//!
//! You can get the binary of predicate from json.
//! The json format is refer to: https://github.com/cryptoeconomicslab/wakkanay/blob/master/packages/ovm-transpiler/src/CompiledPredicate.ts
//! ```ignore
//! use ovmi::prepare;
//! let compiled_predicate = prepare::compile_from_json("<compiled_predicate_json>").unwrap();
//! let binary_predicate = compiled_predicate::encode();
//!
//! if let Err(err) = prepare::validate(binary_predicate) {
//!     panic!(err);
//! }
//! ```
//!
//! # Set external environments and.
//!
//! ## Example) Compiled Predicate Execute
//! ```ignore
//! use ovmi::prepare;
//!
//! type AccountId = u64;
//!
//! // Setting External environment.
//! struct MockExternalCall{..};
//! impl ExternalCall for MockExternalCall {
//!     ...
//! }
//!
//! fn call_execute(inputs: Vec<Vec<u8>>, inputs: PredicateCallInputs<AccountId>) -> ExecResult<AccountId> {
//!     let compiled_predicate = prepare::compile_from_json("<compiled_predicate_json>").unwrap();
//!     let (payout, address_input, bytes_inputs) = prepare::parse_inputs(inputs);
//!     let ext = MockExternalCall{..};
//!     let executable = prepare::executable_from_compiled(
//!         &mut ext,
//!         code: compiled_predicate,
//!         payout,
//!         address_inputs,
//!         bytes_inputs,
//!     );
//!     // execute and return value.
//!     CompiledExecutor::execute(&executable, inputs)
//! }
//! ```
//!
//! ## Example) Logical Connective Predicate Execute
//! ```ignore
//! use ovmi::prepare;
//!
//! type AccountId = u64;
//!
//! // Setting External environment.
//! struct MockExternalCall{..};
//! impl ExternalCall for MockExternalCall {
//!     ...
//! }
//!
//! fn call_execute(address: AccountId, inputs: PredicateCallInputs<AccountId>) -> ExecResult<AccountId> {
//!     let ext = MockExternalCall{..};
//!     let executable = prepare::logical_connective_executable_from_address(
//!         &mut ext,
//!         address,
//!     );
//!     // execute and return value.
//!     LogicalConnectiveExecutor::execute(&executable, inputs)
//! }
//! ```
#![cfg_attr(not(feature = "std"), no_std)]
#![macro_use]

#[macro_export]
macro_rules! require {
    ($val:expr) => {
        if !($val) {
            return Err(crate::executor::ExecError::Require {
                msg: stringify!($val),
            });
        }
    };
}

#[macro_export]
macro_rules! require_with_message {
    ($val:expr, $message:expr) => {
        if !($val) {
            return Err(crate::executor::ExecError::Require { msg: $message });
        }
    };
}

#[macro_use]
pub extern crate alloc;
use alloc::{collections::btree_map::BTreeMap, vec::Vec};

pub mod compiled_predicates;
pub mod executor;
pub mod predicates;
pub mod prepare;
use codec::{Decode, Encode};
pub use compiled_predicates::CompiledPredicate;
#[cfg(feature = "std")]
pub use prepare::compile_from_json;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// An opaque 32-byte cryptographic identifier.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Hash))]
pub struct AccountId([u8; 32]);

/// An opaque Range(u128, u128).
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Hash))]
pub struct Range {
    pub start: u128,
    pub end: u128,
}
