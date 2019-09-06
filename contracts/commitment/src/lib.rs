//! Commitment trait and default implementation which conforms to the PGSpec.
//!
//! Commitment Contract Trait provides a commitment contract specification and default implementations.
//!
//! Each plasma chain must have at least one commitment trait object.
//! Commitment holds the block headers of the plasma chain.
//! Whenever the operator creates a new plasma block, they must use the commitment contract trait.
//!
//! Refer to https://docs.plasma.group/projects/spec/en/latest/src/02-contracts/commitment-contract.html.

#![cfg_attr(not(any(test, feature = "std")), no_std)]

use core::option::Option;
use ink_core::{
    env::{ContractEnv, DefaultSrmlTypes, EnvTypes},
    memory::vec::Vec,
};
use scale::{Codec, Decode, Encode};
pub mod default;
pub mod traits;

#[cfg(feature = "test-env")]
pub mod merkle;

#[cfg(feature = "test-env")]
#[macro_use]
extern crate alloc;

type BlockNumber = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::BlockNumber;
type Hash = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Hash;

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct MerkleIntervalTreeInternalNode<
    I: primitives::traits::Member + primitives::traits::SimpleArithmetic + Codec,
> {
    pub index: I,
    pub hash: Hash,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct InclusionProof<
    I: primitives::traits::Member + primitives::traits::SimpleArithmetic + Codec,
> {
    pub proofs: Vec<MerkleIntervalTreeInternalNode<I>>,
    pub idx: I,
}
