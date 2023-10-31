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

use crate::{AccountId, AssetId};

use frame_support::ensure;
use pallet_evm::{AddressMapping, HashedAddressMapping};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{Hasher, H160, H256};
use sp_std::marker::PhantomData;

use pallet_assets::AssetsCallback;
use pallet_evm_precompile_assets_erc20::AddressToAssetId;

pub type EvmAddress = H160;

/// Revert opt code. It's inserted at the precompile addresses, to make them functional in EVM.
pub const EVM_REVERT_CODE: &[u8] = &[0x60, 0x00, 0x60, 0x00, 0xfd];

/// Handler for automatic revert code registration.
///
/// When an asset is created, it automatically becomes available to the EVM via an `ERC20-like` interface.
/// In order for the precompile to work, dedicated asset address needs to have the revert code registered, otherwise the call will fail.
///
/// It is important to note that if the dedicated asset EVM address is already taken, asset creation should fail.
/// After asset has been destroyed, it is also safe to remove the revert code and free the address for future usage.
pub struct EvmRevertCodeHandler<A, R>(PhantomData<(A, R)>);
impl<A, R> AssetsCallback<AssetId, AccountId> for EvmRevertCodeHandler<A, R>
where
    A: AddressToAssetId<AssetId>,
    R: pallet_evm::Config,
{
    fn created(id: &AssetId, _: &AccountId) -> Result<(), ()> {
        let address = A::asset_id_to_address(*id);
        // In case of collision, we need to cancel the asset creation.
        ensure!(!pallet_evm::AccountCodes::<R>::contains_key(&address), ());
        pallet_evm::AccountCodes::<R>::insert(address, EVM_REVERT_CODE.to_vec());
        Ok(())
    }

    fn destroyed(id: &AssetId) -> Result<(), ()> {
        let address = A::asset_id_to_address(*id);
        pallet_evm::AccountCodes::<R>::remove(address);
        Ok(())
    }
}

/// Mapping between Native and EVM Addresses
pub trait UnifiedAddressMapper<AccountId> {
    /// Gets the account id associated with given evm address, if mapped else None.
    fn to_account_id(evm_address: &EvmAddress) -> Option<AccountId>;

    /// Gets the account id associated with given evm address.
    /// If no mapping exists, then return the default evm address.
    /// Returns `UnifiedAddress` enum which wraps the inner account id
    fn to_account_id_or_default(evm_address: &EvmAddress) -> UnifiedAddress<AccountId> {
        Self::to_account_id(evm_address).map_or_else(
            // fallback to default account_id
            || UnifiedAddress::Default(Self::to_default_account_id(evm_address)),
            |a| UnifiedAddress::Mapped(a),
        )
    }
    /// Gets the default account id which is associated with given evm address.
    fn to_default_account_id(evm_address: &EvmAddress) -> AccountId;

    /// Gets the evm address associated with given account id, if mapped else None.
    fn to_h160(account_id: &AccountId) -> Option<EvmAddress>;

    /// Gets the evm address associated with given account id.
    /// If no mapping exists, then return the default account id.
    /// Returns `UnifiedAddress` enum which wraps the inner evm address
    fn to_h160_or_default(account_id: &AccountId) -> UnifiedAddress<H160> {
        Self::to_h160(account_id).map_or_else(
            // fallback to default account_id
            || UnifiedAddress::Default(Self::to_default_h160(account_id)),
            |a| UnifiedAddress::Mapped(a),
        )
    }

    /// Gets the default evm address which is associated with given account id.
    fn to_default_h160(account_id: &AccountId) -> EvmAddress;
}

/// Mappings derieved from hashing the original address
pub struct HashedDefaultMappings<H>(PhantomData<H>);
impl<H: Hasher<Out = H256>> UnifiedAddressMapper<AccountId> for HashedDefaultMappings<H> {
    fn to_default_account_id(evm_address: &EvmAddress) -> AccountId {
        HashedAddressMapping::<H>::into_account_id(evm_address.clone())
    }

    fn to_default_h160(account_id: &AccountId) -> EvmAddress {
        let payload = (b"evm:", account_id);
        H160::from_slice(&payload.using_encoded(H::hash)[0..20])
    }

    fn to_account_id(_: &EvmAddress) -> Option<AccountId> {
        None
    }

    fn to_h160(_: &AccountId) -> Option<EvmAddress> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum UnifiedAddress<Address> {
    /// The address fetched from the mappings and the account
    /// is unified
    #[codec(index = 0)]
    Mapped(Address),
    /// The default address associated with account as there
    /// is no mapping found and accounts are not unified
    #[codec(index = 1)]
    Default(Address),
}

impl<Address> UnifiedAddress<Address> {
    /// Get the underlying address
    pub fn into_address(self) -> Address {
        match self {
            Self::Default(a) => a,
            Self::Mapped(a) => a,
        }
    }
}
