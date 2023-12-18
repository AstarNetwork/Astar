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
use fp_evm::PrecompileHandle;
use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    pallet_prelude::Weight,
    traits::{ConstU32, Get},
};
type GetXcmSizeLimit = ConstU32<XCM_SIZE_LIMIT>;

use pallet_evm::AddressMapping;
use parity_scale_codec::DecodeLimit;
use sp_core::{H160, H256, U256};

use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use xcm::{latest::prelude::*, VersionedMultiAsset, VersionedMultiAssets, VersionedMultiLocation};
use xcm_executor::traits::Convert;

use pallet_evm_precompile_assets_erc20::AddressToAssetId;
use precompile_utils::prelude::*;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

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

#[precompile_utils::precompile]
#[precompile::test_concrete_types(mock::Runtime, mock::AssetIdConverter<mock::AssetId>)]
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
    XBalanceOf<Runtime>: TryFrom<U256> + Into<U256> + From<u128>,
    <Runtime as orml_xtokens::Config>::CurrencyId:
        From<<Runtime as pallet_assets::Config>::AssetId>,
    C: Convert<MultiLocation, <Runtime as pallet_assets::Config>::AssetId>,
{
    #[precompile::public("assets_withdraw(address[],uint256[],bytes32,bool,uint256,uint256)")]
    fn assets_withdraw_native_v1(
        handle: &mut impl PrecompileHandle,
        assets: BoundedVec<Address, GetMaxAssets<Runtime>>,
        amounts: BoundedVec<U256, GetMaxAssets<Runtime>>,
        recipient_account_id: H256,
        is_relay: bool,
        parachain_id: U256,
        fee_index: U256,
    ) -> EvmResult<bool> {
        let beneficiary: Junction = Junction::AccountId32 {
            network: None,
            id: recipient_account_id.into(),
        }
        .into();
        Self::assets_withdraw_v1_internal(
            handle,
            assets.into(),
            amounts.into(),
            beneficiary,
            is_relay,
            parachain_id,
            fee_index,
        )
    }

    #[precompile::public("assets_withdraw(address[],uint256[],address,bool,uint256,uint256)")]
    fn assets_withdraw_evm_v1(
        handle: &mut impl PrecompileHandle,
        assets: BoundedVec<Address, GetMaxAssets<Runtime>>,
        amounts: BoundedVec<U256, GetMaxAssets<Runtime>>,
        recipient_account_id: Address,
        is_relay: bool,
        parachain_id: U256,
        fee_index: U256,
    ) -> EvmResult<bool> {
        let beneficiary: Junction = Junction::AccountKey20 {
            network: None,
            key: recipient_account_id.0.to_fixed_bytes(),
        }
        .into();
        Self::assets_withdraw_v1_internal(
            handle,
            assets.into(),
            amounts.into(),
            beneficiary,
            is_relay,
            parachain_id,
            fee_index,
        )
    }

    fn assets_withdraw_v1_internal(
        handle: &mut impl PrecompileHandle,
        assets: Vec<Address>,
        amounts: Vec<U256>,
        beneficiary: Junction,
        is_relay: bool,
        parachain_id: U256,
        fee_index: U256,
    ) -> EvmResult<bool> {
        // Read arguments and check it
        let assets = assets
            .iter()
            .cloned()
            .filter_map(|address| {
                Runtime::address_to_asset_id(address.into()).and_then(|x| C::reverse_ref(x).ok())
            })
            .collect::<Vec<MultiLocation>>();

        let amounts = amounts
            .into_iter()
            .map(|x| x.try_into())
            .collect::<Result<Vec<u128>, _>>()
            .map_err(|_| revert("error converting amounts, maybe value too large"))?;

        // Check that assets list is valid:
        // * all assets resolved to multi-location
        // * all assets has corresponded amount
        if assets.len() != amounts.len() || assets.is_empty() {
            return Err(revert("Assets resolution failure."));
        }

        let parachain_id: u32 = parachain_id
            .try_into()
            .map_err(|_| revert("error converting parachain_id, maybe value too large"))?;

        let fee_item: u32 = fee_index
            .try_into()
            .map_err(|_| revert("error converting fee_index, maybe value too large"))?;

        let mut destination = if is_relay {
            MultiLocation::parent()
        } else {
            X1(Junction::Parachain(parachain_id)).into_exterior(1)
        };

        destination
            .push_interior(beneficiary)
            .map_err(|_| revert("error building destination multilocation"))?;

        let assets = assets
            .iter()
            .cloned()
            .zip(amounts.iter().cloned())
            .map(Into::into)
            .collect::<Vec<MultiAsset>>();

        log::trace!(target: "xcm-precompile:assets_withdraw", "Processed arguments: assets {:?}, destination: {:?}", assets, destination);

        // Build call with origin.
        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();

        let call = orml_xtokens::Call::<Runtime>::transfer_multiassets {
            assets: Box::new(VersionedMultiAssets::V3(assets.into())),
            fee_item,
            dest: Box::new(VersionedMultiLocation::V3(destination)),
            dest_weight_limit: WeightLimit::Unlimited,
        };

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;
        Ok(true)
    }

    #[precompile::public("remote_transact(uint256,bool,address,uint256,bytes,uint64)")]
    fn remote_transact_v1(
        handle: &mut impl PrecompileHandle,
        para_id: U256,
        is_relay: bool,
        fee_asset_addr: Address,
        fee_amount: U256,
        remote_call: UnboundedBytes,
        transact_weight: u64,
    ) -> EvmResult<bool> {
        // Raw call arguments
        let para_id: u32 = para_id
            .try_into()
            .map_err(|_| revert("error converting para_id, maybe value too large"))?;

        let fee_amount: u128 = fee_amount
            .try_into()
            .map_err(|_| revert("error converting fee_amount, maybe value too large"))?;

        let remote_call: Vec<u8> = remote_call.into();

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

        Ok(true)
    }

    fn assets_reserve_transfer_v1_internal(
        handle: &mut impl PrecompileHandle,
        assets: Vec<Address>,
        amounts: Vec<U256>,
        beneficiary: Junction,
        is_relay: bool,
        parachain_id: U256,
        fee_item: U256,
    ) -> EvmResult<bool> {
        let assets: Vec<MultiLocation> = assets
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

        let amounts: Vec<u128> = amounts
            .into_iter()
            .map(|x| x.try_into())
            .collect::<Result<Vec<u128>, _>>()
            .map_err(|_| revert("error converting amounts, maybe value too large"))?;

        // Check that assets list is valid:
        // * all assets resolved to multi-location
        // * all assets has corresponded amount
        if assets.len() != amounts.len() || assets.is_empty() {
            return Err(revert("Assets resolution failure."));
        }

        let parachain_id: u32 = parachain_id
            .try_into()
            .map_err(|_| revert("error converting parachain_id, maybe value too large"))?;

        let fee_item: u32 = fee_item
            .try_into()
            .map_err(|_| revert("error converting fee_index, maybe value too large"))?;

        // Prepare pallet-xcm call arguments
        let mut destination = if is_relay {
            MultiLocation::parent()
        } else {
            X1(Junction::Parachain(parachain_id)).into_exterior(1)
        };

        destination
            .push_interior(beneficiary)
            .map_err(|_| revert("error building destination multilocation"))?;

        let assets = assets
            .iter()
            .cloned()
            .zip(amounts.iter().cloned())
            .map(Into::into)
            .collect::<Vec<MultiAsset>>();

        log::trace!(target: "xcm-precompile:assets_reserve_transfer", "Processed arguments: assets {:?}, destination: {:?}", assets, destination);

        // Build call with origin.
        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();

        let call = orml_xtokens::Call::<Runtime>::transfer_multiassets {
            assets: Box::new(VersionedMultiAssets::V3(assets.into())),
            fee_item,
            dest: Box::new(VersionedMultiLocation::V3(destination)),
            dest_weight_limit: WeightLimit::Unlimited,
        };

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(true)
    }

    #[precompile::public(
        "assets_reserve_transfer(address[],uint256[],bytes32,bool,uint256,uint256)"
    )]
    fn assets_reserve_transfer_native_v1(
        handle: &mut impl PrecompileHandle,
        assets: BoundedVec<Address, GetMaxAssets<Runtime>>,
        amounts: BoundedVec<U256, GetMaxAssets<Runtime>>,
        recipient_account_id: H256,
        is_relay: bool,
        parachain_id: U256,
        fee_index: U256,
    ) -> EvmResult<bool> {
        let beneficiary: Junction = Junction::AccountId32 {
            network: None,
            id: recipient_account_id.into(),
        }
        .into();
        Self::assets_reserve_transfer_v1_internal(
            handle,
            assets.into(),
            amounts.into(),
            beneficiary,
            is_relay,
            parachain_id,
            fee_index,
        )
    }

    #[precompile::public(
        "assets_reserve_transfer(address[],uint256[],address,bool,uint256,uint256)"
    )]
    fn assets_reserve_transfer_evm_v1(
        handle: &mut impl PrecompileHandle,
        assets: BoundedVec<Address, GetMaxAssets<Runtime>>,
        amounts: BoundedVec<U256, GetMaxAssets<Runtime>>,
        recipient_account_id: Address,
        is_relay: bool,
        parachain_id: U256,
        fee_index: U256,
    ) -> EvmResult<bool> {
        let beneficiary: Junction = Junction::AccountKey20 {
            network: None,
            key: recipient_account_id.0.to_fixed_bytes(),
        }
        .into();
        Self::assets_reserve_transfer_v1_internal(
            handle,
            assets.into(),
            amounts.into(),
            beneficiary,
            is_relay,
            parachain_id,
            fee_index,
        )
    }

    #[precompile::public("send_xcm((uint8,bytes[]),bytes)")]
    fn send_xcm(
        handle: &mut impl PrecompileHandle,
        dest: MultiLocation,
        xcm_call: BoundedBytes<GetXcmSizeLimit>,
    ) -> EvmResult<bool> {
        // Raw call arguments
        let dest: MultiLocation = dest.into();
        let xcm_call: Vec<u8> = xcm_call.into();

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

        Ok(true)
    }

    #[precompile::public("transfer(address,uint256,(uint8,bytes[]),(uint64,uint64))")]
    fn transfer(
        handle: &mut impl PrecompileHandle,
        currency_address: Address,
        amount_of_tokens: U256,
        destination: MultiLocation,
        weight: WeightV2,
    ) -> EvmResult<bool> {
        // Read call arguments
        let amount_of_tokens: u128 = amount_of_tokens
            .try_into()
            .map_err(|_| revert("error converting amount_of_tokens, maybe value too large"))?;

        let dest_weight_limit = if weight.is_zero() {
            WeightLimit::Unlimited
        } else {
            WeightLimit::Limited(weight.get_weight())
        };

        let call = {
            if currency_address == Address::from(NATIVE_ADDRESS) {
                log::trace!(target: "xcm-precompile::transfer", "Raw arguments: currency_address: {:?} (this is native token), amount_of_tokens: {:?}, destination: {:?}, \
                weight: {:?}",
                currency_address, amount_of_tokens, destination, weight );

                orml_xtokens::Call::<Runtime>::transfer_multiasset {
                    asset: Box::new(VersionedMultiAsset::V3(
                        (MultiLocation::here(), amount_of_tokens).into(),
                    )),
                    dest: Box::new(VersionedMultiLocation::V3(destination)),
                    dest_weight_limit,
                }
            } else {
                let asset_id = Runtime::address_to_asset_id(currency_address.into())
                    .ok_or(revert("Failed to resolve fee asset id from address"))?;

                log::trace!(target: "xcm-precompile::transfer", "Raw arguments: currency_address: {:?}, amount_of_tokens: {:?}, destination: {:?}, \
                weight: {:?}, calculated asset_id: {:?}",
                currency_address, amount_of_tokens, destination, weight, asset_id);

                orml_xtokens::Call::<Runtime>::transfer {
                    currency_id: asset_id.into(),
                    amount: amount_of_tokens.into(),
                    dest: Box::new(VersionedMultiLocation::V3(destination)),
                    dest_weight_limit,
                }
            }
        };

        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(true)
    }

    #[precompile::public(
        "transfer_with_fee(address,uint256,uint256,(uint8,bytes[]),(uint64,uint64))"
    )]
    fn transfer_with_fee(
        handle: &mut impl PrecompileHandle,
        currency_address: Address,
        amount_of_tokens: U256,
        fee: U256,
        destination: MultiLocation,
        weight: WeightV2,
    ) -> EvmResult<bool> {
        // Read call arguments
        let amount_of_tokens: u128 = amount_of_tokens
            .try_into()
            .map_err(|_| revert("error converting amount_of_tokens, maybe value too large"))?;
        let fee: u128 = fee.try_into().map_err(|_| revert("can't convert fee"))?;

        let dest_weight_limit = if weight.is_zero() {
            WeightLimit::Unlimited
        } else {
            WeightLimit::Limited(weight.get_weight())
        };

        let call = {
            if currency_address == Address::from(NATIVE_ADDRESS) {
                log::trace!(target: "xcm-precompile::transfer_with_fee", "Raw arguments: currency_address: {:?} (this is native token), amount_of_tokens: {:?}, destination: {:?}, \
                weight: {:?}, fee {:?}",
                currency_address, amount_of_tokens, destination, weight, fee );

                orml_xtokens::Call::<Runtime>::transfer_multiasset_with_fee {
                    asset: Box::new(VersionedMultiAsset::V3(
                        (MultiLocation::here(), amount_of_tokens).into(),
                    )),
                    fee: Box::new(VersionedMultiAsset::V3((MultiLocation::here(), fee).into())),
                    dest: Box::new(VersionedMultiLocation::V3(destination)),
                    dest_weight_limit,
                }
            } else {
                let asset_id = Runtime::address_to_asset_id(currency_address.into())
                    .ok_or(revert("Failed to resolve fee asset id from address"))?;

                log::trace!(target: "xcm-precompile::transfer_with_fee", "Raw arguments: currency_address: {:?}, amount_of_tokens: {:?}, destination: {:?}, \
                weight: {:?}, calculated asset_id: {:?}, fee: {:?}",
                currency_address, amount_of_tokens, destination, weight, asset_id, fee);

                orml_xtokens::Call::<Runtime>::transfer_with_fee {
                    currency_id: asset_id.into(),
                    amount: amount_of_tokens.into(),
                    fee: fee.into(),
                    dest: Box::new(VersionedMultiLocation::V3(destination)),
                    dest_weight_limit,
                }
            }
        };

        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(true)
    }

    #[precompile::public(
        "transfer_multiasset((uint8,bytes[]),uint256,(uint8,bytes[]),(uint64,uint64))"
    )]
    fn transfer_multiasset(
        handle: &mut impl PrecompileHandle,
        asset_location: MultiLocation,
        amount_of_tokens: U256,
        destination: MultiLocation,
        weight: WeightV2,
    ) -> EvmResult<bool> {
        // Read call arguments
        let amount_of_tokens: u128 = amount_of_tokens
            .try_into()
            .map_err(|_| revert("error converting amount_of_tokens, maybe value too large"))?;

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

        Ok(true)
    }

    #[precompile::public(
        "transfer_multiasset_with_fee((uint8,bytes[]),uint256,uint256,(uint8,bytes[]),(uint64,uint64))"
    )]
    fn transfer_multiasset_with_fee(
        handle: &mut impl PrecompileHandle,
        asset_location: MultiLocation,
        amount_of_tokens: U256,
        fee: U256,
        destination: MultiLocation,
        weight: WeightV2,
    ) -> EvmResult<bool> {
        // Read call arguments
        let amount_of_tokens: u128 = amount_of_tokens
            .try_into()
            .map_err(|_| revert("error converting amount_of_tokens, maybe value too large"))?;
        let fee: u128 = fee.try_into().map_err(|_| revert("can't convert fee"))?;

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

        Ok(true)
    }

    #[precompile::public(
        "transfer_multi_currencies((address,uint256)[],uint32,(uint8,bytes[]),(uint64,uint64))"
    )]
    fn transfer_multi_currencies(
        handle: &mut impl PrecompileHandle,
        currencies: BoundedVec<Currency, GetMaxAssets<Runtime>>,
        fee_item: u32,
        destination: MultiLocation,
        weight: WeightV2,
    ) -> EvmResult<bool> {
        let currencies: Vec<_> = currencies.into();
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

        Ok(true)
    }

    #[precompile::public(
        "transfet_multi_assets(((uint8,bytes[]),uint256)[],uint32,(uint8,bytes[]),(uint64,uint64))"
    )]
    fn transfer_multi_assets(
        handle: &mut impl PrecompileHandle,
        assets: BoundedVec<EvmMultiAsset, GetMaxAssets<Runtime>>,
        fee_item: u32,
        destination: MultiLocation,
        weight: WeightV2,
    ) -> EvmResult<bool> {
        let assets: Vec<_> = assets.into();

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

        Ok(true)
    }
}

