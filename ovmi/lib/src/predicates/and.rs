use crate::executor::*;
use crate::predicates::*;

pub struct AndPredicate<'a, Ext: ExternalCall> {
    pub ext: &'a Ext,
}

impl<Ext: ExternalCall> AndPredicate<'_, Ext> {
    fn create_property_from_input(&self, input: Vec<Vec<u8>>) -> Property<AddressOf<Ext>> {
        Property {
            predicate_address: self.ext.ext_address(),
            inputs: input,
        }
    }
}

impl<Ext: ExternalCall> LogicalConnectiveInterface<AddressOf<Ext>> for AndPredicate<'_, Ext> {
    /// @dev Validates a child node of And property in game tree.
    fn is_valid_challenge(
        &self,
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        challenge: Property<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        // challenge_input is index of child property
        require!((&challenge_inputs).len() > 0);
        let index = Ext::bytes_to_u128(&challenge_inputs[0])? as usize;

        // challenge should be not(p[index])
        // require!(_challenge.predicateAddress == not_predicateAddress);
        require!(challenge.predicate_address == Ext::not_address());

        // require!(keccak256(_inputs[index]) == keccak256(_challenge.inputs[0]));
        require!(inputs.len() > index);
        require!(challenge.inputs.len() > 0);
        require!(inputs[index as usize] == challenge.inputs[0]);
        Ok(true)
    }
}

impl<Ext: ExternalCall> DecidablePredicateInterface<AddressOf<Ext>> for AndPredicate<'_, Ext> {
    /// @dev Can decide true when all child properties are decided true
    fn decide_with_witness(
        &self,
        inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        for input in inputs.iter() {
            let property: PropertyOf<Ext> = Ext::bytes_to_property(input)?;
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
