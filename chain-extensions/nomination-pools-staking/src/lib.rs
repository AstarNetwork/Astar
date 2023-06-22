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
use sp_runtime::DispatchError;

use frame_support::traits::Currency;
use frame_system::RawOrigin;
use nomination_pools_staking_chain_extension_types::{NPSError, NominationPoolStakingValueInput};
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, SysConfig,
};
use pallet_nomination_pools_staking::WeightInfo;
use sp_std::marker::PhantomData;

type BalanceOf<T> = <<T as pallet_nomination_pools_staking::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

enum NominationPoolStakingFunc {
    BondAndStake,
}

impl TryFrom<u16> for NominationPoolStakingFunc {
    type Error = DispatchError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(NominationPoolStakingFunc::BondAndStake),
            _ => Err(DispatchError::Other(
                "NominationPoolsStakingExtension: Unimplemented func_id",
            )),
        }
    }
}

/// Nomination pool Staking chain extension.
pub struct NominationPoolsStakingExtension<T>(PhantomData<T>);

impl<T> Default for NominationPoolsStakingExtension<T> {
    fn default() -> Self {
        NominationPoolsStakingExtension(PhantomData)
    }
}

impl<T> ChainExtension<T> for NominationPoolsStakingExtension<T>
where
    T: pallet_nomination_pools_staking::Config + pallet_contracts::Config,
    <T as pallet_nomination_pools_staking::Config>::SmartContract: From<[u8; 32]>,
    <T as SysConfig>::AccountId: From<[u8; 32]>,
{
    fn call<E: Ext>(&mut self, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
    {
        let func_id = env.func_id().try_into()?;
        let mut env = env.buf_in_buf_out();

        match func_id {
            NominationPoolStakingFunc::BondAndStake => {
                let args: NominationPoolStakingValueInput<BalanceOf<T>> = env.read_as()?;
                let contract = args.contract.into();

                let base_weight =
                    <T as pallet_nomination_pools_staking::Config>::WeightInfo::create_nomination_pool();
                env.charge_weight(base_weight)?;

                let caller = env.ext().address().clone();
                let call_result: Result<
                    frame_support::dispatch::PostDispatchInfo,
                    sp_runtime::DispatchErrorWithPostInfo<
                        frame_support::dispatch::PostDispatchInfo,
                    >,
                > = pallet_nomination_pools_staking::Pallet::<T>::create_nomination_pool(
                    RawOrigin::Signed(caller).into(),
                    contract,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = NPSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error.into()))
                    }
                    Ok(_) => Ok(RetVal::Converging(NPSError::Success.into())),
                };
            }
        }
    }
}
