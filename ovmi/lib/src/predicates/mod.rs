//! # Executable Predicates.
//! Executable Predicates instanced from Compiled Predicates and Atomic Predicates.
//!
//!
use crate::executor::{
    AddressOf, ExecError, ExecResult, ExecResultT, ExternalCall, HashOf, MaybeAddress,
};
use codec::{Decode, Encode};
use core::fmt;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

mod and;
mod executable;
mod for_all;
mod not;
mod or;
mod there_exists;

pub use and::AndPredicate;
pub use executable::CompiledExecutable;
pub use for_all::ForAllPredicate;
pub use not::NotPredicate;
pub use or::OrPredicate;
pub use there_exists::ThereExistsPredicate;

mod equal;
mod is_contained;
mod is_less;
mod is_stored;
mod is_valid_signature;
mod verify_inclusion;
pub use equal::EqualPredicate;
pub use is_contained::IsContainedPredicate;
pub use is_less::IsLessThanPredicate;
pub use is_stored::IsStoredPredicate;
pub use is_valid_signature::IsValidSignaturePredicate;
pub use verify_inclusion::VerifyInclusionPredicate;

// #[derive(Clone, Eq, PartialEq, Encode, Decode, Hash, derive_more::Display)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
// pub enum AtomicExecutable<'a, Ext: ExternalCall> {
//     None,
// }

