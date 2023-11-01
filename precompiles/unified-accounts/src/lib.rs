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

use astar_primitives::evm::{UnifiedAddress, UnifiedAddressMapper};
use core::marker::PhantomData;
use fp_evm::{ExitError, Precompile, PrecompileFailure};
use fp_evm::{PrecompileHandle, PrecompileOutput};
use frame_support::dispatch::Dispatchable;
use frame_system::Account;
use pallet_evm::ExitFatal;
use precompile_utils::{
    revert, succeed, Address, Bytes, EvmDataWriter, EvmResult, FunctionModifier,
    PrecompileHandleExt,
};
use sp_core::{crypto::AccountId32, H160, H256};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Dummy H160 address representing no mapping
const DEFAULT_ADDRESS: H160 = H160::zero();

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    GetEvmAddress = "get_evm_address(bytes)",
    GetEvmAddressOrDefault = "get_evm_address_or_default(bytes)",
    GetNativeAddress = "get_native_address(address)",
    GetNativeAddressOrDefault = "get_native_address_or_default(address)",
}

/// A precompile that expose AU related functions.
pub struct UnifiedAccountsPrecompile<T, UA>(PhantomData<(T, UA)>);

impl<R, UA> Precompile for UnifiedAccountsPrecompile<R, UA>
where
    R: pallet_evm::Config + pallet_unified_accounts::Config,
    <<R as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<R::AccountId>>,
    <R as frame_system::Config>::AccountId: From<AccountId32>,
    UA: UnifiedAddressMapper<R::AccountId>,
{
    fn execute(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "au-precompile", "Execute input = {:?}", handle.input());

        let selector = handle.read_selector()?;

        handle.check_function_modifier(FunctionModifier::View)?;

        match selector {
            // Dispatchables
            Action::GetEvmAddress => Self::get_evm_address(handle),
            Action::GetEvmAddressOrDefault => Self::get_evm_address_or_default(handle),
            Action::GetNativeAddress => Self::get_native_address(handle),
            Action::GetNativeAddressOrDefault => Self::get_native_address_or_default(handle),
        }
    }
}

impl<R, UA> UnifiedAccountsPrecompile<R, UA>
where
    R: pallet_evm::Config + pallet_unified_accounts::Config,
    <<R as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<R::AccountId>>,
    <R as frame_system::Config>::AccountId: From<AccountId32>,
    <R as frame_system::Config>::AccountId: Into<AccountId32>,
    UA: UnifiedAddressMapper<R::AccountId>,
{
    fn get_evm_address(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;
        let account_id = input.read::<H256>()?;
        log::trace!(target: "au-precompile", "get_evm_address account_id (H256) : {:?}",account_id);
        let res: (Address, bool) = {
            if let Some(address) = UA::to_h160(&AccountId32::new(account_id.0).into()) {
                (address.into(), true)
            } else {
                (DEFAULT_ADDRESS.into(), false)
            }
        };
        log::trace!(target: "au-precompile", "get_evm_address accountId : {:?}, (Address,bool): {:?}",account_id, res);

        Ok(succeed(EvmDataWriter::new().write(res).build()))
    }

    fn get_evm_address_or_default(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;
        let account_as_bytes: Vec<u8> = input.read::<Bytes>()?.into();
        log::trace!(target: "au-precompile", "get_evm_address_or_default account_id (Bytes) : {:?}",account_as_bytes);
        let account_id: R::AccountId = match account_as_bytes.len() {
            // public address of the ss58 account has 32 bytes
            32 => {
                let mut account_bytes = [0_u8; 32];
                account_bytes[..].clone_from_slice(&account_as_bytes[0..32]);

                AccountId32::new(account_bytes).into()
            }
            _ => {
                // Return err if account length is wrong
                return Err(revert("Error while parsing staker's address"));
            }
        };
        let res: (Address, bool) = match UA::to_h160_or_default(&account_id) {
            UnifiedAddress::Mapped(address) => (address.into(), true),
            UnifiedAddress::Default(address) => (address.into(), false),
        };
        log::trace!(target: "au-precompile", "accountId : {:?}, (Address,bool): {:?}",account_id, res);

        Ok(succeed(EvmDataWriter::new().write(res).build()))
    }

    fn get_native_address(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;
        let evm_address = input.read::<Address>()?;
        log::trace!(target: "au-precompile", "get_native_address (evmAddress) : {:?}",evm_address);

        let res: (Bytes, bool) = {
            if let Some(account_id) = UA::to_account_id(&evm_address.into()) {
                let converted_account_id: AccountId32 = account_id.into();
                log::trace!(target: "au-precompile", "get_native_address (converted_account_id) : {:?}",converted_account_id);
                (converted_account_id.into(), true)
            } else {
                ([0; 32].as_ref().into(), false)
            }
        };
        Ok(succeed(EvmDataWriter::new().write(res).build()))
    }

    fn get_native_address_or_default(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;
        let evm_address = input.read::<Address>()?;
        log::trace!(target: "au-precompile", "get_native_address_or_default (evmAddress) : {:?}",evm_address);

        let res: (Bytes, bool) = match UA::to_account_id_or_default(&evm_address.into()) {
            UnifiedAddress::Mapped(account_id) => {
                let converted_account_id: AccountId32 = account_id.into();
                log::trace!(target: "au-precompile", "get_native_address_or_default (converted_account_id) : {:?}",converted_account_id);
                (converted_account_id.into(), true)
            }
            UnifiedAddress::Default(account_id) => {
                let converted_account_id: AccountId32 = account_id.into();
                log::trace!(target: "au-precompile", "get_native_address_or_default (converted_account_id) : {:?}",converted_account_id);
                (converted_account_id.into(), false)
            }
        };
        Ok(succeed(EvmDataWriter::new().write(res).build()))
    }
}