#[derive(Debug, Clone, solidity::Codec)]
pub struct WeightV2 {
    ref_time: u64,
    proof_size: u64,
}

impl WeightV2 {
    pub fn from(ref_time: u64, proof_size: u64) -> Self {
        WeightV2 {
            ref_time,
            proof_size,
        }
    }

    pub fn get_weight(&self) -> Weight {
        Weight::from_parts(self.ref_time, self.proof_size)
    }

    pub fn is_zero(&self) -> bool {
        self.ref_time == 0u64
    }
}

#[derive(Debug, Clone, solidity::Codec)]
pub struct Currency {
    address: Address,
    amount: U256,
}

impl Currency {
    pub fn get_address(&self) -> Address {
        self.address
    }

    pub fn get_amount(&self) -> U256 {
        self.amount
    }
}

impl From<(Address, U256)> for Currency {
    fn from(tuple: (Address, U256)) -> Self {
        Currency {
            address: tuple.0,
            amount: tuple.1,
        }
    }
}

#[derive(Debug, Clone, solidity::Codec)]
pub struct EvmMultiAsset {
    location: MultiLocation,
    amount: U256,
}

impl From<(MultiLocation, U256)> for EvmMultiAsset {
    fn from(tuple: (MultiLocation, U256)) -> Self {
        EvmMultiAsset {
            location: tuple.0,
            amount: tuple.1,
        }
    }
}

impl EvmMultiAsset {
    pub fn get_location(&self) -> MultiLocation {
        self.location
    }

    pub fn get_amount(&self) -> U256 {
        self.amount
    }
}
