use crate::executor::*;
use crate::predicates::*;
use crate::Range;

pub struct IsValidSignaturePredicate<'a, Ext: ExternalCall> {
    pub ext: &'a mut Ext,
}

impl<'a, Ext: ExternalCall> AtomicPredicateInterface<AddressOf<Ext>>
    for IsValidSignaturePredicate<'a, Ext>
{
    type Hash = HashOf<Ext>;
    fn decide(&self, inputs: Vec<Vec<u8>>) -> ExecResult<AddressOf<Ext>> {
        // TODO: signature
        // require_with_message!(
        //     keccak256(
        //         abi.encodePacked(string(utils.getInputValue(_inputs[3])))
        //     ) ==
        //         keccak256("secp256k1"),
        //     "verifierType must be secp256k1"
        // );
        // require(
        //     ECRecover.ecverify(
        //         keccak256(_inputs[0]),
        //         _inputs[1],
        //         utils.bytesToAddress(_inputs[2])
        //     ),
        //     "_inputs[1] must be signature of _inputs[0] by _inputs[2]"
        // );
        // return true;
        Ok(true)
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
    for IsValidSignaturePredicate<'a, Ext>
{
    fn decide_with_witness(
        &self,
        inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Self::decide(self, inputs)
    }
}
