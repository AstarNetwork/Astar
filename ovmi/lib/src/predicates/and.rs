use crate::executor::*;
use crate::predicates::*;

pub struct AndPredicate<'a, Ext: ExternalCall> {
    ext: &'a mut Ext,
}

impl<'a, Ext: ExternalCall> AndPredicate<'a, Ext> {
    // TODO これ derive したいね。
    fn create_property_from_input(&self, _input: Vec<Vec<u8>>) -> Property {
        Property {
            predicate_address: ext_address(),
            inputs: _input,
        }
    }
}

impl<'a, Ext: ExternalCall> LogicalConnective<AddressOf<Ext>> for AndPredicate<'a, Ext> {
    /// @dev Validates a child node of And property in game tree.
    fn is_valid_challenge(
        &self,
        _inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        _challenge: Property<Address>,
    ) -> ExecResult<Address> {
        // challenge_input is index of child property
        let index: uint256 = Decode::decode(_challenge_inputs[0]);

        // challenge should be not(p[index])
        // require!(_challnge.predicateAddress == not_predicateAddress);
        // require!(keccak256(_inputs[index]) == keccak256(_challnge.inputs[0]));
        Ok(true);
    }
}

impl<'a, Ext: ExternalCall> DecidablePredicate for AndPredicate<'a, Ext> {
    ///  @dev Can decide true when all child properties are decided true
    fn decide_true(&self, _inputs: Vec<Vec<u8>>) {
        // for (uint256 i = 0; i < inner_properties.length; i++) {
        //     require(
        //         UniversalAdjudicationContract(uacAddress).isDecided(
        //             innerProperties[i]
        //         ),
        //         "This property isn't true"
        //     );
        // }
        // bytes[] memory inputs = new bytes[](innerProperties.length);
        // for (uint256 i = 0; i < inner_properties.length; i++) {
        //     inputs[i] = abi.encode(innerProperties[i]);
        // }
        // types.Property memory property = create_property_from_input(inputs);
        // UniversalAdjudicationContract(uacAddress).setPredicateDecision(
        //     utils.getPropertyId(property),
        //     true
        // );
    }
    fn decide(&self, _inputs: Vec<Vec<u8>>) -> ExecResult<Address> {
        Ok(false)
    }
}
