use super::*;
use crate::compiled_predicates::*;
use crate::predicates::*;
use codec::Codec;
use core::fmt::{Debug, Display};
use core::marker::PhantomData;
pub use hash_db::Hasher;
use snafu::{ResultExt, Snafu};

#[derive(Snafu)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum ExecError<Address> {
    #[snafu(display("Require error: {}", msg))]
    Require { msg: &'static str },
    #[snafu(display(
        "Can not call method error: you call {}, expected {}",
        call_method,
        expected
    ))]
    CallMethod {
        call_method: PredicateCallInputs<Address>,
        expected: &'static str,
    },
    #[snafu(display("Codec error: type name is {}", type_name))]
    CodecError { type_name: &'static str },
    #[snafu(display("Unexpected error: {}", msg))]
    Unexpected { msg: &'static str },
}

pub type ExecResult<Address> = core::result::Result<bool, ExecError<Address>>;
pub type ExecResultT<T, Address> = core::result::Result<T, ExecError<Address>>;
pub type AddressOf<Ext> = <Ext as ExternalCall>::Address;
pub type HashOf<Ext> = <Ext as ExternalCall>::Hash;
pub type HashingOf<Ext> = <Ext as ExternalCall>::Hashing;

pub trait ExternalCall {
    type Address: Codec + Debug + Clone + Eq + PartialEq;
    type Hash: AsRef<[u8]>
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
        + Ord;
    type Hashing: Hasher<Out = Self::Hash>;

    // relation const any atomic predicate address.
    const NotPredicate: Self::Address;
    const AndPredicate: Self::Address;

    /// Call (other predicate) into the specified account.
    fn ext_call(
        &mut self,
        to: &Self::Address,
        input_data: PredicateCallInputs<Self::Address>,
    ) -> ExecResult<Self::Address>;

    /// Returns a reference to the account id of the caller.
    fn ext_caller(&self) -> Self::Address;

    /// Returns a reference to the account id of the current contract.
    fn ext_address(&self) -> Self::Address;

    // Notes a call other storage.
    // Only return true or false.
    // CommitmentAddress(special) isCommitment(address) -> Commitment
    // is_stored_predicate(&mut self, address, key, value);?
    // ref: https://github.com/cryptoeconomicslab/ovm-contracts/blob/master/contracts/Predicate/Atomic/IsStoredPredicate.sol
    fn ext_is_stored(&mut self, address: &Self::Address, key: &[u8], value: &[u8]) -> bool;
}

pub trait OvmExecutor<P> {
    type ExtCall: ExternalCall;
    fn execute(
        executable: P,
        call_method: PredicateCallInputs<AddressOf<Self::ExtCall>>,
    ) -> ExecResult<AddressOf<Self::ExtCall>>;
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
    ) -> ExecResult<AddressOf<Ext>> {
        match call_method {
            PredicateCallInputs::AtomicPredicate(atomic) => {
                match atomic {
                    AtomicPredicateCallInputs::Decide { inputs } => {
                        return predicate.decide(inputs)
                    }
                    AtomicPredicateCallInputs::DecideTrue { inputs } => {
                        predicate.decide_true(inputs);
                        return Ok(true);
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
