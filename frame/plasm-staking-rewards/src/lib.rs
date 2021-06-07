//! # Plasm rewards Module
//!
//! The Plasm rewards module provides functionality for handling whole rewards and era.
//!
//! - [`plasm_rewards::Config`](./trait.Config.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//!
//! The Plasm staking module puts together the management and compensation payment logic of the ERA.
//! The Plasm Rewards module calls the Dapps Staking and Validator.
//! It also allocates rewards to each module according to the [Plasm Token Ecosystem inflation model](https://docs.plasmnet.io/learn/token-economy#inflation-model).
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;

pub mod traits;

use sp_std::prelude::*;
use pallet_session::SessionManager;
pub use crate::traits::{EraFinder, ForDappsEraRewardFinder, HistoryDepthFinder};
use sp_runtime::Percent;
pub use pallet::*;
use pallet_plasm_node_staking as staking;
use plasm_primitives::traits::ForSecurityEraRewardFinder;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*; // Import various types used in the pallet definition
    use frame_system::pallet_prelude::*; // Import some system helper types.
    pub use sp_staking::SessionIndex;
    pub type EraIndex = u32;
    use sp_runtime::{
        traits::{SaturatedConversion, Zero},
        Perbill, 
    };
    pub use plasm_primitives::Forcing;
    pub use frame_support::{traits::{UnixTime, Currency, LockableCurrency} };
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
    use codec::{Decode, Encode};
    use sp_std::{prelude::*, vec::Vec};

// A value placed in storage that represents the current version of the Staking storage.
// This value is used by the `on_runtime_upgrade` logic to determine whether we run
// storage migration logic. This should match directly with the semantic versions of the Rust crate.
#[cfg_attr(feature = "std", derive(Debug, Eq))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub enum Releases {
    V1_0_0,
}

impl Default for Releases {
    fn default() -> Self {
        Releases::V1_0_0
    }
}

/// Information regarding the active era (era in used in session).
#[cfg_attr(feature = "std", derive(Debug, Eq))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub struct ActiveEraInfo {
    /// Index of era.
    pub index: EraIndex,
    /// Moment of start
    ///
    /// Start can be none if start hasn't been set for the era yet,
    /// Start is set on the first on_finalize of the era to guarantee usage of `Time`.
    pub start: Option<u64>,
}

#[pallet::config]
pub trait Config: pallet_session::Config + frame_system::Config + pallet_plasm_node_staking::Config{
    /// The staking balance.
    type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    /// Time used for computing era duration.
    type UnixTime: UnixTime;

    /// Number of sessions per era.
    type SessionsPerEra: Get<SessionIndex>;

    /// The overarching event type.
    type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
}

#[pallet::pallet]
#[pallet::generate_store(trait Store)]
pub struct Pallet<T>(_);

        /// This is the compensation paid for the dapps operator of the Plasm Network.
        /// This is stored on a per-era basis.
        /// A mapping from operators to operated contract
        #[pallet::storage]
        #[pallet::getter(fn for_dapps_era_reward)]
        pub(super) type ForDappsEraReward<T:Config> =
            StorageMap<_, Blake2_128Concat, EraIndex, Option<BalanceOf<T>>, ValueQuery >;

        /// This is the compensation paid for the security of the Plasm Network.
        /// This is stored on a per-era basis.
        #[pallet::storage]
        #[pallet::getter(fn for_security_era_reward)]
        pub(super) type ForSecurityEraReward<T:Config> =
            StorageMap<_, Blake2_128Concat, EraIndex, Option<BalanceOf<T>>, ValueQuery >;

        /// The ideal number of staking participants.
        #[pallet::storage]
        #[pallet::getter(fn validator_count)]
		pub(super) type ValidatorCount<T> =
            StorageValue<_, u32, ValueQuery>;
        
        /// Number of era to keep in history.
        ///
        /// Information is kept for eras in `[current_era - history_depth; current_era]`
        ///
        /// Must be more than the number of era delayed by session otherwise.
        /// i.e. active era must always be in history.
        /// i.e. `active_era > current_era - history_depth` must be guaranteed.
        ///
        /// 24 * 28 = 672 eras is roughly 28 days on current Plasm Network.
        /// That seems like a reasonable length of time for users to claim a payout
        #[pallet::storage]
        #[pallet::getter(fn history_depth)]
        pub(super) type HistoryDepth<T> =
            StorageValue<_, u32, ValueQuery>;

        /// A mapping from still-bonded eras to the first session index of that era.
        ///
        /// Must contains information for eras for the range:
        /// `[active_era - bounding_duration; active_era]`
        #[pallet::storage]
        #[pallet::getter(fn get_bonded_eras)]
        pub(super) type BondedEras<T> =
            StorageValue<_, Vec<(EraIndex, SessionIndex)>, ValueQuery >;

        /// The current era index.
        ///
        /// This is the latest planned era, depending on how session module queues the validator
        /// set, it might be active or not.
        #[pallet::storage]
        #[pallet::getter(fn current_era)]
        pub(super) type CurrentEra<T> =
            StorageValue<_, Option<EraIndex>, ValueQuery>;

        /// The active era information, it holds index and start.
        ///
        /// The active era is the era currently rewarded.
        /// Validator set of this era must be equal to `SessionInterface::validators`.
        #[pallet::storage]
        #[pallet::getter(fn active_era)]
        pub(super) type ActiveEra<T> =
            StorageValue<_, Option<ActiveEraInfo>, ValueQuery>;

        /// The session index at which the era start for the last `HISTORY_DEPTH` eras
        #[pallet::storage]
        #[pallet::getter(fn eras_start_session_index)]
        pub(super) type ErasStartSessionIndex<T> =
            StorageMap<_, Blake2_128Concat, EraIndex, Option<SessionIndex>, ValueQuery >;

        /// True if the next session change will be a new era regardless of index.
        #[pallet::storage]
        #[pallet::getter(fn force_era)]
        pub(super) type ForceEra<T> =
            StorageValue<_, Forcing, ValueQuery>;

        /// Storage version of the pallet.
        ///
        /// This is set to v1.0.0 for new networks.
        #[pallet::storage]
        #[pallet::getter(fn get_storage_version)]
        pub(super) type StorageVersion<T> =
            StorageValue<_, Releases, ValueQuery>;


