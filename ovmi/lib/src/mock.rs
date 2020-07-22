use crate::executor::*;
use crate::predicates::*;
use crate::prepare::*;
use crate::*;
use alloc::collections::btree_map::BTreeMap;
pub use hex_literal::hex;
use lazy_static::lazy_static;

use crate::prepare::{
    base_atomic_executable_from_address, deciable_executable_from_address,
    executable_from_compiled, logical_connective_executable_from_address,
};
use primitive_types::H256;
pub use sp_runtime::traits::Keccak256;

pub use sp_core::{
    crypto::{AccountId32, Pair, UncheckedFrom},
    ecdsa::{Pair as ECDSAPair, Public, Signature},
};
use sp_runtime::{traits::IdentifyAccount, MultiSigner};

pub type Address = AccountId32;
pub type Hash = H256;

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
struct StoredPredicate {
    pub code: CompiledPredicate,
    pub payout: Address,
    address_inputs: BTreeMap<Hash, Address>,
    bytes_inputs: BTreeMap<Hash, Address>,
}

pub struct MockExternalCall {
    mock_stored: BTreeMap<Address, BTreeMap<Vec<u8>, Vec<u8>>>,
    mock_predicate: BTreeMap<Address, StoredPredicate>,
}

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

pub const JSON: &str = r#"
  {
    "type": "CompiledPredicate",
    "name": "Ownership",
    "inputDefs": [
      "owner",
      "tx"
    ],
    "contracts": [
      {
        "type": "IntermediateCompiledPredicate",
        "originalPredicateName": "Ownership",
        "name": "OwnershipT",
        "connective": "ThereExistsSuchThat",
        "inputDefs": [
          "OwnershipT",
          "owner",
          "tx"
        ],
        "inputs": [
          "signatures,KEY,${tx}",
          "v0",
          {
            "type": "AtomicProposition",
            "predicate": {
              "type": "AtomicPredicateCall",
              "source": "IsValidSignature"
            },
            "inputs": [
              {
                "type": "NormalInput",
                "inputIndex": 2,
                "children": []
              },
              {
                "type": "VariableInput",
                "placeholder": "v0",
                "children": []
              },
              {
                "type": "NormalInput",
                "inputIndex": 1,
                "children": []
              },
              {
                "type": "ConstantInput",
                "name": "secp256k1"
              }
            ]
          }
        ],
        "propertyInputs": []
      }
    ],
    "entryPoint": "OwnershipT",
    "constants": [
      {
        "varType": "bytes",
        "name": "secp256k1"
      }
    ]
  }"#;

impl MockExternalCall {
    pub fn init() -> Self {
        MockExternalCall {
            mock_stored: BTreeMap::new(),
            mock_predicate: BTreeMap::new(),
        }
    }

    // test set stored.
    pub fn set_stored(&mut self, address: &Address, key: &[u8], value: &[u8]) {
        if !self.mock_stored.contains_key(&address) {
            self.mock_stored.insert(address.clone(), BTreeMap::new());
        }
        let mut s = self
            .mock_stored
            .get_mut(address)
            .map(|s| s.clone())
            .unwrap();
        s.insert(key.to_vec(), value.to_vec());
        self.mock_stored.insert(address.clone(), s);
    }

    // test deploy
    pub fn deploy(
        &mut self,
        compiled_predicate: CompiledPredicate,
        payout: Address,
        address_inputs: BTreeMap<Hash, Address>,
        bytes_inputs: BTreeMap<Hash, Address>,
    ) -> Address {
        let stored = StoredPredicate {
            code: compiled_predicate,
            payout,
            address_inputs,
            bytes_inputs,
        };
        let hash = Self::hash_of(&stored);
        let address: Address = UncheckedFrom::unchecked_from(hash);
        self.mock_predicate.insert(address.clone(), stored);
        address
    }

    // test compiled_parts_from_address
    pub fn compiled_parts_from_address(
        _address: &Address,
    ) -> (
        CompiledPredicate,
        Address,
        BTreeMap<Hash, Address>,
        BTreeMap<Hash, Vec<u8>>,
    ) {
        let code = compile_from_json(JSON).unwrap();
        let payout = (*PAY_OUT_CONTRACT_ADDRESS).clone();
        let address_inputs = BTreeMap::new();
        let bytes_inputs = vec![(Hash::default(), vec![0 as u8, 1 as u8])]
            .iter()
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect();
        (code, payout, address_inputs, bytes_inputs)
    }

