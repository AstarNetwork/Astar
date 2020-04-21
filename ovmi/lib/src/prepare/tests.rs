#![cfg(test)]
use super::super::*;
use crate::compiled_predicate::*;
use crate::*;
use codec::{Decode, Encode};

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

#[test]
fn ownership_predicate_test() {
    let ans = CompiledPredicate {
        r#type: "CompiledPredicate".to_string(),
        name: "Ownership".to_string(),
        input_defs: vec!["owner".to_string(), "tx".to_string()],
        contracts: vec![IntermediateCompiledPredicate {
            r#type: "IntermediateCompiledPredicate".to_string(),
            original_predicate_name: "Ownership".to_string(),
            name: "OwnershipT".to_string(),
            connective: LogicalConnective::ThereExistsSuchThat,
            input_defs: vec![
                "OwnershipT".to_string(),
                "owner".to_string(),
                "tx".to_string(),
            ],
            inputs: vec![
                AtomicPropositionOrPlaceholder::Placeholder("signatures,KEY,${tx}".to_string()),
                AtomicPropositionOrPlaceholder::Placeholder("v0".to_string()),
                AtomicPropositionOrPlaceholder::AtomicProposition(AtomicProposition {
                    r#type: "AtomicProposition".to_string(),
                    predicate: PredicateCall::AtomicPredicateCall(AtomicPredicateCall {
                        r#type: "AtomicPredicateCall".to_string(),
                        source: "IsValidSignature".to_string(),
                    }),
                    inputs: vec![
                        CompiledInput::NormalInput(NormalInput {
                            r#type: "NormalInput".to_string(),
                            input_index: 2,
                            children: vec![],
                        }),
                        CompiledInput::VariableInput(VariableInput {
                            r#type: "VariableInput".to_string(),
                            placeholder: "v0".to_string(),
                            children: vec![],
                        }),
                        CompiledInput::NormalInput(NormalInput {
                            r#type: "NormalInput".to_string(),
                            input_index: 1,
                            children: vec![],
                        }),
                        CompiledInput::ConstantInput(ConstantInput {
                            r#type: "ConstantInput".to_string(),
                            name: "secp256k1".to_string(),
                        }),
                    ],
                    is_compiled: None,
                }),
            ],
            property_inputs: vec![],
        }],
        constants: Some(vec![ConstantVariable {
            var_type: "bytes".to_string(),
            name: "secp256k1".to_string(),
        }]),
        entry_point: "OwnershipT".to_string(),
    };
    let res = match compile_from_json(JSON) {
        Ok(res) => res,
        Err(err) => {
            println!("ERR: {:?}", err.classify());
            assert!(false);
            return;
        }
    };
    assert_eq!(res, ans);
    let encoded_res = res.encode();
    let encoded_ans = ans.encode();
    assert_eq!(encoded_res, encoded_ans);
    let decoded_res: CompiledPredicate = Decode::decode(&mut &encoded_res[..]).unwrap();
    let decoded_ans: CompiledPredicate = Decode::decode(&mut &encoded_ans[..]).unwrap();
    assert_eq!(decoded_res, decoded_ans);
    assert_eq!(res, decoded_ans);
}
