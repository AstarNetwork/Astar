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

use astar_primitives::xcm::XCM_SIZE_LIMIT;
use fp_evm::{PrecompileHandle, PrecompileOutput};
use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    pallet_prelude::Weight,
    traits::{ConstU32, Get},
};
type GetXcmSizeLimit = ConstU32<XCM_SIZE_LIMIT>;

use pallet_evm::{AddressMapping, Precompile};
use parity_scale_codec::DecodeLimit;
use sp_core::{H160, H256, U256};

use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use xcm::{latest::prelude::*, VersionedMultiAsset, VersionedMultiAssets, VersionedMultiLocation};
use xcm_executor::traits::Convert;

use pallet_evm_precompile_assets_erc20::AddressToAssetId;
use precompile_utils::{
    bytes::BoundedBytes,
    data::BoundedVec,
    revert, succeed,
    xcm::{Currency, EvmMultiAsset, WeightV2},
    Address, Bytes, EvmDataWriter, EvmResult, FunctionModifier, PrecompileHandleExt, RuntimeHelper,
};
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    AssetsWithdrawNative = "assets_withdraw(address[],uint256[],bytes32,bool,uint256,uint256)",
    AssetsWithdrawEvm = "assets_withdraw(address[],uint256[],address,bool,uint256,uint256)",
    RemoteTransact = "remote_transact(uint256,bool,address,uint256,bytes,uint64)",
    AssetsReserveTransferNative =
        "assets_reserve_transfer(address[],uint256[],bytes32,bool,uint256,uint256)",
    AssetsReserveTransferEvm =
        "assets_reserve_transfer(address[],uint256[],address,bool,uint256,uint256)",
    SendXCM = "send_xcm((uint8,bytes[]),bytes)",
    XtokensTransfer = "transfer(address,uint256,(uint8,bytes[]),(uint64,uint64))",
    XtokensTransferWithFee =
        "transfer_with_fee(address,uint256,uint256,(uint8,bytes[]),(uint64,uint64))",
    XtokensTransferMultiasset =
        "transfer_multiasset((uint8,bytes[]),uint256,(uint8,bytes[]),(uint64,uint64))",
    XtokensTransferMultiassetWithFee = "transfer_multiasset_with_fee((uint8,bytes[]),uint256,uint256,(uint8,bytes[]),(uint64,uint64))",
    XtokensTransferMulticurrencies =
        "transfer_multi_currencies((address,uint256)[],uint32,(uint8,bytes[]),(uint64,uint64))",
    XtokensTransferMultiassets =
        "transfet_multi_assets(((uint8,bytes[]),uint256)[],uint32,(uint8,bytes[]),(uint64,uint64))",
}

/// Dummy H160 address representing native currency (e.g. ASTR or SDN)
const NATIVE_ADDRESS: H160 = H160::zero();

/// Default proof_size of 256KB
const DEFAULT_PROOF_SIZE: u64 = 1024 * 256;

pub type XBalanceOf<Runtime> = <Runtime as orml_xtokens::Config>::Balance;

pub struct GetMaxAssets<R>(PhantomData<R>);

impl<R> Get<u32> for GetMaxAssets<R>
where
    R: orml_xtokens::Config,
{
    fn get() -> u32 {
        <R as orml_xtokens::Config>::MaxAssetsForTransfer::get() as u32
    }
}
/// A precompile that expose XCM related functions.
pub struct XcmPrecompile<Runtime, C>(PhantomData<(Runtime, C)>);

