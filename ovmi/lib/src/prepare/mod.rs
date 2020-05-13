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
    ext: &'a Ext,
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
    ext: &'a Ext,
    address: &AddressOf<Ext>,
) -> Option<LogicalConnectiveExecutable<'a, Ext>> where
{
    match address {
        x if x == &Ext::NOT_ADDRESS => Some(LogicalConnectiveExecutable::Not(NotPredicate { ext })),
        x if x == &Ext::AND_ADDRESS => Some(LogicalConnectiveExecutable::And(AndPredicate { ext })),
        x if x == &Ext::OR_ADDRESS => Some(LogicalConnectiveExecutable::Or(OrPredicate { ext })),
        x if x == &Ext::FOR_ALL_ADDRESS => {
            Some(LogicalConnectiveExecutable::ForAll(ForAllPredicate { ext }))
        }
        x if x == &Ext::THERE_EXISTS_ADDRESS => Some(LogicalConnectiveExecutable::ThereExists(
            ThereExistsPredicate { ext },
        )),
        _ => None,
    }
}

pub fn deciable_executable_from_address<'a, Ext: ExternalCall>(
    ext: &'a Ext,
    address: &AddressOf<Ext>,
) -> Option<DecidableExecutable<'a, Ext>> where
{
    match address {
        x if x == &Ext::NOT_ADDRESS => Some(DecidableExecutable::Not(NotPredicate { ext })),
        x if x == &Ext::AND_ADDRESS => Some(DecidableExecutable::And(AndPredicate { ext })),
        x if x == &Ext::OR_ADDRESS => Some(DecidableExecutable::Or(OrPredicate { ext })),
        x if x == &Ext::FOR_ALL_ADDRESS => {
            Some(DecidableExecutable::ForAll(ForAllPredicate { ext }))
        }
        x if x == &Ext::THERE_EXISTS_ADDRESS => {
            Some(DecidableExecutable::ThereExists(ThereExistsPredicate {
                ext,
            }))
        }
        x if x == &Ext::EQUAL_ADDRESS => Some(DecidableExecutable::Equal(EqualPredicate { ext })),
        x if x == &Ext::IS_CONTAINED_ADDRESS => {
            Some(DecidableExecutable::IsContained(IsContainedPredicate {
                ext,
            }))
        }
        x if x == &Ext::IS_LESS_ADDRESS => {
            Some(DecidableExecutable::IsLess(IsLessThanPredicate { ext }))
        }
        x if x == &Ext::IS_STORED_ADDRESS => {
            Some(DecidableExecutable::IsStored(IsStoredPredicate { ext }))
        }
        x if x == &Ext::IS_VALID_SIGNATURE_ADDRESS => Some(DecidableExecutable::IsValidSignature(
            IsValidSignaturePredicate { ext },
        )),
        x if x == &Ext::VERIFY_INCLUAION_ADDRESS => Some(DecidableExecutable::VerifyInclusion(
            VerifyInclusionPredicate { ext },
        )),
        _ => None,
    }
}

pub fn base_atomic_executable_from_address<'a, Ext: ExternalCall>(
    ext: &'a Ext,
    address: &AddressOf<Ext>,
) -> Option<BaseAtomicExecutable<'a, Ext>> where
{
    match address {
        x if x == &Ext::EQUAL_ADDRESS => Some(BaseAtomicExecutable::Equal(EqualPredicate { ext })),
        x if x == &Ext::IS_CONTAINED_ADDRESS => {
            Some(BaseAtomicExecutable::IsContained(IsContainedPredicate {
                ext,
            }))
        }
        x if x == &Ext::IS_LESS_ADDRESS => {
            Some(BaseAtomicExecutable::IsLess(IsLessThanPredicate { ext }))
        }
        x if x == &Ext::IS_STORED_ADDRESS => {
            Some(BaseAtomicExecutable::IsStored(IsStoredPredicate { ext }))
        }
        x if x == &Ext::IS_VALID_SIGNATURE_ADDRESS => Some(BaseAtomicExecutable::IsValidSignature(
            IsValidSignaturePredicate { ext },
        )),
        x if x == &Ext::VERIFY_INCLUAION_ADDRESS => Some(BaseAtomicExecutable::VerifyInclusion(
            VerifyInclusionPredicate { ext },
        )),
        _ => None,
    }
}
