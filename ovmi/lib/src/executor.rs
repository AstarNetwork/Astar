use super::*;
use crate::compiled_predicates::*;
use crate::predicates::*;
use codec::Codec;
use core::fmt::{Debug, Display};
use core::marker::PhantomData;
pub use hash_db::Hasher;
use snafu::{ResultExt, Snafu};

#[derive(Snafu)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum ExecError<Address> {
    #[snafu(display("Require error: {}", msg))]
    Require { msg: &'static str },
    #[snafu(display(
        "Can not call method error: you call {}, expected {}",
        call_method,
        expected
    ))]
    CallMethod {
        call_method: PredicateCallInputs<Address>,
        expected: &'static str,
    },
    #[snafu(display("Codec error: type name is {}", type_name))]
    CodecError { type_name: &'static str },
    #[snafu(display("Unexpected error: {}", msg))]
    Unexpected { msg: &'static str },
}

pub type ExecResult<Address> = core::result::Result<bool, ExecError<Address>>;
pub type ExecResultT<T, Address> = core::result::Result<T, ExecError<Address>>;
pub type AddressOf<Ext> = <Ext as ExternalCall>::Address;
pub type HashOf<Ext> = <Ext as ExternalCall>::Hash;
pub type HashingOf<Ext> = <Ext as ExternalCall>::Hashing;
pub type PropertyOf<Ext> = Property<<Ext as ExternalCall>::Address>;

pub trait MaybeAddress: Codec + Debug + Clone + Eq + PartialEq {}
impl<T: Codec + Debug + Clone + Eq + PartialEq> MaybeAddress for T {}

pub trait MaybeHash:
    AsRef<[u8]>
    + AsMut<[u8]>
    + Default
    + Codec
    + Debug
    + core::hash::Hash
    + Send
    + Sync
    + Clone
    + Copy
    + Eq
    + PartialEq
    + Ord
{
}
impl<
        T: AsRef<[u8]>
            + AsMut<[u8]>
            + Default
            + Codec
            + Debug
            + core::hash::Hash
            + Send
            + Sync
            + Clone
            + Copy
            + Eq
            + PartialEq
            + Ord,
    > MaybeHash for T
{
}

pub trait ExternalCall {
    type Address: MaybeAddress;
    type Hash: MaybeHash;
    type Hashing: Hasher<Out = Self::Hash>;

    // relation const any atomic predicate address.
    const NotPredicate: Self::Address;
    const AndPredicate: Self::Address;

    /// Produce the hash of some codec-encodable value.
    fn hash_of<S: Encode>(s: &S) -> Self::Hash {
        Encode::using_encoded(s, Self::Hashing::hash)
    }

    /// Call (other predicate) into the specified account.
    fn ext_call(
        &mut self,
        to: &Self::Address,
        input_data: PredicateCallInputs<Self::Address>,
    ) -> ExecResult<Self::Address>;

    /// Returns a reference to the account id of the caller.
    fn ext_caller(&self) -> Self::Address;

    /// Returns a reference to the account id of the current contract.
    fn ext_address(&self) -> Self::Address;

    /// Notes a call other storage.
    /// Only return true or false.
    /// CommitmentAddress(special) isCommitment(address) -> Commitment
    /// is_stored_predicate(&mut self, address, key, value);?
    /// ref: https://github.com/cryptoeconomicslab/ovm-contracts/blob/master/contracts/Predicate/Atomic/IsStoredPredicate.sol
    fn ext_is_stored(&mut self, address: &Self::Address, key: &[u8], value: &[u8]) -> bool;

    /* Helpers of UniversalAdjudicationContract. */
    /// `is_decided` function of UniversalAdjudication in OVM module.
    fn ext_is_decided(&self, property: &PropertyOf<Self>) -> bool;
    /// `get_property_id` function of UniversalAdjudication in OVM module.
    fn ext_get_property_id(&self, property: &PropertyOf<Self>) -> Self::Hash;
    /// `set_predicate_decision` function of UniversalAdjudication in OVM module.
    fn ext_set_predicate_decision(
        &self,
        game_id: Self::Hash,
        decision: bool,
    ) -> ExecResult<Self::Address>;
}

pub trait OvmExecutor<P> {
    type ExtCall: ExternalCall;
    fn execute(
        executable: P,
        call_method: PredicateCallInputs<AddressOf<Self::ExtCall>>,
    ) -> ExecResult<AddressOf<Self::ExtCall>>;
}

pub struct AtomicExecutor<P, Ext> {
    _phantom: PhantomData<(P, Ext)>,
}

impl<P, Ext> OvmExecutor<P> for AtomicExecutor<P, Ext>
where
    P: predicates::AtomicPredicateInterface<AddressOf<Ext>>,
    Ext: ExternalCall,
{
    type ExtCall = Ext;
    fn execute(
        predicate: P,
        call_method: PredicateCallInputs<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        match call_method {
            PredicateCallInputs::AtomicPredicate(atomic) => {
                match atomic {
                    AtomicPredicateCallInputs::Decide { inputs } => {
                        return predicate.decide(inputs);
                    }
                    AtomicPredicateCallInputs::DecideTrue { inputs } => {
                        predicate.decide_true(inputs);
                        return Ok(true);
                    }
                };
            }
            other => Err(ExecError::CallMethod {
                call_method: other,
                expected: "AtomicPredicateCallInputs",
            }),
        }
    }
}

pub struct LogicalConnectiveExecutor<P, Ext> {
    _phantom: PhantomData<(P, Ext)>,
}

impl<P, Ext> OvmExecutor<P> for LogicalConnectiveExecutor<P, Ext>
where
    P: predicates::LogicalConnectiveInterface<AddressOf<Ext>>,
    Ext: ExternalCall,
{
    type ExtCall = Ext;
    fn execute(
        predicate: P,
        call_method: PredicateCallInputs<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        match call_method {
            PredicateCallInputs::LogicalConnective(atomic) => {
                match atomic {
                    LogicalConnectiveCallInputs::IsValidChallenge {
                        inputs,
                        challenge_inputs,
                        challenge,
                    } => return predicate.is_valid_challenge(inputs, challenge_inputs, challenge),
                };
            }
            other => Err(ExecError::CallMethod {
                call_method: other,
                expected: "LogicalConnectiveCallInputs",
            }),
        }
    }
}

pub struct DecidableExecutor<P, Ext> {
    _phantom: PhantomData<(P, Ext)>,
}

impl<P, Ext> OvmExecutor<P> for DecidableExecutor<P, Ext>
where
    P: predicates::DecidablePredicateInterface<AddressOf<Ext>>,
    Ext: ExternalCall,
{
    type ExtCall = Ext;
    fn execute(
        predicate: P,
        call_method: PredicateCallInputs<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        match call_method {
            PredicateCallInputs::DecidablePredicate(atomic) => {
                match atomic {
                    DecidablePredicateCallInputs::DecideWithWitness { inputs, witness } => {
                        return predicate.decide_with_witness(inputs, witness);
                    }
                };
            }
            other => Err(ExecError::CallMethod {
                call_method: other,
                expected: "DecidablePredicateCallInputs",
            }),
        }
    }
}

pub struct CompiledExecutor<P, Ext> {
    _phantom: PhantomData<(P, Ext)>,
}

impl<P, Ext> OvmExecutor<P> for CompiledExecutor<P, Ext>
where
    P: predicates::CompiledPredicateInterface<AddressOf<Ext>>,
    Ext: ExternalCall,
{
    type ExtCall = Ext;
    fn execute(
        predicate: P,
        call_method: PredicateCallInputs<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        match call_method {
            PredicateCallInputs::CompiledPredicate(atomic) => {
                match atomic {
                    CompiledPredicateCallInputs::IsValidChallenge {
                        inputs,
                        challenge_inputs,
                        challenge,
                    } => {
                        return predicate.is_valid_challenge(inputs, challenge_inputs, challenge);
                    }
                    CompiledPredicateCallInputs::Decide { inputs, witness } => {
                        return predicate.decide(inputs, witness);
                    }
                    CompiledPredicateCallInputs::DecideTrue { inputs, witness } => {
                        return predicate.decide_true(inputs, witness);
                    }
                    CompiledPredicateCallInputs::DecideWithWitness { inputs, witness } => {
                        return predicate.decide_with_witness(inputs, witness)
                    }
                };
            }
            other => Err(ExecError::CallMethod {
                call_method: other,
                expected: "CompiledPredicateCallInputs",
            }),
        }
    }
}
