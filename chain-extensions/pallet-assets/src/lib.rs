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

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use assets_chain_extension_types::Command::{self, *};
use assets_chain_extension_types::{handle_result, Outcome, LOG_TARGET};
use frame_support::traits::fungibles::{
    approvals::Inspect as AllowanceInspect, metadata::Inspect as MetadataInspect, Inspect,
};
use frame_support::DefaultNoBound;
use frame_system::RawOrigin;
use pallet_assets::WeightInfo;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, SysConfig,
};
use parity_scale_codec::Encode;
use sp_runtime::traits::{Get, StaticLookup};
use sp_runtime::DispatchError;
use sp_std::marker::PhantomData;
type Weight<T> = <T as pallet_assets::Config>::WeightInfo;

/// Pallet Assets chain extension.
#[derive(DefaultNoBound)]
pub struct AssetsExtension<T>(PhantomData<T>);

impl<T> ChainExtension<T> for AssetsExtension<T>
where
    T: pallet_assets::Config + pallet_contracts::Config,
    <T as pallet_assets::Config>::AssetId: Copy,
    <<T as SysConfig>::Lookup as StaticLookup>::Source: From<<T as SysConfig>::AccountId>,
{
    fn call<E: Ext>(&mut self, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
    {
        let mut env = env.buf_in_buf_out();
        match env.func_id().try_into().map_err(|_| {
            DispatchError::Other("Unsupported func id in Pallet Assets Chain Extension")
        })? {
            Transfer => {
                env.charge_weight(Weight::<T>::transfer())?;

                let (id, target, amount): (
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                log::trace!(
                    target: LOG_TARGET,
                    "transfer: raw arguments: id: {:?}, to: {:?}, amount: {:?}",
                    id,
                    target,
                    amount
                );

                let call_result = pallet_assets::Pallet::<T>::transfer(
                    RawOrigin::Signed(env.ext().address().clone()).into(),
                    id.into(),
                    target.into(),
                    amount,
                );
                handle_result!(call_result);
            }
            TransferApproved => {
                env.charge_weight(Weight::<T>::transfer_approved())?;

                let (id, owner, destination, amount): (
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                log::trace!(target: LOG_TARGET,
                    "transfer_approved: raw arguments: id: {:?}, owner: {:?}, destination: {:?}, amount: {:?}",
                      id, owner, destination, amount);

                let call_result = pallet_assets::Pallet::<T>::transfer_approved(
                    RawOrigin::Signed(env.ext().address().clone()).into(),
                    id.into(),
                    owner.into(),
                    destination.into(),
                    amount,
                );
                handle_result!(call_result);
            }
            Mint => {
                env.charge_weight(Weight::<T>::mint())?;

                let (id, beneficiary, amount): (
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                log::trace!(
                    target: LOG_TARGET,
                    "mint: raw arguments: id: {:?}, beneficiary: {:?}, amount: {:?}",
                    id,
                    beneficiary,
                    amount
                );

                let call_result = pallet_assets::Pallet::<T>::mint(
                    RawOrigin::Signed(env.ext().address().clone()).into(),
                    id.into(),
                    beneficiary.into(),
                    amount,
                );
                handle_result!(call_result);
            }
            Burn => {
                env.charge_weight(Weight::<T>::burn())?;

                let (id, who, amount): (
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                log::trace!(
                    target: LOG_TARGET,
                    "burn: raw arguments: id: {:?}, who: {:?}, amount: {:?}",
                    id,
                    who,
                    amount
                );

                let call_result = pallet_assets::Pallet::<T>::burn(
                    RawOrigin::Signed(env.ext().address().clone()).into(),
                    id.into(),
                    who.into(),
                    amount,
                );
                handle_result!(call_result);
            }
            ApproveTransfer => {
                env.charge_weight(Weight::<T>::approve_transfer())?;

                let (id, delegate, amount): (
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                log::trace!(
                    target: LOG_TARGET,
                    "approve_transfer: raw arguments: id: {:?}, delegate: {:?}, amount: {:?}",
                    id,
                    delegate,
                    amount
                );

                let call_result = pallet_assets::Pallet::<T>::approve_transfer(
                    RawOrigin::Signed(env.ext().address().clone()).into(),
                    id.into(),
                    delegate.into(),
                    amount,
                );
                handle_result!(call_result);
            }
            BalanceOf => {
                env.charge_weight(T::DbWeight::get().reads(1_u64))?;

                let (id, who): (<T as pallet_assets::Config>::AssetId, T::AccountId) =
                    env.read_as()?;

                pallet_assets::Pallet::<T>::balance(id, who)
                    .using_encoded(|r| env.write(r, false, None))?;
            }
            TotalSupply => {
                env.charge_weight(T::DbWeight::get().reads(1_u64))?;

                let id: <T as pallet_assets::Config>::AssetId = env.read_as()?;

                pallet_assets::Pallet::<T>::total_supply(id)
                    .using_encoded(|r| env.write(r, false, None))?;
            }
            Allowance => {
                env.charge_weight(T::DbWeight::get().reads(1_u64))?;

                let (id, owner, delegate): (
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::AccountId,
                ) = env.read_as()?;

                <pallet_assets::Pallet<T> as AllowanceInspect<T::AccountId>>::allowance(
                    id, &owner, &delegate,
                )
                .using_encoded(|r| env.write(r, false, None))?;
            }
            MetadataName => {
                env.charge_weight(T::DbWeight::get().reads(1_u64))?;

                let id: <T as pallet_assets::Config>::AssetId = env.read_as()?;

                <pallet_assets::Pallet<T> as MetadataInspect<T::AccountId>>::name(id)
                    .using_encoded(|r| env.write(r, false, None))?;
            }
            MetadataSymbol => {
                env.charge_weight(T::DbWeight::get().reads(1_u64))?;

                let id: <T as pallet_assets::Config>::AssetId = env.read_as()?;

                <pallet_assets::Pallet<T> as MetadataInspect<T::AccountId>>::symbol(id)
                    .using_encoded(|r| env.write(r, false, None))?;
            }
            MetadataDecimals => {
                env.charge_weight(T::DbWeight::get().reads(1_u64))?;

                let id: <T as pallet_assets::Config>::AssetId = env.read_as()?;

                <pallet_assets::Pallet<T> as MetadataInspect<T::AccountId>>::decimals(id)
                    .using_encoded(|r| env.write(r, false, None))?;
            }
            MinimumBalance => {
                env.charge_weight(T::DbWeight::get().reads(1_u64))?;

                let id: <T as pallet_assets::Config>::AssetId = env.read_as()?;

                <pallet_assets::Pallet<T> as Inspect<T::AccountId>>::minimum_balance(id)
                    .using_encoded(|r| env.write(r, false, None))?;
            }
        }

        Ok(RetVal::Converging(Outcome::Success as u32))
    }
}
