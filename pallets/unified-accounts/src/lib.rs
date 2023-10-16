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

//! # Pallet Unified Account
//!
//! A simple module for managing mappings (both ways) between different
//! address schemes, inspired from Acala's evm-accounts pallet
//! https://github.com/AcalaNetwork/Acala/tree/master/modules/evm-accounts
//!
//! - [`Config`]
//! - [`Call`]
//!
//! ## Overview
//!
//! The Unified Accounts module provide functionality for native account holders to
//! connect their evm address to have a unified experence across the different VMs.
//! - Connect evm address you control
//! - Connect default evm address
//!
//! ## Interface
//!
//! * `claim_evm_address`: Creates the double mappings for the provided evm address with caller
//!    account id given that no prior mapping exists for both and signature provided is valid.
//! * `claim_default_evm_address`: Creates the double mapping with default evm address given that
//!    no prior mapping exists.
//!
//! WARNINGS:
//! * This pallet only handles transfer of native balance only, for the rest of native assets
//!   hold by evm address like XC20, DAppStaking unclaimed rewards, etc should be transferred
//!   manually beforehand by user himself otherwise FUNDS WILL BE LOST FOREVER.
//! * Once mapping is created it cannot be changed.
//!
//! ## Traits
//!
//! * `UnifiedAddressMapper`: Interface to access pallet's mappings with defaults
//!
//! ## Implementations
//!
//! * [`StaticLookup`](sp_runtime::traits::StaticLookup): Lookup implementations for accepting H160
//! * [`AddressMapping`](pallet_evm::AddressMapping): Wrapper over `UnifiedAddressMapper` for evm address mapping
//!   to account id.
//! * [`AccountMapping`](astar_primitives::ethereum_checked::AccountMapping): Wrapper over `UnifiedAddressMapper`
//!   for account id mappings to h160.
//! * `KillAccountMapping`: [`OnKilledAccount`](frame_support::traits::OnKilledAccount) implementation to remove
//!   the mappings from storage after account is reaped.

#![cfg_attr(not(feature = "std"), no_std)]

use astar_primitives::{
    ethereum_checked::AccountMapping,
    evm::{EvmAddress, UnifiedAddressMapper},
};
use frame_support::{
    pallet_prelude::*,
    traits::{
        fungible::{Inspect, Mutate},
        tokens::{Fortitude::*, Preservation::*},
        IsType, OnKilledAccount,
    },
};
use frame_system::{ensure_signed, pallet_prelude::*};
use pallet_evm::AddressMapping;
use precompile_utils::keccak256;
use sp_core::{H160, H256, U256};
use sp_io::hashing::keccak_256;
use sp_runtime::{
    traits::{LookupError, StaticLookup, Zero},
    MultiAddress,
};
use sp_std::marker::PhantomData;

pub use pallet::*;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod mock;
mod tests;

