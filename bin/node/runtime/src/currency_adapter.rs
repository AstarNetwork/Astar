//! Adapt other currency traits implementation to `BasicCurrency`.

use codec::Codec;
use sp_runtime::DispatchResult;
use sp_runtime::traits::CheckedSub;
use frame_system::Config;
use frame_support::{
    pallet_prelude::*,
    traits::{
        Currency as PalletCurrency, ExistenceRequirement, LockableCurrency as PalletLockableCurrency,
        ReservableCurrency as PalletReservableCurrency, WithdrawReasons,
    },
};
use orml_traits::{
    arithmetic::{Signed, SimpleArithmetic},
    BalanceStatus, BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency, BasicReservableCurrency,
    LockIdentifier,
};
use sp_std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
    fmt::Debug,
    result,
};

pub struct NativeCurrencyAdapter<T, Currency, Amount, Moment>(
    PhantomData<(T, Currency, Amount, Moment)>
);

type PalletBalanceOf<A, Currency> = <Currency as PalletCurrency<A>>::Balance;

impl<T, AccountId, Currency, Amount, Moment> BasicCurrency<AccountId>
    for NativeCurrencyAdapter<T, Currency, Amount, Moment>
  where
    Currency: PalletCurrency<AccountId>,
    T: Config,
{
    type Balance = PalletBalanceOf<AccountId, Currency>;

    fn minimum_balance() -> Self::Balance {
        Currency::minimum_balance()
    }

    fn total_issuance() -> Self::Balance {
        Currency::total_issuance()
    }

    fn total_balance(who: &AccountId) -> Self::Balance {
        Currency::total_balance(who)
    }

    fn free_balance(who: &AccountId) -> Self::Balance {
        Currency::free_balance(who)
    }

    fn ensure_can_withdraw(who: &AccountId, amount: Self::Balance) -> DispatchResult {
        let new_balance = Self::free_balance(who)
            .checked_sub(&amount)
            .ok_or("BalanceTooLow")?;

        Currency::ensure_can_withdraw(who, amount, WithdrawReasons::all(), new_balance)
    }

    fn transfer(from: &AccountId, to: &AccountId, amount: Self::Balance) -> DispatchResult {
        Currency::transfer(from, to, amount, ExistenceRequirement::AllowDeath)
    }

    fn deposit(who: &AccountId, amount: Self::Balance) -> DispatchResult {
        let _ = Currency::deposit_creating(who, amount);
        Ok(())
    }

    fn withdraw(who: &AccountId, amount: Self::Balance) -> DispatchResult {
        Currency::withdraw(who, amount, WithdrawReasons::all(), ExistenceRequirement::AllowDeath).map(|_| ())
    }

    fn can_slash(who: &AccountId, amount: Self::Balance) -> bool {
        Currency::can_slash(who, amount)
    }

    fn slash(who: &AccountId, amount: Self::Balance) -> Self::Balance {
        let (_, gap) = Currency::slash(who, amount);
        gap
    }
}

impl<T, AccountId, Currency, Amount, Moment> BasicCurrencyExtended<AccountId>
    for NativeCurrencyAdapter<T, Currency, Amount, Moment>
  where
    Amount: Signed
        + TryInto<PalletBalanceOf<AccountId, Currency>>
        + TryFrom<PalletBalanceOf<AccountId, Currency>>
        + SimpleArithmetic
        + Codec
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Default,
    Currency: PalletCurrency<AccountId>,
    T: Config,
{
    type Amount = Amount;

    fn update_balance(who: &AccountId, by_amount: Self::Amount) -> DispatchResult {
        let by_balance = by_amount
            .abs()
            .try_into()
            .map_err(|_| "AmountIntoBalanceFailed")?;
        if by_amount.is_positive() {
            Self::deposit(who, by_balance)
        } else {
            Self::withdraw(who, by_balance)
        }
    }
}

impl<T, AccountId, Currency, Amount, Moment> BasicLockableCurrency<AccountId>
    for NativeCurrencyAdapter<T, Currency, Amount, Moment>
  where
    Currency: PalletLockableCurrency<AccountId>,
    T: Config,
{
    type Moment = Moment;

    fn set_lock(lock_id: LockIdentifier, who: &AccountId, amount: Self::Balance) -> DispatchResult {
        Currency::set_lock(lock_id, who, amount, WithdrawReasons::all());
        Ok(())
    }

    fn extend_lock(lock_id: LockIdentifier, who: &AccountId, amount: Self::Balance) -> DispatchResult {
        Currency::extend_lock(lock_id, who, amount, WithdrawReasons::all());
        Ok(())
    }

    fn remove_lock(lock_id: LockIdentifier, who: &AccountId) -> DispatchResult {
        Currency::remove_lock(lock_id, who);
        Ok(())
    }
}

impl<T, AccountId, Currency, Amount, Moment> BasicReservableCurrency<AccountId>
    for NativeCurrencyAdapter<T, Currency, Amount, Moment>
  where
    Currency: PalletReservableCurrency<AccountId>,
    T: Config,
{
    fn can_reserve(who: &AccountId, value: Self::Balance) -> bool {
        Currency::can_reserve(who, value)
    }

    fn slash_reserved(who: &AccountId, value: Self::Balance) -> Self::Balance {
        let (_, gap) = Currency::slash_reserved(who, value);
        gap
    }

    fn reserved_balance(who: &AccountId) -> Self::Balance {
        Currency::reserved_balance(who)
    }

    fn reserve(who: &AccountId, value: Self::Balance) -> DispatchResult {
        Currency::reserve(who, value)
    }

    fn unreserve(who: &AccountId, value: Self::Balance) -> Self::Balance {
        Currency::unreserve(who, value)
    }

    fn repatriate_reserved(
        slashed: &AccountId,
        beneficiary: &AccountId,
        value: Self::Balance,
        status: BalanceStatus,
    ) -> result::Result<Self::Balance, DispatchError> {
        Currency::repatriate_reserved(slashed, beneficiary, value, status)
    }
}
