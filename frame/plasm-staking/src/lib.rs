//! # Plasm Session Module
//!
//! The Plasm session module manages era and total amounts of rewards and how to distribute.

use support::{decl_module, decl_storage, decl_event, StorageValue, weights::SimpleDispatchInfo,
			  traits::{LockableCurrency, Time, Get, Currency}};
use system::ensure_root;
use session::OnSessionEnding;
use sp_staking::SessionIndex;
use sp_runtime::{
	RuntimeDebug,
};
#[cfg(feature = "std")]
use sp_runtime::{Serialize, Deserialize};

use validator_manager::{EraIndex, OnEraEnding};
use codec::{Encode, Decode};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod migration;

pub type BalanceOf<T> =
<<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
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
	/// The staking balance.
	type Currency: LockableCurrency<Self::AccountId, Moment=Self::BlockNumber>;

	/// Time used for computing era duration.
	type Time: Time;

	/// Number of sessions per era.
	type SessionsPerEra: Get<SessionIndex>;

	/// Handler for when a session is about to end.
	type OnEraEnding: OnEraEnding<Self::AccountId, EraIndex>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
	trait Store for Module<T: Trait> as PlasmStaking {
		/// The current era index.
		pub CurrentEra get(fn current_era) config(): EraIndex;

		/// The start of the current era.
		pub CurrentEraStart get(fn current_era_start): MomentOf<T>;

		/// The session index at which the current era started.
		pub CurrentEraStartSessionIndex get(fn current_era_start_session_index): SessionIndex;

		/// True if the next session change will be a new era regardless of index.
		pub ForceEra get(fn force_era) config(): Forcing;

		/// The version of storage for upgrade.
		pub StorageVersion get(fn storage_version) config(): u32;
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
	}
}

decl_event!(
	pub enum Event<T> where Balance = BalanceOf<T> {
		/// All validators have been rewarded;
		Reward(Balance),
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
		let current_era = Self::current_era();
		CurrentEra::put(current_era + 1);
		<CurrentEraStart<T>>::put(T::Time::now());
		CurrentEraStartSessionIndex::put(will_apply_at - 1);
		<T as Trait>::OnEraEnding::on_era_ending(current_era, current_era + 1)
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
