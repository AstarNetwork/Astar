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
    _code: CompiledPredicate,
    payout: AddressOf<Ext>,
    ext: &'a Ext,
) -> ExecutablePredicate<'a, Ext> {
    ExecutablePredicate { ext, payout }
}
