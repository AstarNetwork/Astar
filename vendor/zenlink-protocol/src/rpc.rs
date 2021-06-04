// Copyright 2020-2021 Zenlink
// Licensed under GPL-3.0.

#![allow(clippy::type_complexity)]

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct PairInfo<AccountId, AssetBalance> {
    pub asset_0: AssetId,
    pub asset_1: AssetId,

    pub account: AccountId,
    pub total_liquidity: AssetBalance,
    pub holding_liquidity: AssetBalance,
    pub reserve_0: AssetBalance,
    pub reserve_1: AssetBalance,
    pub lp_asset_id: AssetId,
}

impl<T: Config> Pallet<T> {
    pub fn get_assets() -> Vec<AssetId> {
        let mut all_assets = Self::foreign_list();

        for (i, _) in Self::lp_pairs().iter().enumerate() {
            all_assets.push(AssetId {
                chain_id: T::SelfParaId::get(),
                asset_type: LIQUIDITY,
                asset_index: i as u32,
            })
        }

        all_assets
    }

    pub fn get_all_pairs() -> Vec<PairInfo<T::AccountId, AssetBalance>> {
        let chain_id = T::SelfParaId::get();
        Self::lp_pairs()
            .iter()
            .enumerate()
            .map(|(i, pair)| {
                let lp_id = AssetId { chain_id, asset_type: LIQUIDITY, asset_index: i as u32 };
                let (para_account, total) = Self::lp_metadata(pair).unwrap_or_default();

                PairInfo {
                    asset_0: pair.0,
                    asset_1: pair.1,
                    account: para_account.clone(),
                    total_liquidity: total,
                    holding_liquidity: Zero::zero(),
                    reserve_0: T::MultiAssetsHandler::balance_of(pair.0, &para_account),
                    reserve_1: T::MultiAssetsHandler::balance_of(pair.1, &para_account),
                    lp_asset_id: lp_id,
                }
            })
            .collect::<Vec<_>>()
    }

    pub fn get_pair_by_asset_id(
        asset_0: AssetId,
        asset_1: AssetId,
    ) -> Option<PairInfo<T::AccountId, AssetBalance>> {
        let sorted_pair = Self::sort_asset_id(asset_0, asset_1);
        let chain_id = T::SelfParaId::get();

        if let Some(index) = Self::lp_pairs().iter().position(|pair| *pair == sorted_pair) {
            let lp_id = AssetId { chain_id, asset_type: LIQUIDITY, asset_index: index as u32 };
            let (para_account, total) = Self::lp_metadata(sorted_pair).unwrap_or_default();

            Some(PairInfo {
                asset_0,
                asset_1,
                account: para_account.clone(),
                total_liquidity: total,
                holding_liquidity: Zero::zero(),
                reserve_0: T::MultiAssetsHandler::balance_of(asset_0, &para_account),
                reserve_1: T::MultiAssetsHandler::balance_of(asset_1, &para_account),
                lp_asset_id: lp_id,
            })
        } else {
            None
        }
    }

    pub fn get_sovereigns_info(asset_id: &AssetId) -> Vec<(u32, T::AccountId, AssetBalance)> {
        T::TargetChains::get()
            .iter()
            .filter_map(|(location, _)| match location {
                MultiLocation::X2(Junction::Parent, Junction::Parachain(id)) => {
                    if let Ok(sovereign) = T::Conversion::convert_ref(location) {
                        Some((*id, sovereign))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .map(|(para_id, account)| {
                let balance = T::MultiAssetsHandler::balance_of(*asset_id, &account);

                (para_id, account, balance)
            })
            .collect::<Vec<_>>()
    }

    pub fn get_owner_pairs(owner: &T::AccountId) -> Vec<PairInfo<T::AccountId, AssetBalance>> {
        Self::get_all_pairs()
            .into_iter()
            .filter_map(|mut pair_info| {
                let hold = T::MultiAssetsHandler::balance_of(pair_info.lp_asset_id, owner);
                if hold > 0 {
                    pair_info.holding_liquidity = hold;

                    Some(pair_info)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    }

    pub fn supply_out_amount(supply: AssetBalance, path: Vec<AssetId>) -> AssetBalance {
        Self::get_amount_out_by_path(supply, &path).map_or(AssetBalance::default(), |amounts| {
            *amounts.last().unwrap_or(&AssetBalance::default())
        })
    }

    pub fn desired_in_amount(desired_amount: AssetBalance, path: Vec<AssetId>) -> AssetBalance {
        Self::get_amount_in_by_path(desired_amount, &path)
            .map_or(AssetBalance::default(), |amounts| {
                *amounts.first().unwrap_or(&AssetBalance::default())
            })
    }

    pub fn get_estimate_lptoken(
        asset_0: AssetId,
        asset_1: AssetId,
        amount_0_desired: AssetBalance,
        amount_1_desired: AssetBalance,
        amount_0_min: AssetBalance,
        amount_1_min: AssetBalance,
    ) -> AssetBalance {
        let sorted_pair = Self::sort_asset_id(asset_0, asset_1);
        Self::lp_metadata(sorted_pair).map_or(Zero::zero(), |(pair_account, total)| {
            let reserve_0 = T::MultiAssetsHandler::balance_of(asset_0, &pair_account);
            let reserve_1 = T::MultiAssetsHandler::balance_of(asset_1, &pair_account);

            Self::calculate_added_amount(
                amount_0_desired,
                amount_1_desired,
                amount_0_min,
                amount_1_min,
                reserve_0,
                reserve_1,
            )
            .map_or(Zero::zero(), |(amount_0, amount_1)| {
                Self::calculate_liquidity(amount_0, amount_1, reserve_0, reserve_1, total)
            })
        })
    }
}
