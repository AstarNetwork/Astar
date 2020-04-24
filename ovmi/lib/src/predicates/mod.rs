//! # Executable Predicates.
//! Executable Predicates instanced from Compiled Predicates and Atomic Predicates.
//!
//!
use crate::executor::{ExecResult, ExecResultT};
use codec::{Decode, Encode};
use core::fmt;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

mod and;
mod executable;
mod not;
pub use and::AndPredicate;
pub use executable::ExecutablePredicate;
pub use not::NotPredicate;

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash, derive_more::Display)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum PredicateCallInputs<Address> {
    AtomicPredicate(AtomicPredicateCallInputs),
    DecidablePredicate(DecidablePredicateCallInputs),
    LogicalConnective(LogicalConnectiveCallInputs<Address>),
    BaseAtomicPredicate(BaseAtomicPredicateCallInputs),
    CompiledPredicate(CompiledPredicateCallInputs<Address>),
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum AtomicPredicateCallInputs {
    DecideTrue { inputs: Vec<Vec<u8>> },
    Decide { inputs: Vec<Vec<u8>> },
}

impl fmt::Display for AtomicPredicateCallInputs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> fmt::Result {
        let state = match self {
            AtomicPredicateCallInputs::Decide { inputs: _ } => "Decide",
            AtomicPredicateCallInputs::DecideTrue { inputs: _ } => "DecideTrue",
        };
        write!(f, "{}", state)
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum DecidablePredicateCallInputs {
    DecideWithWitness {
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    },
}

impl fmt::Display for DecidablePredicateCallInputs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> fmt::Result {
        let state = match self {
            DecidablePredicateCallInputs::DecideWithWitness {
                inputs: _,
                witness: _,
            } => "DecideWithWitness",
        };
        write!(f, "{}", state)
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum LogicalConnectiveCallInputs<Address> {
    IsValidChallenge {
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        challenge: Property<Address>,
    },
}

impl<Address> fmt::Display for LogicalConnectiveCallInputs<Address> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> fmt::Result {
        let state = match self {
            LogicalConnectiveCallInputs::IsValidChallenge {
                inputs: _,
                challenge_inputs: _,
                challenge: _,
            } => "IsValidChallenge",
        };
        write!(f, "{}", state)
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum BaseAtomicPredicateCallInputs {
    Decide {
        inputs: Vec<Vec<u8>>,
    },
    DecideWithWitness {
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    },
    DecideTrue {
        inputs: Vec<Vec<u8>>,
    },
}

impl fmt::Display for BaseAtomicPredicateCallInputs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> fmt::Result {
        let state = match self {
            BaseAtomicPredicateCallInputs::Decide { inputs: _ } => "Decide",
            BaseAtomicPredicateCallInputs::DecideTrue { inputs: _ } => "DecideTrue",
            BaseAtomicPredicateCallInputs::DecideWithWitness {
                inputs: _,
                witness: _,
            } => "DecideWithWitness",
        };
        write!(f, "{}", state)
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum CompiledPredicateCallInputs<Address> {
    IsValidChallenge {
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        challenge: Property<Address>,
    },
    GetChild {
        inputs: Vec<Vec<u8>>,
        challenge_input: Vec<Vec<u8>>,
    },
    Decide {
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    },
    DecideTrue {
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    },
    DecideWithWitness {
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    },
}

impl<Address> fmt::Display for CompiledPredicateCallInputs<Address> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> fmt::Result {
        let state = match self {
            CompiledPredicateCallInputs::IsValidChallenge {
                inputs: _,
                challenge_inputs: _,
                challenge: _,
            } => "IsValidChallenge",
            CompiledPredicateCallInputs::GetChild {
                inputs: _,
                challenge_input: _,
            } => "GetChild",
            CompiledPredicateCallInputs::Decide {
                inputs: _,
                witness: _,
            } => "Decide",
            CompiledPredicateCallInputs::DecideTrue {
                inputs: _,
                witness: _,
            } => "DecideTrue",
            CompiledPredicateCallInputs::DecideWithWitness {
                inputs: _,
                witness: _,
            } => "DecideWithWitness",
        };
        write!(f, "{}", state)
    }
}

/// Property stands for dispute logic and we can claim every Properties to Adjudicator Contract.
/// Property has its predicate address and array of input.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct Property<Address> {
    /// Indicates the address of Predicate.
    predicate_address: Address,
    /// Every input are bytes. Each Atomic Predicate decode inputs to the specific type.
    inputs: Vec<Vec<u8>>,
}

pub trait UniversalAdjudication<Hash> {
    fn set_predicate_decision(&self, game_id: Hash, decision: bool);
}

pub trait Utils<Hash> {
    fn get_property_id(&self) -> Hash;
}

pub trait BaseAtomicPredicate<Address, Hash>:
    AtomicPredicate<Address> + DecidablePredicate<Address>
{
    type UniversalAdjudication: UniversalAdjudication<Hash>;
    type Utils: Utils<Hash>;

    fn decide(&self, _inputs: Vec<Vec<u8>>) -> ExecResult<Address> {
        return Ok(false);
    }

    fn decide_with_witness(
        &self,
        _inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<Address> {
        BaseAtomicPredicate::decide(self, _inputs)
    }

    fn decide_true(&self, _inputs: Vec<Vec<u8>>) {
        // require(decide(_inputs), "must decide true");
        // types.Property memory property = types.Property({
        //     predicateAddress: address(this),
        //     inputs: _inputs,
        // });
        // Self::UniversalAdjudication::set_predicate_decision(
        //     Self::Utils::get_property_id(property),
        //     true
        // );
    }
}

pub trait AtomicPredicate<Address> {
    fn decide_true(&self, _inputs: Vec<Vec<u8>>);
    fn decide(&self, _inputs: Vec<Vec<u8>>) -> ExecResult<Address>;
}

pub trait DecidablePredicate<Address> {
    fn decide_with_witness(
        &self,
        _inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<Address>;
}

pub trait CompiledPredicate<Address> {
    fn payout_contract_address(&self) -> Address;

    fn is_valid_challenge(
        &self,
        _inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        _challenge: Property<Address>,
    ) -> ExecResult<Address>;

    /// @dev get valid child property of game tree with challenge_inputs
    fn get_child(
        &self,
        inputs: Vec<Vec<u8>>,
        challenge_input: Vec<Vec<u8>>,
    ) -> ExecResultT<Property<Address>, Address>;

    fn decide(&self, _inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>) -> ExecResult<Address>;
    fn decide_true(&self, _inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>);
    fn decide_with_witness(
        &self,
        _inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<Address>;
}

pub trait LogicalConnective<Address> {
    fn is_valid_challenge(
        &self,
        _inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        _challenge: Property<Address>,
    ) -> ExecResult<Address>;
}
