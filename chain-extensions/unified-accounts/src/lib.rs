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

use astar_primitives::{
    ethereum_checked::AccountMapping,
    evm::{EvmAddress, UnifiedAddressMapper},
};
use core::marker::PhantomData;
use sp_runtime::DispatchError;

use frame_support::{traits::Get, DefaultNoBound};
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, Result as DispatchResult, RetVal,
};
use pallet_evm::AddressMapping;
use parity_scale_codec::Encode;
pub use unified_accounts_chain_extension_types::{
    Command::{self, *},
    UnifiedAddress,
};

#[derive(DefaultNoBound)]
pub struct UnifiedAccountsExtension<T, UA>(PhantomData<(T, UA)>);

impl<T, UA> ChainExtension<T> for UnifiedAccountsExtension<T, UA>
where
    T: pallet_contracts::Config + pallet_unified_accounts::Config,
    UA: UnifiedAddressMapper<T::AccountId>,
{
    fn call<E>(&mut self, env: Environment<E, InitState>) -> DispatchResult<RetVal>
    where
        E: Ext<T = T>,
    {
        let mut env = env.buf_in_buf_out();
        match env.func_id().try_into().map_err(|_| {
            DispatchError::Other("Unsupported func id in Unified Accounts Chain Extension")
        })? {
            GetEvmAddress => {
                let account_id: T::AccountId = env.read_as()?;

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;
                // write to buffer
                UA::to_h160(&account_id).using_encoded(|r| env.write(r, false, None))?;
            }
            GetEvmAddressOrDefault => {
                let account_id: T::AccountId = env.read_as()?;

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                let evm_address = if let Some(h160) = UA::to_h160(&account_id) {
                    UnifiedAddress::Mapped(h160)
                } else {
                    UnifiedAddress::Default(T::DefaultNativeToEvm::into_h160(account_id))
                };
                // write to buffer
                evm_address.using_encoded(|r| env.write(r, false, None))?;
            }
            GetNativeAddress => {
                let evm_address: EvmAddress = env.read_as()?;

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;
                // write to buffer
                UA::to_account_id(&evm_address).using_encoded(|r| env.write(r, false, None))?;
            }
            GetNativeAddressOrDefault => {
                let evm_address: EvmAddress = env.read_as()?;

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                // read the storage item
                let native_address = if let Some(native) = UA::to_account_id(&evm_address) {
                    UnifiedAddress::Mapped(native)
                } else {
                    UnifiedAddress::Default(T::DefaultEvmToNative::into_account_id(evm_address))
                };

                // write to buffer
                native_address.using_encoded(|r| env.write(r, false, None))?;
            }
        };
        Ok(RetVal::Converging(0))
    }
}
