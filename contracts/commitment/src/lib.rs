#![cfg_attr(not(any(test, feature = "std")), no_std)]
use core::option::Option;
use ink_core::{
    env::{ContractEnv, DefaultSrmlTypes, EnvTypes},
    memory::vec::Vec,
};
use parity_codec::{Codec, Decode, Encode};
pub mod default;
pub mod traits;

type BlockNumber = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::BlockNumber;
type Hash = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Hash;

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct MerkleIndexTreeInternalNode<
    I: primitives::traits::Member + primitives::traits::SimpleArithmetic + Codec,
> {
    pub index: I,
    pub hash: Hash,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct InclusionProof<
    I: primitives::traits::Member + primitives::traits::SimpleArithmetic + Codec,
> {
    pub proofs: Vec<MerkleIndexTreeInternalNode<I>>,
    pub idx: I,
}
