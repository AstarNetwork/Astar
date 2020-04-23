use super::*;
use crate::compiled_predicates::*;
use codec::Codec;
use core::marker::PhantomData;
use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub enum ExecError {
    #[snafu(display("Require error: {}", msg))]
    RequireError { msg: String },
    #[snafu(display(
        "Can not call method error: you call {}, expected {}",
        call_method,
        expected
    ))]
    CallMethod {
        call_method: PredicateCallMethods,
        expected: Vec<PredicateCallMethods>,
    },
    #[snafu(display("Unexpected error: {}", msg))]
    UnexpectedError { msg: String },
}

pub type ExecResult = core::result::Result<bool, ExecError>;

pub trait ExternalCall {
    type Address: Codec;

    /// Call (other predicate) into the specified account.
    fn ext_call(&mut self, to: &Self::Address, input_data: Vec<u8>) -> ExecResult;

    /// Returns a reference to the account id of the caller.
    fn ext_caller(&self) -> &Self::Address;

    /// Returns a reference to the account id of the current contract.
    fn ext_address(&self) -> &Self::Address;

    // Notes a call other storage.
    // Only return true or false.
    // CommitmentAddress(special) isCommitment(address) -> Commitment
    // is_stored_predicate(&mut self, address, key, value);?
    // ref: https://github.com/cryptoeconomicslab/ovm-contracts/blob/master/contracts/Predicate/Atomic/IsStoredPredicate.sol
    fn ext_is_stored(&mut self, address: &Self::Address, key: &[u8], value: &[u8]) -> bool;
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum PredicateCallMethods {
    /// Be able to call by LogicalConnective & CompiledPredicate.
    IsValidChallenge,
    /// Be able to call by BaseAtomicPredicate & AtomicPredicate
    Decide,
    /// Be able to call by BaseAtomicPredicate & DecidablePredicate
    DecideWithWitness,
    /// Be able to call by BaseAtomicPredicate & AtomicPredicate
    DecideTrue,
    /// Be able to call by CompiledPredicate.
    PayoutContractAddress,
    /// Be able to call by CompiledPredicate.
    GetChild,
}

pub trait OvmExecutor {
    type ExtCall: ExternalCall;
    fn execute<P>(executable: P, call_method: PredicateCallMethods) -> ExecResult;
}

pub struct AtomicExeuctor<Ext> {
    _phantom: PhantomData<Ext>,
}

impl OvmExecutor for AtomicExecutor<Ext> {
    type ExtCall = Ext;
    fn execute<P>(predicate: P, call_method: PredicateCallMethods) -> ExecResult
    where
        P: predicates::AtomicPredicate,
    {
        match call_method {
            PredicateCallMethods::Decide => predicate.decide(),
            PredicateCallMethods::DecideTrue => return predicate.decide_true(),
            _ => {
                return Err(ExecError::CallMethod {
                    call_method,
                    expected: vec![
                        PredicateCallMethods::Decide,
                        PredicateCallMethods::DecideTrue,
                    ],
                });
            }
        }
        Ok(true)
    }
}
