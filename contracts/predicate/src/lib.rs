//! Predicate Contract Trait provides a predicate contract specification and the ownership of predicate implementations.
//!
//! Predicate contracts define the rules for particular state objects’ exit game.
//! The most fundamental thing they define is the deprecation logic,
//! which informs the plasma contract that an exit on some states is invalid because it is outdated. Usually,
//! this logic is used to prove that a transaction has been invalidated.
//! Because the predicate contract is a stateful main-chain contract,
//! more advanced predicates can also define custom exit logics which must be evaluated
//! before any state transitions are approved by the predicate.
//! Thus, predicates can be used as fully customized extensions to the base plasma cash exit game.
//!
//! Refer to https://docs.plasma.group/projects/spec/en/latest/src/02-contracts/predicate-contract.html.

#![cfg_attr(not(any(test, feature = "std")), no_std)]

use core::option::Option;
use ink_core::{
    env::{ContractEnv, DefaultSrmlTypes, EnvTypes},
    memory::vec::Vec,
};
use ink_model::EnvHandler;
use scale::{Codec, Decode, Encode};
use primitives::{
    events::*,
    traits::{Member, SimpleArithmetic},
};

pub mod ownership;
pub mod traits;

type AccountId = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::AccountId;
type Balance = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Balance;
type BlockNumber = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::BlockNumber;
type Hash = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Hash;
