//! Compiled Executable is simulated CompiledPredicate.
//! This simulate refer to https://github.com/cryptoeconomicslab/gazelle/tree/master/packages/ovm-solidity-generator/src.

use crate::compiled_predicates::*;
use crate::executor::*;
use crate::predicates::*;
use crate::*;

// Compiled Predicate transpiles to this structure.
pub struct CompiledExecutable<'a, Ext: ExternalCall> {
    pub ext: &'a Ext,
    pub payout: AddressOf<Ext>,
    pub code: CompiledPredicate,
    pub constants: BTreeMap<HashOf<Ext>, VarType>,
    pub address_inputs: BTreeMap<HashOf<Ext>, AddressOf<Ext>>,
    pub bytes_inputs: BTreeMap<HashOf<Ext>, Vec<u8>>,
}

impl<Ext: ExternalCall> CompiledPredicateInterface<AddressOf<Ext>> for CompiledExecutable<'_, Ext> {
    fn payout_contract_address(&self) -> AddressOf<Ext> {
        self.payout.clone()
    }

    fn is_valid_challenge(
        &self,
        _inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        _challenge: Property<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Ok(true)
    }

    /// @dev get valid child property of game tree with challenge_inputs
    fn get_child(
        &self,
        inputs: Vec<Vec<u8>>,
        _challenge_input: Vec<Vec<u8>>,
    ) -> ExecResultTOf<Property<AddressOf<Ext>>, Ext> {
        require!(inputs.len() > 1);
        if (Ext::is_label(&inputs[0])) {
            let intermediate = self.resolve_intermediate(&self.entry_point)?;
            return self.get_child_intermediate(intermediate, &inputs, &challenge_inputs);
        }
        let input0: String = Decode::decode(&mut &Ext::get_input_value(&inputs[0])[..])
            .map_err(|_| codec_error::<Ext>("String"))?;
        let sub_inputs = Ext::sub_array(&inputs, 1, inputs.len() as u128);

        let intermediate = self.resolve_intermediate(&input0)?;
        return self.get_child_intermediate(intermediate, &sub_inputs, &challenge_inputs);
    }

    fn decide(&self, _inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>) -> ExecResult<AddressOf<Ext>> {
        Ok(true)
    }

    fn decide_true(
        &self,
        _inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Ok(true)
    }

    fn decide_with_witness(
        &self,
        _inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Ok(true)
    }
}

