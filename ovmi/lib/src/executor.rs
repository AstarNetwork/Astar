use super::*;
use crate::predicates::*;
use codec::Codec;
use core::fmt::Debug;
use core::marker::PhantomData;
pub use hash_db::Hasher;

#[derive(PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum ExecError<Address> {
    Require {
        msg: &'static str,
    },
    CallMethod {
        call_method: PredicateCallInputs<Address>,
        expected: &'static str,
    },
    CallAddress {
        address: Address,
    },
    CodecError {
        type_name: &'static str,
    },
    ExternalError {
        msg: &'static str,
    },
    Unexpected {
        msg: &'static str,
    },
    /// Unimplemented error.
    Unimplemented,
}

impl<Address> From<&'static str> for ExecError<Address> {
    fn from(msg: &'static str) -> ExecError<Address> {
        ExecError::<Address>::ExternalError { msg }
    }
}

/// convert to error code from error tyoe.
pub fn codec_error<Address>(expected_type_name: &'static str) -> ExecError<Address> {
    ExecError::CodecError {
        type_name: expected_type_name,
    }
}

/// Default ExecResult type bool.
pub type ExecResult<Address> = core::result::Result<bool, ExecError<Address>>;
/// Generic ExecResult type.
pub type ExecResultT<T, Address> = core::result::Result<T, ExecError<Address>>;
/// Generic ExecResult tyoe from Ext.
pub type ExecResultTOf<T, Ext> = core::result::Result<T, ExecError<AddressOf<Ext>>>;
/// Address type from external.
pub type AddressOf<Ext> = <Ext as ExternalCall>::Address;
/// Hash type from external.
pub type HashOf<Ext> = <Ext as ExternalCall>::Hash;
/// Hashing type from external.
pub type HashingOf<Ext> = <Ext as ExternalCall>::Hashing;
/// Property type from external.
pub type PropertyOf<Ext> = Property<<Ext as ExternalCall>::Address>;

/// Maybe Address defines the traits should be implemented.
pub trait MaybeAddress: Codec + Debug + Clone + Eq + PartialEq + Default {}
impl<T: Codec + Debug + Clone + Eq + PartialEq + Default> MaybeAddress for T {}

pub trait MaybeHash:
    AsRef<[u8]>
    + AsMut<[u8]>
    + Default
    + Codec
    + Debug
    + core::hash::Hash
    + Send
    + Sync
    + Clone
    + Copy
    + Eq
    + PartialEq
    + Ord
{
}
impl<
        T: AsRef<[u8]>
            + AsMut<[u8]>
            + Default
            + Codec
            + Debug
            + core::hash::Hash
            + Send
            + Sync
            + Clone
            + Copy
            + Eq
            + PartialEq
            + Ord,
    > MaybeHash for T
{
}

pub const NOT_VARIABLE: &'static [u8] = b"Not";
pub const AND_VARIABLE: &'static [u8] = b"And";
pub const OR_VARIABLE: &'static [u8] = b"Or";
pub const FOR_ALL_VARIABLE: &'static [u8] = b"ForAllSuchThat";
pub const THERE_EXISTS_VARIABLE: &'static [u8] = b"ThereExistsSuchThat";
pub const EQUAL_VARIABLE: &'static [u8] = b"Equal";
pub const IS_CONTAINED_VARIABLE: &'static [u8] = b"IsContained";
pub const IS_LESS_VARIABLE: &'static [u8] = b"IsLessThan";
pub const IS_STORED_VARIABLE: &'static [u8] = b"IsStored";
pub const IS_VALID_SIGNATURE_VARIABLE: &'static [u8] = b"IsValidSignature";
pub const VERIFY_INCLUSION_VARIABLE: &'static [u8] = b"VerifyInclusion";

pub trait ExternalCall {
    /// The address type of Plasma child chain (default: AccountId32)
    type Address: MaybeAddress;
    /// The hash type of Plasma child chain (default: H256)
    type Hash: MaybeHash;
    /// The hashing type of Plasma child chain (default: Keccak256)
    type Hashing: Hasher<Out = Self::Hash>;

    // relation const any atomic predicate address.
    /// The address of not predicate address.
    fn not_address() -> Self::Address;
    /// The address of and predicate address.
    fn and_address() -> Self::Address;
    /// The address of or predicate address.
    fn or_address() -> Self::Address;
    /// The address of for all predicate address.
    fn for_all_address() -> Self::Address;
    /// The address of there exists predicate address.
    fn there_exists_address() -> Self::Address;
    /// The address of equal predicate address.
    fn equal_address() -> Self::Address;
    /// The address of is contained predicate address.
    fn is_contained_address() -> Self::Address;
    /// The address of is less than  predicate address.
    fn is_less_address() -> Self::Address;
    /// The address of is stored predicate address.
    fn is_stored_address() -> Self::Address;
    /// The address of is valid signature predicate address.
    fn is_valid_signature_address() -> Self::Address;
    /// The address of verify inclusion predicate address.
    fn verify_inclusion_address() -> Self::Address;

