// Copyright 2020-2021 Zenlink
// Licensed under GPL-3.0.

use super::*;

pub trait LocalAssetHandler<AccountId> {
    fn local_balance_of(asset_id: AssetId, who: &AccountId) -> AssetBalance;

    fn local_total_supply(asset_id: AssetId) -> AssetBalance;

    fn local_is_exists(asset_id: AssetId) -> bool;

    fn local_transfer(
        asset_id: AssetId,
        origin: &AccountId,
        target: &AccountId,
        amount: AssetBalance,
    ) -> DispatchResult {
        let withdrawn = Self::local_withdraw(asset_id, origin, amount)?;
        let _ = Self::local_deposit(asset_id, target, withdrawn)?;

        Ok(())
    }

    fn local_deposit(
        asset_id: AssetId,
        origin: &AccountId,
        amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError>;

    fn local_withdraw(
        asset_id: AssetId,
        origin: &AccountId,
        amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError>;
}

impl<AccountId> LocalAssetHandler<AccountId> for () {
    fn local_balance_of(_asset_id: AssetId, _who: &AccountId) -> AssetBalance {
        Default::default()
    }

    fn local_total_supply(_asset_id: AssetId) -> AssetBalance {
        Default::default()
    }

    fn local_is_exists(_asset_id: AssetId) -> bool {
        false
    }

    fn local_deposit(
        _asset_id: AssetId,
        _origin: &AccountId,
        _amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError> {
        unimplemented!()
    }

    fn local_withdraw(
        _asset_id: AssetId,
        _origin: &AccountId,
        _amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError> {
        unimplemented!()
    }
}

pub trait OtherAssetHandler<AccountId> {
    fn other_balance_of(asset_id: AssetId, who: &AccountId) -> AssetBalance;

    fn other_total_supply(asset_id: AssetId) -> AssetBalance;

    fn other_is_exists(asset_id: AssetId) -> bool;

    fn other_transfer(
        asset_id: AssetId,
        origin: &AccountId,
        target: &AccountId,
        amount: AssetBalance,
    ) -> DispatchResult {
        let withdrawn = Self::other_withdraw(asset_id, origin, amount)?;
        let _ = Self::other_deposit(asset_id, target, withdrawn)?;

        Ok(())
    }

    fn other_deposit(
        asset_id: AssetId,
        origin: &AccountId,
        amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError>;

    fn other_withdraw(
        asset_id: AssetId,
        origin: &AccountId,
        amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError>;
}

impl<AccountId> OtherAssetHandler<AccountId> for () {
    fn other_balance_of(_asset_id: AssetId, _who: &AccountId) -> AssetBalance {
        Default::default()
    }

    fn other_total_supply(_asset_id: AssetId) -> AssetBalance {
        Default::default()
    }

    fn other_is_exists(_asset_id: AssetId) -> bool {
        false
    }

    fn other_deposit(
        _asset_id: AssetId,
        _origin: &AccountId,
        _amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError> {
        unimplemented!()
    }

    fn other_withdraw(
        _asset_id: AssetId,
        _origin: &AccountId,
        _amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError> {
        unimplemented!()
    }
}