    pub fn call_execute(
        &self,
        to: &Address,
        input_data: PredicateCallInputs<Address>,
    ) -> ExecResultT<Vec<u8>, Address> {
        println!("call_execute to:         {:?}", to);
        println!("call_execute input_data: {:?}", input_data);
        match &input_data {
            PredicateCallInputs::DecidablePredicate(_) => {
                let p =
                    deciable_executable_from_address(self, to).ok_or(ExecError::CallAddress {
                        address: to.clone(),
                    })?;
                DecidableExecutor::<DecidableExecutable<Self>, Self>::execute(p, input_data)
            }
            PredicateCallInputs::LogicalConnective(_) => {
                let p = logical_connective_executable_from_address(self, to).ok_or(
                    ExecError::CallAddress {
                        address: to.clone(),
                    },
                )?;
                LogicalConnectiveExecutor::<LogicalConnectiveExecutable<Self>, Self>::execute(
                    p, input_data,
                )
            }
            PredicateCallInputs::AtomicPredicate(_) => {
                let p = atomic_executable_from_address(self, to).ok_or(ExecError::CallAddress {
                    address: to.clone(),
                })?;
                AtomicExecutor::<AtomicExecutable<Self>, Self>::execute(p, input_data)
            }
            PredicateCallInputs::BaseAtomicPredicate(_) => {
                let p = base_atomic_executable_from_address(self, to).ok_or(
                    ExecError::CallAddress {
                        address: to.clone(),
                    },
                )?;
                BaseAtomicExecutor::<BaseAtomicExecutable<Self>, Self>::execute(p, input_data)
            }
            PredicateCallInputs::CompiledPredicate(_) => {
                let (cp, payout, address_inputs, bytes_inputs) =
                    Self::compiled_parts_from_address(to);
                let p = executable_from_compiled(self, cp, payout, address_inputs, bytes_inputs);
                CompiledExecutor::<CompiledExecutable<Self>, Self>::execute(p, input_data)
            }
        }
    }
}

impl ExternalCall for MockExternalCall {
    type Address = Address;
    type Hash = Hash;
    type Hashing = Keccak256;

    fn not_address() -> Self::Address {
        (*NOT_ADDRESS).clone()
    }
    fn and_address() -> Self::Address {
        (*AND_ADDRESS).clone()
    }
    fn or_address() -> Self::Address {
        (*OR_ADDRESS).clone()
    }
    fn for_all_address() -> Self::Address {
        (*FOR_ALL_ADDRESS).clone()
    }
    fn there_exists_address() -> Self::Address {
        (*THERE_EXISTS_ADDRESS).clone()
    }
    fn equal_address() -> Self::Address {
        (*EQUAL_ADDRESS).clone()
    }
    fn is_contained_address() -> Self::Address {
        (*IS_CONTAINED_ADDRESS).clone()
    }
    fn is_less_address() -> Self::Address {
        (*IS_LESS_ADDRESS).clone()
    }
    fn is_stored_address() -> Self::Address {
        (*IS_STORED_ADDRESS).clone()
    }
    fn is_valid_signature_address() -> Self::Address {
        (*IS_VALID_SIGNATURE_ADDRESS).clone()
    }
    fn verify_inclusion_address() -> Self::Address {
        (*VERIFY_INCLUAION_ADDRESS).clone()
    }
    fn secp256k1() -> Self::Hash {
        (*SECP_256_K1).clone()
    }

    fn ext_call(
        &self,
        to: &Self::Address,
        input_data: PredicateCallInputs<Self::Address>,
    ) -> ExecResultT<Vec<u8>, Address> {
        self.call_execute(to, input_data)
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

pub fn to_account_from_seed(seed: &[u8; 32]) -> Address {
    to_account(ECDSAPair::from_seed(&seed).public().as_ref())
}

pub fn to_account(full_public: &[u8]) -> Address {
    let public = sp_core::ecdsa::Public::from_full(full_public).unwrap();
    MultiSigner::from(public).into_account()
}
