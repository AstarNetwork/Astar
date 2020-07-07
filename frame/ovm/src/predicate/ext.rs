//! The external call function connecting to OVMinterpreter.

use crate::traits::Ext;
use crate::*;
use ovmi::executor::{ExecError, ExecResultT, ExecResultTOf, ExternalCall};
use ovmi::predicates::PredicateCallInputs;
use ovmi::prepare;

pub use sp_core::ecdsa;

lazy_static! {
    pub static ref PAY_OUT_CONTRACT_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000010000"
    ]);
    pub static ref CALLER_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000001"
    ]);
    pub static ref PREDICATE_X_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000002"
    ]);
}

lazy_static! {
    pub static ref NOT_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000003"
    ]);
    pub static ref AND_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000004"
    ]);
    pub static ref OR_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000005"
    ]);
    pub static ref FOR_ALL_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000006"
    ]);
    pub static ref THERE_EXISTS_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000007"
    ]);
    pub static ref EQUAL_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000008"
    ]);
    pub static ref IS_CONTAINED_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000009"
    ]);
    pub static ref IS_LESS_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000010"
    ]);
    pub static ref IS_STORED_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000011"
    ]);
    pub static ref IS_VALID_SIGNATURE_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000012"
    ]);
    pub static ref VERIFY_INCLUAION_ADDRESS: Address = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000013"
    ]);
    pub static ref SECP_256_K1: Hash = BlakeTwo256::hash(&b"secp256k1".to_vec());
}

// Setting External environment.
pub struct ExternalCallImpl<T: Trait, E: Ext<T, ExecError<T::AccountIdL2>>> {
    pub inter: E,
}

impl<T: Trait, E: Ext<T, ExecError<T::AccountIdL2>>> ExternalCallImpl<T, E> {
    pub fn new(inter: E) -> Self {
        ExternalCallImpl { inter }
    }
}

impl<T: Trait, E: Ext<T, T::AccountIdL2>> ExternalCall for ExternalCallImpl<T, E> {
    type Address = T::AccountIdL2;
    type Hash = T::Hash;
    type Hashing = T::HashingL2;

    fn not_address() -> Self::Address {
        (*NOT_ADDRESS).clone().clone()
    }
    fn and_address() -> Self::Address {
        (*AND_ADDRESS).clone().clone()
    }
    fn or_address() -> Self::Address {
        (*OR_ADDRESS).clone().clone()
    }
    fn for_all_address() -> Self::Address {
        (*FOR_ALL_ADDRESS).clone().clone()
    }
    fn there_exists_address() -> Self::Address {
        (*THERE_EXISTS_ADDRESS).clone().clone()
    }
    fn equal_address() -> Self::Address {
        (*EQUAL_ADDRESS).clone().clone()
    }
    fn is_contained_address() -> Self::Address {
        (*IS_CONTAINED_ADDRESS).clone().clone()
    }
    fn is_less_address() -> Self::Address {
        (*IS_LESS_ADDRESS).clone().clone()
    }
    fn is_stored_address() -> Self::Address {
        (*IS_STORED_ADDRESS).clone().clone()
    }
    fn is_valid_signature_address() -> Self::Address {
        (*IS_VALID_SIGNATURE_ADDRESS).clone().clone()
    }
    fn verify_inclusion_address() -> Self::Address {
        (*VERIFY_INCLUAION_ADDRESS).clone().clone()
    }
    fn secp256k1() -> Self::Hash {
        (*SECP_256_K1).clone()
    }

    fn ext_call(
        &self,
        to: &Self::Address,
        input_data: PredicateCallInputs<Self::Address>,
    ) -> ExecResultT<Vec<u8>, Self::Address> {
        self.inter.call(to, input_data.encode())
    }

    fn ext_caller(&self) -> Self::Address {
        self.inter.caller()
    }

    fn ext_address(&self) -> Self::Address {
        self.inter.address()
    }

    fn ext_is_stored(&self, address: &Self::Address, key: &[u8], value: &[u8]) -> bool {
        self.inter.is_stored(address, key, value)
    }

    /// At first implementing, only ECDSA signature.
    fn ext_verify(&self, hash: &Self::Hash, signature: &[u8], address: &Self::Address) -> bool {
        if signature.len() != 65 {
            return false;
        }
        let sig: ecdsa::Signature = ecdsa::Signature::from_slice(signature);
        if let Some(public) = sig.recover(hash) {
            return address == &MultiSigner::from(public).into_account();
        }
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

    fn ext_is_decided(&self, property: &PropertyOf<Self>) -> bool {
        self.inter.is_decided(property)
    }
    fn ext_is_decided_by_id(&self, id: Self::Hash) -> bool {
        self.inter.is_decided_by_id(id)
    }
    fn ext_get_property_id(&self, property: &PropertyOf<Self>) -> Self::Hash {
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
