use super::*;
use crate::compiled_predicates::*;
use crate::predicates::*;
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
        call_method: PredicateCallInputs,
        expected: String,
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

pub trait OvmExecutor {
    type ExtCall: ExternalCall;
    fn execute<P>(executable: P, call_method: PredicateCallInputs) -> ExecResult;
}

pub struct AtomicExeuctor<Ext> {
    _phantom: PhantomData<Ext>,
}

impl OvmExecutor for AtomicExecutor<Ext> {
    type ExtCall = Ext;
    fn execute<P>(predicate: P, call_method: PredicateCallInputs) -> ExecResult
    where
        P: predicates::AtomicPredicate,
    {
        match call_method {
            PredicateCallInputs::AtomicPredicate(atomic) => {
                match atomic {
                    AtomicPredicateCallInputs::Decide { inputs } => return predicate.decide(input),
                    AtomicPredicateCallInputs::DecideTrue { inputs } => {
                        predicate.decide_true(inputs);
                        return Ok(true);
                    }
                };
            }
            other => Err(ExecError::CallMethod {
                call_method: other,
                expected: "AtomicPredicateCallInputs".to_string(),
            }),
        }
    }
}
