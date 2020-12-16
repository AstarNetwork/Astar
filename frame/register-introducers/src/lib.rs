//! # Register introducer Module
//!
//! The register introducers module provides functionality for handling whole rewards and era.
//!
//! - [`register_introducers::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//!
//! In other words, any current PLM holder can be an introducer.
//! This should not be a problem, as at the moment it is basically
//! only a Lockdrop participant. There will be a registration period of
//! about a month after the mainnet relaunch.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::Parameter,
    ensure,
    traits::{Get, Time},
    StorageMap,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::traits::{AtLeast32Bit, Member, Saturating};
use sp_std::prelude::*;

pub mod traits;
pub use traits::*;

pub trait Trait: system::Trait {
    /// Time used for computing era duration.
    type Time: Time<Moment = Self::Moment>;

    /// The end time of enable this modules.
    type EndTimeOfRegistering: Get<Self::Moment>;

    /// Timestamp type.
    type Moment: Member
        + Parameter
        + Saturating
        + AtLeast32Bit
        + Copy
        + Default
        + From<u64>
        + Into<u64>
        + Into<u128>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as DappsStaking {
        /// This is the compensation paid for the dapps operator of the Plasm Network.
        /// This is stored on a per-era basis.
        pub RegisteredIntroducers get(fn registered_introducers): map hasher(twox_64_concat) T::AccountId => Option<()>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        /// The whole reward issued in that Era.
        /// (era_index: EraIndex, reward: Balance)
        Registered(AccountId),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// Expired
        Expired,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Register sender id
        /// TODO: weight
        #[weight = 100_000]
        pub fn register(origin) {
            let origin = ensure_signed(origin)?;

            let expired = T::EndTimeOfRegistering::get();
            let now = T::Time::now();
            ensure!(
                expired >= now,
                Error::<T>::Expired,
            );
            <RegisteredIntroducers<T>>::insert(&origin, ());
            Self::deposit_event(RawEvent::Registered(origin));
        }
    }
}

impl<T: Trait> RegisteredIntroducersChecker<T::AccountId> for Module<T> {
    fn is_registered(account_id: &T::AccountId) -> bool {
        if let Some(_) = Self::registered_introducers(account_id) {
            return true;
        }
        false
    }
}
