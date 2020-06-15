use crate::executor::*;
use crate::predicates::*;

pub struct IsValidSignaturePredicate<'a, Ext: ExternalCall> {
    pub ext: &'a Ext,
}

impl<Ext: ExternalCall> AtomicPredicateInterface<AddressOf<Ext>>
    for IsValidSignaturePredicate<'_, Ext>
{
    fn decide(&self, inputs: Vec<Vec<u8>>) -> ExecResult<AddressOf<Ext>> {
        require!(inputs.len() > 3);
        require_with_message!(
            Ext::Hashing::hash(&inputs[3][..]) == Ext::secp256k1(),
            "verifierType must be secp256k1"
        );

        let hash = Ext::Hashing::hash(&inputs[0][..]);
        let address = Ext::bytes_to_address(&inputs[2])?;
        require_with_message!(
            self.ext.ext_verify(&hash, &inputs[1][..], &address,),
            "_inputs[1] must be signature of _inputs[0] by _inputs[2]"
        );
        Ok(true)
    }
}
impl<Ext: ExternalCall> AtomicHelperInterface<AddressOf<Ext>>
    for IsValidSignaturePredicate<'_, Ext>
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
        self.ext.ext_get_property_id(property)
    }
}

impl<Ext: ExternalCall> DecidablePredicateInterface<AddressOf<Ext>>
    for IsValidSignaturePredicate<'_, Ext>
{
    fn decide_with_witness(
        &self,
        inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Self::decide(self, inputs)
    }
}

impl<Ext: ExternalCall> BaseAtomicPredicateInterface<AddressOf<Ext>>
    for IsValidSignaturePredicate<'_, Ext>
{
}
