use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash, derive_more::Display)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum PredicateType {
    CompiledPredicate,
    IntermediateCompiledPredicate,
    AtomicProposition,
    AtomicPredicateCall,
    InputPredicateCall,
    VariablePredicateCall,
    CompiledPredicateCall,
    CompiledInput,
    ConstantInput,
    LabelInput,
    NormalInput,
    VariableInput,
    SelfInput,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub enum VarType {
    Address,
    Bytes,
}

pub enum CallablePredicate {
    CompiledPredicate(CompiledPredicate),
}

/// Compiled Property definition
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct CompiledPredicate {
    pub r#type: PredicateType,
    pub name: String,
    pub input_defs: Vec<String>,
    pub contracts: Vec<IntermediateCompiledPredicate>,
    pub constants: Option<Vec<ConstantVariable>>,
    pub entry_point: String,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct ConstantVariable {
    pub var_type: VarType,
    pub name: String,
}

/// IntermediateCompiledPredicate is core of compilation which has only atomic propositions as its inputs.
/// When we have for a in B() {Foo(a) and Bar(a)},
/// "for a in B() {...}" and "Foo(a) and Bar(a)" are IntermediateCompiledPredicate.
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct IntermediateCompiledPredicate {
    pub r#type: PredicateType,
    pub name: String,
    pub original_predicate_name: String,
    // logical connective
    pub connective: LogicalConnective,
    pub input_defs: Vec<String>,
    pub inputs: Vec<AtomicPropositionOrPlaceholder>,
    pub property_inputs: Vec<NormalInput>,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(untagged))]
pub enum AtomicPropositionOrPlaceholder {
    AtomicProposition(AtomicProposition),
    Placeholder(Placeholder),
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct AtomicProposition {
    pub r#type: PredicateType,
    pub predicate: PredicateCall,
    pub inputs: Vec<CompiledInput>,
    pub is_compiled: Option<bool>,
}

pub type Placeholder = String;

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(untagged))]
pub enum PredicateCall {
    AtomicPredicateCall(AtomicPredicateCall),
    InputPredicateCall(InputPredicateCall),
    VariablePredicateCall(VariablePredicateCall),
    CompiledPredicateCall(CompiledPredicateCall),
}

/// e.g. IsValidSignature()
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct AtomicPredicateCall {
    pub r#type: PredicateType,
    pub source: String,
}

/// e.g. a() of "def Foo(a) := a()"
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct InputPredicateCall {
    pub r#type: PredicateType,
    pub source: NormalInput,
}

/// e.g. su() of "def Foo(a) := with SU(a) as su {su()}"
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct VariablePredicateCall {
    pub r#type: PredicateType,
}

/// For predicates dynamic linking
/// e.g. Confsig() user defined predicate
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct CompiledPredicateCall {
    pub r#type: PredicateType,
    pub source: String,
}

/// CompiledInput indicates which value to pass to PredicateCall as input of predicate
/// For example,parent_property.inputs[0].inputs[1] is NormalInput andinput_index is 0 and children is [1].
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(untagged))]
pub enum CompiledInput {
    ConstantInput(ConstantInput),
    LabelInput(LabelInput),
    NormalInput(NormalInput),
    VariableInput(VariableInput),
    SelfInput(SelfInput),
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct ConstantInput {
    pub r#type: PredicateType,
    pub name: String,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct LabelInput {
    pub r#type: PredicateType,
    pub label: String,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct NormalInput {
    pub r#type: PredicateType,
    pub input_index: u8,
    pub children: Vec<i8>,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct VariableInput {
    pub r#type: PredicateType,
    pub placeholder: Placeholder,
    pub children: Vec<i8>,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct SelfInput {
    pub r#type: PredicateType,
    pub children: Vec<i8>,
}

/// LogicalConnective
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum LogicalConnective {
    And,
    ForAllSuchThat,
    Not,
    Or,
    ThereExistsSuchThat,
}

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! serde_from_test {
        ($name:ident, $t:ty, $pr:expr) => {
            #[test]
            fn $name() {
                let res: $t = match serde_json::from_str($pr) {
                    Ok(res) => res,
                    Err(err) => {
                        println!(
                            "ERR: {:?}, {}, {}",
                            err.classify(),
                            err.line(),
                            err.column()
                        );
                        assert!(false);
                        return;
                    }
                };
                println!("success: {:?}", res);
            }
        };
    }

    serde_from_test!(
        logical_connective_test,
        LogicalConnective,
        r#""ThereExistsSuchThat""#
    );

    serde_from_test!(
        atomic_predicate_call_test,
        AtomicPredicateCall,
        r#"{
          "type": "AtomicPredicateCall",
          "source": "IsValidSignature"
        }"#
    );

    serde_from_test!(
        normal_input_test,
        NormalInput,
        r#"{
            "type": "NormalInput",
            "inputIndex": 2,
            "children": []
        }"#
    );

    serde_from_test!(
        variable_input_test,
        VariableInput,
        r#"{
            "type": "VariableInput",
            "placeholder": "v0",
            "children": []
        }"#
    );

    serde_from_test!(
        constant_input_test,
        ConstantInput,
        r#"{
            "type": "ConstantInput",
            "name": "secp256k1"
        }"#
    );

    serde_from_test!(
        atomic_proposition_test,
        AtomicProposition,
        r#"{
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
        }"#
    );

    serde_from_test!(
        intermediate_compiled_predicate_test,
        IntermediateCompiledPredicate,
        r#"
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
        }"#
    );

    serde_from_test!(
        constant_variable_test,
        ConstantVariable,
        r#"
        {
            "varType": "bytes",
            "name": "secp256k1"
        }"#
    );

    #[derive(Serialize, Deserialize, Debug)]
    enum Message {
        Request { id: String, method: String },
        Response { id: String, result: u8 },
    }

    const MES: &str = r#"{"Request": {"id": "...", "method": "..."}}"#;

    serde_from_test!(message_test, Message, MES);
}
