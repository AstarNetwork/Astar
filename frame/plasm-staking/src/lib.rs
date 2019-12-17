//! # Plasm Staking Module
//!
//! The Plasm staking module manages era, total amounts of rewards and how to distribute.
#![cfg_attr(not(feature = "std"), no_std)]

use support::{
    decl_module, decl_storage, decl_event, StorageValue,
    weights::SimpleDispatchInfo,
    traits::{Time, Get},
};
use system::ensure_root;
use session::OnSessionEnding;
use sp_runtime::RuntimeDebug;
#[cfg(feature = "std")]
use sp_runtime::{Serialize, Deserialize,};
use codec::{Encode, Decode,};
use sp_std::vec::Vec;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod migration;

pub use sp_staking::SessionIndex;
pub type EraIndex = u32;
pub type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;

/// Mode of era-forcing.
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Forcing {
    /// Not forcing anything - just let whatever happen.
    NotForcing,
    /// Force a new era, then reset to `NotForcing` as soon as it is done.
    ForceNew,
    /// Avoid a new era indefinitely.
    ForceNone,
    /// Force a new era at the end of all sessions indefinitely.
    ForceAlways,
}

impl Default for Forcing {
    fn default() -> Self { Forcing::NotForcing }
}

pub trait Trait: session::Trait {
    /// Time used for computing era duration.
    type Time: Time;

    /// Number of sessions per era.
    type SessionsPerEra: Get<SessionIndex>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as PlasmStaking {
        /// The current era index.
        pub CurrentEra get(fn current_era): EraIndex;

        /// The start of the current era.
        pub CurrentEraStart get(fn current_era_start): MomentOf<T>;

        /// The session index at which the current era started.
        pub CurrentEraStartSessionIndex get(fn current_era_start_session_index): SessionIndex;

        /// True if the next session change will be a new era regardless of index.
        pub ForceEra get(fn force_era) config(): Forcing;

        /// The version of storage for upgrade.
        pub StorageVersion get(fn storage_version) config(): u32;

        /// Set of accounts that can validate blocks.
        pub Validators get(fn validators) config(): Vec<T::AccountId>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Number of sessions per era.
        const SessionsPerEra: SessionIndex = T::SessionsPerEra::get();

        fn deposit_event() = default;

        fn on_initialize() {
            Self::ensure_storage_upgraded();
        }

        fn on_finalize() {
            // Set the start of the first era.
            if !<CurrentEraStart<T>>::exists() {
                <CurrentEraStart<T>>::put(T::Time::now());
            }
        }

        // ----- Root calls.
        /// Force there to be no new eras indefinitely.
        ///
        /// # <weight>
        /// - No arguments.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FreeOperational]
        fn force_no_eras(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceNone);
        }

        /// Force there to be a new era at the end of the next session. After this, it will be
        /// reset to normal (non-forced) behaviour.
        ///
        /// # <weight>
        /// - No arguments.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FreeOperational]
        fn force_new_era(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceNew);
        }

        /// Force there to be a new era at the end of sessions indefinitely.
        ///
        /// # <weight>
        /// - One storage write
        /// # </weight>
        #[weight = SimpleDispatchInfo::FreeOperational]
        fn force_new_era_always(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceAlways);
        }

        /// Manually set new validators. 
        ///
        /// # <weight>
        /// - One storage write
        /// # </weight>
        #[weight = SimpleDispatchInfo::FreeOperational]
        fn set_validators(origin, new_validators: Vec<T::AccountId>) {
            ensure_root(origin)?;
            <Validators<T>>::put(&new_validators);
            Self::deposit_event(RawEvent::NewValidators(new_validators));
        }
    }
}

decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
        /// Validator set changed.
        NewValidators(Vec<AccountId>),
    }
);


impl<T: Trait> Module<T> {
    pub fn new_session(ending: SessionIndex, will_apply_at: SessionIndex) -> Option<Vec<T::AccountId>> {
        let era_length = will_apply_at.checked_sub(Self::current_era_start_session_index()).unwrap_or(0);
        match ForceEra::get() {
            Forcing::ForceNew => ForceEra::kill(),
            Forcing::ForceAlways => (),
            Forcing::NotForcing if era_length > T::SessionsPerEra::get() => (),
            _ => return None,
        }
        Self::new_era(ending, will_apply_at)
    }

    pub fn new_era(_ending: SessionIndex, will_apply_at: SessionIndex) -> Option<Vec<T::AccountId>> {
        CurrentEra::mutate(|era| *era += 1);
        <CurrentEraStart<T>>::put(T::Time::now());
        CurrentEraStartSessionIndex::put(will_apply_at - 1);
        // Apply new validator set
        Some(<Validators<T>>::get())
    }

    /// Ensures storage is upgraded to most recent necessary state.
    fn ensure_storage_upgraded() {
        migration::perform_migrations::<T>();
    }
}

impl<T: Trait> OnSessionEnding<T::AccountId> for Module<T> {
    fn on_session_ending(ending: SessionIndex, will_apply_at: SessionIndex) -> Option<Vec<T::AccountId>> {
        Self::ensure_storage_upgraded();
        Self::new_session(ending, will_apply_at)
    }
}

impl<T: Trait> session::SelectInitialValidators<T::AccountId> for Module<T> {
    fn select_initial_validators() -> Option<Vec<T::AccountId>> {
        Some(<Validators<T>>::get())
    }
}
