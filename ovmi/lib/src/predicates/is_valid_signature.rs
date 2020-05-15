use crate::executor::*;
use crate::predicates::*;
use crate::Range;

pub struct IsValidSignaturePredicate<'a, Ext: ExternalCall> {
    pub ext: &'a Ext,
}

impl<Ext: ExternalCall> AtomicPredicateInterface<AddressOf<Ext>>
    for IsValidSignaturePredicate<'_, Ext>
{
    fn decide(&self, _inputs: Vec<Vec<u8>>) -> ExecResult<AddressOf<Ext>> {
        // require!(inputs.len() > 3);
        // require_with_message!(
        //     Ext::Hashing::hash(&self.get_input_value(inputs[3]).to_vec()[..]) == Ext::SECP_256_K1,
        //     "verifierType must be secp256k1"
        // );
        //
        // /// Verify a signature on a message. Returns true if the signature is good.
        // fn verify<M: AsRef<[u8]>>(sig: &Self::Signature, message: M, pubkey: &Self::Public) -> bool;
        //
        // let hash = Ext::Hashing::hash(&inputs[0][..]);
        // let address = self.bytes_to_address(&inputs[2]);

        // require_with_message!(
        //     ECRecover.ecverify(
        //         keccak256(_inputs[0]),
        //         _inputs[1],
        //         utils.bytesToAddress(_inputs[2])
        //     ),
        //     "_inputs[1] must be signature of _inputs[0] by _inputs[2]"
        // );
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
