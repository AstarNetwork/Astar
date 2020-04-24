use crate::executor::*;
use crate::predicates::PredicateCallInputs;
use crate::*;

type Address = u64;
type Hash = u64;
struct MockExternalCall;

const Caller: Address = 1001;
const PredicateX: Address = 101;

impl ExternalCall for MockExternalCall {
    type Address = Address;
    type Hash = Hash;
    const NotPredicate: Address = 1;
    const AndPredicate: Address = 2;

    /// Call (possibly other predicate) into the specified account.
    fn ext_call(
        &mut self,
        to: &Self::Address,
        input_data: PredicateCallInputs<Self::Address>,
    ) -> ExecResult<Address> {
        Ok(true)
    }

    /// Returns a reference to the account id of the caller.
    fn ext_caller(&self) -> Self::Address {
        Caller
    }

    /// Returns a reference to the account id of the current contract.
    fn ext_address(&self) -> Self::Address {
        PredicateX
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
