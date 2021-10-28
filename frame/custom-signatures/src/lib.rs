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
        traits::{
            Currency, ExistenceRequirement, Get, OnUnbalanced, UnfilteredDispatchable,
            WithdrawReasons,
        },
        weights::GetDispatchInfo,
    };
    use frame_system::{ensure_none, pallet_prelude::*};
    use sp_runtime::traits::{IdentifyAccount, Verify};
    use sp_std::{convert::TryFrom, prelude::*};

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// The balance type of this pallet.
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

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

        /// The currency trait.
        type Currency: Currency<Self::AccountId>;

        /// The call fee destination.
        type OnChargeTransaction: OnUnbalanced<
            <Self::Currency as Currency<Self::AccountId>>::NegativeImbalance,
        >;

        /// The call processing fee amount.
        #[pallet::constant]
        type CallFee: Get<BalanceOf<Self>>;

        /// The call magic number.
        #[pallet::constant]
        type CallMagicNumber: Get<u16>;

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
        /// Bad nonce parameter.
        BadNonce,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
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
            signer: T::AccountId,
            signature: Vec<u8>,
            #[pallet::compact] nonce: T::Index,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            // Ensure that transaction isn't stale
            ensure!(
                nonce == frame_system::Pallet::<T>::account_nonce(signer.clone()),
                Error::<T>::BadNonce,
            );

            let signature = <T as Config>::Signature::try_from(signature)
                .map_err(|_| Error::<T>::DecodeFailure)?;

            // Ensure that transaction signature is valid
            ensure!(
                Self::valid_signature(&call, &signer, &signature, &nonce),
                Error::<T>::InvalidSignature
            );

            // Increment account nonce
            frame_system::Pallet::<T>::inc_account_nonce(signer.clone());

            // Processing fee
            let tx_fee = T::Currency::withdraw(
                &signer,
                T::CallFee::get(),
                WithdrawReasons::FEE,
                ExistenceRequirement::AllowDeath,
            )?;
            T::OnChargeTransaction::on_unbalanced(tx_fee);

            // Dispatch call
            let new_origin = frame_system::RawOrigin::Signed(signer.clone()).into();
            let res = call.dispatch_bypass_filter(new_origin).map(|_| ());
            Self::deposit_event(Event::Executed(signer, res.map_err(|e| e.error)));

            // Fee already charged
            Ok(Pays::No.into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Verify custom signature and returns `true` if correct.
        pub fn valid_signature(
            call: &Box<<T as Config>::Call>,
            signer: &T::AccountId,
            signature: &T::Signature,
            nonce: &T::Index,
        ) -> bool {
            let payload = (T::CallMagicNumber::get(), *nonce, call.clone());
            signature.verify(&payload.encode()[..], signer)
        }
    }

    pub(crate) const SIGNATURE_DECODE_FAILURE: u8 = 1;

    #[pallet::validate_unsigned]
    impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            // Call decomposition (we have only one possible value here)
            let (call, signer, signature, nonce) = match call {
                Call::call {
                    call,
                    signer,
                    signature,
                    nonce,
                } => (call, signer, signature, nonce),
                _ => return InvalidTransaction::Call.into(),
            };

            // Check that tx isn't stale
            if *nonce != frame_system::Pallet::<T>::account_nonce(signer.clone()) {
                return InvalidTransaction::Stale.into();
            }

            // Check signature encoding
            if let Ok(signature) = <T as Config>::Signature::try_from(signature.clone()) {
                // Verify signature
                if Self::valid_signature(call, signer, &signature, nonce) {
                    ValidTransaction::with_tag_prefix("CustomSignatures")
                        .priority(T::UnsignedPriority::get())
                        .and_provides((call, signer, nonce))
                        .longevity(64_u64)
                        .propagate(true)
                        .build()
                } else {
                    // Signature mismatched to given signer
                    InvalidTransaction::BadProof.into()
                }
            } else {
                // Signature encoding broken
                InvalidTransaction::Custom(SIGNATURE_DECODE_FAILURE).into()
            }
        }
    }
}
