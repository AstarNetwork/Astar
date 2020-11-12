//! Transaction call filtering.

use codec::{Decode, Encode};
use frame_support::traits::Filter;
use plasm_primitives::AccountId;
use sp_runtime::{
    traits::{DispatchInfoOf, Dispatchable, SignedExtension},
    transaction_validity::{InvalidTransaction, TransactionValidity, TransactionValidityError},
};

/// Apply a given filter to transactions.
pub struct TransactionCallFilter<T: Filter<Call>, Call>(sp_std::marker::PhantomData<(T, Call)>);

impl<F: Filter<Call>, Call> Default for TransactionCallFilter<F, Call> {
    fn default() -> Self {
        Self::new()
    }
}
impl<F: Filter<Call>, Call> Encode for TransactionCallFilter<F, Call> {
    fn using_encoded<R, FO: FnOnce(&[u8]) -> R>(&self, f: FO) -> R {
        f(&b""[..])
    }
}
impl<F: Filter<Call>, Call> Decode for TransactionCallFilter<F, Call> {
    fn decode<I: codec::Input>(_: &mut I) -> Result<Self, codec::Error> {
        Ok(Self::new())
    }
}
impl<F: Filter<Call>, Call> Clone for TransactionCallFilter<F, Call> {
    fn clone(&self) -> Self {
        Self::new()
    }
}
impl<F: Filter<Call>, Call> Eq for TransactionCallFilter<F, Call> {}
impl<F: Filter<Call>, Call> PartialEq for TransactionCallFilter<F, Call> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl<F: Filter<Call>, Call> sp_std::fmt::Debug for TransactionCallFilter<F, Call> {
    fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        Ok(())
    }
}

fn validate<F: Filter<Call>, Call>(call: &Call) -> TransactionValidity {
    if F::filter(call) {
        Ok(Default::default())
    } else {
        Err(InvalidTransaction::Custom(2).into())
    }
}

impl<F: Filter<Call> + Send + Sync, Call: Dispatchable + Send + Sync> SignedExtension
    for TransactionCallFilter<F, Call>
{
    const IDENTIFIER: &'static str = "TransactionCallFilter";
    type AccountId = AccountId;
    type Call = Call;
    type AdditionalSigned = ();
    type Pre = ();

    fn additional_signed(&self) -> sp_std::result::Result<(), TransactionValidityError> {
        Ok(())
    }

    fn validate(
        &self,
        _: &Self::AccountId,
        call: &Call,
        _: &DispatchInfoOf<Self::Call>,
        _: usize,
    ) -> TransactionValidity {
        validate::<F, _>(call)
    }

    fn validate_unsigned(
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> TransactionValidity {
        validate::<F, _>(call)
    }
}

impl<F: Filter<Call>, Call> TransactionCallFilter<F, Call> {
    /// Create a new instance.
    pub fn new() -> Self {
        Self(sp_std::marker::PhantomData)
    }
}
