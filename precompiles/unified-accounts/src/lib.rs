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
use fp_evm::PrecompileHandle;
use frame_support::dispatch::Dispatchable;
use frame_support::traits::IsType;
use precompile_utils::prelude::*;
use sp_core::{crypto::AccountId32, H256};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// A precompile that expose AU related functions.
pub struct UnifiedAccountsPrecompile<T, UA>(PhantomData<(T, UA)>);

#[precompile_utils::precompile]
impl<R, UA> UnifiedAccountsPrecompile<R, UA>
where
    R: pallet_evm::Config + pallet_unified_accounts::Config,
    <<R as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<R::AccountId>>,
    <R as frame_system::Config>::AccountId: IsType<AccountId32>,
    UA: UnifiedAddressMapper<R::AccountId>,
{
    #[precompile::public("get_evm_address_or_default(bytes32)")]
    #[precompile::view]
    fn get_evm_address_or_default(
        _: &mut impl PrecompileHandle,
        account_id: H256,
    ) -> EvmResult<(Address, bool)> {
        let account_id = AccountId32::new(account_id.into()).into();

        let output: (Address, bool) = match UA::to_h160_or_default(&account_id) {
            UnifiedAddress::Mapped(address) => (address.into(), true),
            UnifiedAddress::Default(address) => (address.into(), false),
        };
        Ok(output)
    }

    #[precompile::public("get_native_address_or_default(address)")]
    #[precompile::view]
    fn get_native_address_or_default(
        _: &mut impl PrecompileHandle,
        evm_address: Address,
    ) -> EvmResult<(H256, bool)> {
        let output: (H256, bool) = match UA::to_account_id_or_default(&evm_address.into()) {
            UnifiedAddress::Mapped(account_id) => (H256::from(account_id.into().as_ref()), true),
            UnifiedAddress::Default(account_id) => (H256::from(account_id.into().as_ref()), false),
        };
        Ok(output)
    }
}