    fn vec_to_address(key: &Vec<u8>) -> Option<Self::Address> {
        match key {
            x if x.as_slice() == NOT_VARIABLE => Some(Self::not_address()),
            x if x.as_slice() == AND_VARIABLE => Some(Self::and_address()),
            x if x.as_slice() == OR_VARIABLE => Some(Self::or_address()),
            x if x.as_slice() == FOR_ALL_VARIABLE => Some(Self::for_all_address()),
            x if x.as_slice() == THERE_EXISTS_VARIABLE => Some(Self::there_exists_address()),
            x if x.as_slice() == EQUAL_VARIABLE => Some(Self::equal_address()),
            x if x.as_slice() == IS_CONTAINED_VARIABLE => Some(Self::is_contained_address()),
            x if x.as_slice() == IS_LESS_VARIABLE => Some(Self::is_less_address()),
            x if x.as_slice() == IS_STORED_VARIABLE => Some(Self::is_stored_address()),
            x if x.as_slice() == IS_VALID_SIGNATURE_VARIABLE => {
                Some(Self::is_valid_signature_address())
            }
            x if x.as_slice() == VERIFY_INCLUSION_VARIABLE => {
                Some(Self::verify_inclusion_address())
            }
            _ => None,
        }
    }

    /// relation const any signature algorithm.
    fn secp256k1() -> Self::Hash;

    /// Produce the hash of some codec-encodable value.
    fn hash_of<S: Encode>(s: &S) -> Self::Hash {
        Encode::using_encoded(s, Self::Hashing::hash)
    }

    /// Call (other predicate) into the specified account.
    fn ext_call(
        &self,
        to: &Self::Address,
        input_data: PredicateCallInputs<Self::Address>,
    ) -> ExecResultT<Vec<u8>, Self::Address>;

    /// Returns a reference to the account id of the caller.
    fn ext_caller(&self) -> Self::Address;

    /// Returns a reference to the account id of the current contract.
    fn ext_address(&self) -> Self::Address;

    /// Notes a call other storage.
    /// Only return true or false.
    /// CommitmentAddress(special) isCommitment(address) -> Commitment
    /// is_stored_predicate(&self, address, key, value);?
    /// ref: https://github.com/cryptoeconomicslab/ovm-contracts/blob/master/contracts/Predicate/Atomic/IsStoredPredicate.sol
    fn ext_is_stored(&self, address: &Self::Address, key: &[u8], value: &[u8]) -> bool;

    /// Verify messagge hash with signature and address.
    /// Should be used by ECDSA.
    fn ext_verify(&self, hash: &Self::Hash, signature: &[u8], address: &Self::Address) -> bool;

    /// verifyInclusionWithRoot method verifies inclusion proof in Double Layer Tree.
    /// Must be used by kind of Commitment contract by Plasma module.
    fn ext_verify_inclusion_with_root(
        &self,
        leaf: Self::Hash,
        token_address: Self::Address,
        range: &[u8],
        inclusion_proof: &[u8],
        root: &[u8],
    ) -> bool;

    /* Helpers of UniversalAdjudicationContract. */
    /// `is_decided` function of UniversalAdjudication in OVM module.
    fn ext_is_decided(&self, property: &PropertyOf<Self>) -> bool;
    /// `is_decided_by_id` function of UniversalAdjudication in OVM module.
    fn ext_is_decided_by_id(&self, id: Self::Hash) -> bool;
    /// `get_property_id` function of UniversalAdjudication in OVM module.
    fn ext_get_property_id(&self, property: &PropertyOf<Self>) -> Self::Hash;
    /// `set_predicate_decision` function of UniversalAdjudication in OVM module.
    fn ext_set_predicate_decision(
        &self,
        game_id: Self::Hash,
        decision: bool,
    ) -> ExecResult<Self::Address>;

    /* Helpers of UtilsContract. */
    /// @dev check target is variable or not.
    /// A variable has prefix V and its length is less than 20.
    fn is_placeholder(target: &Vec<u8>) -> bool {
        return target.len() < 20 && target.get(0) == Some(&(b'V' as u8));
    }

    /// @dev check target is label or not.
    /// A label has prefix L and its length is less than 20.
    fn is_label(target: &Vec<u8>) -> bool {
        return target.len() < 20 && target.get(0) == Some(&(b'L' as u8));
    }

