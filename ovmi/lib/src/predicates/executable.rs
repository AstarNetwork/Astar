use crate::executor::*;
use crate::predicates::*;

// Compiled Predicate transpiles to this structure.
pub struct ExecutablePredicate<'a, Ext: ExternalCall> {
    pub ext: &'a Ext,
    pub payout: AddressOf<Ext>,
    // pub constants: BTreeMap<&'static str, Vec<AddressOf<Ext>>>,
}

impl<'a, Ext: ExternalCall> CompiledPredicate<AddressOf<Ext>> for ExecutablePredicate<'a, Ext> {
    fn payout_contract_address(&self) -> AddressOf<Ext> {
        self.payout.clone()
    }

    fn is_valid_challenge(
        &self,
        _inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        _challenge: Property<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Ok(true)
    }

    /// @dev get valid child property of game tree with challenge_inputs
    fn get_child(
        &self,
        inputs: Vec<Vec<u8>>,
        _challenge_input: Vec<Vec<u8>>,
    ) -> ExecResultT<Property<AddressOf<Ext>>, AddressOf<Ext>> {
        // TODO temp return value
        Ok(Property {
            predicate_address: self.ext.ext_address(),
            inputs,
        })
    }

    fn decide(&self, _inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>) -> ExecResult<AddressOf<Ext>> {
        Ok(true)
    }

    fn decide_true(&self, _inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>) {}

    fn decide_with_witness(
        &self,
        _inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Ok(true)
    }
}
