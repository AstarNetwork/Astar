use super::*;
use crate::compiled_predicates::*;
use codec::Codec;
use crate::errors::ExecError as Error;

pub type ExecResult = core::result::Result<bool, Error>;

pub trait ExternalCall {
    type Address: Codec;

    /// Call (possibly other predicate) into the specified account.
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
    fn execute(code: CompiledPredicate, )
}

pub struct Executor<Ext> {

    _phantom: Phantom<Ext>,
}

impl OvmExecutor for Executor<Ext> {
    type ExtCall = Ext;


}