/// ECDSA Signature type, with last bit for recovering address
type EvmSignature = [u8; 65];

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// The Currency for managing evm address assets
        type Currency: Mutate<Self::AccountId>;
        /// Default evm address to account id conversion
        type DefaultEvmToNative: AddressMapping<Self::AccountId>;
        /// Default account id to evm address conversion
        type DefaultNativeToEvm: AccountMapping<Self::AccountId>;
        /// EVM chain id
        type ChainId: Get<u64>;
        /// Weight information for the extrinsics in this module
        type WeightInfo: WeightInfo;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// AccountId or EvmAddress already mapped
        AlreadyMapped,
        /// The signature is malformed
        UnexpectedSignatureFormat,
        /// The signature verification failed due to mismatch evm address
        InvalidSignature,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Evm Address claimed.
        /// Double Mapping b/w native and evm address created
        AccountClaimed {
            account_id: T::AccountId,
            evm_address: EvmAddress,
        },
    }

    /// Native accounts for evm address
    /// EvmToNative: EvmAddress => Option<AccountId>
    #[pallet::storage]
    pub type EvmToNative<T: Config> =
        StorageMap<_, Blake2_128Concat, EvmAddress, T::AccountId, OptionQuery>;

    /// Evm addresses for native accounts
    /// NativeToEvm: AccountId => Option<EvmAddress>
    #[pallet::storage]
    pub type NativeToEvm<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, EvmAddress, OptionQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Claim account mapping between Substrate account and Evm address.
        /// Ensure no prior mapping exists for evm address.
        ///
        /// - `evm_address`: The evm address to bind to the caller's account
        /// - `signature`: A signature generated by the address to prove ownership
        ///
        /// WARNING:
        /// - This extrisic only handles transfer of native balance, if your EVM
        /// address contains any other native assets like XC20, DAppStaking unclaimed rewards,
        /// etc you need to transfer them before hand, otherwise FUNDS WILL BE LOST FOREVER.
        /// - Once connected user cannot change their mapping EVER.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::claim_evm_address())]
        pub fn claim_evm_address(
            origin: OriginFor<T>,
            evm_address: EvmAddress,
            signature: EvmSignature,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // make sure no prior mapping exists
            ensure!(
                !NativeToEvm::<T>::contains_key(&who),
                Error::<T>::AlreadyMapped
            );
            ensure!(
                !EvmToNative::<T>::contains_key(evm_address),
                Error::<T>::AlreadyMapped
            );

            // recover evm address from signature
            let address = Self::verify_signature(&who, &signature)
                .ok_or(Error::<T>::UnexpectedSignatureFormat)?;

            ensure!(evm_address == address, Error::<T>::InvalidSignature);

            // Check if the default account id already exists for this evm address
            let default_account_id = T::DefaultEvmToNative::into_account_id(evm_address.clone());
            if frame_system::Pallet::<T>::account_exists(&default_account_id) {
                // Transfer all the free native balance from old account id to the newly
                // since this `default_account_id` will no longer be connected to evm address
                // and users cannot access it.
                // For the reset of the assets types (like XC20, etc) that should be handled by UI.
                T::Currency::transfer(
                    &default_account_id,
                    &who,
                    T::Currency::reducible_balance(&default_account_id, Expendable, Polite),
                    Expendable,
                )?;
            }

            // create double mappings for the pair
            EvmToNative::<T>::insert(&evm_address, &who);
            NativeToEvm::<T>::insert(&who, &evm_address);

            Self::deposit_event(Event::AccountClaimed {
                account_id: who,
                evm_address,
            });
            Ok(())
        }

        /// Claim default evm address for given account id
        /// Ensure no prior mapping exists for the account
        ///
        /// WARNINGS: Once connected user cannot change their mapping EVER.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::claim_default_evm_address())]
        pub fn claim_default_evm_address(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // claim default evm address
            let _ = Self::do_claim_default_evm_address(who)?;
            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Claim the default evm address
    fn do_claim_default_evm_address(account_id: T::AccountId) -> Result<EvmAddress, DispatchError> {
        ensure!(
            !NativeToEvm::<T>::contains_key(&account_id),
            Error::<T>::AlreadyMapped
        );
        // get the default evm address
        let evm_address = T::DefaultNativeToEvm::into_h160(account_id.clone());
        // make sure default address is not already mapped, this should not
        // happen but for sanity check.
        ensure!(
            !EvmToNative::<T>::contains_key(&evm_address),
            Error::<T>::AlreadyMapped
        );
        // create double mappings for the pair with default evm address
        EvmToNative::<T>::insert(&evm_address, &account_id);
        NativeToEvm::<T>::insert(&account_id, &evm_address);

        Self::deposit_event(Event::AccountClaimed {
            account_id,
            evm_address,
        });
        Ok(evm_address)
    }
}

/// EIP-712 compatible signature scheme for verifying ownership of EVM Address
/// https://eips.ethereum.org/EIPS/eip-712
///
/// Raw Data = Domain Separator + Type Hash + keccak256(AccountId)
impl<T: Config> Pallet<T> {
    pub fn build_signing_payload(who: &T::AccountId) -> [u8; 32] {
        let domain_separator = Self::build_domain_separator();
        let args_hash = Self::build_args_hash(who);

        let mut payload = b"\x19\x01".to_vec();
        payload.extend_from_slice(&domain_separator);
        payload.extend_from_slice(&args_hash);
        keccak_256(&payload)
    }

    pub fn verify_signature(who: &T::AccountId, sig: &EvmSignature) -> Option<EvmAddress> {
        let payload_hash = Self::build_signing_payload(who);

        sp_io::crypto::secp256k1_ecdsa_recover(sig, &payload_hash)
            .map(|pubkey| H160::from(H256::from_slice(&keccak_256(&pubkey))))
            .ok()
    }

    fn build_domain_separator() -> [u8; 32] {
        let mut domain =
            keccak256!("EIP712Domain(string name,string version,uint256 chainId,bytes32 salt)")
                .to_vec();
        domain.extend_from_slice(&keccak256!("Astar EVM Claim")); // name
        domain.extend_from_slice(&keccak256!("1")); // version
        domain.extend_from_slice(&(<[u8; 32]>::from(U256::from(T::ChainId::get())))); // chain id
        domain.extend_from_slice(
            frame_system::Pallet::<T>::block_hash(T::BlockNumber::zero()).as_ref(),
        ); // genesis block hash
        keccak_256(domain.as_slice())
    }

