use crate::executor::*;
use crate::predicates::PredicateCallInputs;
use crate::*;

use primitive_types::H256;
use sp_runtime::traits::BlakeTwo256;

type Address = u64;
type Hash = H256;
struct MockExternalCall;

const Caller: Address = 1001;
const PredicateX: Address = 101;

impl ExternalCall for MockExternalCall {
    type Address = Address;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    const NotPredicate: Address = 1;
    const AndPredicate: Address = 2;

    fn ext_call(
        &mut self,
        to: &Self::Address,
        input_data: PredicateCallInputs<Self::Address>,
    ) -> ExecResult<Address> {
        Ok(true)
    }

    fn ext_caller(&self) -> Self::Address {
        Caller
    }

    fn ext_address(&self) -> Self::Address {
        PredicateX
    }

    fn ext_is_stored(&mut self, address: &Self::Address, key: &[u8], value: &[u8]) -> bool {
        true
    }

    fn ext_is_decided(&self, property: &PropertyOf<Self>) -> bool {
        true
    }
    fn ext_get_property_id(&self, property: &PropertyOf<Self>) -> Self::Hash {
        Self::hash_of(property)
    }
    fn ext_set_predicate_decision(
        &self,
        game_id: Self::Hash,
        decision: bool,
    ) -> ExecResult<Self::Address> {
        Ok(true)
    }
}

struct MockExecutor {
    call: compiled_predicates::PredicateType,
}
