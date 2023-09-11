// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

//! # Pallet Account

#![cfg_attr(not(feature = "std"), no_std)]

use astar_primitives::ethereum_checked::AccountMapping;
use astar_primitives::evm::EvmAddress;
use frame_support::{
    pallet_prelude::*,
    traits::{
        fungible::{Inspect, Mutate},
        tokens::{Fortitude::*, Preservation::*},
        IsType,
    },
};
use frame_system::{ensure_signed, pallet_prelude::*};
use pallet_evm::AddressMapping;
use sp_std::marker::PhantomData;

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod mock;
mod tests;

mod impls;
pub use impls::*;

type SignatureOf<T> = <<T as Config>::ClaimSignature as ClaimSignature>::Signature;

/// Mapping between Native(AccountId) and EVM Address(H160)
pub trait AddressManager<AccountId, Address> {
    /// Gets the account id associated with given address, if mapped else None.
    fn to_account_id(address: &Address) -> Option<AccountId>;
    /// Gets the account id associated with given address.
    /// If no mapping exists, then return the default address.
    fn to_account_id_or_default(address: &Address) -> AccountId;
    /// Gets the default account which is associated with given address.
    fn to_default_account_id(address: &Address) -> AccountId;

    /// Gets the address associated with given account id, if mapped else None.
    fn to_address(account_id: &AccountId) -> Option<Address>;
    /// Gets the address associated with given account id.
    /// If no mapping exists, then return the default account id.
    fn to_address_or_default(account_id: &AccountId) -> Address;
    /// Gets the default address which is associated with given account id.
    fn to_default_address(account_id: &AccountId) -> Address;
}

/// Signature verification scheme for proving address ownership
pub trait ClaimSignature {
    type AccountId;
    type Address;
    /// Signature type, ideally a 512-bit value for ECDSA signatures
    type Signature: Parameter;

    /// Build raw payload that user will sign (keccack hashed).
    fn build_signing_payload(who: &Self::AccountId) -> [u8; 32];
    /// Verify the provided signature against the given account.
    fn verify_signature(who: &Self::AccountId, sig: &Self::Signature) -> Option<Self::Address>;
}

#[frame_support::pallet(dev_mode)]
pub mod pallet {

    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// The Currency for managing Evm account assets
        type Currency: Mutate<Self::AccountId>;
        /// Default evm address to account id conversion
        type DefaultAddressMapping: AddressMapping<Self::AccountId>;
        /// Default account id to evm address conversion
        type DefaultAccountMapping: AccountMapping<Self::AccountId>;
        /// The Signature verification implementation to use for checking claims
        /// Note: the signature type defined by this will be used as parameter in pallet's extrinsic
        type ClaimSignature: ClaimSignature<AccountId = Self::AccountId, Address = EvmAddress>;

