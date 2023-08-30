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
use astar_primitives::AccountId;
use frame_support::traits::OnKilledAccount;
use frame_support::{
    pallet_prelude::*,
    traits::{Currency, ExistenceRequirement::*, Get, IsType},
};
use frame_system::{ensure_signed, pallet_prelude::*};
use pallet_evm::AddressMapping;
use precompile_utils::keccak256;
use sp_core::{keccak_256, Hasher, H160, H256, U256};
use sp_runtime::traits::{LookupError, StaticLookup, Zero};
use sp_runtime::MultiAddress;
use sp_std::marker::PhantomData;

use pallet::*;

mod mock;
mod tests;

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

    /// Build raw payload (pre-hash) that user will sign.
    fn build_signing_payload(who: &Self::AccountId) -> Vec<u8>;
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
        type Currency: Currency<Self::AccountId>;
        /// Default evm address to account id conversion
        type DefaultAddressMapping: AddressMapping<Self::AccountId>;
        /// Default account id to evm address conversion
        type DefaultAccountMapping: AccountMapping<Self::AccountId>;
        /// The Signature verification implementation to use for checking claims
        /// Note: the signature type defined by this will be used as parameter in pallet's extrinsic
        type ClaimSignature: ClaimSignature<AccountId = Self::AccountId, Address = EvmAddress>;
        /// Chain ID of EVM
        /// TODO: remove this and make it generic parameter of EIP712Signature struct
        #[pallet::constant]
        type ChainId: Get<u64>;

        // TODO: benchmarks
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
                T::Currency::free_balance(&default_account_id),
                AllowDeath,
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

impl<T: Config> AddressManager<T::AccountId, EvmAddress> for Pallet<T> {
    fn to_account_id(address: &EvmAddress) -> Option<T::AccountId> {
        NativeAccounts::<T>::get(address)
    }

    fn to_account_id_or_default(address: &EvmAddress) -> T::AccountId {
        NativeAccounts::<T>::get(address).unwrap_or_else(|| {
            // fallback to default account_id
            T::DefaultAddressMapping::into_account_id(address.clone())
        })
    }

    fn to_default_account_id(address: &EvmAddress) -> T::AccountId {
        T::DefaultAddressMapping::into_account_id(address.clone())
    }

    fn to_address(account_id: &T::AccountId) -> Option<EvmAddress> {
        EvmAccounts::<T>::get(account_id)
    }

    fn to_address_or_default(account_id: &T::AccountId) -> EvmAddress {
        EvmAccounts::<T>::get(account_id).unwrap_or_else(|| {
            // fallback to default account_id
            T::DefaultAccountMapping::into_h160(account_id.clone())
        })
    }

    fn to_default_address(account_id: &T::AccountId) -> EvmAddress {
        T::DefaultAccountMapping::into_h160(account_id.clone())
    }
}

impl<T: Config> AccountMapping<T::AccountId> for Pallet<T> {
    fn into_h160(account: T::AccountId) -> H160 {
        <Self as AddressManager<T::AccountId, EvmAddress>>::to_address_or_default(&account)
    }
}
impl<T: Config> AddressMapping<T::AccountId> for Pallet<T> {
    fn into_account_id(address: H160) -> T::AccountId {
        <Self as AddressManager<T::AccountId, EvmAddress>>::to_account_id_or_default(&address)
    }
}

/// OnKilledAccout hooks implementation for removing storage mapping
/// for killed accounts
pub struct KillAccountMapping<T>(PhantomData<T>);
impl<T: Config> OnKilledAccount<T::AccountId> for KillAccountMapping<T> {
    fn on_killed_account(who: &T::AccountId) {
        // remove mapping created by `claim_account` or `get_or_create_evm_address`
        if let Some(evm_addr) = EvmAccounts::<T>::get(who) {
            NativeAccounts::<T>::remove(evm_addr);
            EvmAccounts::<T>::remove(who);
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
                <Self as AddressManager<T::AccountId, EvmAddress>>::to_account_id_or_default(
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

/// EIP-712 compatible signature scheme for verifying ownership of EVM Address
///
/// Raw Data = Domain Separator + Type Hash + keccak256(AccountId)
pub struct EIP712Signature<T: Config>(PhantomData<T>);
impl<T: Config> ClaimSignature for EIP712Signature<T> {
    type AccountId = T::AccountId;
    /// EVM address type
    type Address = EvmAddress;
    /// A signature (a 512-bit value, plus 8 bits for recovery ID).
    type Signature = [u8; 65];

    fn build_signing_payload(who: &Self::AccountId) -> Vec<u8> {
        let domain_separator = Self::build_domain_separator();
        let args_hash = Self::build_args_hash(who);

        let mut payload = b"\x19\x01".to_vec();
        payload.extend_from_slice(&domain_separator);
        payload.extend_from_slice(&args_hash);
        payload
    }

    fn verify_signature(who: &Self::AccountId, sig: &Self::Signature) -> Option<EvmAddress> {
        let payload = Self::build_signing_payload(who);
        let payload_hash = keccak_256(&payload);

        sp_io::crypto::secp256k1_ecdsa_recover(sig, &payload_hash)
            .map(|pubkey| H160::from(H256::from_slice(&keccak_256(&pubkey))))
            .ok()
    }
}

impl<T: Config> EIP712Signature<T> {
    /// TODO: use hardcoded bytes, configurable via generics
    fn build_domain_separator() -> [u8; 32] {
        let mut hash =
            keccak256!("EIP712Domain(string name,string version,uint256 chainId,bytes32 salt)")
                .to_vec();
        hash.extend_from_slice(&keccak256!("Astar EVM Claim")); // name
        hash.extend_from_slice(&keccak256!("1")); // version
        hash.extend_from_slice(&(<[u8; 32]>::from(U256::from(T::ChainId::get())))); // chain id
        hash.extend_from_slice(
            frame_system::Pallet::<T>::block_hash(T::BlockNumber::zero()).as_ref(),
        ); // genesis block hash
        keccak_256(hash.as_slice())
    }

    fn build_args_hash(account: &T::AccountId) -> [u8; 32] {
        let mut args_hash = keccak256!("Claim(bytes substrateAddress)").to_vec();
        args_hash.extend_from_slice(&keccak_256(&account.encode()));
        keccak_256(args_hash.as_slice())
    }
}

/// Hashed derive mapping for converting account id to evm address
pub struct HashedAccountMapping<H>(sp_std::marker::PhantomData<H>);
impl<H: Hasher<Out = H256>> AccountMapping<AccountId> for HashedAccountMapping<H> {
    fn into_h160(account: AccountId) -> H160 {
        let payload = (b"evm:", account);
        H160::from_slice(&payload.using_encoded(H::hash)[0..20])
    }
}
