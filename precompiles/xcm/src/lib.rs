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
    resolve_transfer_type, split_location_into_chain_part_and_beneficiary, XCM_SIZE_LIMIT,
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

/// Proof size allowance for the `Transact` weight `remote_transact` sends.
const DEFAULT_PROOF_SIZE: u64 = 1024 * 256;

/// Bound for the SCALE-encoded XCM blob accepted by the (deprecated) `send_xcm`.
type GetXcmSizeLimit = ConstU32<XCM_SIZE_LIMIT>;

/// Max number of assets a single cross-chain transfer may carry.
pub const MAX_ASSETS_FOR_TRANSFER: u32 = 2;

/// Bound for the `BoundedVec` arguments of the asset-list based methods.
pub type GetMaxAssets = ConstU32<MAX_ASSETS_FOR_TRANSFER>;

/// Revert reason shared by every deprecated selector.
const DEPRECATED: &str =
    "deprecated: xtokens has been removed. Use assets_withdraw(address[],uint256[],bytes32,bool,uint256,uint256) \
     or transfer(address,uint256,(uint8,bytes[]),(uint64,uint64))";

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
    /// Reserve-transfer a list of XC20 assets to an `AccountId32` beneficiary on the relay chain
    /// or on a sibling parachain.
    ///
    /// The zero address is rejected here: only `assets_reserve_transfer` has ever read it as the
    /// native token, and widening this one would turn a caller's uninitialised address from a
    /// revert into a native-balance transfer.
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
        Self::assets_transfer_v1_internal(
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

    /// As `assets_withdraw` above, except that the zero address is read as the native token.
    ///
    /// Both selectors dispatched the very same `orml-xtokens::transfer_multiassets` call before
    /// xtokens was removed - the reserve is derived per asset, it is never picked by the caller -
    /// and this native-address split is the one behavioural difference they have always had.
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
        Self::assets_transfer_v1_internal(
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

    /// `assets_withdraw` with an `AccountKey20` beneficiary, for destinations that map EVM
    /// addresses to accounts. Substrate-native chains generally do not - prefer the `bytes32`
    /// overload unless the destination is known to accept `AccountKey20`.
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
        Self::assets_transfer_v1_internal(
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

    /// As the `address` overload of `assets_withdraw` above, except that the zero address is
    /// read as the native token.
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
        Self::assets_transfer_v1_internal(
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

    /// Send a `Transact` to a sibling parachain, buying its execution with `fee_asset_addr`.
    ///
    /// `pallet_xcm::send` is `Root`-only, so the message is handed to the router directly with
    /// the caller descended into the origin - byte for byte what the extrinsic used to build for
    /// a signed origin, so the account the destination derives is unchanged.
    ///
    /// The relay chain is not reachable this way, by design: `PendingUpwardMessages` is a single
    /// value that mandatory `on_finalize` proves in full, so an unmetered UMP queue is the one
    /// that can push a candidate past `max_pov_size`. Sibling XCMP is paginated, never enters the
    /// PoV, and is priced on top.
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
        if is_relay {
            return Err(revert(
                "remote_transact to the relay chain is not supported, use a sibling parachain",
            ));
        }

        let para_id: u32 = para_id
            .try_into()
            .map_err(|_| revert("error converting para_id, maybe value too large"))?;

        let fee_amount: u128 = fee_amount
            .try_into()
            .map_err(|_| revert("error converting fee_amount, maybe value too large"))?;

        let remote_call: Vec<u8> = remote_call.into();

        let dest = Junctions::from(Junction::Parachain(para_id)).into_exterior(1);

        let fee_asset = {
            let address: H160 = fee_asset_addr.into();

            // Special case where zero address maps to native token by convention.
            if address == NATIVE_ADDRESS {
                Location::here()
            } else {
                let fee_asset_id = Runtime::address_to_asset_id(address)
                    .ok_or(revert("Failed to resolve fee asset id from address"))?;
                C::convert_back(&fee_asset_id).ok_or(revert(
                    "Failed to resolve fee asset multilocation from local id",
                ))?
            }
        };

        let context = <Runtime as pallet_xcm::Config>::UniversalLocation::get();
        let fee: Asset = (fee_asset, fee_amount).into();
        let fee = fee
            .reanchored(&dest, &context)
            .map_err(|_| revert("Failed to reanchor fee asset"))?;

        let xcm = Xcm(vec![
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
        // the network this chain lives in, which `UniversalLocation` already carries.
        let interior = Junction::AccountId32 {
            network: context.global_consensus().ok(),
            id: Runtime::AddressMapping::into_account_id(handle.context().caller).into(),
        };

        log::trace!(target: "xcm-precompile:remote_transact", "dest: {:?}, interior: {:?}, xcm: {:?}", dest, interior, xcm);

        // Nothing is dispatched any more, so meter the send explicitly.
        handle.record_cost(
            <Runtime as pallet_evm::Config>::GasWeightMapping::weight_to_gas(
                <Runtime as pallet_xcm::Config>::WeightInfo::send(),
            ),
        )?;

        pallet_xcm::Pallet::<Runtime>::send_xcm(interior, dest, xcm).map_err(|error| {
            log::trace!(target: "xcm-precompile:remote_transact", "send_xcm failed: {:?}", error);
            revert("Failed to send xcm")
        })?;

        Ok(true)
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

    /// Shared body of the four asset transfer selectors. `native_address` reflects the one
    /// difference between them: `assets_reserve_transfer` reads the zero address as the native
    /// token, `assets_withdraw` does not.
    fn assets_transfer_v1_internal(
        handle: &mut impl PrecompileHandle,
        assets: BoundedVec<Address, GetMaxAssets>,
        amounts: BoundedVec<U256, GetMaxAssets>,
        beneficiary: Location,
        is_relay: bool,
        parachain_id: U256,
        fee_index: U256,
        native_address: bool,
    ) -> EvmResult<bool> {
        // Read arguments and check them
        let assets: Vec<Address> = assets.into();
        let assets = assets
            .iter()
            .cloned()
            .filter_map(|address| {
                let address: H160 = address.into();
                // Special case where zero address maps to native token by convention.
                if native_address && address == NATIVE_ADDRESS {
                    Some(Location::here())
                } else {
                    Runtime::address_to_asset_id(address).and_then(|x| C::convert_back(&x))
                }
            })
            .collect::<Vec<Location>>();

        let amounts: Vec<U256> = amounts.into();
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

        let fee_asset_item: u32 = fee_index
            .try_into()
            .map_err(|_| revert("error converting fee_index, maybe value too large"))?;

        let destination = if is_relay {
            Location::parent()
        } else {
            Junctions::from(Junction::Parachain(parachain_id)).into_exterior(1)
        };

        // `Assets` sorts and deduplicates on construction, so `fee_asset_item` has to be resolved
        // against the sorted list - same as `orml-xtokens` did.
        let assets: Assets = assets
            .into_iter()
            .zip(amounts)
            .map(Into::into)
            .collect::<Vec<Asset>>()
            .into();

        Self::ensure_dot_transfer_policy(assets.inner(), &destination)?;

        let (assets_transfer_type, fees_transfer_type, fee_asset_id) =
            Self::resolve_transfer_types(&assets, fee_asset_item, &destination)?;

        log::trace!(target: "xcm-precompile:assets_transfer", "Processed arguments: assets {:?}, destination: {:?}, beneficiary: {:?}, transfer types: {:?}/{:?}", assets, destination, beneficiary, assets_transfer_type, fees_transfer_type);

        // Build call with origin.
        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();

        let call = pallet_xcm::Call::<Runtime>::transfer_assets_using_type_and_then {
            dest: Box::new(VersionedLocation::V5(destination)),
            assets: Box::new(VersionedAssets::V5(assets.clone())),
            assets_transfer_type: Box::new(assets_transfer_type),
            remote_fees_id: Box::new(VersionedAssetId::V5(fee_asset_id)),
            fees_transfer_type: Box::new(fees_transfer_type),
            custom_xcm_on_dest: Box::new(VersionedXcm::V5(Self::deposit_to_beneficiary(
                assets.len() as u32,
                beneficiary,
            ))),
            weight_limit: WeightLimit::Unlimited,
        };

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call, 0)?;
        Ok(true)
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
        // Read call arguments
        let amount_of_tokens: u128 = amount_of_tokens
            .try_into()
            .map_err(|_| revert("error converting amount_of_tokens, maybe value too large"))?;

        let weight_limit = if weight.is_zero() {
            WeightLimit::Unlimited
        } else {
            WeightLimit::Limited(weight.get_weight())
        };

        // Special case where zero address maps to native token by convention.
        let asset_location = if currency_address == Address::from(NATIVE_ADDRESS) {
            Location::here()
        } else {
            let asset_id = Runtime::address_to_asset_id(currency_address.into())
                .ok_or(revert("Failed to resolve asset id from address"))?;
            C::convert_back(&asset_id).ok_or(revert(
                "Failed to resolve asset multilocation from local id",
            ))?
        };
        let asset: Asset = (asset_location, amount_of_tokens).into();

        let (dest, beneficiary) = split_location_into_chain_part_and_beneficiary(destination)
            .ok_or(revert(
                "error splitting destination into chain and beneficiary",
            ))?;

        let assets: Assets = asset.into();
        Self::ensure_dot_transfer_policy(assets.inner(), &dest)?;

        let (assets_transfer_type, fees_transfer_type, fee_asset_id) =
            Self::resolve_transfer_types(&assets, 0, &dest)?;

        log::trace!(target: "xcm-precompile::transfer", "Processed arguments: currency_address: {:?}, assets: {:?}, dest: {:?}, beneficiary: {:?}, weight_limit: {:?}, transfer type: {:?}",
        currency_address, assets, dest, beneficiary, weight_limit, assets_transfer_type);

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

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call, 0)?;

        Ok(true)
    }

    // ------------------------------------------------------------------------------------------
    // Deprecated methods.
    // ------------------------------------------------------------------------------------------

    /// Deprecated. Was already unreachable: `SendXcmOrigin` rejects signed origins.
    #[precompile::public("send_xcm((uint8,bytes[]),bytes)")]
    fn send_xcm(
        handle: &mut impl PrecompileHandle,
        dest: Location,
        xcm_call: BoundedBytes<GetXcmSizeLimit>,
    ) -> EvmResult<bool> {
        let _ = (handle, dest, xcm_call);
        Err(revert(DEPRECATED))
    }

    /// Deprecated. Use `transfer` and let the destination charge fees from the transferred asset.
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
        let _ = (
            handle,
            currency_address,
            amount_of_tokens,
            fee,
            destination,
            weight,
        );
        Err(revert(DEPRECATED))
    }

    /// Deprecated. Use `transfer` with the asset's XC20 address.
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
        let _ = (
            handle,
            asset_location,
            amount_of_tokens,
            destination,
            weight,
        );
        Err(revert(DEPRECATED))
    }

    /// Deprecated. Use `transfer` with the asset's XC20 address.
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
        let _ = (
            handle,
            asset_location,
            amount_of_tokens,
            fee,
            destination,
            weight,
        );
        Err(revert(DEPRECATED))
    }

    /// Deprecated. Use `assets_withdraw` for multi-asset transfers.
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
        let _ = (handle, currencies, fee_item, destination, weight);
        Err(revert(DEPRECATED))
    }

    /// Deprecated. Use `assets_withdraw` for multi-asset transfers.
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
        let _ = (handle, assets, fee_item, destination, weight);
        Err(revert(DEPRECATED))
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
