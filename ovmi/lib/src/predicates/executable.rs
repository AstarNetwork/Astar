use crate::compiled_predicates::*;
use crate::executor::*;
use crate::predicates::*;
use crate::*;

// Compiled Predicate transpiles to this structure.
pub struct CompiledExecutable<'a, Ext: ExternalCall> {
    pub ext: &'a Ext,
    pub payout: AddressOf<Ext>,
    pub code: CompiledPredicate,
    pub constants: BTreeMap<HashOf<Ext>, VarType>,
    pub address_inputs: BTreeMap<HashOf<Ext>, AddressOf<Ext>>,
    pub bytes_inputs: BTreeMap<HashOf<Ext>, Vec<u8>>,
}

impl<Ext: ExternalCall> CompiledPredicateInterface<AddressOf<Ext>> for CompiledExecutable<'_, Ext> {
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

    fn decide_true(
        &self,
        _inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Ok(true)
    }

    fn decide_with_witness(
        &self,
        _inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Ok(true)
    }
}
