use crate::compiled_predicates::*;
use crate::*;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum PredicateTypeSerializable {
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

impl From<PredicateTypeSerializable> for PredicateType {
    fn from(f: PredicateTypeSerializable) -> PredicateType {
        match f {
            PredicateTypeSerializable::CompiledPredicate => PredicateType::CompiledPredicate,
            PredicateTypeSerializable::IntermediateCompiledPredicate => {
                PredicateType::IntermediateCompiledPredicate
            }
            PredicateTypeSerializable::AtomicProposition => PredicateType::AtomicProposition,
            PredicateTypeSerializable::AtomicPredicateCall => PredicateType::AtomicPredicateCall,
            PredicateTypeSerializable::InputPredicateCall => PredicateType::InputPredicateCall,
            PredicateTypeSerializable::VariablePredicateCall => {
                PredicateType::VariablePredicateCall
            }
            PredicateTypeSerializable::CompiledPredicateCall => {
                PredicateType::CompiledPredicateCall
            }
            PredicateTypeSerializable::CompiledInput => PredicateType::CompiledInput,
            PredicateTypeSerializable::ConstantInput => PredicateType::ConstantInput,
            PredicateTypeSerializable::LabelInput => PredicateType::LabelInput,
            PredicateTypeSerializable::NormalInput => PredicateType::NormalInput,
            PredicateTypeSerializable::VariableInput => PredicateType::VariableInput,
            PredicateTypeSerializable::SelfInput => PredicateType::SelfInput,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub enum VarTypeSerializable {
    Address,
    Bytes,
}

impl From<VarTypeSerializable> for VarType {
    fn from(f: VarTypeSerializable) -> VarType {
        match f {
            VarTypeSerializable::Address => VarType::Address,
            VarTypeSerializable::Bytes => VarType::Bytes,
        }
    }
}

/// Compiled Property definition
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct CompiledPredicateSerializable {
    pub r#type: PredicateTypeSerializable,
    pub name: String,
    pub input_defs: Vec<String>,
    pub contracts: Vec<IntermediateCompiledPredicateSerializable>,
    pub constants: Option<Vec<ConstantVariableSerializable>>,
    pub entry_point: String,
}

impl From<CompiledPredicateSerializable> for CompiledPredicate {
    fn from(f: CompiledPredicateSerializable) -> CompiledPredicate {
        CompiledPredicate {
            r#type: f.r#type.into(),
            name: f.name.as_bytes().to_vec(),
            input_defs: f.input_defs.iter().map(|a| a.as_bytes().to_vec()).collect(),
            contracts: f.contracts.iter().map(|a| a.clone().into()).collect(),
            constants: match f.constants {
                Some(constants) => Some(constants.iter().map(|a| a.clone().into()).collect()),
                None => None,
            },
            entry_point: f.entry_point.as_bytes().to_vec(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct ConstantVariableSerializable {
    pub var_type: VarTypeSerializable,
    pub name: String,
}

impl From<ConstantVariableSerializable> for ConstantVariable {
    fn from(f: ConstantVariableSerializable) -> ConstantVariable {
        ConstantVariable {
            var_type: f.var_type.into(),
            name: f.name.as_bytes().to_vec(),
        }
    }
}

/// IntermediateCompiledPredicate is core of compilation which has only atomic propositions as its inputs.
/// When we have for a in B() {Foo(a) and Bar(a)},
/// "for a in B() {...}" and "Foo(a) and Bar(a)" are IntermediateCompiledPredicate.
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct IntermediateCompiledPredicateSerializable {
    pub r#type: PredicateTypeSerializable,
    pub name: String,
    pub original_predicate_name: String,
    // logical connective
    pub connective: LogicalConnectiveSerializable,
    pub input_defs: Vec<String>,
    pub inputs: Vec<AtomicPropositionOrPlaceholderSerializable>,
    pub property_inputs: Vec<NormalInputSerializable>,
}

impl From<IntermediateCompiledPredicateSerializable> for IntermediateCompiledPredicate {
    fn from(f: IntermediateCompiledPredicateSerializable) -> IntermediateCompiledPredicate {
        IntermediateCompiledPredicate {
            r#type: f.r#type.into(),
            name: f.name.as_bytes().to_vec(),
            original_predicate_name: f.original_predicate_name.as_bytes().to_vec(),
            connective: f.connective.into(),
            input_defs: f.input_defs.iter().map(|a| a.as_bytes().to_vec()).collect(),
            inputs: f.inputs.iter().map(|a| a.clone().into()).collect(),
            property_inputs: f.property_inputs.iter().map(|a| a.clone().into()).collect(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(untagged))]
pub enum AtomicPropositionOrPlaceholderSerializable {
    AtomicProposition(AtomicPropositionSerializable),
    Placeholder(String),
}

impl From<AtomicPropositionOrPlaceholderSerializable> for AtomicPropositionOrPlaceholder {
    fn from(f: AtomicPropositionOrPlaceholderSerializable) -> AtomicPropositionOrPlaceholder {
        match f {
            AtomicPropositionOrPlaceholderSerializable::AtomicProposition(a) => {
                AtomicPropositionOrPlaceholder::AtomicProposition(a.clone().into())
            }
            AtomicPropositionOrPlaceholderSerializable::Placeholder(a) => {
                AtomicPropositionOrPlaceholder::Placeholder(a.as_bytes().to_vec())
            }
        }
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct AtomicPropositionSerializable {
    pub r#type: PredicateTypeSerializable,
    pub predicate: PredicateCallSerializable,
    pub inputs: Vec<CompiledInputSerializable>,
    pub is_compiled: Option<bool>,
}

impl From<AtomicPropositionSerializable> for AtomicProposition {
    fn from(f: AtomicPropositionSerializable) -> AtomicProposition {
        AtomicProposition {
            r#type: f.r#type.into(),
            predicate: f.predicate.into(),
            inputs: f.inputs.iter().map(|a| a.clone().into()).collect(),
            is_compiled: f.is_compiled,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(untagged))]
pub enum PredicateCallSerializable {
    AtomicPredicateCall(AtomicPredicateCallSerializable),
    InputPredicateCall(InputPredicateCallSerializable),
    VariablePredicateCall(VariablePredicateCallSerializable),
    CompiledPredicateCall(CompiledPredicateCallSerializable),
}

impl From<PredicateCallSerializable> for PredicateCall {
    fn from(f: PredicateCallSerializable) -> PredicateCall {
        match f {
            PredicateCallSerializable::AtomicPredicateCall(a) => {
                PredicateCall::AtomicPredicateCall(a.clone().into())
            }
            PredicateCallSerializable::InputPredicateCall(a) => {
                PredicateCall::InputPredicateCall(a.clone().into())
            }
            PredicateCallSerializable::VariablePredicateCall(a) => {
                PredicateCall::VariablePredicateCall(a.clone().into())
            }
            PredicateCallSerializable::CompiledPredicateCall(a) => {
                PredicateCall::CompiledPredicateCall(a.clone().into())
            }
        }
    }
}

/// e.g. IsValidSignature()
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct AtomicPredicateCallSerializable {
    pub r#type: PredicateTypeSerializable,
    pub source: String,
}

impl From<AtomicPredicateCallSerializable> for AtomicPredicateCall {
    fn from(f: AtomicPredicateCallSerializable) -> AtomicPredicateCall {
        AtomicPredicateCall {
            r#type: f.r#type.into(),
            source: f.source.as_bytes().to_vec(),
        }
    }
}

/// e.g. a() of "def Foo(a) := a()"
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct InputPredicateCallSerializable {
    pub r#type: PredicateTypeSerializable,
    pub source: NormalInputSerializable,
}

impl From<InputPredicateCallSerializable> for InputPredicateCall {
    fn from(f: InputPredicateCallSerializable) -> InputPredicateCall {
        InputPredicateCall {
            r#type: f.r#type.into(),
            source: f.source.into(),
        }
    }
}

/// e.g. su() of "def Foo(a) := with SU(a) as su {su()}"
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct VariablePredicateCallSerializable {
    pub r#type: PredicateTypeSerializable,
}

impl From<VariablePredicateCallSerializable> for VariablePredicateCall {
    fn from(f: VariablePredicateCallSerializable) -> VariablePredicateCall {
        VariablePredicateCall {
            r#type: f.r#type.into(),
        }
    }
}

/// For predicates dynamic linking
/// e.g. Confsig() user defined predicate
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct CompiledPredicateCallSerializable {
    pub r#type: PredicateTypeSerializable,
    pub source: String,
}

impl From<CompiledPredicateCallSerializable> for CompiledPredicateCall {
    fn from(f: CompiledPredicateCallSerializable) -> CompiledPredicateCall {
        CompiledPredicateCall {
            r#type: f.r#type.into(),
            source: f.source.as_bytes().to_vec(),
        }
    }
}

/// CompiledInput indicates which value to pass to PredicateCall as input of predicate
/// For example,parent_property.inputs[0].inputs[1] is NormalInput andinput_index is 0 and children is [1].
#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(untagged))]
pub enum CompiledInputSerializable {
    ConstantInput(ConstantInputSerializable),
    LabelInput(LabelInputSerializable),
    NormalInput(NormalInputSerializable),
    VariableInput(VariableInputSerializable),
    SelfInput(SelfInputSerializable),
}

impl From<CompiledInputSerializable> for CompiledInput {
    fn from(f: CompiledInputSerializable) -> CompiledInput {
        match f {
            CompiledInputSerializable::ConstantInput(a) => {
                CompiledInput::ConstantInput(a.clone().into())
            }
            CompiledInputSerializable::LabelInput(a) => CompiledInput::LabelInput(a.clone().into()),
            CompiledInputSerializable::NormalInput(a) => {
                CompiledInput::NormalInput(a.clone().into())
            }
            CompiledInputSerializable::VariableInput(a) => {
                CompiledInput::VariableInput(a.clone().into())
            }
            CompiledInputSerializable::SelfInput(a) => CompiledInput::SelfInput(a.clone().into()),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct ConstantInputSerializable {
    pub r#type: PredicateTypeSerializable,
    pub name: String,
}

impl From<ConstantInputSerializable> for ConstantInput {
    fn from(f: ConstantInputSerializable) -> ConstantInput {
        ConstantInput {
            r#type: f.r#type.into(),
            name: f.name.as_bytes().to_vec(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct LabelInputSerializable {
    pub r#type: PredicateTypeSerializable,
    pub label: String,
}

impl From<LabelInputSerializable> for LabelInput {
    fn from(f: LabelInputSerializable) -> LabelInput {
        LabelInput {
            r#type: f.r#type.into(),
            label: f.label.as_bytes().to_vec(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct NormalInputSerializable {
    pub r#type: PredicateTypeSerializable,
    pub input_index: u8,
    pub children: Vec<i8>,
}

impl From<NormalInputSerializable> for NormalInput {
    fn from(f: NormalInputSerializable) -> NormalInput {
        NormalInput {
            r#type: f.r#type.into(),
            input_index: f.input_index,
            children: f.children,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct VariableInputSerializable {
    pub r#type: PredicateTypeSerializable,
    pub placeholder: String,
    pub children: Vec<i8>,
}

impl From<VariableInputSerializable> for VariableInput {
    fn from(f: VariableInputSerializable) -> VariableInput {
        VariableInput {
            r#type: f.r#type.into(),
            placeholder: f.placeholder.as_bytes().to_vec(),
            children: f.children,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct SelfInputSerializable {
    pub r#type: PredicateTypeSerializable,
    pub children: Vec<i8>,
}

impl From<SelfInputSerializable> for SelfInput {
    fn from(f: SelfInputSerializable) -> SelfInput {
        SelfInput {
            r#type: f.r#type.into(),
            children: f.children,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum LogicalConnectiveSerializable {
    And,
    ForAllSuchThat,
    Not,
    Or,
    ThereExistsSuchThat,
}

impl From<LogicalConnectiveSerializable> for LogicalConnective {
    fn from(f: LogicalConnectiveSerializable) -> LogicalConnective {
        match f {
            LogicalConnectiveSerializable::And => LogicalConnective::And,
            LogicalConnectiveSerializable::ForAllSuchThat => LogicalConnective::ForAllSuchThat,
            LogicalConnectiveSerializable::Not => LogicalConnective::Not,
            LogicalConnectiveSerializable::Or => LogicalConnective::Or,
            LogicalConnectiveSerializable::ThereExistsSuchThat => {
                LogicalConnective::ThereExistsSuchThat
            }
        }
    }
}
