// Copyright 2020-2021 Zenlink
// Licensed under GPL-3.0.

//! # Foreign Asset Module
//!
//! ## Overview
//!
//! Built-in assets module in Zenlink Protocol, handle the foreign assets
//! which are reserved other chain and teleported to this chain by xcm,

use super::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// The Zenlink Protocol foreign foreign which reserved other chain assets
impl<T: Config> Pallet<T> {
    /// public mutable functions

    /// Implement of the transfer function.
    pub(crate) fn foreign_transfer(
        id: AssetId,
        owner: &T::AccountId,
        target: &T::AccountId,
        amount: AssetBalance,
    ) -> DispatchResult {
        let owner_balance = <ForeignLedger<T>>::get((&id, owner));
        ensure!(owner_balance >= amount, Error::<T>::InsufficientAssetBalance);

        let new_balance = owner_balance.saturating_sub(amount);

        <ForeignLedger<T>>::mutate((id, owner), |balance| *balance = new_balance);
        <ForeignLedger<T>>::mutate((id, target), |balance| {
            *balance = balance.saturating_add(amount)
        });

        Self::deposit_event(Event::Transferred(id, owner.clone(), target.clone(), amount));

        Ok(())
    }

    /// Increase the total supply of the foreign
    /// Note: no need check Exists, because it be created when it not exist
    pub(crate) fn foreign_mint(
        id: AssetId,
        owner: &T::AccountId,
        amount: AssetBalance,
    ) -> DispatchResult {
        if !Self::foreign_list().contains(&id) {
            <ForeignList<T>>::mutate(|assets| assets.push(id));
        }

        let new_balance = <ForeignLedger<T>>::get((id, owner)).saturating_add(amount);

        <ForeignLedger<T>>::try_mutate::<_, _, Error<T>, _>((id, owner), |balance| {
            *balance = new_balance;

            Ok(())
        })?;

        <ForeignMeta<T>>::try_mutate::<_, _, Error<T>, _>(id, |supply| {
            *supply = supply.saturating_add(amount);

            Ok(())
        })?;

        Self::deposit_event(Event::Minted(id, owner.clone(), amount));

        Ok(())
    }

    /// Decrease the total supply of the foreign
    pub(crate) fn foreign_burn(
        id: AssetId,
        owner: &T::AccountId,
        amount: AssetBalance,
    ) -> DispatchResult {
        ensure!(Self::foreign_list().contains(&id), Error::<T>::AssetNotExists);
        let new_balance = <ForeignLedger<T>>::get((id, owner))
            .checked_sub(amount)
            .ok_or(Error::<T>::InsufficientAssetBalance)?;

        <ForeignLedger<T>>::mutate((id, owner), |balance| *balance = new_balance);

        <ForeignMeta<T>>::mutate(id, |supply| {
            *supply = supply.saturating_sub(amount);
        });

        Self::deposit_event(Event::Burned(id, owner.clone(), amount));

        Ok(())
    }

    // Public immutable functions

    /// Get the foreign `id` balance of `owner`.
    pub fn foreign_balance_of(id: AssetId, owner: &T::AccountId) -> AssetBalance {
        Self::foreign_ledger((id, owner))
    }

    /// Get the total supply of an foreign `id`.
    pub fn foreign_total_supply(id: AssetId) -> AssetBalance {
        Self::foreign_meta(id)
    }

    pub fn foreign_is_exists(id: AssetId) -> bool {
        Self::foreign_list().contains(&id)
    }
}
