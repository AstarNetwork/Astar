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

//! Core Astar types.
//!
//! These core Astar types are used by the Shiden, Shibuya, Astar and Local runtime.
pub mod xcm;

/// Checked Ethereum transaction primitives.
pub mod ethereum_checked;

/// XVM primitives.
pub mod xvm;

use frame_support::ensure;
use sp_runtime::{
    generic,
    traits::{BlakeTwo256, IdentifyAccount, Verify},
};
use sp_std::marker::PhantomData;

use pallet_assets::AssetsCallback;
use pallet_evm_precompile_assets_erc20::AddressToAssetId;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = sp_runtime::MultiSignature;
/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Balance of an account.
pub type Balance = u128;
/// An index to a block.
pub type BlockNumber = u32;
/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;
/// Id used for identifying assets.
///
/// AssetId allocation:
/// [1; 2^32-1]     Custom user assets (permissionless)
/// [2^32; 2^64-1]  Statemine assets (simple map)
/// [2^64; 2^128-1] Ecosystem assets
/// 2^128-1         Relay chain token (KSM)
pub type AssetId = u128;
/// Index of a transaction in the chain.
pub type Index = u32;
/// Block type.
pub type Block = sp_runtime::generic::Block<Header, sp_runtime::OpaqueExtrinsic>;
/// Index of a transaction in the chain.
pub type Nonce = u32;
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
