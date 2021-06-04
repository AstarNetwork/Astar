// Copyright 2020-2021 Zenlink
// Licensed under GPL-3.0.

use super::*;

pub trait MultiAssetsHandler<AccountId> {
    fn balance_of(asset_id: AssetId, who: &AccountId) -> AssetBalance;

    fn total_supply(asset_id: AssetId) -> AssetBalance;

    fn is_exists(asset_id: AssetId) -> bool;

    fn transfer(
        asset_id: AssetId,
        origin: &AccountId,
        target: &AccountId,
        amount: AssetBalance,
    ) -> DispatchResult {
        let withdrawn = Self::withdraw(asset_id, origin, amount)?;
        let _ = Self::deposit(asset_id, target, withdrawn)?;

        Ok(())
    }

    fn deposit(
        asset_id: AssetId,
        target: &AccountId,
        amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError>;

    fn withdraw(
        asset_id: AssetId,
        origin: &AccountId,
        amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError>;
}

pub struct ZenlinkMultiAssets<T, Native = (), Local = (), Other = ()>(
    PhantomData<(T, Native, Local, Other)>,
);

impl<T: Config, NativeCurrency, Local, Other> MultiAssetsHandler<T::AccountId>
    for ZenlinkMultiAssets<Pallet<T>, NativeCurrency, Local, Other>
where
    NativeCurrency: Currency<T::AccountId>,
    Local: LocalAssetHandler<T::AccountId>,
    Other: OtherAssetHandler<T::AccountId>,
{
    fn balance_of(asset_id: AssetId, who: &<T as frame_system::Config>::AccountId) -> AssetBalance {
        let self_chain_id: u32 = T::SelfParaId::get();
        match asset_id.asset_type {
            NATIVE if asset_id.is_native(self_chain_id) => {
                NativeCurrency::free_balance(who).saturated_into::<AssetBalance>()
            }
            LIQUIDITY if asset_id.chain_id == self_chain_id => {
                Pallet::<T>::lp_balance_of(asset_id, who)
            }
            LOCAL if asset_id.chain_id == self_chain_id => Local::local_balance_of(asset_id, who),
            RESERVED if asset_id.chain_id == self_chain_id => {
                Other::other_balance_of(asset_id, who)
            }
            _ if asset_id.is_foreign(self_chain_id) => {
                Pallet::<T>::foreign_balance_of(asset_id, who)
            }
            _ => Default::default(),
        }
    }

    fn total_supply(asset_id: AssetId) -> AssetBalance {
        let self_chain_id: u32 = T::SelfParaId::get();
        match asset_id.asset_type {
            NATIVE if asset_id.is_native(T::SelfParaId::get()) => {
                NativeCurrency::total_issuance().saturated_into::<AssetBalance>()
            }
            LIQUIDITY if asset_id.chain_id == self_chain_id => {
                Pallet::<T>::lp_total_supply(asset_id)
            }
            LOCAL if asset_id.chain_id == self_chain_id => Local::local_total_supply(asset_id),
            RESERVED if asset_id.chain_id == self_chain_id => Other::other_total_supply(asset_id),
            _ if asset_id.is_foreign(self_chain_id) => Pallet::<T>::foreign_total_supply(asset_id),
            _ => Default::default(),
        }
    }

    fn is_exists(asset_id: AssetId) -> bool {
        let self_chain_id: u32 = T::SelfParaId::get();
        match asset_id.asset_type {
            NATIVE if asset_id.chain_id == self_chain_id => {
                asset_id.is_native(T::SelfParaId::get())
            }
            LIQUIDITY if asset_id.chain_id == self_chain_id => Pallet::<T>::lp_is_exists(asset_id),
            LOCAL if asset_id.chain_id == self_chain_id => Local::local_is_exists(asset_id),
            RESERVED if asset_id.chain_id == self_chain_id => Other::other_is_exists(asset_id),
            _ if asset_id.is_foreign(T::SelfParaId::get()) => {
                Pallet::<T>::foreign_is_exists(asset_id)
            }
            _ => Default::default(),
        }
    }

    fn transfer(
        asset_id: AssetId,
        origin: &<T as frame_system::Config>::AccountId,
        target: &<T as frame_system::Config>::AccountId,
        amount: AssetBalance,
    ) -> DispatchResult {
        let self_chain_id: u32 = T::SelfParaId::get();
        match asset_id.asset_type {
            NATIVE if asset_id.is_native(T::SelfParaId::get()) => {
                let balance_amount = amount
                    .try_into()
                    .map_err(|_| DispatchError::Other("AmountToBalanceConversionFailed"))?;

                NativeCurrency::transfer(&origin, &target, balance_amount, KeepAlive)
            }
            LIQUIDITY if asset_id.chain_id == self_chain_id => {
                Pallet::<T>::lp_transfer(asset_id, origin, target, amount)
            }
            LOCAL if asset_id.chain_id == self_chain_id => {
                Local::local_transfer(asset_id, origin, target, amount)
            }
            RESERVED if asset_id.chain_id == self_chain_id => {
                Other::other_transfer(asset_id, origin, target, amount)
            }
            _ if asset_id.is_foreign(T::SelfParaId::get()) => {
                Pallet::<T>::foreign_transfer(asset_id, origin, target, amount)
            }
            _ => Err(Error::<T>::UnsupportedAssetType.into()),
        }
    }

    fn deposit(
        asset_id: AssetId,
        target: &<T as frame_system::Config>::AccountId,
        amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError> {
        let self_chain_id: u32 = T::SelfParaId::get();
        match asset_id.asset_type {
            NATIVE if asset_id.is_native(T::SelfParaId::get()) => {
                let balance_amount = amount
                    .try_into()
                    .map_err(|_| DispatchError::Other("AmountToBalanceConversionFailed"))?;

                let _ = NativeCurrency::deposit_creating(target, balance_amount);

                Ok(amount)
            }
            LIQUIDITY if asset_id.chain_id == self_chain_id => {
                Pallet::<T>::lp_mint(asset_id, target, amount).map(|_| amount)
            }
            LOCAL if asset_id.chain_id == self_chain_id => {
                Local::local_deposit(asset_id, target, amount)
            }
            RESERVED if asset_id.chain_id == self_chain_id => {
                Other::other_deposit(asset_id, target, amount)
            }
            _ if asset_id.is_foreign(T::SelfParaId::get()) => {
                Pallet::<T>::foreign_mint(asset_id, target, amount).map(|_| amount)
            }
            _ => Err(Error::<T>::UnsupportedAssetType.into()),
        }
    }

    fn withdraw(
        asset_id: AssetId,
        origin: &<T as frame_system::Config>::AccountId,
        amount: AssetBalance,
    ) -> Result<AssetBalance, DispatchError> {
        let self_chain_id: u32 = T::SelfParaId::get();
        match asset_id.asset_type {
            NATIVE if asset_id.is_native(self_chain_id) => {
                let balance_amount = amount
                    .try_into()
                    .map_err(|_| DispatchError::Other("AmountToBalanceConversionFailed"))?;

                let _ = NativeCurrency::withdraw(
                    &origin,
                    balance_amount,
                    WithdrawReasons::TRANSFER,
                    ExistenceRequirement::AllowDeath,
                )?;

                Ok(amount)
            }
            LIQUIDITY if asset_id.chain_id == self_chain_id => {
                Pallet::<T>::lp_burn(asset_id, origin, amount).map(|_| amount)
            }
            LOCAL if asset_id.chain_id == self_chain_id => {
                Local::local_withdraw(asset_id, origin, amount)
            }
            RESERVED if asset_id.chain_id == self_chain_id => {
                Other::other_withdraw(asset_id, origin, amount)
            }
            _ if asset_id.is_foreign(T::SelfParaId::get()) => {
                Pallet::<T>::foreign_burn(asset_id, origin, amount).map(|_| amount)
            }
            _ => Err(Error::<T>::UnsupportedAssetType.into()),
        }
    }
}
