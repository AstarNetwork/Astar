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

use astar_primitives::evm::{EvmAddress, UnifiedAddressMapper};
use core::marker::PhantomData;
use sp_runtime::DispatchError;

use frame_support::DefaultNoBound;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, Result as DispatchResult, RetVal,
};
use pallet_unified_accounts::WeightInfo;
use parity_scale_codec::Encode;
pub use unified_accounts_chain_extension_types::Command::{self, *};

type UAWeight<T> = <T as pallet_unified_accounts::Config>::WeightInfo;

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
                // charge weight
                env.charge_weight(UAWeight::<T>::to_h160())?;
                // write to buffer
                UA::to_h160(&account_id).using_encoded(|r| env.write(r, false, None))?;
            }
            GetEvmAddressOrDefault => {
                let account_id: T::AccountId = env.read_as()?;
                // charge weight
                env.charge_weight(UAWeight::<T>::to_h160_or_default())?;

                // write to buffer
                UA::to_h160_or_default(&account_id).using_encoded(|r| env.write(r, false, None))?;
            }
            GetNativeAddress => {
                let evm_address: EvmAddress = env.read_as()?;
                // charge weight
                env.charge_weight(UAWeight::<T>::to_account_id())?;
                // write to buffer
                UA::to_account_id(&evm_address).using_encoded(|r| env.write(r, false, None))?;
            }
            GetNativeAddressOrDefault => {
                let evm_address: EvmAddress = env.read_as()?;
                // charge weight
                env.charge_weight(UAWeight::<T>::to_account_id_or_default())?;

                // write to buffer
                UA::to_account_id_or_default(&evm_address)
                    .using_encoded(|r| env.write(r, false, None))?;
            }
        };
        Ok(RetVal::Converging(0))
    }
}
