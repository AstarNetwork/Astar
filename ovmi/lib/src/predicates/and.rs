use crate::executor::*;
use crate::predicates::*;

pub struct AndPredicate<'a, Ext: ExternalCall> {
    ext: &'a mut Ext,
}

impl<'a, Ext: ExternalCall> AndPredicate<'a, Ext> {
    fn create_property_from_input(&self, input: Vec<Vec<u8>>) -> Property<AddressOf<Ext>> {
        Property {
            predicate_address: self.ext.ext_address(),
            inputs: input,
        }
    }
}

impl<'a, Ext: ExternalCall> LogicalConnectiveInterface<AddressOf<Ext>> for AndPredicate<'a, Ext> {
    /// @dev Validates a child node of And property in game tree.
    fn is_valid_challenge(
        &self,
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        challenge: Property<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        // challenge_input is index of child property
        require!((&challenge_inputs).len() > 0);
        let index: u128 = Decode::decode(&mut &challenge_inputs[0][..])
            .map_err(|err| ExecError::CodecError { type_name: "u128" })?;
        let index: usize = index as usize;

        // challenge should be not(p[index])
        // require!(_challnge.predicateAddress == not_predicateAddress);
        require!(challenge.predicate_address == Ext::NotPredicate);

        // require!(keccak256(_inputs[index]) == keccak256(_challnge.inputs[0]));
        require!(inputs.len() > index);
        require!(challenge.inputs.len() > 0);
        require!(inputs[index as usize] == challenge.inputs[0]);
        Ok(true)
    }
}

impl<'a, Ext: ExternalCall> DecidablePredicateInterface<AddressOf<Ext>> for AndPredicate<'a, Ext> {
    //  @dev Can decide true when all child properties are decided true
    // fn decide_true(&self, _inputs: Vec<Vec<u8>>) {
    // for (uint256 i = 0; i < inner_properties.length; i++) {
    //     require(
    //         UniversalAdjudicationContract(uacAddress).isDecided(
    //             innerProperties[i]
    //         ),
    //         "This property isn't true"
    //     );
    // }Property
    // bytes[] memory inputs = new bytes[](innerProperties.length);
    // for (uint256 i = 0; i < inner_properties.length; i++) {
    //     inputs[i] = abi.encode(innerProperties[i]);
    // }
    // types.Property memory property = create_property_from_input(inputs);
    // UniversalAdjudicationContract(uacAddress).setPredicateDecision(
    //     utils.getPropertyId(property),
    //     true
    // );
    // }
    fn decide_with_witness(
        &self,
        _inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Ok(false)
    }
}
