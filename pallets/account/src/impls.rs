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

#![cfg_attr(not(feature = "std"), no_std)]

use astar_primitives::ethereum_checked::AccountMapping;
use astar_primitives::evm::EvmAddress;
use astar_primitives::AccountId;
use frame_support::traits::OnKilledAccount;
use frame_support::{pallet_prelude::*, traits::Get};
use pallet_evm::AddressMapping;
use precompile_utils::keccak256;
use sp_core::{Hasher, H160, H256, U256};
use sp_io::hashing::keccak_256;
use sp_runtime::traits::{LookupError, StaticLookup, Zero};
use sp_runtime::MultiAddress;
use sp_std::marker::PhantomData;

use crate::*;

/// AddressManager implementation
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

/// AccountMapping wrapper implementation over AddressManager
impl<T: Config> AccountMapping<T::AccountId> for Pallet<T> {
    fn into_h160(account: T::AccountId) -> H160 {
        <Self as AddressManager<T::AccountId, EvmAddress>>::to_address_or_default(&account)
    }
}

/// AddresstMapping wrapper implementation over AddressManager
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

    fn build_signing_payload(who: &Self::AccountId) -> [u8; 32] {
        let domain_separator = Self::build_domain_separator();
        let args_hash = Self::build_args_hash(who);

        let mut payload = b"\x19\x01".to_vec();
        payload.extend_from_slice(&domain_separator);
        payload.extend_from_slice(&args_hash);
        keccak_256(&payload)
    }

    fn verify_signature(who: &Self::AccountId, sig: &Self::Signature) -> Option<EvmAddress> {
        let payload_hash = Self::build_signing_payload(who);

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
