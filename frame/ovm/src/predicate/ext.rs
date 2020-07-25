//! The external call function connecting to OVMinterpreter.

use crate::traits::Ext;
use crate::*;
use ovmi::executor::{ExecResultT, ExternalCall};
use ovmi::predicates::PredicateCallInputs;

// Setting External environment.
pub struct ExternalCallImpl<'a, T: Trait> {
    pub inter: &'a T::ExternalCall,
}

impl<'a, T: Trait> ExternalCallImpl<'a, T> {
    pub fn new(inter: &'a T::ExternalCall) -> Self {
        ExternalCallImpl { inter }
    }
}

impl<T: Trait> ExternalCall for ExternalCallImpl<'_, T> {
    type Address = T::AccountId;
    type Hash = T::Hash;
    type Hashing = T::HashingL2;

    fn not_address() -> Self::Address {
        T::AtomicPredicateIdConfig::get().not_address
    }
    fn and_address() -> Self::Address {
        T::AtomicPredicateIdConfig::get().and_address
    }
    fn or_address() -> Self::Address {
        T::AtomicPredicateIdConfig::get().or_address
    }
    fn for_all_address() -> Self::Address {
        T::AtomicPredicateIdConfig::get().for_all_address
    }
    fn there_exists_address() -> Self::Address {
        T::AtomicPredicateIdConfig::get().there_exists_address
    }
    fn equal_address() -> Self::Address {
        T::AtomicPredicateIdConfig::get().equal_address
    }
    fn is_contained_address() -> Self::Address {
        T::AtomicPredicateIdConfig::get().is_contained_address
    }
    fn is_less_address() -> Self::Address {
        T::AtomicPredicateIdConfig::get().is_less_address
    }
    fn is_stored_address() -> Self::Address {
        T::AtomicPredicateIdConfig::get().is_stored_address
    }
    fn is_valid_signature_address() -> Self::Address {
        T::AtomicPredicateIdConfig::get().is_valid_signature_address
    }
    fn verify_inclusion_address() -> Self::Address {
        T::AtomicPredicateIdConfig::get().verify_inclusion_address
    }
    fn secp256k1() -> Self::Hash {
        T::AtomicPredicateIdConfig::get().secp256k1
    }

    fn ext_call(
        &self,
        to: &Self::Address,
        input_data: PredicateCallInputs<Self::Address>,
    ) -> ExecResultT<Vec<u8>, Self::Address> {
        self.inter.call(to, input_data.encode())
    }

    fn ext_caller(&self) -> Self::Address {
        self.inter.caller().clone()
    }

    fn ext_address(&self) -> Self::Address {
        self.inter.address().clone()
    }

    fn ext_is_stored(&self, address: &Self::Address, key: &[u8], value: &[u8]) -> bool {
        self.inter.is_stored(address, key, value)
    }

    /// At first implementing, only ECDSA signature.
    fn ext_verify(&self, _hash: &Self::Hash, _signature: &[u8], _address: &Self::Address) -> bool {
        // if signature.len() != 65 {
        //     return false;
        // }
        // let sig: ecdsa::Signature = ecdsa::Signature::from_slice(signature);
        // if let Some(public) = sig.recover(hash) {
        //     return Self::hash_of(address)
        //         == Self::hash_of(&MultiSigner::from(public).into_account());
        // }
        false
    }

    fn ext_verify_inclusion_with_root(
        &self,
        leaf: Self::Hash,
        token_address: Self::Address,
        range: &[u8],
        inclusion_proof: &[u8],
        root: &[u8],
    ) -> bool {
        self.inter
            .verify_inclusion_with_root(leaf, token_address, range, inclusion_proof, root)
    }

    fn ext_is_decided(&self, property: &ovmi::executor::PropertyOf<Self>) -> bool {
        if let Ok(property) = Decode::decode(&mut &property.encode()[..]) {
            return self.inter.is_decided(&property);
        }
        false
    }
    fn ext_is_decided_by_id(&self, id: Self::Hash) -> bool {
        self.inter.is_decided_by_id(id)
    }
    fn ext_get_property_id(&self, property: &ovmi::executor::PropertyOf<Self>) -> Self::Hash {
        Self::hash_of(property)
    }
    fn ext_set_predicate_decision(
        &self,
        game_id: Self::Hash,
        decision: bool,
    ) -> ExecResultT<bool, Self::Address> {
        self.inter.set_predicate_decision(game_id, decision)
    }
}
