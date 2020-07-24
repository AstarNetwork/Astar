use crate::executor::*;
use crate::predicates::*;
use crate::*;

#[cfg(feature = "std")]
use std::fs::File;
#[cfg(feature = "std")]
use std::io::Read;

#[cfg(feature = "std")]
mod serializable_predicates;

#[cfg(feature = "std")]
pub use serializable_predicates::*;

#[cfg(test)]
mod tests;

#[cfg(feature = "std")]
pub fn compile_from_json(json: &str) -> Result<CompiledPredicate, serde_json::Error> {
    let ret: CompiledPredicateSerializable = serde_json::from_str(json)?;
    Ok(ret.into())
}

#[cfg(feature = "std")]
pub fn load_predicate_json(filename: &str) -> String {
    let path = ["tests/", filename].concat();
    let mut file = File::open(path).expect("file laod error");
    let mut predicate_json = String::new();
    file.read_to_string(&mut predicate_json)
        .expect("something went wrong reading the file");
    println!("predicate json : {:?}", predicate_json);
    predicate_json
}

use crate::compiled_predicates::VarType;

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
                    Ext::Hashing::hash(con.name.as_slice()),
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
        x if x == &Ext::not_address() => {
            Some(LogicalConnectiveExecutable::Not(NotPredicate { ext }))
        }
        x if x == &Ext::and_address() => {
            Some(LogicalConnectiveExecutable::And(AndPredicate { ext }))
        }
        x if x == &Ext::or_address() => Some(LogicalConnectiveExecutable::Or(OrPredicate { ext })),
        x if x == &Ext::for_all_address() => {
            Some(LogicalConnectiveExecutable::ForAll(ForAllPredicate { ext }))
        }
        x if x == &Ext::there_exists_address() => Some(LogicalConnectiveExecutable::ThereExists(
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
        x if x == &Ext::not_address() => Some(DecidableExecutable::Not(NotPredicate { ext })),
        x if x == &Ext::and_address() => Some(DecidableExecutable::And(AndPredicate { ext })),
        x if x == &Ext::or_address() => Some(DecidableExecutable::Or(OrPredicate { ext })),
        x if x == &Ext::for_all_address() => {
            Some(DecidableExecutable::ForAll(ForAllPredicate { ext }))
        }
        x if x == &Ext::there_exists_address() => {
            Some(DecidableExecutable::ThereExists(ThereExistsPredicate {
                ext,
            }))
        }
        x if x == &Ext::equal_address() => Some(DecidableExecutable::Equal(EqualPredicate { ext })),
        x if x == &Ext::is_contained_address() => {
            Some(DecidableExecutable::IsContained(IsContainedPredicate {
                ext,
            }))
        }
        x if x == &Ext::is_less_address() => {
            Some(DecidableExecutable::IsLess(IsLessThanPredicate { ext }))
        }
        x if x == &Ext::is_stored_address() => {
            Some(DecidableExecutable::IsStored(IsStoredPredicate { ext }))
        }
        x if x == &Ext::is_valid_signature_address() => Some(
            DecidableExecutable::IsValidSignature(IsValidSignaturePredicate { ext }),
        ),
        x if x == &Ext::verify_inclusion_address() => Some(DecidableExecutable::VerifyInclusion(
            VerifyInclusionPredicate { ext },
        )),
        _ => None,
    }
}

pub fn atomic_executable_from_address<'a, Ext: ExternalCall>(
    ext: &'a Ext,
    address: &AddressOf<Ext>,
) -> Option<AtomicExecutable<'a, Ext>> where
{
    match address {
        x if x == &Ext::equal_address() => Some(AtomicExecutable::Equal(EqualPredicate { ext })),
        x if x == &Ext::is_contained_address() => {
            Some(AtomicExecutable::IsContained(IsContainedPredicate { ext }))
        }
        x if x == &Ext::is_less_address() => {
            Some(AtomicExecutable::IsLess(IsLessThanPredicate { ext }))
        }
        x if x == &Ext::is_stored_address() => {
            Some(AtomicExecutable::IsStored(IsStoredPredicate { ext }))
        }
        x if x == &Ext::is_valid_signature_address() => Some(AtomicExecutable::IsValidSignature(
            IsValidSignaturePredicate { ext },
        )),
        x if x == &Ext::verify_inclusion_address() => Some(AtomicExecutable::VerifyInclusion(
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
        x if x == &Ext::equal_address() => {
            Some(BaseAtomicExecutable::Equal(EqualPredicate { ext }))
        }
        x if x == &Ext::is_contained_address() => {
            Some(BaseAtomicExecutable::IsContained(IsContainedPredicate {
                ext,
            }))
        }
        x if x == &Ext::is_less_address() => {
            Some(BaseAtomicExecutable::IsLess(IsLessThanPredicate { ext }))
        }
        x if x == &Ext::is_stored_address() => {
            Some(BaseAtomicExecutable::IsStored(IsStoredPredicate { ext }))
        }
        x if x == &Ext::is_valid_signature_address() => Some(
            BaseAtomicExecutable::IsValidSignature(IsValidSignaturePredicate { ext }),
        ),
        x if x == &Ext::verify_inclusion_address() => Some(BaseAtomicExecutable::VerifyInclusion(
            VerifyInclusionPredicate { ext },
        )),
        _ => None,
    }
}
