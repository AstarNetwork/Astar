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
use frame_support::traits::IsType;
use core::marker::PhantomData;
use fp_evm::Precompile;
use fp_evm::{PrecompileHandle, PrecompileOutput};
use frame_support::dispatch::Dispatchable;
use precompile_utils::{
    succeed, Address, EvmDataWriter, EvmResult, FunctionModifier, PrecompileHandleExt,
};
use sp_core::{crypto::AccountId32, H256};
use sp_std::prelude::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    GetEvmAddressOrDefault = "get_evm_address_or_default(bytes32)",
    GetNativeAddressOrDefault = "get_native_address_or_default(address)",
}

/// A precompile that expose AU related functions.
pub struct UnifiedAccountsPrecompile<T, UA>(PhantomData<(T, UA)>);

impl<R, UA> Precompile for UnifiedAccountsPrecompile<R, UA>
where
    R: pallet_evm::Config + pallet_unified_accounts::Config,
    <<R as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<R::AccountId>>,
    <R as frame_system::Config>::AccountId: IsType<AccountId32>,
    UA: UnifiedAddressMapper<R::AccountId>,
{
    fn execute(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "au-precompile", "Execute input = {:?}", handle.input());

        let selector = handle.read_selector()?;

        handle.check_function_modifier(FunctionModifier::View)?;

        match selector {
            // Dispatchables
            Action::GetEvmAddressOrDefault => Self::get_evm_address_or_default(handle),
            Action::GetNativeAddressOrDefault => Self::get_native_address_or_default(handle),
        }
    }
}

impl<R, UA> UnifiedAccountsPrecompile<R, UA>
where
    R: pallet_evm::Config + pallet_unified_accounts::Config,
    <<R as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<R::AccountId>>,
    <R as frame_system::Config>::AccountId: IsType<AccountId32>,
    UA: UnifiedAddressMapper<R::AccountId>,
{
    fn get_evm_address_or_default(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;
        let account_id = AccountId32::new(input.read::<H256>()?.into()).into() ;
        log::trace!(target: "au-precompile", "get_evm_address_or_default account_id (Bytes) : {:?}",account_id);
        let res: (Address, bool) = match UA::to_h160_or_default(&account_id) {
            UnifiedAddress::Mapped(address) => (address.into(), true),
            UnifiedAddress::Default(address) => (address.into(), false),
        };
        log::trace!(target: "au-precompile", "accountId : {:?}, (Address,bool): {:?}",account_id, res);

        Ok(succeed(EvmDataWriter::new().write(res).build()))
    }

    fn get_native_address_or_default(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;
        let evm_address = input.read::<Address>()?;
        log::trace!(target: "au-precompile", "get_native_address_or_default (evmAddress) : {:?}",evm_address);

        let res: (H256, bool) = match UA::to_account_id_or_default(&evm_address.into()) {
            UnifiedAddress::Mapped(account_id) => {
                let converted_account_id: AccountId32 = account_id.into();
                log::trace!(target: "au-precompile", "get_native_address_or_default (Mapped : converted_account_id) : {:?}",converted_account_id);
                let mapped_account: &[u8; 32] = converted_account_id.as_ref();
                (mapped_account.into(), true)
            }
            UnifiedAddress::Default(account_id) => {
                let converted_account_id: AccountId32 = account_id.into();
                log::trace!(target: "au-precompile", "get_native_address_or_default (Default : converted_account_id) : {:?}",converted_account_id);
                let mapped_account: &[u8; 32] = converted_account_id.as_ref();
                (mapped_account.into(), false)
            }
        };
        Ok(succeed(EvmDataWriter::new().write(res).build()))
    }
}
