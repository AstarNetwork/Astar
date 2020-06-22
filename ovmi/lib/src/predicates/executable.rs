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

    /// @dev Validates a child node of the property in game tree.
    fn is_valid_challenge(
        &self,
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        challenge: Property<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        require_with_message!(
            Ext::hash_of(&self.get_child(inputs, challenge_inputs)?) == Ext::hash_of(&challenge),
            "_challenge must be valud child of game tree"
        );
        Ok(true)
    }

    /// @dev get valid child property of game tree with challenge_inputs
    fn get_child(
        &self,
        inputs: Vec<Vec<u8>>,
        challenge_input: Vec<Vec<u8>>,
    ) -> ExecResultTOf<Property<AddressOf<Ext>>, Ext> {
        require!(inputs.len() > 1);
        if (Ext::is_label(&inputs[0])) {
            let intermediate = self.resolve_intermediate(&self.code.entry_point)?;
            return self.get_child_intermediate(intermediate, &inputs, &challenge_input);
        }
        let input_0: String = Ext::bytes_to_bytes_string(&Ext::get_input_value(&inputs[0]))?;
        let sub_inputs = Ext::sub_array(&inputs, 1, inputs.len());

        let intermediate = self.resolve_intermediate(&input_0)?;
        return self.get_child_intermediate(intermediate, &sub_inputs, &challenge_input);
    }

    fn decide(&self, inputs: Vec<Vec<u8>>, witness: Vec<Vec<u8>>) -> ExecResult<AddressOf<Ext>> {
        if !Ext::is_label(&inputs[0]) {
            return self.decide(inputs, witness);
        }
        let input_0 = Ext::bytes_to_bytes_string(&Ext::get_input_value(&inputs[0]))?;
        let sub_inputs = Ext::sub_array(&inputs, 1, inputs.len());
        let intermediate = self.resolve_intermediate(&input_0)?;
        self.decide_intermediate(&intermediate, &sub_inputs, &witness)
    }

    fn decide_true(
        &self,
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        self.decide(inputs, witness)
    }

    fn decide_with_witness(
        &self,
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        let result = self.decide(inputs.clone(), witness)?;
        require_with_message!(result, "must be true");
        let property = Property {
            predicate_address: self.ext.ext_address(),
            inputs: inputs,
        };
        self.ext
            .ext_set_predicate_decision(self.ext.ext_get_property_id(&property), true)
    }
}

