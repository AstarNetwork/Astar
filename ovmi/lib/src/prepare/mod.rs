use crate::executor::*;
use crate::predicates::*;
use crate::*;

mod errors;

#[cfg(test)]
mod tests;

#[cfg(feature = "std")]
pub fn compile_from_json(json: &str) -> Result<CompiledPredicate, serde_json::Error> {
    serde_json::from_str(json)
}

use crate::compiled_predicates::VarType;
pub use errors::Error;

pub fn validate(code: Vec<u8>) -> Result<(), Error> {
    Ok(())
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum VarValue<Address> {
    Address(Address),
    Bytes(Vec<u8>),
}

pub fn executable_from_compiled<'a, Ext: ExternalCall>(
    ext: &'a mut Ext,
    code: CompiledPredicate,
    payout: AddressOf<Ext>,
    address_inputs: BTreeMap<HashOf<Ext>, AddressOf<Ext>>,
    bytes_inputs: BTreeMap<HashOf<Ext>, Vec<u8>>,
) -> CompiledExecutable<'a, Ext> {
    // mapping constants[ hash(name) ] = var_type.
    let constants = code.constants.clone().map_or(BTreeMap::new(), |var| {
        var.iter()
            .map(|con| {
                (
                    Ext::Hashing::hash(con.name.as_bytes()),
                    con.var_type.clone(),
                )
            })
            .collect::<BTreeMap<HashOf<Ext>, VarType>>()
    });
    CompiledExecutable {
        ext,
        payout,
        code,
        constants,
        address_inputs,
        bytes_inputs,
    }
}

pub fn logical_connective_executable_from_address<'a, Ext: ExternalCall>(
    ext: &'a mut Ext,
    address: AddressOf<Ext>,
) -> Option<LogicalConnectiveExecutable<'a, Ext>> where
{
    match address {
        x if x == Ext::NotPredicate => Some(LogicalConnectiveExecutable::Not(NotPredicate { ext })),
        x if x == Ext::AndPredicate => Some(LogicalConnectiveExecutable::And(AndPredicate { ext })),
        _ => None,
    }
}
