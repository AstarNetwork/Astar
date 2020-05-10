use crate::executor::*;
use crate::predicates::*;

pub struct OrPredicate<'a, Ext: ExternalCall> {
    pub ext: &'a mut Ext,
}

impl<'a, Ext: ExternalCall> OrPredicate<'a, Ext> {
    fn create_property_from_input(&self, input: Vec<Vec<u8>>) -> Property<AddressOf<Ext>> {
        Property {
            predicate_address: self.ext.ext_address(),
            inputs: input,
        }
    }
}

impl<'a, Ext: ExternalCall> LogicalConnectiveInterface<AddressOf<Ext>> for OrPredicate<'a, Ext> {
    /// @dev Validates a child node of Or property in game tree.
    fn is_valid_challenge(
        &self,
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        challenge: Property<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        // challenge must be and(not(p[0]), not(p[1]), ...)
        require_with_message!(
            challenge.predicate_address == Ext::AndPredicate,
            "challenge must be And"
        );
        for (i, input) in inputs.enumerate() {
            let not_inputs = vec![input.clone()];
            inputs[0] = input;
            let p = Property {
                predicate_address: Ext::NotPredicate,
                inputs: not_inputs,
            };
            require!(challenge.inputs.len() > i);
            require_with_message!(p == challnge.inputs[i], "inputs must be same");
        }
        Ok(true)
    }
}

impl<'a, Ext: ExternalCall> DecidablePredicateInterface<AddressOf<Ext>> for OrPredicate<'a, Ext> {
    /// @dev Can decide true when all child properties are decided true
    fn decide_with_witness(
        &self,
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        require!(witness.len() > 0);
        let index: u128 = Decode::decode(&mut &witness[0][..]);
        uint256 index = abi.decode(_witness[0], (uint256));
        require(
            index < _inputs.length,
            "witness must be smaller than inputs length"
        );
        bytes memory propertyBytes = _inputs[index];
        types.Property memory property = abi.decode(
            propertyBytes,
            (types.Property)
        );
        DecidablePredicate predicate = DecidablePredicate(
            property.predicateAddress
        );
        bytes[] memory witness = new bytes[](_witness.length - 1);
        for (uint256 i = 0; i < _witness.length - 1; i++) {
            witness[i] = _witness[i + 1];
        }
        return predicate.decideWithWitness(property.inputs, witness);


        for input in inputs.iter() {
            let property: PropertyOf<Ext> =
                Decode::decode(&mut &input[..]).map_err(|_| ExecError::CodecError {
                    type_name: "Property<Ext>",
                })?;
            require_with_message!(
                self.ext.ext_is_decided(&property),
                "This property isn't true"
            );
        }
        let property = self.create_property_from_input(inputs);
        let property_id = self.ext.ext_get_property_id(&property);
        self.ext.ext_set_predicate_decision(property_id, true)?;
        Ok(false)
    }
}
