use crate::predicates::*;
use crate::executor::*;

// Compiled Predicate transpiles to this structure.
pub struct ExecutablePredicate<Address> {
    payout: Address,
    // not_predicate: NotPrediaate,
}

impl<Address> CompiledPredicate<Address> for ExecutablePredicate<Address> {
    fn payout_contract_address(&self) -> Address {
        self.address
    }

    fn is_valid_challenge(
        &self,
        _inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        _challenge: Property<Address>,
    ) -> ExecResult<Address> {
        Ok(true)
    }

    /// @dev get valid child property of game tree with challenge_inputs
    fn get_child(&self, inputs: Vec<Vec<u8>>, challenge_input: Vec<Vec<u8>>) -> Property<Address> {
        Ok(true)
    }

    fn decide(&self, _inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>) -> ExecResult<Address> {
        Ok(true)
    }

    fn decide_true(&self, _inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>) {

    }

    fn decide_with_witness(&self, _inputs: Vec<Vec<u8>>, _witness: Vec<Vec<u8>>) -> ExecResult<Address> {
        Ok(true)
    }
}
