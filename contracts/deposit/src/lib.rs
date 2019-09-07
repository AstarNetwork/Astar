//! Deposit Contract Trait provides deposit contract specification and default implementations.
//!
//! Deposit contracts are the contracts into which assets are deposited–custodying the money.
//! It is transacted on plasma and playing out the exit games to resolve the rightful owners of deposited assets.
//! As such, it contains the bulk of the logic for the plasma exit games.
//! The things it does not cover are 1) block commitments, and 2) state deprecation,
//! that are handled by calls to the commitment contract and predicate contracts specifically.
//!
//! Refer to https://docs.plasma.group/projects/spec/en/latest/src/02-contracts/commitment-contract.html.

#![cfg_attr(not(any(test, feature = "std")), no_std)]

use core::option::Option;
use ink_core::{
    env::{ContractEnv, DefaultSrmlTypes, EnvTypes},
};
use ink_model::EnvHandler;
use scale::Codec;
use primitives::{
    events::*,
    traits::{Member, SimpleArithmetic},
};

pub mod default;
pub mod traits;

type AccountId = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::AccountId;
type Balance = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Balance;
type BlockNumber = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::BlockNumber;
type Hash = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Hash;

/// Status of a particular checkpoint attempt.
#[derive(Clone, scale::Encode, scale::Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct CheckpointStatus {
	/// Ethereum block number until which the checkpoint can still be challenged.
    challengeable_until: BlockNumber,
	/// Number of outstanding challenges.
    outstanding_challenges: u128,
}

impl ink_core::storage::Flush for CheckpointStatus {
	fn flush(&mut self) {}
}

#[cfg(not(any(test, feature = "std")))]
mod no_std {
	extern crate alloc;
	pub use alloc::string::{String, ToString};
	pub use alloc::vec::Vec;
}