    /// sub_bytes of [start_idnex, end_idnex).
    fn sub_bytes(target: &Vec<u8>, start_index: u128, end_index: u128) -> Vec<u8> {
        target
            .as_slice()
            .get((start_index as usize)..(end_index as usize))
            .unwrap_or(vec![].as_slice())
            .to_vec()
    }

    /// sub array of [start_idnex, end_idnex).
    fn sub_array(target: &Vec<Vec<u8>>, start_index: usize, end_index: usize) -> Vec<Vec<u8>> {
        target
            .as_slice()
            .get((start_index)..(end_index))
            .unwrap_or(vec![].as_slice())
            .to_vec()
    }

    /// sub_bytes of [1...).
    fn get_input_value(target: &Vec<u8>) -> Vec<u8> {
        Self::sub_bytes(target, 1, target.len() as u128)
    }

    /// Decoded to u128
    fn bytes_to_u128(target: &Vec<u8>) -> ExecResultT<u128, Self::Address> {
        Decode::decode(&mut &target[..]).map_err(|_| codec_error::<Self::Address>("u128"))
    }

    /// Decoded to range
    fn bytes_to_range(target: &Vec<u8>) -> ExecResultT<Range, Self::Address> {
        Decode::decode(&mut &target[..]).map_err(|_| codec_error::<Self::Address>("Range"))
    }

    /// Decoded to Address
    fn bytes_to_address(target: &Vec<u8>) -> ExecResultT<Self::Address, Self::Address> {
        Decode::decode(&mut &target[..]).map_err(|_| codec_error::<Self::Address>("Address"))
    }

    /// Decoded to bool
    fn bytes_to_bool(target: &Vec<u8>) -> ExecResultT<bool, Self::Address> {
        Decode::decode(&mut &target[..]).map_err(|_| codec_error::<Self::Address>("bool"))
    }

    /// Decoded to Property
    fn bytes_to_property(target: &Vec<u8>) -> ExecResultT<PropertyOf<Self>, Self::Address> {
        Decode::decode(&mut &target[..])
            .map_err(|_| codec_error::<Self::Address>("PropertyOf<Ext>"))
    }

    /// Decoded to Vec<Vec<u8>>
    fn bytes_to_bytes_array(target: &Vec<u8>) -> ExecResultT<Vec<Vec<u8>>, Self::Address> {
        Decode::decode(&mut &target[..]).map_err(|_| codec_error::<Self::Address>("Vec<Vec<u8>>"))
    }

    fn prefix_label(source: &Vec<u8>) -> Vec<u8> {
        Self::prefix(b'L', source)
    }

    fn prefix_variable(source: &Vec<u8>) -> Vec<u8> {
        Self::prefix(b'V', source)
    }

    fn prefix(prefix: u8, source: &Vec<u8>) -> Vec<u8> {
        vec![vec![prefix], source.clone()].concat()
    }
}

pub trait OvmExecutor<P> {
    type ExtCall: ExternalCall;
    fn execute(
        executable: P,
        call_method: PredicateCallInputs<AddressOf<Self::ExtCall>>,
    ) -> ExecResultT<Vec<u8>, AddressOf<Self::ExtCall>>;
}

pub struct AtomicExecutor<P, Ext> {
    _phantom: PhantomData<(P, Ext)>,
}

impl<P, Ext> OvmExecutor<P> for AtomicExecutor<P, Ext>
where
    P: predicates::AtomicPredicateInterface<AddressOf<Ext>>,
    Ext: ExternalCall,
{
    type ExtCall = Ext;
    fn execute(
        predicate: P,
        call_method: PredicateCallInputs<AddressOf<Ext>>,
    ) -> ExecResultT<Vec<u8>, Ext::Address> {
        match call_method {
            PredicateCallInputs::AtomicPredicate(atomic) => {
                match atomic {
                    AtomicPredicateCallInputs::Decide { inputs } => {
                        return Ok(predicate.decide(inputs)?.encode());
                    }
                    AtomicPredicateCallInputs::DecideTrue { inputs } => {
                        predicate.decide_true(inputs)?;
                        return Ok(true.encode());
                    }
                };
            }
            other => Err(ExecError::CallMethod {
                call_method: other,
                expected: "AtomicPredicateCallInputs",
            }),
        }
    }
}

pub struct BaseAtomicExecutor<P, Ext> {
    _phantom: PhantomData<(P, Ext)>,
}

