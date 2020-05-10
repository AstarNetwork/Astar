use crate::executor::*;
use crate::predicates::*;

pub struct ThereExistsSuchThatQuantifierPredicate<'a, Ext: ExternalCall> {
    pub ext: &'a mut Ext,
}

impl<'a, Ext: ExternalCall> ThereExistsSuchThatQuantifierPredicate<'a, Ext> {
    fn create_property_from_input(&self, input: Vec<Vec<u8>>) -> Property<AddressOf<Ext>> {
        Property {
            predicate_address: self.ext.ext_address(),
            inputs: input,
        }
    }
}

impl<'a, Ext: ExternalCall> LogicalConnectiveInterface<AddressOf<Ext>> for ThereExistsSuchThatQuantifierPredicate<'a, Ext> {
    /// @dev Validates a child node of ThereExistsSuchThatQuantifier property in game tree.
    fn is_valid_challenge(
        &self,
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        challenge: Property<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        // challenge must be for(, , not(p))
        require!(
            challnge.predicate_address == forAddress,
            "challenge must be ForAllSuchThat"
        );
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = _inputs[2];
        types.Property memory p = types.Property({
            predicateAddress: notAddress,
            inputs: inputs
        });
        require(
            keccak256(_inputs[1]) == keccak256(_challnge.inputs[1]),
            "variable must be same"
        );
        require(
            keccak256(abi.encode(p)) == keccak256(_challnge.inputs[2]),
            "inputs must be same"
        );
        return true;

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

impl<'a, Ext: ExternalCall> DecidablePredicateInterface<AddressOf<Ext>> for ThereExistsSuchThatQuantifierPredicate<'a, Ext> {
    /// @dev Can decide true when all child properties are decided true
    fn decide_with_witness(
        &self,
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        require!(witness.len() > 0);
        let index: u128 = Decode::decode(&mut &witness[0][..])?;
        require_with_message!(
            (index as usize) < inputs.length,
            "witness must be smaller than inputs length"
        );
        let property_bytes = inputs[index as usize];
        let property: Property<AddressOf<Ext>> = Decode::decode(&mut &property_bytes[0][..])?;

        self.ext.ext_call(
            property.predicate_address,
            PredicateCallInputs::DecidablePredicate(
                DeciablePredicateCallInput::DecideWithWitness {
                    inputs: property.inputs,
                    witness: witness.as_slice().get(1..).to_vec(),
                },
            ),
        )
    }
}
