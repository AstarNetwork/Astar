use super::*;
use super::executor::*;

type Address = u64;
struct MockExternalCall;

const Caller: Address = 0;
const PredicateX: Address = 1;

impl ExternalCall for MockExternalCall {
    type Address = Address;

    /// Call (possibly other predicate) into the specified account.
    fn ext_call(&mut self, to: &Self::Address, input_data: Vec<u8>) -> ExecResult<Address> {
        Ok(true)
    }

    /// Returns a reference to the account id of the caller.
    fn ext_caller(&self) -> &Self::Address {
        &Caller
    }

    /// Returns a reference to the account id of the current contract.
    fn ext_address(&self) -> &Self::Address {
        &PredicateX
    }

    // Notes a call other storage.
    // Only return true or false.
    // CommitmentAddress(special) isCommitment(address) -> Commitment
    // is_stored_predicate(&mut self, address, key, value);?
    // ref: https://github.com/cryptoeconomicslab/ovm-contracts/blob/master/contracts/Predicate/Atomic/IsStoredPredicate.sol
    fn ext_is_stored(&mut self, address: &Self::Address, key: &[u8], value: &[u8]) -> bool {
        true
    }
}

struct MockExecutor {
    call: compiled_predicates::PredicateType,

}
