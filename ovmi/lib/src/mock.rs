use crate::executor::*;
use crate::predicates::*;
use crate::prepare::*;
use crate::*;

use crate::prepare::{
    base_atomic_executable_from_address, deciable_executable_from_address,
    executable_from_compiled, logical_connective_executable_from_address,
};
use primitive_types::H256;
use sp_runtime::traits::BlakeTwo256;

type Address = u64;
type Hash = H256;
struct MockExternalCall;

const PayOutContract: Address = 10001;
const Caller: Address = 1001;
const PredicateX: Address = 101;

const JSON: &str = r#"
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
    // test compiled_parts_from_address
    fn compiled_parts_from_address(
        address: &Address,
    ) -> (
        CompiledPredicate,
        Address,
        BTreeMap<Hash, Address>,
        BTreeMap<Hash, Vec<u8>>,
    ) {
        let code = compile_from_json(JSON).unwrap();
        let payout = PayOutContract;
        let address_inputs = BTreeMap::new();
        let bytes_inputs = vec![(Hash::default(), vec![0 as u8, 1 as u8])]
            .iter()
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect();
        (code, payout, address_inputs, bytes_inputs)
    }
}

impl ExternalCall for MockExternalCall {
    type Address = Address;
    type Hash = Hash;
    type Hashing = BlakeTwo256;

    const NotAddress: Address = 1;
    const AndAddress: Address = 2;
    const OrAddress: Address = 3;
    const ForAllAddress: Address = 4;
    const ThereExistsAddress: Address = 5;
    const EqualAddress: Address = 6;
    const IsContainedAddress: Address = 7;
    const IsLessAddress: Address = 8;
    const IsStoredAddress: Address = 9;
    const IsValidSignatureAddress: Address = 10;
    const VerifyInclusion: Address = 11;

    fn ext_call(
        &self,
        to: &Self::Address,
        input_data: PredicateCallInputs<Self::Address>,
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

    fn ext_caller(&self) -> Self::Address {
        Caller
    }

    fn ext_address(&self) -> Self::Address {
        PredicateX
    }

    fn ext_is_stored(&self, address: &Self::Address, key: &[u8], value: &[u8]) -> bool {
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