    fn build_args_hash(account: &T::AccountId) -> [u8; 32] {
        let mut args_hash = keccak256!("Claim(bytes substrateAddress)").to_vec();
        args_hash.extend_from_slice(&keccak_256(&account.encode()));
        keccak_256(args_hash.as_slice())
    }
}

#[cfg(any(feature = "std", feature = "runtime-benchmarks"))]
impl<T: Config> Pallet<T> {
    /// Sign the given prehash with provided eth private key
    pub fn eth_sign_prehash(prehash: &[u8; 32], secret: &libsecp256k1::SecretKey) -> [u8; 65] {
        let (sig, recovery_id) = libsecp256k1::sign(&libsecp256k1::Message::parse(prehash), secret);
        let mut r = [0u8; 65];
        r[0..64].copy_from_slice(&sig.serialize()[..]);
        r[64] = recovery_id.serialize();
        r
    }

    /// Get the eth address for given eth private key
    pub fn eth_address(secret: &libsecp256k1::SecretKey) -> EvmAddress {
        EvmAddress::from_slice(
            &sp_io::hashing::keccak_256(
                &libsecp256k1::PublicKey::from_secret_key(secret).serialize()[1..65],
            )[12..],
        )
    }
}

/// UnifiedAddressMapper implementation using pallet's mapping
/// and default address scheme from pallet's config
impl<T: Config> UnifiedAddressMapper<T::AccountId> for Pallet<T> {
    fn to_account_id(evm_address: &EvmAddress) -> Option<T::AccountId> {
        EvmToNative::<T>::get(evm_address)
    }

    fn to_account_id_or_default(evm_address: &EvmAddress) -> T::AccountId {
        EvmToNative::<T>::get(evm_address).unwrap_or_else(|| {
            // fallback to default account_id
            T::DefaultEvmToNative::into_account_id(evm_address.clone())
        })
    }

    fn to_default_account_id(evm_address: &EvmAddress) -> T::AccountId {
        T::DefaultEvmToNative::into_account_id(evm_address.clone())
    }

    fn to_h160(account_id: &T::AccountId) -> Option<EvmAddress> {
        NativeToEvm::<T>::get(account_id)
    }

    fn to_h160_or_default(account_id: &T::AccountId) -> EvmAddress {
        NativeToEvm::<T>::get(account_id).unwrap_or_else(|| {
            // fallback to default account_id
            T::DefaultNativeToEvm::into_h160(account_id.clone())
        })
    }

    fn to_default_h160(account_id: &T::AccountId) -> EvmAddress {
        T::DefaultNativeToEvm::into_h160(account_id.clone())
    }
}

/// AccountMapping wrapper implementation
impl<T: Config> AccountMapping<T::AccountId> for Pallet<T> {
    fn into_h160(account: T::AccountId) -> H160 {
        <Self as UnifiedAddressMapper<T::AccountId>>::to_h160_or_default(&account)
    }
}

/// AddressMapping wrapper implementation
impl<T: Config> AddressMapping<T::AccountId> for Pallet<T> {
    fn into_account_id(evm_address: H160) -> T::AccountId {
        <Self as UnifiedAddressMapper<T::AccountId>>::to_account_id_or_default(&evm_address)
    }
}

/// OnKilledAccout hooks implementation for removing storage mapping
/// for killed accounts
pub struct KillAccountMapping<T>(PhantomData<T>);
impl<T: Config> OnKilledAccount<T::AccountId> for KillAccountMapping<T> {
    fn on_killed_account(who: &T::AccountId) {
        // remove mapping created by `claim_account` or `get_or_create_evm_address`
        if let Some(evm_addr) = NativeToEvm::<T>::take(who) {
            EvmToNative::<T>::remove(evm_addr);
            NativeToEvm::<T>::remove(who);
        }
    }
}

/// A lookup implementation returning the `AccountId` from `MultiAddress::Address20` (EVM Address).
impl<T: Config> StaticLookup for Pallet<T> {
    type Source = MultiAddress<T::AccountId, ()>;
    type Target = T::AccountId;

    fn lookup(a: Self::Source) -> Result<Self::Target, LookupError> {
        match a {
            MultiAddress::Address20(i) => Ok(
                <Self as UnifiedAddressMapper<T::AccountId>>::to_account_id_or_default(
                    &EvmAddress::from_slice(&i),
                ),
            ),
            _ => Err(LookupError),
        }
    }

    fn unlookup(a: Self::Target) -> Self::Source {
        MultiAddress::Id(a)
    }
}
