use super::*;
use crate::predicate::{ExecResult, ExecError};

/// A function that generates an `AccountId` for a predicate upon instantiation.
pub trait PredicateAddressFor<PredicateHash, AccountId> {
    fn predicate_address_for(
        code_hash: &PredicateHash,
        data: &[u8],
        origin: &AccountId,
    ) -> AccountId;
}

/// An interface that provides access to the external environment in which the
/// predicate-contract is executed similar to a smart-contract.
///
/// This interface is specialized to an account of the executing code, so all
/// operations are implicitly performed on that account.
pub trait Ext {
    type T: Trait;

    /// Instantiate a predicate from the given code.
    ///
    /// The newly created account will be associated with `code`.
    fn instantiate(
        &mut self,
        code: &PredicateHash<Self::T>,
        input_data: Vec<u8>,
    ) -> Result<AccountIdOf<Self::T>, ExecError>;

    /// Call (possibly other predicate) into the specified account.
    fn call(
        &mut self,
        to: &AccountIdOf<Self::T>,
        input_data: Vec<u8>,
    ) -> bool;

    /// Returns a reference to the account id of the caller.
    fn caller(&self) -> &AccountIdOf<Self::T>;

    /// Returns a reference to the account id of the current contract.
    fn address(&self) -> &AccountIdOf<Self::T>;

    /// Returns a reference to the timestamp of the current block
    fn now(&self) -> &MomentOf<Self::T>;

    /// Returns a random number for the current block with the given subject.
    fn random(&self, subject: &[u8]) -> SeedOf<Self::T>;

    /// Deposit an event with the given topics.
    ///
    /// There should not be any duplicates in `topics`.
    fn deposit_event(&mut self, topics: Vec<TopicOf<Self::T>>, data: Vec<u8>);

    /// Returns the current block number.
    fn block_number(&self) -> BlockNumberOf<Self::T>;
}

/// A trait that represent an optimistic virtual machine.
///
/// You can view an optimistic virtual machine as something that takes code, an input data buffer,
/// queries it and/or performs actions on the given `Ext` and optionally
/// returns an output data buffer. The type of code depends on the particular virtual machine.
///
/// Execution of code can end by either implicit termination (that is, reached the end of
/// executable), explicit termination via returning a buffer or termination due to a trap.
pub trait Vm<T: Trait> {
    type Executable;

    fn execute<E: Ext<T = T>>(
        &self,
        exec: &Self::Executable,
        ext: E,
        input_data: Vec<u8>,
    ) -> ExecResult;
}


pub trait AtomicPredicate {
    fn decide_true(inputs: Vec<u8>);
    fn decide(inputs: Vec<u8>) -> Decision;
}

pub trait DecidablePredicate {
    fn decide_with_witness(inputs: Vec<u8>, witness: Vec<u8>) -> Decision;
}

pub trait LogicalConnective<AccountId> {
    fn is_valid_challenge(
        inputs: Vec<u8>,
        challenge_inputs: Vec<u8>,
        challenge: Property<AccountId>,
    ) -> Decision;
}
