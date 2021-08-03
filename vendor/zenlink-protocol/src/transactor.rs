// Copyright 2020-2021 Zenlink
// Licensed under GPL-3.0.

//! # XCMP Support
//!
//! Includes an implementation for the `TransactAsset` trait, thus enabling
//! withdrawals and deposits to assets via XCMP message execution.
#![allow(unused_variables)]

use super::*;

/// Asset transaction errors.
enum Error {
	/// `MultiLocation` to `AccountId` Conversion failed.
	AccountIdConversionFailed,
	/// Zenlink only use X4 format xcm
	XcmNotX4Format,
	/// Zenlink only use MultiAssetHandler::ConcreteFungible
	XcmNotConcreteFungible,
}

impl From<Error> for XcmError {
	fn from(e: Error) -> Self {
		match e {
			Error::AccountIdConversionFailed => XcmError::FailedToTransactAsset("AccountIdConversionFailed"),
			Error::XcmNotX4Format => XcmError::FailedToTransactAsset("XcmNotX4Format"),
			Error::XcmNotConcreteFungible => XcmError::FailedToTransactAsset("XcmNotConcreteFungible"),
		}
	}
}

pub struct TrustedParas<ParaChains>(PhantomData<ParaChains>);

impl<ParaChains: Get<Vec<(MultiLocation, u128)>>> FilterAssetLocation for TrustedParas<ParaChains> {
	fn filter_asset_location(_asset: &MultiAsset, origin: &MultiLocation) -> bool {
		log::info!(target: LOG_TARGET, "filter_asset_location: origin = {:?}", origin);

		ParaChains::get()
			.iter()
			.map(|(location, _)| location)
			.any(|l| *l == *origin)
	}
}

pub struct TransactorAdaptor<ZenlinkAssets, AccountIdConverter, AccountId>(
	PhantomData<(ZenlinkAssets, AccountIdConverter, AccountId)>,
);

impl<
		ZenlinkAssets: MultiAssetsHandler<AccountId>,
		AccountIdConverter: Convert<MultiLocation, AccountId>,
		AccountId: sp_std::fmt::Debug + Clone,
	> TransactAsset for TransactorAdaptor<ZenlinkAssets, AccountIdConverter, AccountId>
{
	fn deposit_asset(asset: &MultiAsset, who: &MultiLocation) -> XcmResult {
		log::info!(
			target: LOG_TARGET,
			"deposit_asset: asset = {:?}, who = {:?}",
			asset,
			who,
		);

		let who = AccountIdConverter::convert_ref(who).map_err(|()| Error::AccountIdConversionFailed)?;

		match asset {
			MultiAsset::ConcreteFungible { id, amount } => {
				if let Some(asset_id) = multilocation_to_asset(id) {
					ZenlinkAssets::deposit(asset_id, &who, *amount)
						.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;

					Ok(())
				} else {
					Err(XcmError::from(Error::XcmNotX4Format))
				}
			}
			_ => Err(XcmError::from(Error::XcmNotConcreteFungible)),
		}
	}

	fn withdraw_asset(asset: &MultiAsset, who: &MultiLocation) -> Result<Assets, XcmError> {
		log::info!(
			target: LOG_TARGET,
			"withdraw_asset: asset = {:?}, who = {:?}",
			asset,
			who,
		);

		let who = AccountIdConverter::convert_ref(who).map_err(|()| Error::AccountIdConversionFailed)?;

		match asset {
			MultiAsset::ConcreteFungible { id, amount } => {
				if let Some(asset_id) = multilocation_to_asset(id) {
					ZenlinkAssets::withdraw(asset_id, &who, *amount)
						.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;

					Ok(asset.clone().into())
				} else {
					Err(XcmError::from(Error::XcmNotX4Format))
				}
			}
			_ => Err(XcmError::from(Error::XcmNotConcreteFungible)),
		}
	}
}

fn multilocation_to_asset(location: &MultiLocation) -> Option<AssetId> {
	match location {
		MultiLocation::X4(
			Junction::Parent,
			Junction::Parachain(chain_id),
			Junction::PalletInstance(asset_type),
			Junction::GeneralIndex { id: asset_index },
		) => Some(AssetId {
			chain_id: *chain_id,
			asset_type: *asset_type,
			asset_index: (*asset_index) as u32,
		}),
		_ => None,
	}
}
