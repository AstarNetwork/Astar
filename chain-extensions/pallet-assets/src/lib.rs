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

pub mod weights;

use assets_chain_extension_types::{select_origin, Origin, Outcome};
use frame_support::traits::fungibles::InspectMetadata;
use frame_support::traits::tokens::fungibles::approvals::Inspect;
use frame_system::RawOrigin;
use pallet_assets::WeightInfo;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, SysConfig,
};
use parity_scale_codec::Encode;
use sp_runtime::traits::StaticLookup;
use sp_runtime::DispatchError;
use sp_std::marker::PhantomData;
use sp_std::vec::Vec;

enum AssetsFunc {
    Create,
    Transfer,
    Mint,
    Burn,
    BalanceOf,
    TotalSupply,
    Allowance,
    ApproveTransfer,
    CancelApproval,
    TransferApproved,
    SetMetadata,
    MetadataName,
    MetadataSymbol,
    MetadataDecimals,
    TransferOwnership,
}

impl TryFrom<u16> for AssetsFunc {
    type Error = DispatchError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(AssetsFunc::Create),
            2 => Ok(AssetsFunc::Transfer),
            3 => Ok(AssetsFunc::Mint),
            4 => Ok(AssetsFunc::Burn),
            5 => Ok(AssetsFunc::BalanceOf),
            6 => Ok(AssetsFunc::TotalSupply),
            7 => Ok(AssetsFunc::Allowance),
            8 => Ok(AssetsFunc::ApproveTransfer),
            9 => Ok(AssetsFunc::CancelApproval),
            10 => Ok(AssetsFunc::TransferApproved),
            11 => Ok(AssetsFunc::SetMetadata),
            12 => Ok(AssetsFunc::MetadataName),
            13 => Ok(AssetsFunc::MetadataSymbol),
            14 => Ok(AssetsFunc::MetadataDecimals),
            15 => Ok(AssetsFunc::TransferOwnership),
            _ => Err(DispatchError::Other(
                "PalletAssetsExtension: Unimplemented func_id",
            )),
        }
    }
}

/// Pallet Assets chain extension.
pub struct AssetsExtension<T, W>(PhantomData<(T, W)>);

impl<T, W> Default for AssetsExtension<T, W> {
    fn default() -> Self {
        AssetsExtension(PhantomData)
    }
}

impl<T, W> ChainExtension<T> for AssetsExtension<T, W>
where
    T: pallet_assets::Config + pallet_contracts::Config,
    <<T as SysConfig>::Lookup as StaticLookup>::Source: From<<T as SysConfig>::AccountId>,
    <T as SysConfig>::AccountId: From<[u8; 32]>,
    W: weights::WeightInfo,
{
    fn call<E: Ext>(&mut self, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
    {
        let func_id = env.func_id().try_into()?;
        let mut env = env.buf_in_buf_out();

        match func_id {
            AssetsFunc::Create => {
                let (origin, id, admin, min_balance): (
                    Origin,
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                let base_weight = <T as pallet_assets::Config>::WeightInfo::create();
                env.charge_weight(base_weight)?;

                let raw_origin = select_origin!(&origin, env.ext().address().clone());

                let call_result = pallet_assets::Pallet::<T>::create(
                    raw_origin.into(),
                    id.into(),
                    admin.into(),
                    min_balance,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = Outcome::from(e);
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(Outcome::Success as u32)),
                };
            }
            AssetsFunc::Transfer => {
                let (origin, id, target, amount): (
                    Origin,
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::Balance,
                ) = env.read_as()?;

                let base_weight = <T as pallet_assets::Config>::WeightInfo::transfer();
                env.charge_weight(base_weight)?;

                let raw_origin = select_origin!(&origin, env.ext().address().clone());

                let call_result = pallet_assets::Pallet::<T>::transfer(
                    raw_origin.into(),
                    id.into(),
                    target.into(),
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
            AssetsFunc::Mint => {
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
            AssetsFunc::CancelApproval => {
                let (origin, id, delegate): (
                    Origin,
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                ) = env.read_as()?;

                let base_weight = <T as pallet_assets::Config>::WeightInfo::cancel_approval();
                env.charge_weight(base_weight)?;

                let raw_origin = select_origin!(&origin, env.ext().address().clone());

                let call_result = pallet_assets::Pallet::<T>::cancel_approval(
                    raw_origin.into(),
                    id.into(),
                    delegate.into(),
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
            AssetsFunc::SetMetadata => {
                let (origin, id, name, symbol, decimals): (
                    Origin,
                    <T as pallet_assets::Config>::AssetId,
                    Vec<u8>,
                    Vec<u8>,
                    u8,
                ) = env.read_as_unbounded(env.in_len())?;

                let base_weight = <T as pallet_assets::Config>::WeightInfo::set_metadata(
                    name.len() as u32,
                    symbol.len() as u32,
                );
                env.charge_weight(base_weight)?;

                let raw_origin = select_origin!(&origin, env.ext().address().clone());

                let call_result = pallet_assets::Pallet::<T>::set_metadata(
                    raw_origin.into(),
                    id.into(),
                    name,
                    symbol,
                    decimals,
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

                let name = pallet_assets::Pallet::<T>::name(&id);
                env.write(&name.encode(), false, None)?;
            }
            AssetsFunc::MetadataSymbol => {
                let id: <T as pallet_assets::Config>::AssetId = env.read_as()?;

                let base_weight = <W as weights::WeightInfo>::metadata_symbol();
                env.charge_weight(base_weight)?;

                let symbol = pallet_assets::Pallet::<T>::symbol(&id);
                env.write(&symbol.encode(), false, None)?;
            }
            AssetsFunc::MetadataDecimals => {
                let id: <T as pallet_assets::Config>::AssetId = env.read_as()?;

                let base_weight = <W as weights::WeightInfo>::metadata_decimals();
                env.charge_weight(base_weight)?;

                let decimals = pallet_assets::Pallet::<T>::decimals(&id);
                env.write(&decimals.encode(), false, None)?;
            }
            AssetsFunc::TransferOwnership => {
                let (origin, id, owner): (
                    Origin,
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                ) = env.read_as()?;

                let base_weight = <T as pallet_assets::Config>::WeightInfo::transfer_ownership();
                env.charge_weight(base_weight)?;

                let raw_origin = select_origin!(&origin, env.ext().address().clone());

                let call_result = pallet_assets::Pallet::<T>::transfer_ownership(
                    raw_origin.into(),
                    id.into(),
                    owner.into(),
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = Outcome::from(e);
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(Outcome::Success as u32)),
                };
            }
        }

        Ok(RetVal::Converging(Outcome::Success as u32))
    }
}
