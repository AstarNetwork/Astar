use crate::executor::*;
use crate::predicates::ExecutablePredicate;
use crate::*;

mod errors;

#[cfg(test)]
mod tests;

#[cfg(feature = "std")]
pub fn compile_from_json(json: &str) -> Result<CompiledPredicate, serde_json::Error> {
    serde_json::from_str(json)
}

pub use errors::Error;

pub fn validate(code: Vec<u8>) -> Result<(), Error> {
    Ok(())
}

pub fn executable_from_compiled<'a, Ext: ExternalCall>(
    ext: &'a Ext,
    code: CompiledPredicate,
    payout: AddressOf<Ext>,
    inputs: Vec<Vec<u8>>,
) -> ExecutablePredicate<'a, Ext> {
    ExecutablePredicate { ext, payout }
}
