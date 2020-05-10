use crate::executor::*;
use crate::predicates::*;
use crate::Range;

pub struct VerifyInclusionPredicate<'a, Ext: ExternalCall> {
    pub ext: &'a mut Ext,
}

impl<'a, Ext: ExternalCall> AtomicPredicateInterface<AddressOf<Ext>>
    for VerifyInclusionPredicate<'a, Ext>
{
    type Hash = HashOf<Ext>;
    fn decide(&self, inputs: Vec<Vec<u8>>) -> ExecResult<AddressOf<Ext>> {
        require!(inputs.len() > 4);
        let address = Ext::bytes_to_address(&inputs[1])?;
        Ok(self.ext.ext_verify_inclusion_with_root(
            Ext::hash_of(&inputs[0]),
            address,
            &inputs[2][..], // range
            &inputs[3][..], // inclusionProof
            &inputs[4][..], // bytes32
        ))
    }

    fn ext_address(&self) -> AddressOf<Ext> {
        self.ext.ext_address()
    }
    fn ext_set_predicate_decision(
        &self,
        game_id: Self::Hash,
        decision: bool,
    ) -> ExecResult<AddressOf<Ext>> {
        self.ext.ext_set_predicate_decision(game_id, decision)
    }
    fn ext_get_property_id(&self, property: &Property<AddressOf<Ext>>) -> Self::Hash {
        self.ext_get_property_id(property)
    }
}

impl<'a, Ext: ExternalCall> DecidablePredicateInterface<AddressOf<Ext>>
    for VerifyInclusionPredicate<'a, Ext>
{
    fn decide_with_witness(
        &self,
        inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Self::decide(self, inputs)
    }
}
