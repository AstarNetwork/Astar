//! The external call function connecting to OVMinterpreter.

use crate::*;
use ovmi::prepare;
use ovmi::ExternalCall;
use ovmi::PredicateCallInputs;
use crate::traits::Ext;

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
    pub static ref SECP_256_K1: Hash = Hash::from(&hex![
        "d4fa99b1e08c4e5e6deb461846aa629344d95ff03ed04754c2053d54c756f439"
    ]);
}

// Setting External environment.
struct ExternalCallImpl<T: Trait, E: Ext> {
    pub inter: Ext,
}

impl<T: Trait, E: Ext> ExternalCall for ExternalCallImpl<T, E> {
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
    ) -> ExecResultT<Vec<u8>, Address> {
        self.inter.call(to, input_data.encode());
    }

    fn ext_caller(&self) -> Self::Address {
        (*CALLER_ADDRESS).clone()
    }

    fn ext_address(&self) -> Self::Address {
        (*PREDICATE_X_ADDRESS).clone()
    }

    fn ext_is_stored(&self, address: &Self::Address, key: &[u8], value: &[u8]) -> bool {
        if let Some(s) = self.mock_stored.get(address) {
            if let Some(res) = s.get(&key.to_vec()) {
                return res == &value.to_vec();
            }
        }
        false
    }

    fn ext_verify(&self, hash: &Self::Hash, signature: &[u8], address: &Self::Address) -> bool {
        println!("ext_verify hash     : {:?}", hash);
        println!("ext_verify signature: {:?}", signature);
        println!("ext_verify address  : {:?}", address);
        if signature.len() != 65 {
            return false;
        }
        let sig: Signature = Signature::from_slice(signature);
        println!("ext_verify after    : {:?}", sig);
        if let Some(public) = sig.recover(hash) {
            println!(
                "ext_verify public    : {:?}",
                &MultiSigner::from(public.clone()).into_account()
            );
            return address == &MultiSigner::from(public).into_account();
        }
        false
    }

    fn ext_verify_inclusion_with_root(
        &self,
        _leaf: Self::Hash,
        _token_address: Self::Address,
        _range: &[u8],
        _inclusion_proof: &[u8],
        _root: &[u8],
    ) -> bool {
        true
    }

    fn ext_is_decided(&self, _property: &PropertyOf<Self>) -> bool {
        true
    }
    fn ext_is_decided_by_id(&self, _id: Self::Hash) -> bool {
        true
    }
    fn ext_get_property_id(&self, property: &PropertyOf<Self>) -> Self::Hash {
        Self::hash_of(property)
    }
    fn ext_set_predicate_decision(
        &self,
        _game_id: Self::Hash,
        _decision: bool,
    ) -> ExecResult<Self::Address> {
        Ok(true)
    }
}

fn call_execute(
    inputs: Vec<Vec<u8>>,
    inputs: PredicateCallInputs<AccountId>,
) -> ExecResult<AccountId> {
    let compiled_predicate = prepare::compile_from_json("<compiled_predicate_json>").unwrap();
    let (payout, address_input, bytes_inputs) = prepare::parse_inputs(inputs);
    let ext = MockExternalCall {};
    let executable = prepare::executable_from_compiled(
        &mut ext,
        code: compiled_predicate,
        payout,
        address_inputs,
        bytes_inputs,
    );
    // execute and return value.
    CompiledExecutor::execute(&executable, inputs)
}
