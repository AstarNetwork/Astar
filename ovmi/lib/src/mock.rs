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


fn get_deciable_predicate_struct(address: &Self::Address) -> DecidableExecutable<'a, MockExternalCall> {
    match address {
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
}

impl ExternalCall for MockExternalCall {
    type Address = Address;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    const NotPredicate: Address = 1;
    const AndPredicate: Address = 2;
    const OrAddress: Address = 3;
    const ForAllAddress: Address = 4;
    const ThereExistsAddress: Address = 5;
    const EqualAddress: Address = 6;
    const IsContainedAddress: Address = 7;
    const IsLessAddress: Address = 8;
    const IsStoredAddress: Address = 9;
    const IsValidSignatureAddress: Address = 10;

    fn ext_call(
        &self,
        to: &Self::Address,
        input_data: PredicateCallInputs<Self::Address>,
    ) -> ExecResult<Address> {
        match &input_data {
            PredicateCallInputs::AtomicPredicate(_) => {
            },
            PredicateCallInputs::DecidablePredicate(_) => {
              let p = get_predicate_struct()
            },
            PredicateCallInputs::LogicalConnective(_),
            PredicateCallInputs::BaseAtomicPredicate(_),
            PredicateCallInputs::CompiledPredicate(_),
        }
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

    fn ext_verify_inclusion_with_root(
        &self,
        leaf: Self::Hash,
        token_address: Self::Address,
        range: &[u8],
        inclusion_proof: &[u8],
        root: &[u8],
    ) -> bool {
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
