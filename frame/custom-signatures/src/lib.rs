#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_event, decl_module, decl_error, Parameter,
    weights::{GetDispatchInfo, Pays},
    traits::{UnfilteredDispatchable, Get},
    dispatch::DispatchResultWithPostInfo,
};
use sp_runtime::{
    DispatchResult,
    traits::{Verify, IdentifyAccount},
    transaction_validity::{
        TransactionValidity, ValidTransaction, InvalidTransaction, TransactionSource,
        TransactionPriority,
    },
};
use frame_system::ensure_none;
use sp_std::prelude::*;
use codec::Encode;

pub mod ethereum;

/// The module's configuration trait.
pub trait Trait: frame_system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// A signable call.
    type Call: Parameter + UnfilteredDispatchable<Origin=Self::Origin> + GetDispatchInfo;

    /// User defined signature type.
    type Signature: Parameter + Verify<Signer = Self::Signer>;

    /// User defined signer type.
    type Signer: IdentifyAccount<AccountId = Self::AccountId>;

    /// A configuration for base priority of unsigned transactions.
    ///
    /// This is exposed so that it can be tuned for particular runtime, when
    /// multiple pallets send unsigned transactions.
    type UnsignedPriority: Get<TransactionPriority>;
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Provided invalid signature data.
        InvalidSignature,
    }
}

decl_event!(
    pub enum Event<T> where AccountId = <T as frame_system::Trait>::AccountId {
        /// A call just executed. \[result\]
        Executed(AccountId, DispatchResult),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = (call.get_dispatch_info().weight + 10_000, call.get_dispatch_info().class)]
        fn call(
            origin,
            call: Box<<T as Trait>::Call>,
            signer: T::AccountId,
            signature: <T as Trait>::Signature,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            if signature.verify(call.encode().as_ref(), &signer) {
                let signer_origin = frame_system::RawOrigin::Signed(signer.clone()).into();
                let res = call.dispatch_bypass_filter(signer_origin).map(|_| ());
                Self::deposit_event(RawEvent::Executed(signer, res.map_err(|e| e.error)));
                Ok(Pays::No.into())
            } else {
                Err(Error::<T>::InvalidSignature)?
            }
        }
    }
}

impl<T: Trait> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    fn validate_unsigned(
        _source: TransactionSource,
        call: &Self::Call,
    ) -> TransactionValidity {
        if let Call::call(call, signer, signature) = call {
            if !signature.verify(call.encode().as_ref(), &signer) {
                return InvalidTransaction::BadProof.into();
            }

            ValidTransaction::with_tag_prefix("CustomSignatures")
                .priority(T::UnsignedPriority::get())
                .and_provides((call, signer))
                .longevity(64_u64)
                .propagate(true)
                .build()
        } else {
            InvalidTransaction::Call.into()
        }
    }
}
