#![cfg_attr(not(any(test, feature = "std")), no_std)]
use core::option::Option;
use ink_core::env::{ContractEnv, DefaultSrmlTypes, EnvTypes};
use parity_codec::{Encode, Decode, Codec};

pub mod default;
pub mod traits;

type BlockNumber = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::BlockNumber;
type Hash = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Hash;

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
struct MerkleIndexTreeInternalNode<I: primitives::traits::Member + primitives::traits::SimpleArithmetic + Codec> {
	index: I,
	hash: Hash,
}
