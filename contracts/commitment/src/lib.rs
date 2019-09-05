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
