use super::*;
use codec::FullCodec;
use frame_support::{
    dispatch::DispatchResult,
    pallet_prelude::MaybeSerializeDeserialize,
    traits::{Currency, ExistenceRequirement},
};
use sp_runtime::traits::AtLeast32BitUnsigned;
use sp_std::{fmt::Debug, marker::PhantomData};

pub trait ERC20Trait<AccountId> {
    type C: Currency<AccountId>;
    type B: AtLeast32BitUnsigned + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;
    fn transfer_from(&self, from: &AccountId, to: &AccountId, amount: Self::B) -> DispatchResult;
    fn transfer(&self, to: &AccountId, amount: Self::B) -> DispatchResult;
}

pub struct PseudoERC20<T: Config> {
    origin: T::AccountId,
    _phantom: PhantomData<T>,
}

impl<T: Config> PseudoERC20<T> {
    pub fn new(origin: T::AccountId) -> Self {
        PseudoERC20::<T> {
            origin,
            _phantom: Default::default(),
        }
    }
}

impl<T: Config> ERC20Trait<T::AccountId> for PseudoERC20<T> {
    type C = <T as Config>::Currency;
    type B = BalanceOf<T>;
    fn transfer_from(
        &self,
        from: &T::AccountId,
        to: &T::AccountId,
        amount: Self::B,
    ) -> DispatchResult {
        Self::C::transfer(from, to, amount, ExistenceRequirement::AllowDeath)
    }
    fn transfer(&self, to: &T::AccountId, amount: Self::B) -> DispatchResult {
        Self::C::transfer(&self.origin, &to, amount, ExistenceRequirement::AllowDeath)
    }
}
