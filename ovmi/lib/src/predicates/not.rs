use crate::executor::*;
use crate::predicates::*;

pub struct NotPredicate<'a, Ext: ExternalCall> {
    ext: &'a mut Ext,
}

impl<'a, Ext: ExternalCall> NotPredicate<'a, Ext> {
    fn create_property_from_input(&self, input: Vec<Vec<u8>>) -> Property<AddressOf<Ext>> {
        Property {
            predicate_address: self.ext.ext_address(),
            inputs: input,
        }
    }
}

impl<'a, Ext: ExternalCall> LogicalConnective<AddressOf<Ext>> for NotPredicate<'a, Ext> {
    /// @dev Validates a child node of Not property in game tree.
    fn is_valid_challenge(
        &self,
        _inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        _challenge: Property<AddressOf<Ext>>,
    ) -> ExecResult<AddressOf<Ext>> {
        // The valid challenge of not(p) is p and _inputs[0] is p here
        // return keccak256(_inputs[0]) == keccak256(abi.encode(_challenge));
        Ok(true)
    }
}
impl<'a, Ext: ExternalCall> DecidablePredicate<AddressOf<Ext>> for NotPredicate<'a, Ext> {
    /// @dev Decides true
    fn decide_with_witness(
        &self,
        _inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> ExecResult<AddressOf<Ext>> {
        Ok(false)
    }
}
