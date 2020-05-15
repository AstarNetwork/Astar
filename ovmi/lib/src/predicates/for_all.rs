use crate::executor::*;
use crate::predicates::*;

pub struct ForAllPredicate<'a, Ext: ExternalCall> {
    pub ext: &'a Ext,
}

impl<Ext: ExternalCall> ForAllPredicate<'_, Ext> {
    /// @dev Replace placeholder by quantified in propertyBytes
    fn replace_variable(
        &self,
        property_bytes: &Vec<u8>,
        placeholder: &Vec<u8>,
        quantified: &Vec<u8>,
    ) -> ExecResultT<Vec<u8>, AddressOf<Ext>> {
        // Support property as the variable in ForAllSuchThatQuantifier.
        // This code enables meta operation which we were calling eval without adding specific "eval" contract.
        // For instance, we can write a property like `∀su ∈ SU: su()`.
        if Ext::is_placeholder(property_bytes) {
            if &Ext::get_input_value(property_bytes) == placeholder {
                return Ok(quantified.clone());
            }
        }
        let mut property: Property<AddressOf<Ext>> = Decode::decode(&mut &property_bytes[..])
            .map_err(|_| ExecError::CodecError {
                type_name: "Property<Ext>",
            })?;
        if property.predicate_address == Ext::NOT_ADDRESS {
            require!(property.inputs.len() > 0);
            property.inputs[0] =
                self.replace_variable(&property.inputs[0], placeholder, quantified)?;
        } else if property.predicate_address == self.ext.ext_address() {
            require!(property.inputs.len() > 2);
            property.inputs[2] =
                self.replace_variable(&property.inputs[2], placeholder, quantified)?;
        } else if property.predicate_address == Ext::AND_ADDRESS {
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

impl<Ext: ExternalCall> LogicalConnectiveInterface<AddressOf<Ext>> for ForAllPredicate<'_, Ext> {
    /// @dev Validates a child node of ForAll property in game tree.
    fn is_valid_challenge(
        &self,
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        challenge: Property<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        // challenge should be not(p[quantified])
        require_with_message!(
            challenge.predicate_address == Ext::NOT_ADDRESS,
            "_challenge must be Not predicate"
        );
        // check inner property
        require!(inputs.len() > 2);
        require!(challenge_inputs.len() > 0);
        require!(challenge.inputs.len() > 0);
        let replace_variabled =
            self.replace_variable(&inputs[2], &inputs[1], &challenge_inputs[0])?;
        require_with_message!(
            replace_variabled == challenge.inputs[0],
            "must be valid inner property"
        );
        Ok(true)
    }
}

impl<Ext: ExternalCall> DecidablePredicateInterface<AddressOf<Ext>> for ForAllPredicate<'_, Ext> {
    /// @dev Can decide true when all child properties are decided true
    fn decide_with_witness(
        &self,
        _inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Ok(false)
    }
}
