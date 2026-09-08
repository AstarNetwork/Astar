// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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

use astar_primitives::xcm::{
    resolve_transfer_type, split_location_into_chain_part_and_beneficiary, ASSET_HUB_PARA_ID,
    XCM_SIZE_LIMIT,
};
use fp_evm::PrecompileHandle;
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    pallet_prelude::Weight,
    traits::{ConstU32, Get},
};
use sp_runtime::traits::{Dispatchable, MaybeEquivalence};

use pallet_evm::{AddressMapping, GasWeightMapping};
use sp_core::{H160, H256, U256};

use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use xcm::{latest::prelude::*, VersionedAssetId, VersionedAssets, VersionedLocation, VersionedXcm};
use xcm_executor::traits::TransferType;

use pallet_evm_precompile_assets_erc20::AddressToAssetId;
use pallet_xcm::WeightInfo as PalletXcmWeightInfo;
use precompile_utils::prelude::*;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Dummy H160 address representing native currency (e.g. ASTR or SDN)
const NATIVE_ADDRESS: H160 = H160::zero();

/// Bound for the SCALE-encoded XCM blob accepted by the (deprecated) `send_xcm`.
type GetXcmSizeLimit = ConstU32<XCM_SIZE_LIMIT>;

/// Max number of assets a single cross-chain transfer may carry.
pub const MAX_ASSETS_FOR_TRANSFER: u32 = 2;

/// Bound for the `BoundedVec` arguments of the asset-list based methods.
pub type GetMaxAssets = ConstU32<MAX_ASSETS_FOR_TRANSFER>;

/// Default proof_size of 256KB
const DEFAULT_PROOF_SIZE: u64 = 1024 * 256;

/// Bound for the `Transact` call blob `remote_transact` forwards.
pub const REMOTE_CALL_SIZE_LIMIT: u32 = 64 * 1024;

/// Bound for the `remote_call` argument of `remote_transact`.
pub type GetRemoteCallSizeLimit = ConstU32<REMOTE_CALL_SIZE_LIMIT>;

/// Revert reason for `send_xcm`.
const SEND_XCM_UNSUPPORTED: &str =
    "send_xcm is not supported: sending an arbitrary XCM requires Root. \
     Use remote_transact(uint256,bool,address,uint256,bytes,uint64) for a sibling Transact.";

/// A precompile that expose XCM related functions.
pub struct XcmPrecompile<Runtime, C>(PhantomData<(Runtime, C)>);