impl<Ext: ExternalCall> CompiledExecutable<'_, Ext> {
    fn get_child_intermediate(
        &self,
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
    ) -> ExecResultTOf<PropertyOf<Ext>, Ext> {
        let input_property_list = inter
            .property_inputs
            .iter()
            .map(|property_input| {
                Decode::decode(&mut &inputs[property_input.input_index - 1][..])
                    .map_err(codec_error::<Ext>("PropertyOf<Ext>"))?
            })
            .collect::<Vec<PropertyOf<Ext>>>();
        let input_property_list_child_list = inter
            .property_inputs
            .iter()
            .map(|property_input| {
                let mut ret = BTreeMap::new();
                if property_input.children.len() > 0 {
                    require!(
                        input_property[property_input.input_index].inputs
                            > property_input.children[0]
                    );
                    ret.insert(
                        property_input.children[0],
                        input_property[property_input.input_index].inputs
                            [property_input.children[0]],
                    );
                }
                Ok(ret)
            })
            .map(|res| res?)
            .collect::<Vec<BTreeMap<i8, PropertyOf<Ext>>>>();

        match &inter.connective {
            LogicalConnective::And => self.get_child_and(
                inter,
                inputs,
                challenge_inputs,
                &input_property_list,
                &input_property_list_child_list,
            ),
            LogicalConnective::ForAllSuchThat => {
                self.get_child_for_all_such_that(inter, inputs, challenge_inputs)
            }
            LogicalConnective::Not => self.get_child_not(inter, inputs, challenge_inputs),
            LogicalConnective::Or => self.get_child_or(inter, inputs, challenge_inputs),
            LogicalConnective::ThereExistsSuchThat => {
                self.get_child_there_exists_such_that(inter, inputs, challenge_inputs)
            }
        }
    }

    fn get_child_and(
        &self,
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
        input_property: &Vec<PropertyOf<Ext>>,
        input_property_list_child_list: &Vec<BTreeMap<i8, PropertyOf<Ext>>>,
    ) -> ExecResultTOf<PropertyOf<Ext>, Ext> {
        let challenge_input: u128 =
            Decode::decode(&mut &challenge_inputs[0]).map_err(|_| codec_error::<Ext>("u128"))?;
        let not_inputs = vec![vec![1 as u8]];

        require!(inter.inputs.len() > challenge_input as u8);
        let item = inter.inputs[challenge_input];
        match item {
            AtomicPropositionOrPlaceholder::AtomicProposition(item) => {
                if let Some(is_compiled) = item.is_compiled {
                    if is_compiled {
                        require!(item.inputs.len() > 1);
                        let child_inputs = self.construct_inputs(
                            &item,
                            &challenge_inputs[0],
                            inputs,
                            input_property,
                            input_property_list_child_list,
                        )?;
                        return self.get_child(
                            child_inputs,
                            Ext::sub_array(challenge_inputs, 1, challenge_inputs.len() as u128),
                        );
                    } else if let PredicateCall::CompiledPredicateCall(pred) = item.predicate {
                        let child_inputs = self.construct_inputs(
                            &item,
                            &challenge_inputs[0],
                            inputs,
                            input_property,
                            input_property_list_child_list,
                        )?;
                        let ret = self.ext.ext_call(
                            self.get_address_variable(&pred.source)?,
                            PredicateCallInputs::CompiledPredicate(
                                CompiledPredicateCallInputs::GetChild {
                                    inputs: child_inputs,
                                    challenge_input: self.sub_array(
                                        challenge_input,
                                        1,
                                        challenge_input.len() as u128,
                                    ),
                                },
                            ),
                        )?;
                        return Ok(Decode::decode(&mut &ret[..])
                            .map_err(|_| codec_error::<Ext>("PropertyOf<Ext>"))?);
                    }
                }
                // TODO: construct_property
                let not_inputs = self.construct_property(&item);
                Ok(Property {
                    predicate_address: self.ext.not_address(),
                    inputs: not_inputs,
                })
            }
            _ => Err(ExecError::Unexpected {
                msg: "get_child_and must be AtomicProposition.",
            }),
        }
    }
    // <%  if(property.connective == 'And') { -%>
    //         uint256 challengeInput = abi.decode(challengeInputs[0], (uint256));
    //         bytes[] memory notInputs = new bytes[](1);
    // <%
    //       for(var j = 0;j < property.inputs.length;j++) {
    //         var item = property.inputs[j]
    // -%>
    //         if(challengeInput == <%= j %>) {
    // <%      if(item.isCompiled) { -%>
    //             bytes[] memory childInputs = new bytes[](<%= item.inputs.length %>);
    // <%- indent(include('constructInputs', {property: item, valName: 'childInputs', witnessName: 'challengeInputs[0]'}), 4) -%>
    //             return getChild(childInputs, utils.subArray(challengeInputs, 1, challengeInputs.length));
    // <%      } else if(item.predicate.type == 'CompiledPredicateCall') { -%>
    //             // This is for predicates dynamic linking
    //             bytes[] memory childInputs = new bytes[](<%= item.inputs.length %>);
    // <%- indent(include('constructInputs', {property: item, valName: 'childInputs', witnessName: 'challengeInputs[0]'}), 4) -%>
    //             return CompiledPredicate(<%= item.predicate.source %>).getChild(childInputs, utils.subArray(challengeInputs, 1, challengeInputs.length));
    // <%      } else { -%>
    // <%-  indent(include('constructProperty', {property: item, valName: 'notInputs[0]', propIndex: j, freeVariable: false}), 4) -%>
    //             return types.Property({
    //                 predicateAddress: notAddress,
    //                 inputs: notInputs
    //             });
    // <%      } -%>
    //         }
    // <%    } -%>
    // <%  } else if(property.connective == 'ForAllSuchThat') {

    fn get_child_for_all_such_that(
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
    ) -> ExecResultTOf<PropertyOf<Ext>, Ext> {
    }

    fn get_child_not(
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
    ) -> ExecResultTOf<PropertyOf<Ext>, Ext> {
    }

    fn get_child_or(
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
    ) -> ExecResultTOf<PropertyOf<Ext>, Ext> {
    }

    fn get_child_there_exists_such_that(
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
    ) -> ExecResultTOf<PropertyOf<Ext>, Ext> {
    }

    fn resolve_intermediate(
        &self,
        name: &String,
    ) -> ExecResultTOf<&IntermediateCompiledPredicate, Ext> {
        if let Some(index) = self
            .code
            .contracts
            .iter()
            .position(|inter| inter.name == name)
        {
            Ok(&self.code.contracts[index])
        }
        Err(ExecError::Require {
            msg: "Required error by: resolve_intermediate",
        })
    }

    fn construct_inputs(
        &self,
        property: &AtomicProposition,
        witness: &Vec<u8>,
        inputs: &Vec<Vec<u8>>,
        input_property: &Vec<PropertyOf<Ext>>,
        input_property_list_child_list: &Vec<BTreeMap<i8, PropertyOf<Ext>>>,
    ) -> ExecResultTOf<Vec<Vec<u8>>, Ext> {
        property
            .inputs
            .iter()
            .map(|input| {
                self.construct_input(
                    input,
                    witness,
                    inputs,
                    input_property,
                    input_property_list_child_list,
                )?
            })
            .collect()
    }

    fn construct_input(
        &self,
        compiled_input: &CompiledInput,
        witness: &Vec<u8>,
        inputs: &Vec<Vec<u8>>,
        input_property: &Vec<PropertyOf<Ext>>,
        input_property_list_child_list: &Vec<BTreeMap<i8, PropertyOf<Ext>>>,
    ) -> ExecResultTOf<Vec<u8>, Ext> {
        match compiled_input {
            CompiledInput::ConstantInput(inp) => Ok(inp.name.encode()),
            CompiledInput::LabelInput(inp) => Ok(Self::prefix_label(
                self.bytes_inputs(self.get_bytes_variable(&inp.label)),
            )),
            CompiledInput::NormalInput(inp) => {
                if inp.children.len() == 1 {
                    require!(input_property.len() > inp.input_index);
                    let input_property_input = input_property[inp.input_index];
                    if inp.children[0] >= 0 {
                        require!(input_property_input.len() > inp.children[0]);
                        Ok(input_property_input.inputs[inp.children[0]])
                    } else {
                        Ok(input_property_input.predicate_address.encode())
                    }
                } else if inp.children.len() == 2 {
                    require!(input_property_list_child_list.len() > inp.input_index);
                    let input_child_list = input_property_list_child_list[inp.input_index]
                        .get(inp.children[0])
                        .ok_or(ExecErrpr::Require("invalid index children[0]"))?;
                    if inp.children[1] >= 0 {
                        Ok(input_child_list[inp.children[1]])
                    } else {
                        Ok(input_child_list.predicate_address.encode())
                    }
                }
                require!(inputs.len() > inp.input_index - 1);
                Ok(inputs[inp.input_index - 1])
            }
            CompiledInput::VariableInput(_) => Ok(witness.clone()),
            CompiledInput::SelfInput(_) => Ok(self.ext_address().encode()),
            _ => Err(ExecError::Unexpected {
                msg: "error unknown input type",
            }),
        }
    }

    fn construct_property(
        compiled_input: &CompiledInput,
        prop_index: usize,
        free_variable: bool,
        inputs: &Vec<Vec<u8>>,
    ) -> ExecResultTOf<PropertyOf<Ext>, Ext> {
        Err(ExecError::Unexpected {
            msg: "error unknown input type",
        })
    }

    fn get_bytes_variable(&self, key: &String) -> ExecResultTOf<Vec<u8>, Ext> {
        if let Some(ret) = self
            .bytes_inputs
            .get(Ext::Hashing::hash(&mut &key.encode()[..]))
        {
            Ok(ret.clone())
        }
        Err(ExecError::Require {
            msg: "invalid bytes variable name.",
        })
    }

    fn get_address_variable(&self, key: &String) -> ExecResultTOf<AddressOf<Ext>, Ext> {
        if let Some(ret) = self
            .address_inputs
            .get(Ext::Hashing::hash(&mut &key.encode()[..]))
        {
            Ok(ret.clone())
        }
        Err(ExecError::Require {
            msg: "invalid address variable name.",
        })
    }
}
