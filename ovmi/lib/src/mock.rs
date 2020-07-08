use crate::executor::*;
use crate::predicates::*;
use crate::prepare::*;
use crate::*;
use alloc::collections::btree_map::BTreeMap;

use crate::prepare::{
    base_atomic_executable_from_address, deciable_executable_from_address,
    executable_from_compiled, logical_connective_executable_from_address,
};
use primitive_types::H256;
pub use sp_runtime::traits::BlakeTwo256;

pub type Address = u64;
pub type Hash = H256;
pub struct MockExternalCall {
    mock_stored: BTreeMap<Address, BTreeMap<Vec<u8>, Vec<u8>>>,
}

pub const PAY_OUT_CONTRACT_ADDRESS: Address = 10001;
pub const CALLER_ADDRESS: Address = 1001;
pub const PREDICATE_X_ADDRESS: Address = 101;

pub const NOT_ADDRESS: Address = 1;
pub const AND_ADDRESS: Address = 2;
pub const OR_ADDRESS: Address = 3;
pub const FOR_ALL_ADDRESS: Address = 4;
pub const THERE_EXISTS_ADDRESS: Address = 5;
pub const EQUAL_ADDRESS: Address = 6;
pub const IS_CONTAINED_ADDRESS: Address = 7;
pub const IS_LESS_ADDRESS: Address = 8;
pub const IS_STORED_ADDRESS: Address = 9;
pub const IS_VALID_SIGNATURE_ADDRESS: Address = 10;
pub const VERIFY_INCLUAION_ADDRESS: Address = 11;

// pub const SECP_256_K1: Hash = ;

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

    // test compiled_parts_from_address
    pub fn compiled_parts_from_address(
        address: &Address,
    ) -> (
        CompiledPredicate,
        Address,
        BTreeMap<Hash, Address>,
        BTreeMap<Hash, Vec<u8>>,
    ) {
        let code = compile_from_json(JSON).unwrap();
        let payout = PAY_OUT_CONTRACT_ADDRESS;
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
    ) -> ExecResult<Address> {
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
            _ => Err(ExecError::Unimplemented),
        }
    }
}

impl ExternalCall for MockExternalCall {
    type Address = Address;
    type Hash = Hash;
    type Hashing = BlakeTwo256;

    const NOT_ADDRESS: Address = NOT_ADDRESS;
    const AND_ADDRESS: Address = AND_ADDRESS;
    const OR_ADDRESS: Address = OR_ADDRESS;
    const FOR_ALL_ADDRESS: Address = FOR_ALL_ADDRESS;
    const THERE_EXISTS_ADDRESS: Address = THERE_EXISTS_ADDRESS;
    const EQUAL_ADDRESS: Address = EQUAL_ADDRESS;
    const IS_CONTAINED_ADDRESS: Address = IS_CONTAINED_ADDRESS;
    const IS_LESS_ADDRESS: Address = IS_LESS_ADDRESS;
    const IS_STORED_ADDRESS: Address = IS_STORED_ADDRESS;
    const IS_VALID_SIGNATURE_ADDRESS: Address = IS_VALID_SIGNATURE_ADDRESS;
    const VERIFY_INCLUAION_ADDRESS: Address = VERIFY_INCLUAION_ADDRESS;

    // const SECP_256_K1: Hash = SECP_256_K1;

    fn ext_call(
        &self,
        to: &Self::Address,
        input_data: PredicateCallInputs<Self::Address>,
    ) -> ExecResult<Address> {
        self.call_execute(to, input_data)
    }

    fn ext_caller(&self) -> Self::Address {
        CALLER_ADDRESS
    }

    fn ext_address(&self) -> Self::Address {
        PREDICATE_X_ADDRESS
    }

    fn ext_is_stored(&self, address: &Self::Address, key: &[u8], value: &[u8]) -> bool {
        if let Some(s) = self.mock_stored.get(address) {
            if let Some(res) = s.get(&key.to_vec()) {
                return res == &value.to_vec();
            }
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
