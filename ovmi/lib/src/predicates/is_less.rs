use crate::executor::*;
use crate::predicates::*;
use crate::Range;

pub struct IsLessThanPredicate<'a, Ext: ExternalCall> {
    pub ext: &'a Ext,
}

impl<'a, Ext: ExternalCall> AtomicPredicateInterface<AddressOf<Ext>>
    for IsLessThanPredicate<'a, Ext>
{
    fn decide(&self, inputs: Vec<Vec<u8>>) -> ExecResult<AddressOf<Ext>> {
        let first: u128 = Ext::bytes_to_u128(&inputs[0])?;
        let second: u128 = Ext::bytes_to_u128(&inputs[1])?;
        require_with_message!(first < second, "first input is not less than second input");
        Ok(true)
    }
}
impl<'a, Ext: ExternalCall> AtomicHelperInterface<AddressOf<Ext>> for IsLessThanPredicate<'a, Ext> {
    type Hash = HashOf<Ext>;
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
    for IsLessThanPredicate<'a, Ext>
{
    fn decide_with_witness(
        &self,
        inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Self::decide(self, inputs)
    }
}

impl<'a, Ext: ExternalCall> BaseAtomicPredicateInterface<AddressOf<Ext>>
    for IsLessThanPredicate<'a, Ext>
{
}
