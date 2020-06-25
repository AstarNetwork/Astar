use crate::executor::*;
use crate::predicates::*;

pub struct ThereExistsPredicate<'a, Ext: ExternalCall> {
    pub ext: &'a Ext,
}

impl<'a, Ext: ExternalCall> ThereExistsPredicate<'a, Ext> {
    /// @dev Replace placeholder by quantified in propertyBytes
    fn replace_variable(
        &self,
        property_bytes: &Vec<u8>,
        placeholder: &Vec<u8>,
        quantified: &Vec<u8>,
    ) -> ExecResultT<Vec<u8>, AddressOf<Ext>> {
        // Support property as the variable in ThereExistsSuchThatQuantifier.
        // This code enables meta operation which we were calling eval without adding specific "eval" contract.
        // For instance, we can write a property like `∀su ∈ SU: su()`.
        if Ext::is_placeholder(property_bytes) {
            if &Ext::get_input_value(property_bytes) == placeholder {
                return Ok(quantified.clone());
            }
        }
        let mut property: Property<AddressOf<Ext>> = Ext::bytes_to_property(&property_bytes)?;
        if property.predicate_address == Ext::not_address() {
            require!(property.inputs.len() > 0);
            property.inputs[0] =
                self.replace_variable(&property.inputs[0], placeholder, quantified)?;
        } else if property.predicate_address == self.ext.ext_address() {
            require!(property.inputs.len() > 2);
            property.inputs[2] =
                self.replace_variable(&property.inputs[2], placeholder, quantified)?;
        } else if property.predicate_address == Ext::and_address()
            || property.predicate_address == Ext::or_address()
        {
            property.inputs = property
                .inputs
                .iter()
                .map(|input| self.replace_variable(input, placeholder, quantified))
                .collect::<Result<Vec<_>, _>>()?;
        } else {
            property.inputs = property
                .inputs
                .iter()
                .map(|input| {
                    if Ext::is_placeholder(input) {
                        if &Ext::get_input_value(input) == placeholder {
                            return quantified.clone();
                        }
                    }
                    input.clone()
                })
                .collect();
        }
        Ok(property.encode())
    }
}

impl<'a, Ext: ExternalCall> LogicalConnectiveInterface<AddressOf<Ext>>
    for ThereExistsPredicate<'a, Ext>
{
    /// @dev Validates a child node of ThereExists property in game tree.
    fn is_valid_challenge(
        &self,
        inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        challenge: Property<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        // challenge must be for(, , not(p))
        require_with_message!(
            challenge.predicate_address == Ext::for_all_address(),
            "challenge must be ForAllSuchThat"
        );
        require!(inputs.len() > 2);
        let new_inputs = vec![inputs[2].clone()];
        let p = Property::<AddressOf<Ext>> {
            predicate_address: Ext::not_address().clone(),
            inputs: new_inputs,
        };

        require!(challenge.inputs.len() > 2);
        require_with_message!(inputs[1] == challenge.inputs[1], "variable must be same");
        require_with_message!(p.encode() == challenge.inputs[2], "inputs must be same");
        Ok(true)
    }
}

impl<'a, Ext: ExternalCall> DecidablePredicateInterface<AddressOf<Ext>>
    for ThereExistsPredicate<'a, Ext>
{
    /// @dev Can decide true when all child properties are decided true
    fn decide_with_witness(
        &self,
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        require!(inputs.len() > 2);
        require!(witness.len() > 0);
        let property_bytes = self.replace_variable(&inputs[2], &inputs[1], &witness[0])?;
        let property: Property<AddressOf<Ext>> = Ext::bytes_to_property(&property_bytes)?;
        Ok(Ext::bytes_to_bool(
            &self.ext.ext_call(
                &property.predicate_address,
                PredicateCallInputs::DecidablePredicate(
                    DecidablePredicateCallInputs::DecideWithWitness {
                        inputs: property.inputs,
                        witness: witness
                            .as_slice()
                            .get(1..)
                            .unwrap_or(vec![].as_slice())
                            .to_vec(),
                    },
                ),
            )?,
        )?)
    }
}