pub enum LogicalConnectiveExecutable<'a, Ext: ExternalCall> {
    And(AndPredicate<'a, Ext>),
    Not(NotPredicate<'a, Ext>),
    Or(OrPredicate<'a, Ext>),
    ForAll(ForAllPredicate<'a, Ext>),
    ThereExists(ThereExistsPredicate<'a, Ext>),
}

impl<Ext: ExternalCall> LogicalConnectiveInterface<AddressOf<Ext>>
    for LogicalConnectiveExecutable<'_, Ext>
{
    fn is_valid_challenge(
        &self,
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        challenge: Property<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        match self {
            LogicalConnectiveExecutable::And(and) => {
                and.is_valid_challenge(inputs, challenge_inputs, challenge)
            }
            LogicalConnectiveExecutable::Not(not) => {
                not.is_valid_challenge(inputs, challenge_inputs, challenge)
            }
            LogicalConnectiveExecutable::Or(or) => {
                or.is_valid_challenge(inputs, challenge_inputs, challenge)
            }
            LogicalConnectiveExecutable::ForAll(for_all) => {
                for_all.is_valid_challenge(inputs, challenge_inputs, challenge)
            }
            LogicalConnectiveExecutable::ThereExists(there_exists) => {
                there_exists.is_valid_challenge(inputs, challenge_inputs, challenge)
            }
        }
    }
}

pub enum DecidableExecutable<'a, Ext: ExternalCall> {
    And(AndPredicate<'a, Ext>),
    Not(NotPredicate<'a, Ext>),
    Or(OrPredicate<'a, Ext>),
    ForAll(ForAllPredicate<'a, Ext>),
    ThereExists(ThereExistsPredicate<'a, Ext>),
    Equal(EqualPredicate<'a, Ext>),
    IsContained(IsContainedPredicate<'a, Ext>),
    IsLess(IsLessThanPredicate<'a, Ext>),
    IsStored(IsStoredPredicate<'a, Ext>),
    IsValidSignature(IsValidSignaturePredicate<'a, Ext>),
    VerifyInclusion(VerifyInclusionPredicate<'a, Ext>),
}

impl<Ext: ExternalCall> DecidablePredicateInterface<AddressOf<Ext>>
    for DecidableExecutable<'_, Ext>
{
    fn decide_with_witness(
        &self,
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        match self {
            DecidableExecutable::And(and) => and.decide_with_witness(inputs, witness),
            DecidableExecutable::Not(not) => not.decide_with_witness(inputs, witness),
            DecidableExecutable::Or(or) => or.decide_with_witness(inputs, witness),
            DecidableExecutable::ForAll(for_all) => for_all.decide_with_witness(inputs, witness),
            DecidableExecutable::ThereExists(p) => p.decide_with_witness(inputs, witness),
            DecidableExecutable::Equal(p) => p.decide_with_witness(inputs, witness),
            DecidableExecutable::IsContained(p) => p.decide_with_witness(inputs, witness),
            DecidableExecutable::IsLess(p) => p.decide_with_witness(inputs, witness),
            DecidableExecutable::IsStored(p) => p.decide_with_witness(inputs, witness),
            DecidableExecutable::IsValidSignature(p) => p.decide_with_witness(inputs, witness),
            DecidableExecutable::VerifyInclusion(p) => p.decide_with_witness(inputs, witness),
        }
    }
}

pub enum BaseAtomicExecutable<'a, Ext: ExternalCall> {
    Equal(EqualPredicate<'a, Ext>),
    IsContained(IsContainedPredicate<'a, Ext>),
    IsLess(IsLessThanPredicate<'a, Ext>),
    IsStored(IsStoredPredicate<'a, Ext>),
    IsValidSignature(IsValidSignaturePredicate<'a, Ext>),
    VerifyInclusion(VerifyInclusionPredicate<'a, Ext>),
}

impl<Ext: ExternalCall> DecidablePredicateInterface<AddressOf<Ext>>
    for BaseAtomicExecutable<'_, Ext>
{
    fn decide_with_witness(
        &self,
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        match self {
            BaseAtomicExecutable::Equal(p) => p.decide_with_witness(inputs, witness),
            BaseAtomicExecutable::IsContained(p) => p.decide_with_witness(inputs, witness),
            BaseAtomicExecutable::IsLess(p) => p.decide_with_witness(inputs, witness),
            BaseAtomicExecutable::IsStored(p) => p.decide_with_witness(inputs, witness),
            BaseAtomicExecutable::IsValidSignature(p) => p.decide_with_witness(inputs, witness),
            BaseAtomicExecutable::VerifyInclusion(p) => p.decide_with_witness(inputs, witness),
        }
    }
}

impl<Ext: ExternalCall> AtomicPredicateInterface<AddressOf<Ext>> for BaseAtomicExecutable<'_, Ext> {
    fn decide(&self, inputs: Vec<Vec<u8>>) -> ExecResult<AddressOf<Ext>> {
        match self {
            BaseAtomicExecutable::Equal(p) => p.decide(inputs),
            BaseAtomicExecutable::IsContained(p) => p.decide(inputs),
            BaseAtomicExecutable::IsLess(p) => p.decide(inputs),
            BaseAtomicExecutable::IsStored(p) => p.decide(inputs),
            BaseAtomicExecutable::IsValidSignature(p) => p.decide(inputs),
            BaseAtomicExecutable::VerifyInclusion(p) => p.decide(inputs),
        }
    }

    fn decide_true(&self, inputs: Vec<Vec<u8>>) -> ExecResult<AddressOf<Ext>> {
        match self {
            BaseAtomicExecutable::Equal(p) => p.decide_true(inputs),
            BaseAtomicExecutable::IsContained(p) => p.decide_true(inputs),
            BaseAtomicExecutable::IsLess(p) => p.decide_true(inputs),
            BaseAtomicExecutable::IsStored(p) => p.decide_true(inputs),
            BaseAtomicExecutable::IsValidSignature(p) => p.decide_true(inputs),
            BaseAtomicExecutable::VerifyInclusion(p) => p.decide_true(inputs),
        }
    }
}

impl<Ext: ExternalCall> BaseAtomicPredicateInterface<AddressOf<Ext>>
    for BaseAtomicExecutable<'_, Ext>
{
}

impl<Ext: ExternalCall> AtomicHelperInterface<AddressOf<Ext>> for BaseAtomicExecutable<'_, Ext> {
    type Hash = HashOf<Ext>;
    fn ext_address(&self) -> AddressOf<Ext> {
        AddressOf::<Ext>::default()
    }
    fn ext_set_predicate_decision(
        &self,
        _game_id: Self::Hash,
        _decision: bool,
    ) -> ExecResult<AddressOf<Ext>> {
        Err(ExecError::Unimplemented)
    }
    fn ext_get_property_id(&self, _property: &Property<AddressOf<Ext>>) -> Self::Hash {
        Self::Hash::default()
    }
}

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
    fn ext_set_predicate_decision(&self, game_id: Hash, decision: bool);
}

pub trait Utils<Hash> {
    fn ext_get_property_id(&self) -> Hash;
}

pub trait BaseAtomicPredicateInterface<Address>:
    AtomicPredicateInterface<Address> + DecidablePredicateInterface<Address>
{
}

pub trait AtomicPredicateInterface<Address>: AtomicHelperInterface<Address> {
    fn decide(&self, _inputs: Vec<Vec<u8>>) -> ExecResult<Address> {
        return Ok(false);
    }

    fn decide_true(&self, inputs: Vec<Vec<u8>>) -> ExecResult<Address> {
        let result_of_decide = AtomicPredicateInterface::decide(self, inputs.clone())?;
        require_with_message!(result_of_decide, "must decide true");
        let property = Property {
            predicate_address: self.ext_address(),
            inputs: inputs,
        };
        self.ext_set_predicate_decision(self.ext_get_property_id(&property), true)?;
        Ok(true)
    }
}

pub trait AtomicHelperInterface<Address> {
    type Hash;
    fn ext_address(&self) -> Address;
    fn ext_set_predicate_decision(
        &self,
        game_id: Self::Hash,
        decision: bool,
    ) -> ExecResult<Address>;
    fn ext_get_property_id(&self, property: &Property<Address>) -> Self::Hash;
}

pub trait DecidablePredicateInterface<Address> {
    fn decide_with_witness(
        &self,
        _inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<Address>;
}

pub trait CompiledPredicateInterface<Address> {
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
    fn decide_true(&self, _inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>) -> ExecResult<Address>;
    fn decide_with_witness(
        &self,
        _inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<Address>;
}

pub trait LogicalConnectiveInterface<Address> {
    fn is_valid_challenge(
        &self,
        _inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        _challenge: Property<Address>,
    ) -> ExecResult<Address>;
}
