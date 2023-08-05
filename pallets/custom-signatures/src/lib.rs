// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod tests;

use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo},
    pallet_prelude::*,
    traits::{Currency, ExistenceRequirement, Get, OnUnbalanced, WithdrawReasons},
};
use frame_system::{ensure_none, pallet_prelude::*};
use pallet_evm::AddressMapping;
use sp_core::H160;
use sp_runtime::traits::{IdentifyAccount, MaybeDisplay, Verify};
use sp_std::{fmt::Debug, prelude::*};

/// The balance type of this pallet.
pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// A signable call.
        type RuntimeCall: Parameter
            + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
            + GetDispatchInfo;

        /// User defined signature type.
        type Signature: Parameter + Verify<Signer = Self::Signer> + Decode;

        /// User defined signer
        type Signer: IdentifyAccount<AccountId = Self::SignerAccountId>;

        /// User defined mappings
        type AddressMapping: AddressMapping<Self::AccountId>;

        // user defined accountid
        type SignerAccountId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Debug
            + MaybeDisplay
            + Ord
            + MaxEncodedLen
            + Into<H160>;

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
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
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
        /// - Weight of derivative `call` execution + read/write + 10_000.
        /// # </weight>
        #[pallet::call_index(0)]
        #[pallet::weight({
            let dispatch_info = call.get_dispatch_info();
            (dispatch_info.weight.saturating_add(T::DbWeight::get().reads(1))
                                 .saturating_add(T::DbWeight::get().writes(1))
                                 .saturating_add(Weight::from_parts(10_000, 0)),
             dispatch_info.class)
        })]
        pub fn call(
            origin: OriginFor<T>,
            call: Box<<T as Config>::RuntimeCall>,
            signer: T::SignerAccountId,
            signature: Vec<u8>,
            #[pallet::compact] nonce: T::Index,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            let account = T::AddressMapping::into_account_id(signer.clone().into());

            // Ensure that transaction isn't stale
            ensure!(
                nonce == frame_system::Pallet::<T>::account_nonce(account.clone()),
                Error::<T>::BadNonce,
            );

            let signature = <T as Config>::Signature::decode(&mut signature.as_slice())
                .map_err(|_| Error::<T>::DecodeFailure)?;

            // Ensure that transaction signature is valid
            ensure!(
                Self::valid_signature(&call, &signer, &signature, &nonce),
                Error::<T>::InvalidSignature
            );

            // Increment account nonce
            frame_system::Pallet::<T>::inc_account_nonce(account.clone());

            // Processing fee
            let tx_fee = T::Currency::withdraw(
                &account,
                T::CallFee::get(),
                WithdrawReasons::FEE,
                ExistenceRequirement::AllowDeath,
            )?;
            T::OnChargeTransaction::on_unbalanced(tx_fee);

            // Dispatch call
            let new_origin = frame_system::RawOrigin::Signed(account.clone()).into();
            let res: Result<
                (),
                sp_runtime::DispatchErrorWithPostInfo<
                    <<T as Config>::RuntimeCall as Dispatchable>::PostInfo,
                >,
            > = call.dispatch(new_origin).map(|_| ());
            Self::deposit_event(Event::Executed(account, res.map_err(|e| e.error)));

            // Fee already charged
            Ok(Pays::No.into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Verify custom signature and returns `true` if correct.
        pub fn valid_signature(
            call: &Box<<T as Config>::RuntimeCall>,
            signer: &T::SignerAccountId,
            signature: &T::Signature,
            nonce: &T::Index,
        ) -> bool {
            let payload = (T::CallMagicNumber::get(), *nonce, call.clone());
            signature.verify(&Self::signable_message(payload.encode())[..], signer)
        }

        /// Constructs the message that Ethereum RPC's `personal_sign` and `eth_sign` would sign.
        ///
        /// Note: sign message hash to escape of message length estimation.
        pub fn signable_message<I: AsRef<[u8]>>(what: I) -> Vec<u8> {
            let what = what.as_ref();
            let hash = sp_io::hashing::keccak_256(what);
            let mut v = b"\x19Ethereum Signed Message:\n32".to_vec();
            v.extend_from_slice(&hash[..]);
            v
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

            let account = T::AddressMapping::into_account_id(signer.clone().into());

            // Check that tx isn't stale
            if *nonce != frame_system::Pallet::<T>::account_nonce(account.clone()) {
                return InvalidTransaction::Stale.into();
            }

            // Check signature encoding
            if let Ok(signature) = <T as Config>::Signature::decode(&mut signature.as_slice()) {
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
