//! # Executable Predicates.
//! Executable Predicates instanced from Compiled Predicates and Atomic Predicates.
//!
//!

use crate::executor::ExecResult;

macro_rules! require {
    ($val:expr) => {
        if not $val {
            return ExecError::RequireError{msg: "Required error by: $val"}
        }
    };
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum PredicateCallInputs {
    AtomicPredicate(AtomicPredicateCallInputs),
    DecidablePredicate(DecidablePredicateCallInputs),
    LogicalConnective(LogicalConnectiveCallInputs),
    BaseAtomicPredicate(BaseAtomicPredicateCallInputs),
    CompiledPredicate(CompiledPredicateCallInputs),
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum AtomicPredicateCallInputs {
    DecideTrue { inputs: Vec<Vec<u8>> },
    Decide { inputs: Vec<Vec<u8>> },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum DecidablePredicateCallInputs {
    DecideWithWitness {
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum LogicalConnectiveCallInputs {
    IsValidChallenge {
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        challenge: Property<Address>,
    },
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

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum CompiledPredicateCallInputs {
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

/// Property stands for dispute logic and we can claim every Properties to Adjudicator Contract.
/// Property has its predicate address and array of input.
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Property<Address> {
    /// Indicates the address of Predicate.
    predicate_address: Address,
    /// Every input are bytes. Each Atomic Predicate decode inputs to the specific type.
    inputs: Vec<u8>,
}

pub trait UniversalAdjudication<Hash> {
    fn set_predicate_decision(game_id: Hash, decision: bool);
}

pub trait Utils<Hash> {
    fn get_property_id() -> Hash;
}

pub trait BaseAtomicPredicate<Address, Hash>:
    AtomicPredicate<Address> + DecidablePredicate
{
    type UniversalAdjudication: UniversalAdjudication<Hash>;
    type Utils: Utils<Hash>;

    fn decide(_inputs: Vec<Vec<u8>>) -> ExecResult {
        return Ok(false);
    }

    fn decide_with_witness(_inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>) -> ExecResult {
        return Self::decide(_inputs);
    }

    fn decide_true(_inputs: Vec<Vec<u8>>) {
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

pub trait AtomicPredicate {
    fn decide_true(_inputs: Vec<Vec<u8>>);
    fn decide(_inputs: Vec<Vec<u8>>) -> ExecResult;
}

pub trait DecidablePredicate {
    fn decide_with_witness(_inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>) -> ExecResult;
}

pub trait CompiledPredicate<Address> {
    fn payout_contract_address() -> Address;

    fn is_valid_challenge(
        _inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        _challenge: Property<Address>,
    ) -> ExecResult;

    /// @dev get valid child property of game tree with challenge_inputs
    fn get_child(inputs: Vec<Vec<u8>>, challenge_input: Vec<Vec<u8>>) -> Property<Address>;

    fn decide(_inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>) -> ExecResult;
    fn decide_true(_inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>);
    fn decide_with_witness(_inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>) -> ExecResult;
}

pub trait LogicalConnective<Address> {
    fn is_valid_challenge(
        _inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        _challenge: Property<Address>,
    ) -> ExecResult;
}
