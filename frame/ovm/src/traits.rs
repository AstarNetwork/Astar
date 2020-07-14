use super::*;
use crate::predicate::ExecutionContext;

/// A function that generates an `AccountId` for a predicate upon instantiation.
pub trait PredicateAddressFor<PredicateHash, AccountId> {
    fn predicate_address_for(
        code_hash: &PredicateHash,
        data: &[u8],
        origin: &AccountId,
    ) -> AccountId;
}

/// Loader is a companion of the `Vm` trait. It loads an appropriate abstract
/// executable to be executed by an accompanying `Vm` implementation.
pub trait Loader<T: Trait> {
    type Executable;

    /// Load the main portion of the code specified by the `code_hash`. This executable
    /// is called for each call to a contract.
    fn load_main(
        &self,
        predicate: PredicateContractOf<T>,
    ) -> Result<Self::Executable, &'static str>;
}

/// For the initalize call context.
pub trait NewCallContext<T: Trait> {
    fn new(ctx: Rc<ExecutionContext<T>>, caller: AccountIdOf<T>) -> Self;
}

/// An interface that provides access to the external environment in which the
/// predicate-contract is executed similar to a smart-contract.
///
/// This interface is specialized to an account of the executing code, so all
/// operations are implicitly performed on that account.
///
/// Predicate call the only below functions.
/// - call: call to other predicate.
/// - caller: get of the caller of this predicate.
/// - address: the predicate's address.
/// - is_stored: check the storage of other modules or contracts.
pub trait Ext<T: Trait> {
    /// Call (possibly other predicate) into the specified account.
    fn call(&self, to: &AccountIdOf<T>, input_data: Vec<u8>) -> ExecResult<T>;

    /// Returns a reference to the account id of the caller.
    fn caller(&self) -> &AccountIdOf<T>;

    /// Returns a reference to the account id of the current contract.
    fn address(&self) -> &AccountIdOf<T>;

    // TODO: Notes a call other storage.
    // Only return true or false.
    // CommitmentAddress(special) isCommitment(address) -> Commitment
    // is_stored_predicate(&mut self, address, key, value);?
    // ref: https://github.com/cryptoeconomicslab/ovm-contracts/blob/master/contracts/Predicate/Atomic/IsStoredPredicate.sol
    fn is_stored(&self, _address: &AccountIdOf<T>, _key: &[u8], _value: &[u8]) -> bool;

    /// Needs Plasma.
    fn verify_inclusion_with_root(
        &self,
        leaf: T::Hash,
        token_address: AccountIdOf<T>,
        range: &[u8],
        inclusion_proof: &[u8],
        root: &[u8],
    ) -> bool;

    fn is_decided(&self, property: &PropertyOf<T>) -> bool;
    fn is_decided_by_id(&self, id: T::Hash) -> bool;

    fn set_predicate_decision(
        &self,
        game_id: T::Hash,
        decision: bool,
    ) -> Result<bool, ExecError<T::AccountId>>;
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

    fn execute(
        &self,
        exec: Self::Executable,
        ext: T::ExternalCall,
        input_data: Vec<u8>,
    ) -> ExecResult<T>;
}
