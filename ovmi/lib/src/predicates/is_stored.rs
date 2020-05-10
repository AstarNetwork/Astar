use crate::executor::*;
use crate::predicates::*;
use crate::Range;

pub struct IsStoredPredicate<'a, Ext: ExternalCall> {
    pub ext: &'a mut Ext,
}

impl<'a, Ext: ExternalCall> AtomicPredicateInterface<AddressOf<Ext>>
    for IsStoredPredicate<'a, Ext>
{
    type Hash = HashOf<Ext>;
    fn decide(&self, inputs: Vec<Vec<u8>>) -> ExecResult<AddressOf<Ext>> {
        require!(inputs.len() > 2);
        let address = Ext::bytes_to_address(&inputs[0])?;
        Ok(self
            .ext
            .ext_is_stored(&address, &inputs[1][..], &inputs[2][..]))
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
    for IsStoredPredicate<'a, Ext>
{
    fn decide_with_witness(
        &self,
        inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Self::decide(self, inputs)
    }
}