#[pallet::event]
#[pallet::generate_deposit(pub(super) fn deposit_event)]
pub enum Event<T: Config> {
        /// The whole reward issued in that Era.
        /// (era_index: EraIndex, reward: Balance)
        WholeEraReward(EraIndex, BalanceOf<T>),
    }


/// Error for the staking module.
#[pallet::error]
pub enum Error<T> {
        /// Duplicate index.
        DuplicateIndex,
        /// Invalid era to reward.
        InvalidEraToReward,
}

#[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {

        fn on_runtime_upgrade() -> Weight {
            Self::migrate();
            50_000
        }

        /// On finalize is called at after rotate session.
        fn on_finalize(_n: BlockNumberFor<T>) {
            // Set the start of the first era.
			if let Some(mut active_era) = Self::active_era() {
                // if the era is untreated
				if active_era.start.is_none() {
					let now_as_millis_u64 = <T as Config>::UnixTime::now().as_millis().saturated_into::<u64>();
					active_era.start = Some(now_as_millis_u64);
                    Self::end_era(active_era.clone());
					ActiveEra::<T>::put(Some(active_era));
				}
			}
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // ----- Root calls.
        /// Force there to be no new eras indefinitely.
        ///
        /// # <weight>
        /// - No arguments.
        /// # </weight>
        #[pallet::weight(5_000)]
        pub fn force_no_eras(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            ForceEra::<T>::put(Forcing::ForceNone);
            Ok(().into())
        }

        /// Force there to be a new era at the end of the next session. After this, it will be
        /// reset to normal (non-forced) behaviour.
        ///
        /// # <weight>
        /// - No arguments.
        /// # </weight>
        #[pallet::weight(5_000)]
        pub fn force_new_era(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            ForceEra::<T>::put(Forcing::ForceNew);
            Ok(().into())
        }

        /// Force there to be a new era at the end of sessions indefinitely.
        ///
        /// # <weight>
        /// - One storage write
        /// # </weight>
        #[pallet::weight(5_000)]
        pub fn force_new_era_always(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            ForceEra::<T>::put(Forcing::ForceAlways);
            Ok(().into())
        }

        /// Set history_depth value.
        ///
        /// Origin must be root.
        #[pallet::weight(5_000)]
        fn set_history_depth(origin: OriginFor<T>, new_history_depth: EraIndex) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            if let Some(current_era) = Self::current_era() {
                HistoryDepth::<T>::mutate(|history_depth| {
                    let last_kept = current_era.checked_sub(*history_depth).unwrap_or(0);
                    let new_last_kept = current_era.checked_sub(new_history_depth).unwrap_or(0);
                    for era_index in last_kept..new_last_kept {
                        Self::clear_era_information(era_index);
                    }
                    *history_depth = new_history_depth
                })
            }
            Ok(().into())
        }
    }

    #[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
        pub validator_count: u32,
        pub _phantom: PhantomData<T>,
    }

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
                validator_count: 0u32,
                _phantom: PhantomData::<T>,
            }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
            StorageVersion::<T>::put(Releases::V1_0_0);
			ValidatorCount::<T>::put(self.validator_count);
            HistoryDepth::<T>::put(672u32); // 24 * 28 = 672 eras is roughly 28 days
		}
	}


