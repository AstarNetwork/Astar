use crate::executor::*;
use crate::predicates::*;

pub struct OrPredicate<'a, Ext: ExternalCall> {
    pub ext: &'a Ext,
}

impl<'a, Ext: ExternalCall> LogicalConnectiveInterface<AddressOf<Ext>> for OrPredicate<'a, Ext> {
    /// @dev Validates a child node of Or property in game tree.
    fn is_valid_challenge(
        &self,
        inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        challenge: Property<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        // challenge must be and(not(p[0]), not(p[1]), ...)
        require_with_message!(
            challenge.predicate_address == Ext::and_address(),
            "challenge must be And"
        );
        for (i, input) in inputs.iter().enumerate() {
            let not_inputs = vec![input.clone()];
            let p = Property {
                predicate_address: Ext::not_address(),
                inputs: not_inputs,
            };
            require!(challenge.inputs.len() > i);
            require_with_message!(p.encode() == challenge.inputs[i], "inputs must be same");
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
        let index: u128 = Ext::bytes_to_u128(&witness[0])?;
        require_with_message!(
            (index as usize) < inputs.len(),
            "witness must be smaller than inputs length"
        );
        let property_bytes = inputs[index as usize].clone();
        let property: Property<AddressOf<Ext>> = Ext::bytes_to_property(&property_bytes)?;

        Ext::bytes_to_bool(
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
        )
    }
}
