use crate::executor::*;
use crate::predicates::*;
use crate::Range;

pub struct IsContainedPredicate<'a, Ext: ExternalCall> {
    pub ext: &'a Ext,
}

impl<'a, Ext: ExternalCall> AtomicPredicateInterface<AddressOf<Ext>>
    for IsContainedPredicate<'a, Ext>
{
    fn decide(&self, inputs: Vec<Vec<u8>>) -> ExecResult<AddressOf<Ext>> {
        require!(inputs.len() > 1);
        let range: Range = Ext::bytes_to_range(&inputs[0])?;
        let sub_range: Range = Ext::bytes_to_range(&inputs[1])?;
        require_with_message!(
            range.start <= sub_range.start && sub_range.end <= range.end,
            "range must contain subrange"
        );
        Ok(true)
    }
}

impl<'a, Ext: ExternalCall> AtomicHelperInterface<AddressOf<Ext>>
    for IsContainedPredicate<'a, Ext>
{
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
    for IsContainedPredicate<'a, Ext>
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
    for IsContainedPredicate<'a, Ext>
{
}