impl<T: Config> Pallet<T> {
    // MUTABLES (DANGEROUS)

fn migrate() {
    // TODO: When runtime upgrade, migrate stroage.
    // if let Some(current_era) = CurrentEra::get() {
    //     let history_depth = HistoryDepth::get();
    //     for era in current_era.saturating_sub(history_depth)..=current_era {
    //         ErasStartSessionIndex::migrate_key_from_blake(era);
    //     }
    // }
}
    /// Plan a new session potentially trigger a new era.
    pub fn plasm_new_session(session_index: SessionIndex) -> Option<Vec<T::AccountId>> {

        if let Some(current_era) = Self::current_era() {
            // Initial era has been set.

            let current_era_start_session_index = Self::eras_start_session_index(current_era)
                .unwrap_or_else(|| {
                    frame_support::print("Error: start_session_index must be set for current_era");
                    0
                });

            let era_length = session_index
                .checked_sub(current_era_start_session_index)
                .unwrap_or(0); // Must never happen.

            match ForceEra::<T>::get() {
                Forcing::ForceNew => ForceEra::<T>::kill(),
                Forcing::ForceAlways => (),
                Forcing::NotForcing if era_length >= <T as Config>::SessionsPerEra::get() => (),
                _ => return None,
            }

            Self::new_era(session_index)
        } else {
            // Set initial era
            Self::new_era(session_index)
        }
    }

    /// Start a session potentially starting an era.
    pub fn plasm_start_session(start_session: sp_staking::SessionIndex) {
        let next_active_era = Self::active_era().map(|e| e.index + 1).unwrap_or(0);
        if let Some(next_active_era_start_session_index) =
            Self::eras_start_session_index(next_active_era)
        {
            if next_active_era_start_session_index == start_session {
                Self::start_era(start_session);
            } else if next_active_era_start_session_index < start_session {
                // This arm should never happen, but better handle it than to stall the
                // staking pallet.
                frame_support::print("Warning: A session appears to have been skipped.");
                Self::start_era(start_session);
            }
        }
    }

    /// End a session potentially ending an era.
    pub fn plasm_end_session(session_index: sp_staking::SessionIndex) {
        if let Some(active_era) = Self::active_era() {
            if let Some(next_active_era_start_session_index) =
                Self::eras_start_session_index(active_era.index + 1)
            {
                if next_active_era_start_session_index == session_index + 1 {
                    Self::end_era(active_era);
                }
            }
        }
    }

    /// * Increment `active_era.index`,
    /// * reset `active_era.start`,
    /// * update `BondedEras` and apply slashes.
    pub fn start_era(_start_session: sp_staking::SessionIndex) {
        let _active_era = ActiveEra::<T>::mutate(|active_era| {
			let new_index = active_era.as_ref().map(|info| info.index + 1).unwrap_or(0);
			*active_era = Some(ActiveEraInfo {
				index: new_index,
				// Set new active era start in next `on_finalize`. To guarantee usage of `Time`
				start: None,
			});
			new_index
		});
    }

    /// Compute payout for era.
    pub fn end_era(active_era: ActiveEraInfo) {
        // Note: active_era_start can be None if end era is called during genesis config.
        if let Some(active_era_start) = active_era.start {
            // The set of total amount of staking.
			let now_as_millis_u64 = <T as Config>::UnixTime::now().as_millis().saturated_into::<u64>();

			let era_duration = now_as_millis_u64 - active_era_start;
            let for_security = Self::validator_count();

            if era_duration != 0 {
                let total_payout = <T as Config>::Currency::total_issuance();
                let (for_security_reward, for_dapps_rewards) = Self::compute_total_rewards(
                    total_payout,
                    era_duration.saturated_into::<u64>(),
                    for_security,
                    0u32,
                );
                <ForSecurityEraReward<T>>::insert(active_era.index, Some(for_security_reward));
                <ForDappsEraReward<T>>::insert(active_era.index, Some(for_dapps_rewards));
                Self::deposit_event(Event::WholeEraReward(active_era.index, total_payout));
            }
        }
    }

    /// Plan a new era. Return the potential new staking set.
    pub fn new_era(start_session_index: sp_staking::SessionIndex) -> Option<Vec<T::AccountId>> {
        // Increment or set current era.
        let current_era = CurrentEra::<T>::get().map(|s| s + 1).unwrap_or(0);
        CurrentEra::<T>::put(Some(current_era.clone()));
        ErasStartSessionIndex::<T>::insert(&current_era, Some(&start_session_index));

        // Clean old era information.
        if let Some(old_era) = current_era.checked_sub(Self::history_depth() + 1) {
            Self::clear_era_information(old_era);
        }
        None
    }

