#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

/// Ethereum-compatible signatures (eth_sign API call).
pub mod ethereum;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Get, UnfilteredDispatchable},
        weights::GetDispatchInfo,
    };
    use frame_system::{ensure_none, pallet_prelude::*};
    use sp_runtime::traits::{IdentifyAccount, Verify};
    use sp_std::{convert::TryFrom, prelude::*};

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// A signable call.
        type Call: Parameter + UnfilteredDispatchable<Origin = Self::Origin> + GetDispatchInfo;

        /// User defined signature type.
        type Signature: Parameter + Verify<Signer = Self::Signer> + TryFrom<Vec<u8>>;

        /// User defined signer type.
        type Signer: IdentifyAccount<AccountId = Self::AccountId>;

        /// A configuration for base priority of unsigned transactions.
        ///
        /// This is exposed so that it can be tuned for particular runtime, when
        /// multiple pallets send unsigned transactions.
        type UnsignedPriority: Get<TransactionPriority>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Signature decode fails.
        DecodeFailure,
        /// Signature and account mismatched.
        InvalidSignature,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        /// A call just executed. \[result\]
        Executed(T::AccountId, DispatchResult),
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// # <weight>
        /// - O(1).
        /// - Limited storage reads.
        /// - One DB write (event).
        /// - Weight of derivative `call` execution + 10,000.
        /// # </weight>
        #[pallet::weight({
            let dispatch_info = call.get_dispatch_info();
            (dispatch_info.weight + 10_000, dispatch_info.class)
        })]
        pub fn call(
            origin: OriginFor<T>,
            call: Box<<T as Config>::Call>,
            account: T::AccountId,
            signature: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            let signature = <T as Config>::Signature::try_from(signature)
                .map_err(|_| Error::<T>::DecodeFailure)?;
            if signature.verify(&call.encode()[..], &account) {
                let new_origin = frame_system::RawOrigin::Signed(account.clone()).into();
                let res = call.dispatch_bypass_filter(new_origin).map(|_| ());
                Self::deposit_event(Event::Executed(account, res.map_err(|e| e.error)));
                Ok(Pays::No.into())
            } else {
                Err(Error::<T>::InvalidSignature)?
            }
        }
    }

    pub(crate) const SIGNATURE_DECODE_FAILURE: u8 = 1;

    #[pallet::validate_unsigned]
    impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            if let Call::call(call, signer, signature) = call {
                if let Ok(signature) = <T as Config>::Signature::try_from(signature.clone()) {
                    if signature.verify(&call.encode()[..], &signer) {
                        return ValidTransaction::with_tag_prefix("CustomSignatures")
                            .priority(T::UnsignedPriority::get())
                            .and_provides((call, signer))
                            .longevity(64_u64)
                            .propagate(true)
                            .build();
                    } else {
                        InvalidTransaction::BadProof.into()
                    }
                } else {
                    InvalidTransaction::Custom(SIGNATURE_DECODE_FAILURE).into()
                }
            } else {
                InvalidTransaction::Call.into()
            }
        }
    }
}