impl<Ext: ExternalCall> CompiledExecutable<'_, Ext> {
    // decide_** --------------------------------------
    fn decide_intermediate(
        &self,
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        witness: &Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        let input_property_list = Self::get_input_property_list(inter, inputs)?;
        let input_property_list_child_list =
            Self::get_input_property_list_child_list(inter, inputs)?;

        match &inter.connective {
            LogicalConnective::And => self.decide_and(
                inter,
                inputs,
                witness,
                &input_property_list,
                &input_property_list_child_list,
            ),
            LogicalConnective::ThereExistsSuchThat => self.decide_there_exists_such_that(
                inter,
                inputs,
                witness,
                &input_property_list,
                &input_property_list_child_list,
            ),
            LogicalConnective::Or => self.decide_or(
                inter,
                inputs,
                witness,
                &input_property_list,
                &input_property_list_child_list,
            ),
            _ => Ok(false),
        }
    }

    fn decide_and(
        &self,
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        witness: &Vec<Vec<u8>>,
        input_property: &Vec<PropertyOf<Ext>>,
        input_property_list_child_list: &Vec<BTreeMap<i8, PropertyOf<Ext>>>,
    ) -> ExecResult<AddressOf<Ext>> {
        require!(witness.len() >= inter.inputs.len());
        for (i, item) in inter.inputs.iter().enumerate() {
            if let AtomicPropositionOrPlaceholder::AtomicProposition(item) = item {
                if item.is_compiled.unwrap_or(false) {
                    let child_inputs = self.construct_inputs(
                        item,
                        &witness[0],
                        inputs,
                        input_property,
                        input_property_list_child_list,
                    )?;
                    require!(self.decide(child_inputs, Ext::bytes_to_bytes_array(&witness[i])?)?);
                }
                self.decide_property(
                    item,
                    witness,
                    inputs,
                    input_property,
                    input_property_list_child_list,
                    &Ext::bytes_to_bytes_array(&witness[i])?,
                );
            }
        }
        Ok(true)
    }

    fn decide_there_exists_such_that(
        &self,
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        witness: &Vec<Vec<u8>>,
        input_property: &Vec<PropertyOf<Ext>>,
        input_property_list_child_list: &Vec<BTreeMap<i8, PropertyOf<Ext>>>,
    ) -> ExecResult<AddressOf<Ext>> {
        require!(inter.inputs.len() > 2);
        let inner_property = match &inter.inputs[2] {
            AtomicPropositionOrPlaceholder::AtomicProposition(x) => x,
            _ => return Ok(true),
        };
        if inner_property.is_compiled.unwrap_or(false) {
            let child_inputs = self.construct_inputs(
                inner_property,
                &witness[0],
                inputs,
                input_property,
                input_property_list_child_list,
            )?;
            require!(self.decide(child_inputs, Ext::sub_array(&witness, 1, witness.len()))?);
        } else {
            require!(self.decide_property(
                inner_property,
                witness,
                inputs,
                input_property,
                input_property_list_child_list,
                &Ext::sub_array(&witness, 1, witness.len() as usize),
            )?);
        }
        Ok(true)
    }

    fn decide_or(
        &self,
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        witness: &Vec<Vec<u8>>,
        input_property: &Vec<PropertyOf<Ext>>,
        input_property_list_child_list: &Vec<BTreeMap<i8, PropertyOf<Ext>>>,
    ) -> ExecResult<AddressOf<Ext>> {
        let or_index = Ext::bytes_to_u128(&witness[0])?;
        for (index, item) in inter.inputs.iter().enumerate() {
            if or_index as usize == index {
                if let AtomicPropositionOrPlaceholder::AtomicProposition(item) = item {
                    if item.is_compiled.unwrap_or(false) {
                        let child_inputs = self.construct_inputs(
                            item,
                            &witness[0],
                            inputs,
                            input_property,
                            input_property_list_child_list,
                        )?;
                        require!(
                            self.decide(child_inputs, Ext::sub_array(witness, 1, witness.len()))
                        );
                    } else {
                        return self.decide_property(
                            item,
                            inputs,
                            witness,
                            input_property,
                            input_property_list_child_list,
                            &Ext::sub_array(witness, 1, witness.len()),
                        );
                    }
                }
            }
        }
        Ok(true)
    }

    fn decide_property(
        &self,
        inter: &AtomicProposition,
        inputs: &Vec<Vec<u8>>,
        witness: &Vec<Vec<u8>>,
        input_property: &Vec<PropertyOf<Ext>>,
        input_property_list_child_list: &Vec<BTreeMap<i8, PropertyOf<Ext>>>,
        child_witnesses: &Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        if let PredicateCall::InputPredicateCall(call) = &inter.predicate {
            require!(inputs.len() > (call.source.input_index - 1) as usize);
            if inter.inputs.len() == 0 {
                require!(self.ext.ext_is_decided(&Ext::bytes_to_property(
                    &inputs[(call.source.input_index - 1) as usize]
                )?));
            } else {
                let input_predicate_property =
                    Ext::bytes_to_property(&inputs[(call.source.input_index - 1) as usize])?;
                let mut new_inputs = input_predicate_property.inputs.clone();
                if let CompiledInput::NormalInput(normal_input) = &inter.inputs[0] {
                    require!(inputs.len() > (normal_input.input_index - 1) as usize);
                    new_inputs.push(inputs[(normal_input.input_index - 1) as usize].clone());
                    let result = self.ext.ext_call(
                        &input_predicate_property.predicate_address,
                        PredicateCallInputs::CompiledPredicate(
                            CompiledPredicateCallInputs::Decide {
                                inputs: new_inputs,
                                witness: child_witnesses.clone(),
                            },
                        ),
                    )?;
                    require_with_message!(
                        Ext::bytes_to_bool(&result)?,
                        "InputPredicate must be true"
                    );
                }
            }
        } else if let PredicateCall::VariablePredicateCall(call) = &inter.predicate {
            // TODO: executable
            // require_with_message!(
            //     self.ext
            //         .ext_is_decided(&Ext::bytes_to_property(&challenge_input)),
            //     "VariablePredicate must be true"
            // );
            return Ok(true);
        } else {
            let new_inputs = self.construct_inputs(
                inter,
                &witness[0],
                inputs,
                input_property,
                input_property_list_child_list,
            )?;
            if let PredicateCall::CompiledPredicateCall(call) = &inter.predicate {
                // This is for predicates dynamic linking.
                let source = Self::get_source_str_from_inter(&inter.predicate)?;
                let result = self.ext.ext_call(
                    &self.get_address_variable(&source)?,
                    PredicateCallInputs::CompiledPredicate(CompiledPredicateCallInputs::Decide {
                        inputs: new_inputs,
                        witness: child_witnesses.clone(),
                    }),
                )?;
                require_with_message!(
                    Ext::bytes_to_bool(&result)?,
                    "CompiledPredicate(property.predicate.source) must be true"
                );
            } else {
                let source = Self::get_source_str_from_inter(&inter.predicate)?;
                let result = self.ext.ext_call(
                    &self.get_address_variable(&source)?,
                    PredicateCallInputs::AtomicPredicate(AtomicPredicateCallInputs::Decide {
                        inputs: new_inputs,
                    }),
                )?;
                require_with_message!(
                    Ext::bytes_to_bool(&result)?,
                    "CompiledPredicate(property.predicate.source) must be true"
                );
            }
        }
        Ok(true)
    }

    // get_child_** --------------------------------------
    fn get_child_intermediate(
        &self,
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
    ) -> ExecResultTOf<PropertyOf<Ext>, Ext> {
        let input_property_list = Self::get_input_property_list(inter, inputs)?;
        let input_property_list_child_list =
            Self::get_input_property_list_child_list(inter, inputs)?;

        match &inter.connective {
            LogicalConnective::And => self.get_child_and(
                inter,
                inputs,
                challenge_inputs,
                &input_property_list,
                &input_property_list_child_list,
            ),
            LogicalConnective::ForAllSuchThat => self.get_child_for_all_such_that(
                inter,
                inputs,
                challenge_inputs,
                &input_property_list,
                &input_property_list_child_list,
            ),
            LogicalConnective::Not => self.get_child_not(
                inter,
                inputs,
                challenge_inputs,
                &input_property_list,
                &input_property_list_child_list,
            ),
            LogicalConnective::Or => self.get_child_or(
                inter,
                inputs,
                challenge_inputs,
                &input_property_list,
                &input_property_list_child_list,
            ),
            LogicalConnective::ThereExistsSuchThat => self.get_child_there_exists_such_that(
                inter,
                inputs,
                challenge_inputs,
                &input_property_list,
                &input_property_list_child_list,
            ),
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
        let challenge_input: usize = (Ext::bytes_to_u128(&challenge_inputs[0])? as usize);
        require!(inter.inputs.len() > challenge_input as usize);
        let item = &inter.inputs[challenge_input];
        match item {
            AtomicPropositionOrPlaceholder::AtomicProposition(item) => {
                if item.is_compiled.unwrap_or(false) {
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
                        Ext::sub_array(challenge_inputs, 1, challenge_inputs.len()),
                    );
                } else if let PredicateCall::CompiledPredicateCall(pred) = &item.predicate {
                    let child_inputs = self.construct_inputs(
                        &item,
                        &challenge_inputs[0],
                        inputs,
                        input_property,
                        input_property_list_child_list,
                    )?;
                    let ret = self.ext.ext_call(
                        &self.get_address_variable(&pred.source)?,
                        PredicateCallInputs::CompiledPredicate(
                            CompiledPredicateCallInputs::GetChild {
                                inputs: child_inputs,
                                challenge_input: Ext::sub_array(
                                    challenge_inputs,
                                    1,
                                    challenge_inputs.len(),
                                ),
                            },
                        ),
                    )?;
                    return Ok(Ext::bytes_to_property(&ret)?);
                } else {
                    let not_inputs = vec![self
                        .construct_property(
                            &item,
                            false,
                            inputs,
                            challenge_inputs,
                            input_property,
                            input_property_list_child_list,
                        )?
                        .encode()];
                    Ok(Property {
                        predicate_address: Ext::not_address(),
                        inputs: not_inputs,
                    })
                }
            }
            _ => Err(ExecError::Unexpected {
                msg: "get_child_and must be AtomicProposition.",
            }),
        }
    }

    fn get_child_for_all_such_that(
        &self,
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
        input_property: &Vec<PropertyOf<Ext>>,
        input_property_list_child_list: &Vec<BTreeMap<i8, PropertyOf<Ext>>>,
    ) -> ExecResultTOf<PropertyOf<Ext>, Ext> {
        require!(inter.inputs.len() > 2);
        let quantifier = &inter.inputs[0];
        let inner_property = &inter.inputs[2];
        match inner_property {
            AtomicPropositionOrPlaceholder::AtomicProposition(inner_property) => {
                if inner_property.is_compiled.unwrap_or(false) {
                    let child_inputs = self.construct_inputs(
                        inner_property,
                        &challenge_inputs[0],
                        inputs,
                        input_property,
                        input_property_list_child_list,
                    )?;
                    self.get_child(
                        child_inputs,
                        Ext::sub_array(challenge_inputs, 1, challenge_inputs.len()),
                    )
                } else {
                    let not_inputs = vec![self
                        .construct_property(
                            inner_property,
                            false,
                            inputs,
                            challenge_inputs,
                            input_property,
                            input_property_list_child_list,
                        )?
                        .encode()];
                    Ok(Property {
                        predicate_address: Ext::not_address(),
                        inputs: not_inputs,
                    })
                }
            }
            _ => Err(ExecError::Unexpected {
                msg: "get_child_for_all_such_that must be AtomicProposition.",
            }),
        }
    }

    fn get_child_not(
        &self,
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
        input_property: &Vec<PropertyOf<Ext>>,
        input_property_list_child_list: &Vec<BTreeMap<i8, PropertyOf<Ext>>>,
    ) -> ExecResultTOf<PropertyOf<Ext>, Ext> {
        let inner_property = &inter.inputs[0];
        if let AtomicPropositionOrPlaceholder::AtomicProposition(inner_property) = inner_property {
            return self.construct_property(
                inner_property,
                false,
                inputs,
                challenge_inputs,
                input_property,
                input_property_list_child_list,
            );
        }
        Err(ExecError::Unexpected {
            msg: "get_child_not must be AtomicProposition.",
        })
    }

    fn get_child_there_exists_such_that(
        &self,
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
        input_property: &Vec<PropertyOf<Ext>>,
        input_property_list_child_list: &Vec<BTreeMap<i8, PropertyOf<Ext>>>,
    ) -> ExecResultTOf<PropertyOf<Ext>, Ext> {
        if let AtomicPropositionOrPlaceholder::AtomicProposition(inner_property) = &inter.inputs[2]
        {
            if let AtomicPropositionOrPlaceholder::Placeholder(property_input_1) = &inter.inputs[1]
            {
                let not_input = self.construct_property(
                    inner_property,
                    Ext::bytes_to_bool(&mut &self.get_bytes_variable(&Ext::bytes_to_string(
                        &Ext::prefix_variable(&property_input_1.encode()),
                    )?)?)?,
                    inputs,
                    challenge_inputs,
                    input_property,
                    input_property_list_child_list,
                )?;
                let for_all_such_that_inputs = vec![
                    vec![],
                    property_input_1.encode(),
                    Property {
                        predicate_address: Ext::not_address(),
                        inputs: vec![not_input.encode()],
                    }
                    .encode(),
                ];
                return Ok(Property {
                    predicate_address: Ext::for_all_address(),
                    inputs: for_all_such_that_inputs,
                });
            }
        }
        Err(ExecError::Unexpected {
            msg: "get_child_there_exists_such_that must be inter.inputs[1] is Placeholder, inter.inputs[2] is AtomicProposition.",
        })
    }

    fn get_child_or(
        &self,
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
        input_property: &Vec<PropertyOf<Ext>>,
        input_property_list_child_list: &Vec<BTreeMap<i8, PropertyOf<Ext>>>,
    ) -> ExecResultTOf<PropertyOf<Ext>, Ext> {
        let inputs = inter.inputs.iter().map(|item| {
            if let AtomicPropositionOrPlaceholder::AtomicProposition(item) = &item {
                if let PredicateCall::CompiledPredicateCall(predicate) = &item.predicate {
                    // not (compiled predicate)
                    let not_inputs = vec![self
                        .construct_property(
                            item,
                            false,
                            inputs,
                            challenge_inputs,
                            input_property,
                            input_property_list_child_list,
                        )?
                        .encode()];
                    return Ok(Property {
                        predicate_address: Ext::not_address(),
                        inputs: not_inputs,
                    });
                } else {
                    // The valid challenge of "p1 ∨ p2" is "¬(p1) ∧ ¬(p2)".
                    // If p1 is "¬(p1_1)", the valid challenge is "p1_1 ∧ ¬(p2)",
                    //   then returning getChild of "¬(p1_1)" here.
                    let child_inputs = self.construct_inputs(
                        item,
                        &challenge_inputs[0],
                        inputs,
                        input_property,
                        input_property_list_child_list,
                    )?;
                    return Ok(self.get_child(child_inputs, challenge_inputs.clone())?);
                }
            }
            Err(ExecError::Unexpected {
                msg: "get_child_or must be all inter.inputs AtomicProposition.",
            })
        })?;
        Ok(Property {
            predicate_address: Ext::and_address(),
            inputs: inputs,
        })
    }

    // helper -----------------------------------------------
    fn resolve_intermediate(
        &self,
        name: &String,
    ) -> ExecResultTOf<&IntermediateCompiledPredicate, Ext> {
        if let Some(index) = self
            .code
            .contracts
            .iter()
            .position(|inter| &inter.name == name)
        {
            return Ok(&self.code.contracts[index]);
        }
        Err(ExecError::Require {
            msg: "Required error by: resolve_intermediate",
        })
    }

    fn get_input_property_list(
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
    ) -> ExecResultTOf<Vec<PropertyOf<Ext>>, Ext> {
        Ok(inter
            .property_inputs
            .iter()
            .map(|property_input| {
                Ext::bytes_to_property(&inputs[(property_input.input_index - 1) as usize])?
            })
            .collect::<Vec<PropertyOf<Ext>>>())
    }

    fn get_input_property_list_child_list(
        inter: &IntermediateCompiledPredicate,
        inputs: &Vec<Vec<u8>>,
    ) -> ExecResultTOf<Vec<BTreeMap<i8, PropertyOf<Ext>>>, Ext> {
        Ok(inter
            .property_inputs
            .iter()
            .map(|property_input| {
                let mut ret = BTreeMap::new();
                if property_input.children.len() > 0 {
                    require!(
                        property_input[property_input.input_index as usize].inputs
                            > property_input.children[0]
                    );
                    ret.insert(
                        property_input.children[0],
                        property_input[property_input.input_index as usize].inputs
                            [property_input.children[0]],
                    );
                }
                Ok(ret)
            })
            .map(|res| res?)
            .collect::<Vec<BTreeMap<i8, PropertyOf<Ext>>>>())
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
                        .ok_or(ExecError::Require {
                            msg: "invalid index children[0]",
                        })?;
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
        &self,
        property: &AtomicProposition,
        free_variable: bool,
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
        input_property: &Vec<PropertyOf<Ext>>,
        input_property_list_child_list: &Vec<BTreeMap<i8, PropertyOf<Ext>>>,
    ) -> ExecResultTOf<PropertyOf<Ext>, Ext> {
        match &property.predicate {
            PredicateCall::InputPredicateCall(call) => {
                require!(inputs.len() > call.source.input_index - 1);
                if property.inputs.len() == 0 {
                    return Ext::bytes_to_property(&inputs[(call.source.input_index - 1) as usize]);
                }
                require!(inputs.len() > call.source.input_index - 1);
                require!(challenge_inputs.len() > 0);
                let input_predicate_property: PropertyOf<Ext> =
                    Ext::bytes_to_property(&inputs[(call.source.input_index - 1) as usize])?;
                let mut child_inputs_of = input_predicate_property.inputs;
                child_inputs_of.push(self.construct_input(
                    &property.inputs[0],
                    &challenge_inputs[0],
                    inputs,
                    input_property,
                    input_property_list_child_list,
                )?);
                Ok(Property {
                    predicate_address: input_predicate_property.predicate_address,
                    inputs: child_inputs_of,
                })
            }
            PredicateCall::VariablePredicateCall(call) => {
                if property.inputs.len() == 0 {
                    require!(challenge_inputs.len() > 0);
                    return Ext::bytes_to_property(&challenge_inputs[0]);
                }

                let input_predicate_property: PropertyOf<Ext> =
                    Ext::bytes_to_property(&challenge_inputs[0])?;
                let mut child_inputs_of = input_predicate_property.inputs;
                child_inputs_of.push(inputs[property.inputs[0].input_index - 1]);
                Ok(Property {
                    predicate_address: input_predicate_property.predicate_address,
                    inputs: child_inputs_of,
                })
            }
            call => {
                let witness = if free_variable {
                    &self.get_bytes_variable(&"freeVariable".to_string())?
                } else {
                    &challenge_inputs[0]
                };
                let child_inputs_of = self.construct_inputs(
                    property,
                    witness,
                    inputs,
                    input_property,
                    input_property_list_child_list,
                )?;
                if property.is_compiled {
                    return Ok(Property {
                        predicate_address: self.ext.ext_address(),
                        inputs: child_inputs_of,
                    });
                }
                let predicate_address = match property {
                    PredicateCall::AtomicPredicateCall(call) => {
                        self.get_bytes_variable(&call.source)
                    }
                    PredicateCall::VariablePredicateCall(call) => {
                        self.get_bytes_variable(&call.source)
                    }
                    PredicateCall::CompiledPredicateCall(call) => {
                        self.get_bytes_variable(&call.source)
                    }
                    _ => Err(ExecError::Unexpected {
                        msg: "unexpected predicate address call in construct_property.",
                    }),
                }?;
                Ok(PropertyOf::<Ext> {
                    predicate_address,
                    inputs: child_inputs_of,
                })
            }
        }
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

    fn get_source_str_from_inter(predicate: &PredicateCall) -> ExecResultTOf<String, Ext> {
        match predicate {
            PredicateCall::AtomicPredicateCall(predicate) => Ok(predicate.source.clone()),
            PredicateCall::CompiledPredicateCall(predicate) => Ok(predicate.source.clone()),
            _ => Err(ExecError::Unexpected {
                msg: "The intermediate must have source as String.",
            }),
        }
    }

    fn get_source_normal_from_inter(predicate: &PredicateCall) -> ExecResultTOf<NormalInput, Ext> {
        match predicate {
            PredicateCall::InputPredicateCall(predicate) => Ok(predicate.source.clone()),
            _ => Err(ExecError::Unexpected {
                msg: "The intermediate must have source as NormalInput.",
            }),
        }
    }
}