impl<P, Ext> OvmExecutor<P> for BaseAtomicExecutor<P, Ext>
where
    P: predicates::BaseAtomicPredicateInterface<AddressOf<Ext>>,
    Ext: ExternalCall,
{
    type ExtCall = Ext;
    fn execute(
        predicate: P,
        call_method: PredicateCallInputs<AddressOf<Ext>>,
    ) -> ExecResultT<Vec<u8>, Ext::Address> {
        match call_method {
            PredicateCallInputs::BaseAtomicPredicate(atomic) => {
                match atomic {
                    BaseAtomicPredicateCallInputs::Decide { inputs } => {
                        return Ok(predicate.decide(inputs)?.encode());
                    }
                    BaseAtomicPredicateCallInputs::DecideTrue { inputs } => {
                        predicate.decide_true(inputs)?;
                        return Ok(true.encode());
                    }
                    BaseAtomicPredicateCallInputs::DecideWithWitness { inputs, witness } => {
                        return Ok(predicate.decide_with_witness(inputs, witness)?.encode());
                    }
                };
            }
            other => Err(ExecError::CallMethod {
                call_method: other,
                expected: "BaseAtomicPredicateCallInputs",
            }),
        }
    }
}

pub struct LogicalConnectiveExecutor<P, Ext> {
    _phantom: PhantomData<(P, Ext)>,
}

impl<P, Ext> OvmExecutor<P> for LogicalConnectiveExecutor<P, Ext>
where
    P: predicates::LogicalConnectiveInterface<AddressOf<Ext>>,
    Ext: ExternalCall,
{
    type ExtCall = Ext;
    fn execute(
        predicate: P,
        call_method: PredicateCallInputs<AddressOf<Ext>>,
    ) -> ExecResultT<Vec<u8>, Ext::Address> {
        match call_method {
            PredicateCallInputs::LogicalConnective(atomic) => {
                match atomic {
                    LogicalConnectiveCallInputs::IsValidChallenge {
                        inputs,
                        challenge_inputs,
                        challenge,
                    } => {
                        return Ok(predicate
                            .is_valid_challenge(inputs, challenge_inputs, challenge)?
                            .encode())
                    }
                };
            }
            other => Err(ExecError::CallMethod {
                call_method: other,
                expected: "LogicalConnectiveCallInputs",
            }),
        }
    }
}

pub struct DecidableExecutor<P, Ext> {
    _phantom: PhantomData<(P, Ext)>,
}

impl<P, Ext> OvmExecutor<P> for DecidableExecutor<P, Ext>
where
    P: predicates::DecidablePredicateInterface<AddressOf<Ext>>,
    Ext: ExternalCall,
{
    type ExtCall = Ext;
    fn execute(
        predicate: P,
        call_method: PredicateCallInputs<AddressOf<Ext>>,
    ) -> ExecResultT<Vec<u8>, Ext::Address> {
        match call_method {
            PredicateCallInputs::DecidablePredicate(atomic) => {
                match atomic {
                    DecidablePredicateCallInputs::DecideWithWitness { inputs, witness } => {
                        return Ok(predicate.decide_with_witness(inputs, witness)?.encode());
                    }
                };
            }
            other => Err(ExecError::CallMethod {
                call_method: other,
                expected: "DecidablePredicateCallInputs",
            }),
        }
    }
}

pub struct CompiledExecutor<P, Ext> {
    _phantom: PhantomData<(P, Ext)>,
}

impl<P, Ext> OvmExecutor<P> for CompiledExecutor<P, Ext>
where
    P: predicates::CompiledPredicateInterface<AddressOf<Ext>>,
    Ext: ExternalCall,
{
    type ExtCall = Ext;
    fn execute(
        predicate: P,
        call_method: PredicateCallInputs<AddressOf<Ext>>,
    ) -> ExecResultT<Vec<u8>, Ext::Address> {
        match call_method {
            PredicateCallInputs::CompiledPredicate(atomic) => {
                match atomic {
                    CompiledPredicateCallInputs::IsValidChallenge {
                        inputs,
                        challenge_inputs,
                        challenge,
                    } => {
                        return Ok(predicate
                            .is_valid_challenge(inputs, challenge_inputs, challenge)?
                            .encode());
                    }
                    CompiledPredicateCallInputs::Decide { inputs, witness } => {
                        return Ok(predicate.decide(inputs, witness)?.encode());
                    }
                    CompiledPredicateCallInputs::DecideTrue { inputs, witness } => {
                        return Ok(predicate.decide_true(inputs, witness)?.encode());
                    }
                    CompiledPredicateCallInputs::DecideWithWitness { inputs, witness } => {
                        return Ok(predicate.decide_with_witness(inputs, witness)?.encode());
                    }
                    CompiledPredicateCallInputs::GetChild {
                        inputs,
                        challenge_input,
                    } => {
                        return Ok(predicate.get_child(inputs, challenge_input)?.encode());
                    }
                };
            }
            other => Err(ExecError::CallMethod {
                call_method: other,
                expected: "CompiledPredicateCallInputs",
            }),
        }
    }
}
