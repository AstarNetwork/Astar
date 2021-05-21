// Copyright 2020-2021 Zenlink
// Licensed under GPL-3.0.

//! # Liquidity Asset Module
//!
//! ## Overview
//!
//! Built-in Liquidity module in Zenlink Protocol, expose the some functions
//! for transfer-by-xcm to other parachain and swap to other assets.

use super::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// The Zenlink Protocol swap liquidity foreign
impl<T: Config> Pallet<T> {
    /// public mutable functions

    /// Implement of the transfer function.
    pub(crate) fn lp_transfer(
        id: AssetId,
        owner: &T::AccountId,
        target: &T::AccountId,
        amount: AssetBalance,
    ) -> DispatchResult {
        let pair = Self::get_lp_pair(id.asset_index).ok_or(Error::<T>::AssetNotExists)?;

        let owner_balance = <LiquidityLedger<T>>::get((&pair, owner));
        ensure!(owner_balance >= amount, Error::<T>::InsufficientAssetBalance);

        let new_balance = owner_balance.saturating_sub(amount);

        <LiquidityLedger<T>>::mutate((pair, owner), |balance| *balance = new_balance);
        <LiquidityLedger<T>>::mutate((pair, target), |balance| {
            *balance = balance.saturating_add(amount)
        });

        Self::deposit_event(Event::Transferred(id, owner.clone(), target.clone(), amount));

        Ok(())
    }

    /// Increase the total supply of the foreign
    /// Note: need to check Exist. because it be created by Create-Pair in swap
    pub(crate) fn lp_mint(
        id: AssetId,
        owner: &T::AccountId,
        amount: AssetBalance,
    ) -> DispatchResult {
        let pair = Self::get_lp_pair(id.asset_index).ok_or(Error::<T>::AssetNotExists)?;

        let new_balance = <LiquidityLedger<T>>::get((pair, owner))
            .checked_add(amount)
            .ok_or(Error::<T>::Overflow)?;

        <LiquidityLedger<T>>::mutate((pair, owner), |balance| *balance = new_balance);

        <LiquidityMeta<T>>::try_mutate_exists::<_, _, Error<T>, _>(pair, |meta| {
            let meta = meta.as_mut().ok_or(Error::<T>::AssetNotExists)?;

            meta.1 = meta.1.checked_add(amount).ok_or(Error::<T>::Overflow)?;

            Ok(())
        })?;

        Self::deposit_event(Event::Minted(id, owner.clone(), amount));

        Ok(())
    }

    /// Decrease the total supply of the foreign
    pub(crate) fn lp_burn(
        id: AssetId,
        owner: &T::AccountId,
        amount: AssetBalance,
    ) -> DispatchResult {
        let pair = Self::get_lp_pair(id.asset_index).ok_or(Error::<T>::AssetNotExists)?;

        let new_balance = <LiquidityLedger<T>>::get((pair, owner))
            .checked_sub(amount)
            .ok_or(Error::<T>::InsufficientLiquidity)?;

        <LiquidityLedger<T>>::mutate((pair, owner), |balance| *balance = new_balance);

        <LiquidityMeta<T>>::try_mutate::<_, _, Error<T>, _>(pair, |meta| {
            let meta = meta.as_mut().ok_or(Error::<T>::AssetNotExists)?;

            meta.1 = meta.1.checked_sub(amount).ok_or(Error::<T>::InsufficientLiquidity)?;

            Ok(())
        })?;

        Self::deposit_event(Event::Burned(id, owner.clone(), amount));

        Ok(())
    }

    // Public immutable functions

    /// Get the local liquidity `id` balance of `owner`.
    pub fn lp_balance_of(id: AssetId, owner: &T::AccountId) -> AssetBalance {
        if let Some(pair) = Self::get_lp_pair(id.asset_index) {
            Self::lp_ledger((pair, owner))
        } else {
            Default::default()
        }
    }

    /// Get the total supply of an foreign `id`.
    /// return default value if none
    pub fn lp_total_supply(id: AssetId) -> AssetBalance {
        if let Some(pair) = Self::get_lp_pair(id.asset_index) {
            Self::lp_metadata(pair).map(|(_, total)| total).unwrap_or_default()
        } else {
            Default::default()
        }
    }

    pub fn lp_is_exists(id: AssetId) -> bool {
        Self::get_lp_pair(id.asset_index).is_some()
    }
}