impl<Runtime, C> Precompile for XcmPrecompile<Runtime, C>
where
    Runtime: pallet_evm::Config
        + pallet_xcm::Config
        + pallet_assets::Config
        + orml_xtokens::Config
        + AddressToAssetId<<Runtime as pallet_assets::Config>::AssetId>,
    <<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<Runtime::AccountId>>,
    <Runtime as frame_system::Config>::AccountId: Into<[u8; 32]>,
    <Runtime as frame_system::Config>::RuntimeCall: From<pallet_xcm::Call<Runtime>>
        + From<orml_xtokens::Call<Runtime>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    XBalanceOf<Runtime>: TryFrom<U256> + Into<U256>,
    <Runtime as orml_xtokens::Config>::CurrencyId:
        From<<Runtime as pallet_assets::Config>::AssetId>,
    C: Convert<MultiLocation, <Runtime as pallet_assets::Config>::AssetId>,
{
    fn execute(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "xcm-precompile", "In XCM precompile");

        let selector = handle.read_selector()?;

        handle.check_function_modifier(FunctionModifier::NonPayable)?;

        // Dispatch the call
        match selector {
            Action::AssetsWithdrawNative => {
                Self::assets_withdraw_v1(handle, BeneficiaryType::Account32)
            }
            Action::AssetsWithdrawEvm => {
                Self::assets_withdraw_v1(handle, BeneficiaryType::Account20)
            }
            Action::RemoteTransact => Self::remote_transact_v1(handle),
            Action::AssetsReserveTransferNative => {
                Self::assets_reserve_transfer_v1(handle, BeneficiaryType::Account32)
            }
            Action::AssetsReserveTransferEvm => {
                Self::assets_reserve_transfer_v1(handle, BeneficiaryType::Account20)
            }
            Action::SendXCM => Self::send_xcm(handle),
            Action::XtokensTransfer => Self::transfer(handle),
            Action::XtokensTransferWithFee => Self::transfer_with_fee(handle),
            Action::XtokensTransferMultiasset => Self::transfer_multiasset(handle),
            Action::XtokensTransferMultiassetWithFee => Self::transfer_multiasset_with_fee(handle),
            Action::XtokensTransferMulticurrencies => Self::transfer_multi_currencies(handle),
            Action::XtokensTransferMultiassets => Self::transfer_multi_assets(handle),
        }
    }
}

/// The supported beneficiary account types
enum BeneficiaryType {
    /// 256 bit (32 byte) public key
    Account32,
    /// 160 bit (20 byte) address is expected
    Account20,
}