        // /// Weight information for the extrinsics in this module
        // type WeightInfo: WeightInfo;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// AccountId has mapped
        AccountIdHasMapped,
        /// Eth address has mapped
        EthAddressHasMapped,
        /// Bad signature
        BadSignature,
        /// Invalid signature
        InvalidSignature,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// EVM Account claimed.
        /// Double Mapping b/w native and evm address created
        AccountClaimed {
            account_id: T::AccountId,
            evm_address: EvmAddress,
        },
    }

    /// Native accounts for EVM address
    /// NativeAccounts: EvmAddress => Option<AccountId>
    #[pallet::storage]
    pub type NativeAccounts<T: Config> =
        StorageMap<_, Twox64Concat, EvmAddress, T::AccountId, OptionQuery>;

    /// EVM Addresses for native accounts
    /// EvmAccounts: AccountId => Option<EvmAddress>
    #[pallet::storage]
    pub type EvmAccounts<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, EvmAddress, OptionQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Claim account mapping between Substrate accounts and EVM accounts.
        /// Ensure no prior mapping exists for evm address.
        ///
        /// - `evm_address`: The address to bind to the caller's account
        /// - `signature`: A signature generated by the address to prove ownership
        #[pallet::call_index(0)]
        #[pallet::weight(0)]
        pub fn claim_evm_account(
            origin: OriginFor<T>,
            evm_address: EvmAddress,
            signature: SignatureOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // make sure no prior mapping exists
            Self::enure_no_mapping(&who, &Some(evm_address))?;
            // claim the address
            Self::do_claim_address(who, evm_address, signature)
        }

        /// Claim default evm address for given account id
        /// Ensure no prior mapping exists for the account
        #[pallet::call_index(1)]
        pub fn claim_default_evm_account(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // make sure no prior mapping exists
            Self::enure_no_mapping(&who, &None)?;
            // claim default address
            let _ = Self::do_claim_default_address(who)?;
            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Ensure no mappings exists for given pair of account/address
    fn enure_no_mapping(account_id: &T::AccountId, address: &Option<EvmAddress>) -> DispatchResult {
        // ensure account_id and address has not been mapped
        ensure!(
            !EvmAccounts::<T>::contains_key(&account_id),
            Error::<T>::AccountIdHasMapped
        );
        // This is not required since checking one mapping is sufficent
        // but this is just for sanity check
        if let Some(addr) = address {
            ensure!(
                !NativeAccounts::<T>::contains_key(addr),
                Error::<T>::EthAddressHasMapped
            );
        }
        Ok(())
    }

    /// Add the given pair to create double mappings
    fn add_mappings(account_id: T::AccountId, address: EvmAddress) {
        NativeAccounts::<T>::insert(&address, &account_id);
        EvmAccounts::<T>::insert(&account_id, &address);

        Self::deposit_event(Event::AccountClaimed {
            account_id,
            evm_address: address,
        });
    }

    /// Claim the given evm address by providing claim signature
    fn do_claim_address(
        account_id: T::AccountId,
        evm_address: EvmAddress,
        signature: SignatureOf<T>,
    ) -> DispatchResult {
        // recover evm address from signature
        let address = T::ClaimSignature::verify_signature(&account_id, &signature)
            .ok_or(Error::<T>::BadSignature)?;
        ensure!(evm_address == address, Error::<T>::InvalidSignature);

        // Check if the default account id already exists for this eth address
        let default_account_id = T::DefaultAddressMapping::into_account_id(evm_address.clone());
        if frame_system::Pallet::<T>::account_exists(&default_account_id) {
            // Transfer all the free balance from old account id to the newly
            // since this `default_account_id` will no longer be connected to evm address
            // and users cannot access it.
            T::Currency::transfer(
                &default_account_id,
                &account_id,
                T::Currency::reducible_balance(&default_account_id, Expendable, Polite),
                Expendable,
            )?;
        }

        // create double mappings for the pair
        Self::add_mappings(account_id, evm_address);
        Ok(())
    }

    /// Claim the default evm address
    fn do_claim_default_address(account_id: T::AccountId) -> Result<EvmAddress, DispatchError> {
        // get the default evm address
        let address = T::DefaultAccountMapping::into_h160(account_id.clone());
        // create double mappings for the pair with default evm address
        Self::add_mappings(account_id, address.clone());
        Ok(address)
    }
}

impl<T: Config> Pallet<T> {
    #[cfg(any(feature = "std", feature = "runtime-benchmarks"))]
    pub fn eth_sign_prehash(prehash: &[u8; 32], secret: &libsecp256k1::SecretKey) -> [u8; 65] {
        let (sig, recovery_id) = libsecp256k1::sign(&libsecp256k1::Message::parse(prehash), secret);
        let mut r = [0u8; 65];
        r[0..64].copy_from_slice(&sig.serialize()[..]);
        r[64] = recovery_id.serialize();
        r
    }

    #[cfg(any(feature = "std", feature = "runtime-benchmarks"))]
    pub fn eth_address(secret: &libsecp256k1::SecretKey) -> EvmAddress {
        EvmAddress::from_slice(
            &sp_core::keccak_256(&Self::eth_public(secret).serialize()[1..65])[12..],
        )
    }

    #[cfg(any(feature = "std", feature = "runtime-benchmarks"))]
    // Returns an Ethereum public key derived from an Ethereum secret key.
    pub fn eth_public(secret: &libsecp256k1::SecretKey) -> libsecp256k1::PublicKey {
        libsecp256k1::PublicKey::from_secret_key(secret)
    }
}