#[precompile_utils::precompile]
#[precompile::test_concrete_types(mock::Runtime, mock::AssetIdConverter<mock::AssetId>)]
impl<Runtime, C> XcmPrecompile<Runtime, C>
where
    Runtime: pallet_evm::Config
        + pallet_xcm::Config
        + pallet_assets::Config
        + AddressToAssetId<<Runtime as pallet_assets::Config>::AssetId>,
    <<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<Runtime::AccountId>>,
    <Runtime as frame_system::Config>::RuntimeCall: From<pallet_xcm::Call<Runtime>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    C: MaybeEquivalence<Location, <Runtime as pallet_assets::Config>::AssetId>,
    <Runtime as pallet_evm::Config>::AddressMapping: AddressMapping<Runtime::AccountId>,
    Runtime::AccountId: Into<[u8; 32]>,
{
    // ------------------------------------------------------------------------------------------
    // Asset transfers. Every selector below funnels into `do_transfer`, which dispatches
    // `pallet_xcm::transfer_assets_using_type_and_then` - the reserve is derived per asset rather
    // than chosen by the caller.
    // ------------------------------------------------------------------------------------------

    /// Transfer XC20 assets to an `AccountId32` beneficiary on the relay chain or a sibling.
    #[precompile::public("assets_withdraw(address[],uint256[],bytes32,bool,uint256,uint256)")]
    fn assets_withdraw_native_v1(
        handle: &mut impl PrecompileHandle,
        assets: BoundedVec<Address, GetMaxAssets>,
        amounts: BoundedVec<U256, GetMaxAssets>,
        recipient_account_id: H256,
        is_relay: bool,
        parachain_id: U256,
        fee_index: U256,
    ) -> EvmResult<bool> {
        Self::assets_transfer_v1(
            handle,
            assets,
            amounts,
            Self::beneficiary_32(recipient_account_id),
            is_relay,
            parachain_id,
            fee_index,
            false,
        )
    }

    /// As above, with an `AccountKey20` beneficiary. Substrate-native destinations generally
    /// cannot resolve one - prefer the `bytes32` overload unless the destination accepts it.
    #[precompile::public("assets_withdraw(address[],uint256[],address,bool,uint256,uint256)")]
    fn assets_withdraw_evm_v1(
        handle: &mut impl PrecompileHandle,
        assets: BoundedVec<Address, GetMaxAssets>,
        amounts: BoundedVec<U256, GetMaxAssets>,
        recipient_account_id: Address,
        is_relay: bool,
        parachain_id: U256,
        fee_index: U256,
    ) -> EvmResult<bool> {
        Self::assets_transfer_v1(
            handle,
            assets,
            amounts,
            Self::beneficiary_key_20(recipient_account_id),
            is_relay,
            parachain_id,
            fee_index,
            false,
        )
    }

    /// As `assets_withdraw`, except the zero address is read as the native token.
    ///
    /// That is the only difference between the two names, and it has always been so: widening
    /// `assets_withdraw` instead would turn a caller's uninitialised address from a revert into a
    /// native-balance transfer.
    #[precompile::public(
        "assets_reserve_transfer(address[],uint256[],bytes32,bool,uint256,uint256)"
    )]
    fn assets_reserve_transfer_native_v1(
        handle: &mut impl PrecompileHandle,
        assets: BoundedVec<Address, GetMaxAssets>,
        amounts: BoundedVec<U256, GetMaxAssets>,
        recipient_account_id: H256,
        is_relay: bool,
        parachain_id: U256,
        fee_index: U256,
    ) -> EvmResult<bool> {
        Self::assets_transfer_v1(
            handle,
            assets,
            amounts,
            Self::beneficiary_32(recipient_account_id),
            is_relay,
            parachain_id,
            fee_index,
            true,
        )
    }

    /// As `assets_reserve_transfer` above, with an `AccountKey20` beneficiary.
    #[precompile::public(
        "assets_reserve_transfer(address[],uint256[],address,bool,uint256,uint256)"
    )]
    fn assets_reserve_transfer_evm_v1(
        handle: &mut impl PrecompileHandle,
        assets: BoundedVec<Address, GetMaxAssets>,
        amounts: BoundedVec<U256, GetMaxAssets>,
        recipient_account_id: Address,
        is_relay: bool,
        parachain_id: U256,
        fee_index: U256,
    ) -> EvmResult<bool> {
        Self::assets_transfer_v1(
            handle,
            assets,
            amounts,
            Self::beneficiary_key_20(recipient_account_id),
            is_relay,
            parachain_id,
            fee_index,
            true,
        )
    }

    /// Transfer a single token - native currency (zero address) or an XC20 - to a combined
    /// destination location that embeds the beneficiary.
    #[precompile::public("transfer(address,uint256,(uint8,bytes[]),(uint64,uint64))")]
    fn transfer(
        handle: &mut impl PrecompileHandle,
        currency_address: Address,
        amount_of_tokens: U256,
        destination: Location,
        weight: WeightV2,
    ) -> EvmResult<bool> {
        let currency_address: H160 = currency_address.into();
        // Special case where zero address maps to native token by convention.
        let asset_location = if currency_address == NATIVE_ADDRESS {
            Location::here()
        } else {
            let asset_id = Runtime::address_to_asset_id(currency_address)
                .ok_or(revert("Failed to resolve asset id from address"))?;
            C::convert_back(&asset_id).ok_or(revert(
                "Failed to resolve asset multilocation from local id",
            ))?
        };
        let asset: Asset = (asset_location, Self::amount(amount_of_tokens)?).into();

        Self::transfer_to_combined_destination(handle, asset.into(), 0, destination, weight)
    }

    /// As `transfer`, with the asset named by its location instead of its XC20 address.
    #[precompile::public(
        "transfer_multiasset((uint8,bytes[]),uint256,(uint8,bytes[]),(uint64,uint64))"
    )]
    fn transfer_multiasset(
        handle: &mut impl PrecompileHandle,
        asset_location: Location,
        amount_of_tokens: U256,
        destination: Location,
        weight: WeightV2,
    ) -> EvmResult<bool> {
        let asset: Asset = (asset_location, Self::amount(amount_of_tokens)?).into();

        Self::transfer_to_combined_destination(handle, asset.into(), 0, destination, weight)
    }

    /// Transfer several XC20 assets to a combined destination location, `fee_item` naming the one
    /// that pays for execution there.
    #[precompile::public(
        "transfer_multi_currencies((address,uint256)[],uint32,(uint8,bytes[]),(uint64,uint64))"
    )]
    fn transfer_multi_currencies(
        handle: &mut impl PrecompileHandle,
        currencies: BoundedVec<Currency, GetMaxAssets>,
        fee_item: u32,
        destination: Location,
        weight: WeightV2,
    ) -> EvmResult<bool> {
        let currencies: Vec<Currency> = currencies.into();
        let unsorted = currencies
            .into_iter()
            .map(|currency| {
                Ok((
                    Self::asset_location(currency.get_address().into(), false)
                        .ok_or(revert("can't convert into currency id"))?,
                    Self::amount(currency.get_amount())?,
                )
                    .into())
            })
            .collect::<EvmResult<Vec<Asset>>>()?;

        let assets: Assets = unsorted.clone().into();
        let fee_item = Self::fee_index_after_sort(&unsorted, &assets, fee_item)?;

        Self::transfer_to_combined_destination(handle, assets, fee_item, destination, weight)
    }

    /// As `transfer_multi_currencies`, with the assets named by their locations.
    ///
    /// The list must already be sorted and deduplicated, since `fee_item` indexes it.
    #[precompile::public(
        "transfer_multi_assets(((uint8,bytes[]),uint256)[],uint32,(uint8,bytes[]),(uint64,uint64))"
    )]
    fn transfer_multi_assets(
        handle: &mut impl PrecompileHandle,
        assets: BoundedVec<EvmMultiAsset, GetMaxAssets>,
        fee_item: u32,
        destination: Location,
        weight: WeightV2,
    ) -> EvmResult<bool> {
        let assets: Vec<EvmMultiAsset> = assets.into();
        let assets = assets
            .into_iter()
            .map(|asset| Ok((asset.get_location(), Self::amount(asset.get_amount())?).into()))
            .collect::<EvmResult<Vec<Asset>>>()?;

        let assets = Assets::from_sorted_and_deduplicated(assets).map_err(|_| {
            revert("In field Assets, Provided assets either not sorted nor deduplicated")
        })?;

        Self::transfer_to_combined_destination(handle, assets, fee_item, destination, weight)
    }

    /// As `transfer`, with the destination's fee named separately.
    #[precompile::public(
        "transfer_with_fee(address,uint256,uint256,(uint8,bytes[]),(uint64,uint64))"
    )]
    fn transfer_with_fee(
        handle: &mut impl PrecompileHandle,
        currency_address: Address,
        amount_of_tokens: U256,
        fee: U256,
        destination: Location,
        weight: WeightV2,
    ) -> EvmResult<bool> {
        Self::transfer(
            handle,
            currency_address,
            Self::total_with_fee(amount_of_tokens, fee)?,
            destination,
            weight,
        )
    }

    /// As `transfer_with_fee`, with the asset named by its location.
    #[precompile::public(
        "transfer_multiasset_with_fee((uint8,bytes[]),uint256,uint256,(uint8,bytes[]),(uint64,uint64))"
    )]
    fn transfer_multiasset_with_fee(
        handle: &mut impl PrecompileHandle,
        asset_location: Location,
        amount_of_tokens: U256,
        fee: U256,
        destination: Location,
        weight: WeightV2,
    ) -> EvmResult<bool> {
        Self::transfer_multiasset(
            handle,
            asset_location,
            Self::total_with_fee(amount_of_tokens, fee)?,
            destination,
            weight,
        )
    }

    // ------------------------------------------------------------------------------------------
    // Remote execution.
    // ------------------------------------------------------------------------------------------

    /// Send a `Transact` to a sibling parachain, buying its execution with `fee_asset_addr`.
    ///
    /// `pallet_xcm::send` is `Root`-only, so the message goes to the router directly with the
    /// caller descended into the origin - byte for byte what the extrinsic built for a signed
    /// origin, so the account the destination derives does not move.
    #[precompile::public("remote_transact(uint256,bool,address,uint256,bytes,uint64)")]
    fn remote_transact_v1(
        handle: &mut impl PrecompileHandle,
        para_id: U256,
        is_relay: bool,
        fee_asset_addr: Address,
        fee_amount: U256,
        remote_call: BoundedBytes<GetRemoteCallSizeLimit>,
        transact_weight: u64,
    ) -> EvmResult<bool> {
        if is_relay {
            return Err(revert(
                "remote_transact to the relay chain is not supported, use a sibling parachain",
            ));
        }

        let dest = Self::chain_part(false, para_id)?;
        let remote_call: Vec<u8> = remote_call.into();
        let remote_call_len = remote_call.len() as u64;

        let fee_asset_addr: H160 = fee_asset_addr.into();
        // Special case where zero address maps to native token by convention.
        let fee_asset = if fee_asset_addr == NATIVE_ADDRESS {
            Location::here()
        } else {
            let fee_asset_id = Runtime::address_to_asset_id(fee_asset_addr)
                .ok_or(revert("Failed to resolve fee asset id from address"))?;
            C::convert_back(&fee_asset_id).ok_or(revert(
                "Failed to resolve fee asset multilocation from local id",
            ))?
        };
        let fee: Asset = (fee_asset, Self::amount(fee_amount)?).into();

        let context = <Runtime as pallet_xcm::Config>::UniversalLocation::get();
        let fee = fee
            .reanchored(&dest, &context)
            .map_err(|_| revert("Failed to reanchor fee asset"))?;

        let message = Xcm(vec![
            WithdrawAsset(fee.clone().into()),
            BuyExecution {
                fees: fee,
                weight_limit: WeightLimit::Unlimited,
            },
            Transact {
                origin_kind: OriginKind::SovereignAccount,
                fallback_max_weight: Some(Weight::from_parts(transact_weight, DEFAULT_PROOF_SIZE)),
                call: remote_call.into(),
            },
        ]);

        // The interior `pallet_xcm::send` derived for a signed origin: `SignedToAccountId32` with
        // the network this chain lives in, which `UniversalLocation` already carries. Refuse to
        // guess it - `network: None` is a different location to the destination, so it would
        // silently move every caller's derived sovereign account there.
        let network = context.global_consensus().map_err(|_| {
            revert(
                "UniversalLocation carries no global consensus: cannot derive the caller's origin",
            )
        })?;
        let interior = Junction::AccountId32 {
            network: Some(network),
            id: Runtime::AddressMapping::into_account_id(handle.context().caller).into(),
        };

        log::trace!(target: "xcm-precompile:remote_transact", "dest: {:?}, interior: {:?}, message: {:?}", dest, interior, message);

        let weight = <Runtime as pallet_xcm::Config>::WeightInfo::send()
            .saturating_add(Weight::from_parts(0, remote_call_len));
        RuntimeHelper::<Runtime>::record_external_cost(handle, weight, 0)?;
        handle.record_cost(
            <Runtime as pallet_evm::Config>::GasWeightMapping::weight_to_gas(weight),
        )?;

        pallet_xcm::Pallet::<Runtime>::send_xcm(interior, dest, message).map_err(|error| {
            log::trace!(target: "xcm-precompile:remote_transact", "send_xcm failed: {:?}", error);
            revert("Failed to send xcm")
        })?;

        Ok(true)
    }

    // ------------------------------------------------------------------------------------------
    // Deprecated selectors.
    // ------------------------------------------------------------------------------------------

    /// An arbitrary XCM to an arbitrary destination from a signed origin is what
    /// `SendXcmOrigin` was locked down to `Root` to prevent.
    #[precompile::public("send_xcm((uint8,bytes[]),bytes)")]
    fn send_xcm(
        handle: &mut impl PrecompileHandle,
        dest: Location,
        xcm_call: BoundedBytes<GetXcmSizeLimit>,
    ) -> EvmResult<bool> {
        let _ = (handle, dest, xcm_call);
        Err(revert(SEND_XCM_UNSUPPORTED))
    }

    // ------------------------------------------------------------------------------------------
    // Internals.
    // ------------------------------------------------------------------------------------------

    /// Shared body of the four `assets_*` selectors, which name the destination chain with the
    /// `is_relay` / `parachain_id` pair and carry the beneficiary separately.
    ///
    /// `native_address` is the one behavioural difference between the two names: only
    /// `assets_reserve_transfer` has ever read the zero address as the native token.
    fn assets_transfer_v1(
        handle: &mut impl PrecompileHandle,
        assets: BoundedVec<Address, GetMaxAssets>,
        amounts: BoundedVec<U256, GetMaxAssets>,
        beneficiary: Location,
        is_relay: bool,
        parachain_id: U256,
        fee_index: U256,
        native_address: bool,
    ) -> EvmResult<bool> {
        let addresses: Vec<Address> = assets.into();
        let locations = addresses
            .into_iter()
            .filter_map(|address| Self::asset_location(address.into(), native_address))
            .collect::<Vec<Location>>();

        let amounts: Vec<U256> = amounts.into();
        let amounts = amounts
            .into_iter()
            .map(Self::amount)
            .collect::<EvmResult<Vec<u128>>>()?;

        // Check that assets list is valid:
        // * all assets resolved to multi-location
        // * all assets has corresponded amount
        if locations.len() != amounts.len() || locations.is_empty() {
            return Err(revert("Assets resolution failure."));
        }

        let assets = locations
            .into_iter()
            .zip(amounts)
            .map(Into::into)
            .collect::<Vec<Asset>>();

        let fee_index: u32 = fee_index
            .try_into()
            .map_err(|_| revert("error converting fee_index, maybe value too large"))?;

        Self::do_transfer(
            handle,
            assets.into(),
            fee_index,
            Self::chain_part(is_relay, parachain_id)?,
            beneficiary,
            WeightLimit::Unlimited,
        )
    }

    /// Shared body of the selectors that take one location holding both the destination chain and
    /// the beneficiary.
    fn transfer_to_combined_destination(
        handle: &mut impl PrecompileHandle,
        assets: Assets,
        fee_index: u32,
        destination: Location,
        weight: WeightV2,
    ) -> EvmResult<bool> {
        let (dest, beneficiary) = split_location_into_chain_part_and_beneficiary(destination)
            .ok_or(revert(
                "error splitting destination into chain and beneficiary",
            ))?;

        // Without one the destination deposits to itself and the assets are trapped there.
        if beneficiary == Location::here() {
            return Err(revert(
                "destination carries no beneficiary: append the recipient junction to it",
            ));
        }

        Self::do_transfer(
            handle,
            assets,
            fee_index,
            dest,
            beneficiary,
            Self::weight_limit(&weight)?,
        )
    }

    /// Resolve the reserves and hand the transfer to pallet-xcm.
    fn do_transfer(
        handle: &mut impl PrecompileHandle,
        assets: Assets,
        fee_index: u32,
        dest: Location,
        beneficiary: Location,
        weight_limit: WeightLimit,
    ) -> EvmResult<bool> {
        if assets.len() == 0 {
            return Err(revert("Assets resolution failure."));
        }

        let dest = Self::redirect_relay_to_asset_hub(&assets, dest);
        Self::ensure_dot_transfer_policy(assets.inner(), &dest)?;

        let (assets_transfer_type, fees_transfer_type, fee_asset_id) =
            Self::resolve_transfer_types(&assets, fee_index, &dest)?;

        log::trace!(target: "xcm-precompile:transfer", "assets: {:?}, dest: {:?}, beneficiary: {:?}, transfer types: {:?}/{:?}", assets, dest, beneficiary, assets_transfer_type, fees_transfer_type);

        let call = pallet_xcm::Call::<Runtime>::transfer_assets_using_type_and_then {
            dest: Box::new(VersionedLocation::V5(dest)),
            assets: Box::new(VersionedAssets::V5(assets.clone())),
            assets_transfer_type: Box::new(assets_transfer_type),
            remote_fees_id: Box::new(VersionedAssetId::V5(fee_asset_id)),
            fees_transfer_type: Box::new(fees_transfer_type),
            custom_xcm_on_dest: Box::new(VersionedXcm::V5(Self::deposit_to_beneficiary(
                assets.len() as u32,
                beneficiary,
            ))),
            weight_limit,
        };

        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();

        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call, 0)?;

        Ok(true)
    }

    /// The relay chain holds no reserve for this chain's own token - Asset Hub does. Substitute
    /// the destination rather than depositing assets on a chain that cannot account for them
    fn redirect_relay_to_asset_hub(assets: &Assets, dest: Location) -> Location {
        let local_asset_present = assets
            .inner()
            .iter()
            .any(|asset| asset.id.0 == Location::here());

        if dest == Location::parent() && local_asset_present {
            Location::new(1, [Junction::Parachain(ASSET_HUB_PARA_ID)])
        } else {
            dest
        }
    }

    /// The destination chain named by the legacy `is_relay` / `parachain_id` pair.
    fn chain_part(is_relay: bool, parachain_id: U256) -> EvmResult<Location> {
        if is_relay {
            return Ok(Location::parent());
        }

        let parachain_id: u32 = parachain_id
            .try_into()
            .map_err(|_| revert("error converting parachain_id, maybe value too large"))?;

        Ok(Junctions::from(Junction::Parachain(parachain_id)).into_exterior(1))
    }

    /// XC20 address to asset location. `native_address` allows the zero address to stand for the
    /// native token, which only some selectors have ever accepted.
    ///
    /// Returns `None` rather than reverting: each selector keeps the wording it has always used.
    fn asset_location(address: H160, native_address: bool) -> Option<Location> {
        if native_address && address == NATIVE_ADDRESS {
            return Some(Location::here());
        }

        Runtime::address_to_asset_id(address).and_then(|id| C::convert_back(&id))
    }

    fn amount(amount: U256) -> EvmResult<u128> {
        let amount: u128 = amount
            .try_into()
            .map_err(|_| revert("error converting amount, maybe value too large"))?;

        if amount == 0 {
            return Err(revert("amount must be greater than zero"));
        }

        Ok(amount)
    }

    fn total_with_fee(amount_of_tokens: U256, fee: U256) -> EvmResult<U256> {
        amount_of_tokens
            .checked_add(fee)
            .ok_or(revert("error adding fee to amount, maybe value too large"))
    }

    /// The `WeightLimit` a caller's `(ref_time, proof_size)` pair asks for.
    ///
    /// `(0, 0)` is the documented spelling of `Unlimited`. Neither mixed form is reinterpreted:
    /// `(0, n)` would silently drop the caller's proof-size limit, and `(n, 0)` is weighed as
    /// overweight by every destination, which strands the assets there instead of here.
    fn weight_limit(weight: &WeightV2) -> EvmResult<WeightLimit> {
        match (weight.ref_time, weight.proof_size) {
            (0, 0) => Ok(WeightLimit::Unlimited),
            (0, _) => Err(revert(
                "weight.ref_time is zero but weight.proof_size is not: pass (0, 0) for an \
                 unlimited weight limit",
            )),
            (_, 0) => Err(revert(
                "weight.proof_size is zero: every destination weighs a message with a non-zero \
                 proof size, so the transfer would be rejected there as overweight",
            )),
            (ref_time, proof_size) => Ok(WeightLimit::Limited(Weight::from_parts(
                ref_time, proof_size,
            ))),
        }
    }

    /// `fee_item` indexes the list in the order the caller wrote it, but `Assets` sorts and merges
    /// what it is built from. Map the caller's index onto the sorted list rather than letting it
    /// slide onto a neighbouring asset.
    fn fee_index_after_sort(unsorted: &[Asset], sorted: &Assets, fee_item: u32) -> EvmResult<u32> {
        let fee_asset_id = unsorted
            .get(fee_item as usize)
            .ok_or(revert("fee_index is out of bounds of the assets list"))?
            .id
            .clone();

        sorted
            .inner()
            .iter()
            .position(|asset| asset.id == fee_asset_id)
            .map(|index| index as u32)
            .ok_or(revert("fee_index is out of bounds of the assets list"))
    }

    /// `AccountId32` beneficiary, as the `bytes32` overloads take it.
    fn beneficiary_32(recipient_account_id: H256) -> Location {
        Junction::AccountId32 {
            network: None,
            id: recipient_account_id.into(),
        }
        .into()
    }

    /// `AccountKey20` beneficiary, as the `address` overloads take it.
    fn beneficiary_key_20(recipient_account_id: Address) -> Location {
        Junction::AccountKey20 {
            network: None,
            key: recipient_account_id.0.to_fixed_bytes(),
        }
        .into()
    }

    /// Picks the reserve model for `assets` and, separately, for the fee asset at
    /// `fee_asset_item`.
    fn resolve_transfer_types(
        assets: &Assets,
        fee_asset_item: u32,
        dest: &Location,
    ) -> EvmResult<(TransferType, TransferType, AssetId)> {
        let assets = assets.inner();
        let fee_asset = assets
            .get(fee_asset_item as usize)
            .ok_or(revert("fee_index is out of bounds of the assets list"))?;

        let resolve = |asset: &Asset| {
            resolve_transfer_type::<<Runtime as pallet_xcm::Config>::XcmExecutor>(asset, dest)
                .ok_or(revert("cannot determine the reserve location for asset"))
        };

        let fees_transfer_type = resolve(fee_asset)?;

        let mut assets_transfer_type = None;
        for (idx, asset) in assets.iter().enumerate() {
            if idx == fee_asset_item as usize {
                continue;
            }
            let transfer_type = resolve(asset)?;
            match &assets_transfer_type {
                Some(existing) if existing != &transfer_type => {
                    return Err(revert("all non-fee assets must share the same reserve"))
                }
                Some(_) => {}
                None => assets_transfer_type = Some(transfer_type),
            }
        }

        // A lone asset also acts as the fee asset.
        let assets_transfer_type =
            assets_transfer_type.unwrap_or_else(|| fees_transfer_type.clone());

        Ok((
            assets_transfer_type,
            fees_transfer_type,
            fee_asset.id.clone(),
        ))
    }

    /// The XCM executed on the destination chain: hand everything that survived the transfer to the
    /// beneficiary.
    fn deposit_to_beneficiary(assets_count: u32, beneficiary: Location) -> Xcm<()> {
        Xcm(vec![DepositAsset {
            assets: Wild(AllCounted(assets_count)),
            beneficiary,
        }])
    }

    /// Enforces DOT transfer routing policy.
    ///
    /// Currently prevents direct DOT transfers to the relay chain,
    /// requiring routing through AssetHub (parachain 1000).
    ///
    /// `dest_chain` is the destination *chain* location - it must not carry the beneficiary.
    fn ensure_dot_transfer_policy(assets: &[Asset], dest_chain: &Location) -> EvmResult<()> {
        if dest_chain != &Location::parent() {
            return Ok(());
        }

        let deprecated_dot_location = Location::new(1, Junctions::Here);

        for asset in assets {
            let AssetId(location) = &asset.id;
            if location == &deprecated_dot_location {
                return Err(revert(
                    "DOT cannot be sent directly to the relay. \
                 Route via AssetHub (parachain 1000).",
                ));
            }
        }

        Ok(())
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
    location: Location,
    amount: U256,
}

impl From<(Location, U256)> for EvmMultiAsset {
    fn from(tuple: (Location, U256)) -> Self {
        EvmMultiAsset {
            location: tuple.0,
            amount: tuple.1,
        }
    }
}

impl EvmMultiAsset {
    pub fn get_location(&self) -> Location {
        self.location.clone()
    }

    pub fn get_amount(&self) -> U256 {
        self.amount
    }
}