impl<Runtime, C> XcmPrecompile<Runtime, C>
where
    Runtime: pallet_evm::Config
        + pallet_xcm::Config
        + orml_xtokens::Config
        + pallet_assets::Config
        + AddressToAssetId<<Runtime as pallet_assets::Config>::AssetId>,
    <<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<Runtime::AccountId>>,
    <Runtime as frame_system::Config>::AccountId: Into<[u8; 32]>,
    <Runtime as frame_system::Config>::RuntimeCall: From<pallet_xcm::Call<Runtime>>
        + From<orml_xtokens::Call<Runtime>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    XBalanceOf<Runtime>: TryFrom<U256> + Into<U256>,
    <Runtime as orml_xtokens::Config>::CurrencyId:
        From<<Runtime as pallet_assets::Config>::AssetId>,
    C: Convert<MultiLocation, <Runtime as pallet_assets::Config>::AssetId>,
{
    fn assets_withdraw_v1(
        handle: &mut impl PrecompileHandle,
        beneficiary_type: BeneficiaryType,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(6)?;

        // Read arguments and check it
        let assets: Vec<MultiLocation> = input
            .read::<Vec<Address>>()?
            .iter()
            .cloned()
            .filter_map(|address| {
                Runtime::address_to_asset_id(address.into()).and_then(|x| C::reverse_ref(x).ok())
            })
            .collect();
        let amounts_raw = input.read::<Vec<U256>>()?;
        if amounts_raw.iter().any(|x| *x > u128::MAX.into()) {
            return Err(revert("Asset amount is too big"));
        }
        let amounts: Vec<u128> = amounts_raw.iter().map(|x| x.low_u128()).collect();

        // Check that assets list is valid:
        // * all assets resolved to multi-location
        // * all assets has corresponded amount
        if assets.len() != amounts.len() || assets.is_empty() {
            return Err(revert("Assets resolution failure."));
        }

        let beneficiary: MultiLocation = match beneficiary_type {
            BeneficiaryType::Account32 => {
                let recipient: [u8; 32] = input.read::<H256>()?.into();
                X1(Junction::AccountId32 {
                    network: None,
                    id: recipient,
                })
            }
            BeneficiaryType::Account20 => {
                let recipient: H160 = input.read::<Address>()?.into();
                X1(Junction::AccountKey20 {
                    network: None,
                    key: recipient.to_fixed_bytes(),
                })
            }
        }
        .into();

        let is_relay = input.read::<bool>()?;
        let parachain_id: u32 = input.read::<U256>()?.low_u32();
        let fee_asset_item: u32 = input.read::<U256>()?.low_u32();

        if fee_asset_item as usize > assets.len() {
            return Err(revert("Bad fee index."));
        }

        // Prepare pallet-xcm call arguments
        let dest = if is_relay {
            MultiLocation::parent()
        } else {
            X1(Junction::Parachain(parachain_id)).into_exterior(1)
        };

        let assets: MultiAssets = assets
            .iter()
            .cloned()
            .zip(amounts.iter().cloned())
            .map(Into::into)
            .collect::<Vec<MultiAsset>>()
            .into();

        // Build call with origin.
        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();
        let call = pallet_xcm::Call::<Runtime>::reserve_withdraw_assets {
            dest: Box::new(dest.into()),
            beneficiary: Box::new(beneficiary.into()),
            assets: Box::new(assets.into()),
            fee_asset_item,
        };

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn remote_transact_v1(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(6)?;

        // Raw call arguments
        let para_id: u32 = input.read::<U256>()?.low_u32();
        let is_relay = input.read::<bool>()?;

        let fee_asset_addr = input.read::<Address>()?;
        let fee_amount = input.read::<U256>()?;

        let remote_call: Vec<u8> = input.read::<Bytes>()?.into();
        let transact_weight = input.read::<u64>()?;

        log::trace!(target: "xcm-precompile:remote_transact", "Raw arguments: para_id: {}, is_relay: {}, fee_asset_addr: {:?}, \
         fee_amount: {:?}, remote_call: {:?}, transact_weight: {}",
        para_id, is_relay, fee_asset_addr, fee_amount, remote_call, transact_weight);

        // Process arguments
        let dest = if is_relay {
            MultiLocation::parent()
        } else {
            X1(Junction::Parachain(para_id)).into_exterior(1)
        };

        let fee_asset = {
            let address: H160 = fee_asset_addr.into();

            // Special case where zero address maps to native token by convention.
            if address == NATIVE_ADDRESS {
                Here.into()
            } else {
                let fee_asset_id = Runtime::address_to_asset_id(address)
                    .ok_or(revert("Failed to resolve fee asset id from address"))?;
                C::reverse_ref(fee_asset_id).map_err(|_| {
                    revert("Failed to resolve fee asset multilocation from local id")
                })?
            }
        };

        if fee_amount > u128::MAX.into() {
            return Err(revert("Fee amount is too big"));
        }
        let fee_amount = fee_amount.low_u128();

        let context = <Runtime as pallet_xcm::Config>::UniversalLocation::get();
        let fee_multilocation: MultiAsset = (fee_asset, fee_amount).into();
        let fee_multilocation = fee_multilocation
            .reanchored(&dest, context)
            .map_err(|_| revert("Failed to reanchor fee asset"))?;

        // Prepare XCM
        let xcm = Xcm(vec![
            WithdrawAsset(fee_multilocation.clone().into()),
            BuyExecution {
                fees: fee_multilocation.clone().into(),
                weight_limit: WeightLimit::Unlimited,
            },
            Transact {
                origin_kind: OriginKind::SovereignAccount,
                require_weight_at_most: Weight::from_parts(transact_weight, DEFAULT_PROOF_SIZE),
                call: remote_call.into(),
            },
        ]);

        log::trace!(target: "xcm-precompile:remote_transact", "Processed arguments: dest: {:?}, fee asset: {:?}, XCM: {:?}", dest, fee_multilocation, xcm);

        // Build call with origin.
        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();
        let call = pallet_xcm::Call::<Runtime>::send {
            dest: Box::new(dest.into()),
            message: Box::new(xcm::VersionedXcm::V3(xcm)),
        };

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn assets_reserve_transfer_v1(
        handle: &mut impl PrecompileHandle,
        beneficiary_type: BeneficiaryType,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(6)?;

        // Read arguments and check it
        let assets: Vec<MultiLocation> = input
            .read::<Vec<Address>>()?
            .iter()
            .cloned()
            .filter_map(|address| {
                let address: H160 = address.into();

                // Special case where zero address maps to native token by convention.
                if address == NATIVE_ADDRESS {
                    Some(Here.into())
                } else {
                    Runtime::address_to_asset_id(address).and_then(|x| C::reverse_ref(x).ok())
                }
            })
            .collect();
        let amounts_raw = input.read::<Vec<U256>>()?;
        if amounts_raw.iter().any(|x| *x > u128::MAX.into()) {
            return Err(revert("Asset amount is too big"));
        }
        let amounts: Vec<u128> = amounts_raw.iter().map(|x| x.low_u128()).collect();

        log::trace!(target: "xcm-precompile:assets_reserve_transfer", "Processed arguments: assets {:?}, amounts: {:?}", assets, amounts);

        // Check that assets list is valid:
        // * all assets resolved to multi-location
        // * all assets has corresponded amount
        if assets.len() != amounts.len() || assets.is_empty() {
            return Err(revert("Assets resolution failure."));
        }

        let beneficiary: MultiLocation = match beneficiary_type {
            BeneficiaryType::Account32 => {
                let recipient: [u8; 32] = input.read::<H256>()?.into();
                X1(Junction::AccountId32 {
                    network: None,
                    id: recipient,
                })
            }
            BeneficiaryType::Account20 => {
                let recipient: H160 = input.read::<Address>()?.into();
                X1(Junction::AccountKey20 {
                    network: None,
                    key: recipient.to_fixed_bytes(),
                })
            }
        }
        .into();

        let is_relay = input.read::<bool>()?;
        let parachain_id: u32 = input.read::<U256>()?.low_u32();
        let fee_asset_item: u32 = input.read::<U256>()?.low_u32();

        if fee_asset_item as usize > assets.len() {
            return Err(revert("Bad fee index."));
        }

        // Prepare pallet-xcm call arguments
        let dest = if is_relay {
            MultiLocation::parent()
        } else {
            X1(Junction::Parachain(parachain_id)).into_exterior(1)
        };

        let assets: MultiAssets = assets
            .iter()
            .cloned()
            .zip(amounts.iter().cloned())
            .map(Into::into)
            .collect::<Vec<MultiAsset>>()
            .into();

        // Build call with origin.
        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();
        let call = pallet_xcm::Call::<Runtime>::reserve_transfer_assets {
            dest: Box::new(dest.into()),
            beneficiary: Box::new(beneficiary.into()),
            assets: Box::new(assets.into()),
            fee_asset_item,
        };

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn send_xcm(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(2)?;

        // Raw call arguments
        let dest: MultiLocation = input.read::<MultiLocation>()?;
        let xcm_call: Vec<u8> = input.read::<BoundedBytes<GetXcmSizeLimit>>()?.into();

        log::trace!(target:"xcm-precompile::send_xcm", "Raw arguments: dest: {:?}, xcm_call: {:?}", dest, xcm_call);

        let xcm = xcm::VersionedXcm::<()>::decode_all_with_depth_limit(
            xcm::MAX_XCM_DECODE_DEPTH,
            &mut xcm_call.as_slice(),
        )
        .map_err(|_| revert("Failed to decode xcm instructions"))?;

        // Build call with origin.
        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();
        let call = pallet_xcm::Call::<Runtime>::send {
            dest: Box::new(dest.into()),
            message: Box::new(xcm),
        };
        log::trace!(target: "xcm-send_xcm", "Processed arguments:  XCM call: {:?}", call);
        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn transfer(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(4)?;

        // Read call arguments
        let currency_address = input.read::<Address>()?;
        let amount_of_tokens = input
            .read::<U256>()?
            .try_into()
            .map_err(|_| revert("error converting amount_of_tokens, maybe value too large"))?;
        let destination = input.read::<MultiLocation>()?;
        let weight = input.read::<WeightV2>()?;

        let asset_id = Runtime::address_to_asset_id(currency_address.into())
            .ok_or(revert("Failed to resolve fee asset id from address"))?;
        let dest_weight_limit = if weight.is_zero() {
            WeightLimit::Unlimited
        } else {
            WeightLimit::Limited(weight.get_weight())
        };

        log::trace!(target: "xcm-precompile::transfer", "Raw arguments: currency_address: {:?}, amount_of_tokens: {:?}, destination: {:?}, \
        weight: {:?}, calculated asset_id: {:?}",
        currency_address, amount_of_tokens, destination, weight, asset_id);

        let call = orml_xtokens::Call::<Runtime>::transfer {
            currency_id: asset_id.into(),
            amount: amount_of_tokens,
            dest: Box::new(VersionedMultiLocation::V3(destination)),
            dest_weight_limit,
        };

        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn transfer_with_fee(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(5)?;

        // Read call arguments
        let currency_address = input.read::<Address>()?;
        let amount_of_tokens = input
            .read::<U256>()?
            .try_into()
            .map_err(|_| revert("error converting amount_of_tokens, maybe value too large"))?;
        let fee = input
            .read::<U256>()?
            .try_into()
            .map_err(|_| revert("can't convert fee"))?;

        let destination = input.read::<MultiLocation>()?;
        let weight = input.read::<WeightV2>()?;

        let asset_id = Runtime::address_to_asset_id(currency_address.into())
            .ok_or(revert("Failed to resolve fee asset id from address"))?;
        let dest_weight_limit = if weight.is_zero() {
            WeightLimit::Unlimited
        } else {
            WeightLimit::Limited(weight.get_weight())
        };

        log::trace!(target: "xcm-precompile::transfer_with_fee", "Raw arguments: currency_address: {:?}, amount_of_tokens: {:?}, destination: {:?}, \
        weight: {:?}, calculated asset_id: {:?}",
        currency_address, amount_of_tokens, destination, weight, asset_id);

        let call = orml_xtokens::Call::<Runtime>::transfer_with_fee {
            currency_id: asset_id.into(),
            amount: amount_of_tokens,
            fee,
            dest: Box::new(VersionedMultiLocation::V3(destination)),
            dest_weight_limit,
        };

        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn transfer_multiasset(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(4)?;

        // Read call arguments
        let asset_location = input.read::<MultiLocation>()?;
        let amount_of_tokens: u128 = input
            .read::<U256>()?
            .try_into()
            .map_err(|_| revert("error converting amount_of_tokens, maybe value too large"))?;
        let destination = input.read::<MultiLocation>()?;
        let weight = input.read::<WeightV2>()?;

        let dest_weight_limit = if weight.is_zero() {
            WeightLimit::Unlimited
        } else {
            WeightLimit::Limited(weight.get_weight())
        };

        log::trace!(target: "xcm-precompile::transfer_multiasset", "Raw arguments: asset_location: {:?}, amount_of_tokens: {:?}, destination: {:?}, \
        weight: {:?}",
        asset_location, amount_of_tokens, destination, weight);

        let call = orml_xtokens::Call::<Runtime>::transfer_multiasset {
            asset: Box::new(VersionedMultiAsset::V3(
                (asset_location, amount_of_tokens).into(),
            )),
            dest: Box::new(VersionedMultiLocation::V3(destination)),
            dest_weight_limit,
        };

        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn transfer_multiasset_with_fee(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(5)?;

        // Read call arguments
        let asset_location = input.read::<MultiLocation>()?;
        let amount_of_tokens: u128 = input
            .read::<U256>()?
            .try_into()
            .map_err(|_| revert("error converting amount_of_tokens, maybe value too large"))?;
        let fee: u128 = input
            .read::<U256>()?
            .try_into()
            .map_err(|_| revert("can't convert fee"))?;
        let destination = input.read::<MultiLocation>()?;
        let weight = input.read::<WeightV2>()?;

        let dest_weight_limit = if weight.is_zero() {
            WeightLimit::Unlimited
        } else {
            WeightLimit::Limited(weight.get_weight())
        };

        log::trace!(target: "xcm-precompile::transfer_multiasset_with_fee", "Raw arguments: asset_location: {:?}, amount_of_tokens: {:?}, fee{:?}, destination: {:?}, \
        weight: {:?}",
        asset_location, amount_of_tokens, fee, destination, weight);

        let call = orml_xtokens::Call::<Runtime>::transfer_multiasset_with_fee {
            asset: Box::new(VersionedMultiAsset::V3(
                (asset_location, amount_of_tokens).into(),
            )),
            fee: Box::new(VersionedMultiAsset::V3((asset_location, fee).into())),
            dest: Box::new(VersionedMultiLocation::V3(destination)),
            dest_weight_limit,
        };

        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn transfer_multi_currencies(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(4)?;

        let currencies: Vec<_> = input
            .read::<BoundedVec<Currency, GetMaxAssets<Runtime>>>()?
            .into();
        let fee_item = input.read::<u32>()?;
        let destination = input.read::<MultiLocation>()?;
        let weight = input.read::<WeightV2>()?;

        let currencies = currencies
            .into_iter()
            .map(|currency| {
                let currency_address: H160 = currency.get_address().into();
                let amount = currency
                    .get_amount()
                    .try_into()
                    .map_err(|_| revert("value too large: in currency"))?;

                Ok((
                    Runtime::address_to_asset_id(currency_address.into())
                        .ok_or(revert("can't convert into currency id"))?
                        .into(),
                    amount,
                ))
            })
            .collect::<EvmResult<_>>()?;
        let dest_weight_limit = if weight.is_zero() {
            WeightLimit::Unlimited
        } else {
            WeightLimit::Limited(weight.get_weight())
        };

        log::trace!(target: "xcm-precompile::transfer_multi_currencies", "Raw arguments: currencies: {:?}, fee_item{:?}, destination: {:?}, \
        weight: {:?}",
        currencies, fee_item, destination, weight);

        let call = orml_xtokens::Call::<Runtime>::transfer_multicurrencies {
            currencies,
            fee_item,
            dest: Box::new(VersionedMultiLocation::V3(destination)),
            dest_weight_limit,
        };

        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn transfer_multi_assets(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(4)?;

        let assets: Vec<_> = input
            .read::<BoundedVec<EvmMultiAsset, GetMaxAssets<Runtime>>>()?
            .into();
        let fee_item = input.read::<u32>()?;
        let destination = input.read::<MultiLocation>()?;
        let weight = input.read::<WeightV2>()?;

        let dest_weight_limit = if weight.is_zero() {
            WeightLimit::Unlimited
        } else {
            WeightLimit::Limited(weight.get_weight())
        };

        log::trace!(target: "xcm-precompile::transfer_multi_assets", "Raw arguments: assets: {:?}, fee_item{:?}, destination: {:?}, \
        weight: {:?}",
        assets, fee_item, destination, weight);

        let multiasset_vec: EvmResult<Vec<MultiAsset>> = assets
            .into_iter()
            .map(|evm_multiasset| {
                let to_balance: u128 = evm_multiasset
                    .get_amount()
                    .try_into()
                    .map_err(|_| revert("value too large in assets"))?;
                Ok((evm_multiasset.get_location(), to_balance).into())
            })
            .collect();

        // Since multiassets sorts them, we need to check whether the index is still correct,
        // and error otherwise as there is not much we can do other than that
        let multiassets =
            MultiAssets::from_sorted_and_deduplicated(multiasset_vec?).map_err(|_| {
                revert("In field Assets, Provided assets either not sorted nor deduplicated")
            })?;

        let call = orml_xtokens::Call::<Runtime>::transfer_multiassets {
            assets: Box::new(VersionedMultiAssets::V3(multiassets)),
            fee_item,
            dest: Box::new(VersionedMultiLocation::V3(destination)),
            dest_weight_limit,
        };

        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }
}
