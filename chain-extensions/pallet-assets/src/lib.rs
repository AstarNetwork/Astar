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
use assets_chain_extension_types::Outcome;
use frame_system::RawOrigin;
use pallet_assets::WeightInfo;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, SysConfig,
};
use sp_runtime::traits::StaticLookup;
use sp_runtime::DispatchError;
use sp_std::marker::PhantomData;
type Weight<T> = <T as pallet_assets::Config>::WeightInfo;

/// Pallet Assets chain extension.
pub struct AssetsExtension<T>(PhantomData<T>);

impl<T> Default for AssetsExtension<T> {
    fn default() -> Self {
        AssetsExtension(PhantomData)
    }
}

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
                let (id, target, amount): (
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                log::trace!(target: "pallet-chain-extension-assets::transfer",
                    "Raw arguments: id: {:?}, to: {:?}, amount: {:?}",
                      id, target, amount);

                env.charge_weight(Weight::<T>::transfer())?;

                let call_result = pallet_assets::Pallet::<T>::transfer(
                    RawOrigin::Signed(env.ext().address().clone()).into(),
                    id.into(),
                    target.into(),
                    amount,
                );
                return match call_result {
                    Err(e) => {
                        log::trace!(
                            target: "pallet-chain-extension-assets::transfer",
                            "err: {:?}", e
                        );
                        let mapped_error = Outcome::from(e);
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(Outcome::Success as u32)),
                };
            }
            TransferApproved => {
                let (id, owner, destination, amount): (
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                log::trace!(target: "pallet-chain-extension-assets::transfer_approved",
                    "Raw arguments: id: {:?}, owner: {:?}, destination: {:?}, amount: {:?}",
                      id, owner, destination, amount);

                env.charge_weight(Weight::<T>::transfer_approved())?;

                let call_result = pallet_assets::Pallet::<T>::transfer_approved(
                    RawOrigin::Signed(env.ext().address().clone()).into(),
                    id.into(),
                    owner.into(),
                    destination.into(),
                    amount,
                );
                return match call_result {
                    Err(e) => {
                        log::trace!(
                            target: "pallet-chain-extension-assets::transfer_approved",
                            "err: {:?}", e
                        );
                        let mapped_error = Outcome::from(e);
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(Outcome::Success as u32)),
                };
            }
            Mint => {
                let (id, beneficiary, amount): (
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                log::trace!(target: "pallet-chain-extension-assets::mint",
                    "Raw arguments: id: {:?}, beneficiary: {:?}, amount: {:?}",
                      id, beneficiary, amount);

                env.charge_weight(Weight::<T>::mint())?;

                let call_result = pallet_assets::Pallet::<T>::mint(
                    RawOrigin::Signed(env.ext().address().clone()).into(),
                    id.into(),
                    beneficiary.into(),
                    amount,
                );
                return match call_result {
                    Err(e) => {
                        log::trace!(
                            target: "pallet-chain-extension-assets::mint",
                            "err: {:?}", e
                        );
                        let mapped_error = Outcome::from(e);
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(Outcome::Success as u32)),
                };
            }
            Burn => {
                let (id, who, amount): (
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                log::trace!(target: "pallet-chain-extension-assets::burn",
                    "Raw arguments: id: {:?}, who: {:?}, amount: {:?}",
                      id, who, amount);

                env.charge_weight(Weight::<T>::burn())?;

                let call_result = pallet_assets::Pallet::<T>::burn(
                    RawOrigin::Signed(env.ext().address().clone()).into(),
                    id.into(),
                    who.into(),
                    amount,
                );
                return match call_result {
                    Err(e) => {
                        log::trace!(
                            target: "pallet-chain-extension-assets::burn",
                            "err: {:?}", e
                        );
                        let mapped_error = Outcome::from(e);
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(Outcome::Success as u32)),
                };
            }
            /*            AssetsFunc::Mint => {
                let (origin, id, beneficiary, amount): (
                    Origin,
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                let base_weight = <T as pallet_assets::Config>::WeightInfo::mint();
                env.charge_weight(base_weight)?;

                let raw_origin = select_origin!(&origin, env.ext().address().clone());

                let call_result = pallet_assets::Pallet::<T>::mint(
                    raw_origin.into(),
                    id.into(),
                    beneficiary.into(),
                    amount,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = Outcome::from(e);
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(Outcome::Success as u32)),
                };
            }
            AssetsFunc::Burn => {
                let (origin, id, who, amount): (
                    Origin,
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                let base_weight = <T as pallet_assets::Config>::WeightInfo::burn();
                env.charge_weight(base_weight)?;

                let raw_origin = select_origin!(&origin, env.ext().address().clone());

                let call_result = pallet_assets::Pallet::<T>::burn(
                    raw_origin.into(),
                    id.into(),
                    who.into(),
                    amount,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = Outcome::from(e);
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(Outcome::Success as u32)),
                };
            }
            AssetsFunc::BalanceOf => {
                let (id, who): (<T as pallet_assets::Config>::AssetId, T::AccountId) =
                    env.read_as()?;

                let base_weight = <W as weights::WeightInfo>::balance_of();
                env.charge_weight(base_weight)?;

                let balance = pallet_assets::Pallet::<T>::balance(id, who);
                env.write(&balance.encode(), false, None)?;
            }
            AssetsFunc::TotalSupply => {
                let id: <T as pallet_assets::Config>::AssetId = env.read_as()?;

                let base_weight = <W as weights::WeightInfo>::total_supply();
                env.charge_weight(base_weight)?;

                let total_supply = pallet_assets::Pallet::<T>::total_supply(id);
                env.write(&total_supply.encode(), false, None)?;
            }
            AssetsFunc::Allowance => {
                let (id, owner, delegate): (
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::AccountId,
                ) = env.read_as()?;

                let base_weight = <W as weights::WeightInfo>::allowance();
                env.charge_weight(base_weight)?;

                let allowance = pallet_assets::Pallet::<T>::allowance(id, &owner, &delegate);
                env.write(&allowance.encode(), false, None)?;
            }
            AssetsFunc::ApproveTransfer => {
                let (origin, id, delegate, amount): (
                    Origin,
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                let base_weight = <T as pallet_assets::Config>::WeightInfo::approve_transfer();
                env.charge_weight(base_weight)?;

                let raw_origin = select_origin!(&origin, env.ext().address().clone());

                let call_result = pallet_assets::Pallet::<T>::approve_transfer(
                    raw_origin.into(),
                    id.into(),
                    delegate.into(),
                    amount,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = Outcome::from(e);
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(Outcome::Success as u32)),
                };
            }
            AssetsFunc::TransferApproved => {
                let (origin, id, owner, destination, amount): (
                    Origin,
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                let base_weight = <T as pallet_assets::Config>::WeightInfo::transfer_approved();
                env.charge_weight(base_weight)?;

                let raw_origin = select_origin!(&origin, env.ext().address().clone());

                let call_result = pallet_assets::Pallet::<T>::transfer_approved(
                    raw_origin.into(),
                    id.into(),
                    owner.into(),
                    destination.into(),
                    amount,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = Outcome::from(e);
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(Outcome::Success as u32)),
                };
            }
            AssetsFunc::MetadataName => {
                let id: <T as pallet_assets::Config>::AssetId = env.read_as()?;

                let base_weight = <W as weights::WeightInfo>::metadata_name();
                env.charge_weight(base_weight)?;

                let name = pallet_assets::Pallet::<T>::name(id);
                env.write(&name.encode(), false, None)?;
            }
            AssetsFunc::MetadataSymbol => {
                let id: <T as pallet_assets::Config>::AssetId = env.read_as()?;

                let base_weight = <W as weights::WeightInfo>::metadata_symbol();
                env.charge_weight(base_weight)?;

                let symbol = pallet_assets::Pallet::<T>::symbol(id);
                env.write(&symbol.encode(), false, None)?;
            }
            AssetsFunc::MetadataDecimals => {
                let id: <T as pallet_assets::Config>::AssetId = env.read_as()?;

                let base_weight = <W as weights::WeightInfo>::metadata_decimals();
                env.charge_weight(base_weight)?;

                let decimals = pallet_assets::Pallet::<T>::decimals(id);
                env.write(&decimals.encode(), false, None)?;
            }*/
            _ => {}
        }

        Ok(RetVal::Converging(Outcome::Success as u32))
    }
}
