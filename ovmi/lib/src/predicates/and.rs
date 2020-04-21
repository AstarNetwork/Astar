use crate::predicates::*;
use crate::executor::*;

pub struct AndPredicate {
    // not_predicate: NotPrediaate,
}

impl AndPredicate {
    fn create_property_from_input(_input: Vec<Vec<u8>>) -> Property {
            Property {
                predicate_address: ext_address(),
                inputs: _input
            }
    }
}

impl LogicalConnective for AndPredicate {
    /// @dev Validates a child node of And property in game tree.
    fn is_valid_challenge(
        _inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        _challenge: Property<Address>,
    ) -> ExecResult {
        // challenge_input is index of child property
        let index: uint256 = Decode::decode(_challenge_inputs[0]);

        // challenge should be not(p[index])
        // require!(_challnge.predicateAddress == not_predicateAddress);
        // require!(keccak256(_inputs[index]) == keccak256(_challnge.inputs[0]));
        Ok(true);
    }
}

impl DecidablePredicate for AndPredicate {
    ///  @dev Can decide true when all child properties are decided true
    fn decide_true(_inputs: Vec<Vec<u8>>) {
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
    fn decide(_inputs: Vec<Vec<u8>>) -> ExecResult {
        Ok(false)
    }
}