    /// Clear all era information for given era.
    pub fn clear_era_information(era_index: EraIndex) {
        ErasStartSessionIndex::<T>::remove(era_index);
        <ForDappsEraReward<T>>::remove(era_index);
        <ForSecurityEraReward<T>>::remove(era_index);
    }

    pub fn compute_total_rewards(
        total_tokens: BalanceOf<T>,
        era_duration: u64,
        number_of_validator: u32,
        _dapps_staking: u32,
    ) -> (BalanceOf<T>, BalanceOf<T>)
    {
        const TARGETS_NUMBER: u128 = 100;
        const MILLISECONDS_PER_YEAR: u128 = 1000 * 3600 * 24 * 36525 / 100;
        // I_0 = 2.5%.
        const I_0_DENOMINATOR: u128 = 25;
        const I_0_NUMERATOR: u128 = 1000;
        let number_of_validator_clone: u128 = number_of_validator.clone().into();
        let era_duration_clone: u128 = era_duration.clone().into();
        let number_of_validator: u128 = number_of_validator.into();
        let portion = if TARGETS_NUMBER < number_of_validator_clone {
            // TotalForSecurityRewards
            // = TotalAmountOfIssue * I_0% * (EraDuration / 1year)

            // denominator: I_0_DENOMINATOR * EraDuration
            // numerator: 1year * I_0_NUMERATOR
            Perbill::from_rational_approximation(
                I_0_DENOMINATOR * era_duration_clone,
                MILLISECONDS_PER_YEAR * I_0_NUMERATOR,
            )
        } else {
            // TotalForSecurityRewards
            // = TotalAmountOfIssue * I_0% * (NumberOfValidators/TargetsNumber) * (EraDuration/1year)

            // denominator: I_0_DENOMINATOR * NumberOfValidators * EraDuration
            // numerator: 1year * I_0_NUMERATOR * TargetsNumber
            Perbill::from_rational_approximation(
                I_0_DENOMINATOR * number_of_validator * era_duration_clone,
                MILLISECONDS_PER_YEAR * I_0_NUMERATOR * TARGETS_NUMBER,
            )
        };
        let payout = portion * total_tokens;
        (payout, BalanceOf::<T>::zero())
    }

}
}

/// In this implementation `new_session(session)` must be called before `end_session(session-1)`
/// i.e. the new session must be planned before the ending of the previous session.
///
/// Once the first new_session is planned, all session must start and then end in order, though
/// some session can lag in between the newest session planned and the latest session started.
impl<T: Config + pallet::Config> SessionManager<T::AccountId> for Pallet<T> {
    fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        staking::Module::<T>::new_session(new_index);
        Self::plasm_new_session(new_index)
    }
    fn start_session(start_index: SessionIndex) {
        staking::Module::<T>::start_session(start_index);
        Self::plasm_start_session(start_index)
    }
    fn end_session(end_index: SessionIndex) {
        staking::Module::<T>::end_session(end_index);
        Self::plasm_end_session(end_index)
    }
}

/// In this implementation using validator and dapps rewards module.
impl<T: Config + pallet::Config> EraFinder<EraIndex, SessionIndex> for Pallet<T> {
    fn current() -> Option<EraIndex> {
        Self::current_era()
    }
    fn active() -> Option<ActiveEraInfo> {
        Self::active_era()
    }
    fn start_session_index(era: &EraIndex) -> Option<SessionIndex> {
        Self::eras_start_session_index(&era)
    }
}

/// Get the security rewards for validator module.
impl<T: Config + pallet::Config> ForSecurityEraRewardFinder<BalanceOf<T>> for Pallet<T> {
    fn get(era: &EraIndex) -> Option<BalanceOf<T>> {
        Self::for_security_era_reward(&era)
    }

    fn validator_count() -> u32 {
        Self::validator_count()
    }

    fn set_validator_count(new: u32){
        ValidatorCount::<T>::put(new);
    }

    fn increase_validator_count(additional: u32) {
        ValidatorCount::<T>::mutate(|n| *n += additional);
    }

    fn scale_validator_count(factor: Percent) {
        ValidatorCount::<T>::mutate(|n| *n += factor * *n);
    }

}

/// Get the dapps rewards for dapps staking module.
impl<T: Config + pallet::Config> ForDappsEraRewardFinder<BalanceOf<T>> for Pallet<T> {
    fn get(era: &EraIndex) -> Option<BalanceOf<T>> {
        Self::for_dapps_era_reward(&era)
    }
}

/// Get the history depth
impl<T: Config + pallet::Config> HistoryDepthFinder for Pallet<T> {
    fn get() -> u32 {
        Self::history_depth()
    }
}
