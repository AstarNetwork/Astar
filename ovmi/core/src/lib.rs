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
//! # Instantiation
//! TODO
//!
//! # Execution
//! TODO
//!

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

mod prepare;

