// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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

use fp_evm::AccountProvider;
use frame_support::{
    ensure,
    traits::{
        fungible::{Balanced, Credit},
        tokens::{fungible::Inspect, imbalance::OnUnbalanced},
    },
};
use pallet_evm::{AddressMapping, HashedAddressMapping, OnChargeEVMTransaction};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
pub use sp_core::{Hasher, H160, H256, U256};
use sp_runtime::traits::UniqueSaturatedInto;
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
        HashedAddressMapping::<H>::into_account_id(*evm_address)
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

/// Wrapper around the `EvmFungibleAdapter` from the `pallet-evm`.
///
/// While it provides most of the functionality we need,
/// it doesn't allow the tip to be deposited into an arbitrary account.
/// This adapter allows us to do that.
///
/// Two separate `OnUnbalanced` handers are used:
/// - `UOF` for the fee
/// - `OUT` for the tip
pub struct EVMFungibleAdapterWrapper<F, FeeHandler, TipHandler>(
    core::marker::PhantomData<(F, FeeHandler, TipHandler)>,
);
impl<T, F, FeeHandler, TipHandler> OnChargeEVMTransaction<T>
    for EVMFungibleAdapterWrapper<F, FeeHandler, TipHandler>
where
    T: pallet_evm::Config,
    F: Balanced<<T::AccountProvider as AccountProvider>::AccountId>,
    FeeHandler: OnUnbalanced<Credit<<T::AccountProvider as AccountProvider>::AccountId, F>>,
    TipHandler: OnUnbalanced<Credit<<T::AccountProvider as AccountProvider>::AccountId, F>>,
    U256: UniqueSaturatedInto<
        <F as Inspect<<T::AccountProvider as AccountProvider>::AccountId>>::Balance,
    >,
{
    // Kept type as Option to satisfy bound of Default
    type LiquidityInfo = Option<Credit<<T::AccountProvider as AccountProvider>::AccountId, F>>;

    fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo, pallet_evm::Error<T>> {
        pallet_evm::EVMFungibleAdapter::<F, FeeHandler>::withdraw_fee(who, fee)
    }

    fn can_withdraw(who: &H160, amount: U256) -> Result<(), pallet_evm::Error<T>> {
        pallet_evm::EVMFungibleAdapter::<F, FeeHandler>::can_withdraw(who, amount)
    }

    fn correct_and_deposit_fee(
        who: &H160,
        corrected_fee: U256,
        base_fee: U256,
        already_withdrawn: Self::LiquidityInfo,
    ) -> Self::LiquidityInfo {
        <pallet_evm::EVMFungibleAdapter::<F, FeeHandler> as OnChargeEVMTransaction<T>>::correct_and_deposit_fee(
            who,
            corrected_fee,
            base_fee,
            already_withdrawn,
        )
    }

    fn pay_priority_fee(tip: Self::LiquidityInfo) {
        if let Some(tip) = tip {
            TipHandler::on_unbalanceds(Some(tip).into_iter());
        }
    }
}
