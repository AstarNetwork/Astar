//! # Plasm rewards Module
//!
//! The Plasm rewards module provides functionality for handling whole rewards and era.
//!
//! - [`plasm_rewards::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//!
//! The Plasm staking module puts together the management and compensation payment logic of the ERA.
//! The Plasm Rewards module calls the Dapps Staking and Validator.
//! It also allocates rewards to each module according to the [Plasm Token Ecosystem inflation model](https://docs.plasmnet.io/PlasmNetwork/TokenEcosystem.html).
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    traits::{Currency, Get, LockableCurrency, Time},
    weights::{SimpleDispatchInfo, Weight},
    StorageMap, StorageValue,
};
use frame_system::{self as system, ensure_root};
use pallet_session::SessionManager;
pub use pallet_staking::Forcing;
use sp_runtime::{
    traits::{SaturatedConversion, Zero},
    Perbill, RuntimeDebug,
};
use sp_std::{prelude::*, vec::Vec};

pub mod inflation;
#[cfg(test)]
mod mock;
pub mod traits;
pub use traits::*;
#[cfg(test)]
mod tests;

pub use sp_staking::SessionIndex;

pub type EraIndex = u32;
pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
pub type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;

// A value placed in storage that represents the current version of the Staking storage.
// This value is used by the `on_runtime_upgrade` logic to determine whether we run
// storage migration logic. This should match directly with the semantic versions of the Rust crate.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug)]
pub enum Releases {
    V1_0_0,
}

impl Default for Releases {
    fn default() -> Self {
        Releases::V1_0_0
    }
}

/// Information regarding the active era (era in used in session).
#[derive(Encode, Decode, RuntimeDebug, PartialEq, Eq)]
pub struct ActiveEraInfo<Moment> {
    /// Index of era.
    pub index: EraIndex,
    /// Moment of start
    ///
    /// Start can be none if start hasn't been set for the era yet,
    /// Start is set on the first on_finalize of the era to guarantee usage of `Time`.
    pub start: Option<Moment>,
}

pub trait Trait: pallet_session::Trait {
    /// The staking balance.
    type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    /// Time used for computing era duration.
    type Time: Time;

    /// Number of sessions per era.
    type SessionsPerEra: Get<SessionIndex>;

    /// Number of eras that staked funds must remain bonded for.
    type BondingDuration: Get<EraIndex>;

    /// Get the amount of staking for dapps per era.
    type GetForDappsStaking: traits::GetEraStakingAmount<EraIndex, BalanceOf<Self>>;

    /// Get the amount of staking for security per era.
    type GetForSecurityStaking: traits::GetEraStakingAmount<EraIndex, BalanceOf<Self>>;

    /// How to compute total issue PLM for rewards.
    type ComputeTotalPayout: traits::ComputeTotalPayout;

    /// Maybe next validators.
    type MaybeValidators: traits::MaybeValidators<EraIndex, Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as DappsStaking {
        /// This is the compensation paid for the dapps operator of the Plasm Network.
        /// This is stored on a per-era basis.
        pub ForDappsEraReward get(fn for_dapps_era_reward): map hasher(twox_64_concat) EraIndex => Option<BalanceOf<T>>;

        /// This is the compensation paid for the security of the Plasm Network.
        /// This is stored on a per-era basis.
        pub ForSecurityEraReward get(fn for_security_era_reward): map hasher(twox_64_concat) EraIndex => Option<BalanceOf<T>>;

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
        pub HistoryDepth get(fn history_depth) config(): u32 = 672;

        /// A mapping from still-bonded eras to the first session index of that era.
        ///
        /// Must contains information for eras for the range:
        /// `[active_era - bounding_duration; active_era]`
        pub BondedEras: Vec<(EraIndex, SessionIndex)>;

        /// The current era index.
        ///
        /// This is the latest planned era, depending on how session module queues the validator
        /// set, it might be active or not.
        pub CurrentEra get(fn current_era): Option<EraIndex>;

        /// The active era information, it holds index and start.
        ///
        /// The active era is the era currently rewarded.
        /// Validator set of this era must be equal to `SessionInterface::validators`.
        pub ActiveEra get(fn active_era): Option<ActiveEraInfo<MomentOf<T>>>;

        /// The session index at which the era start for the last `HISTORY_DEPTH` eras
        pub ErasStartSessionIndex get(fn eras_start_session_index):
            map hasher(twox_64_concat) EraIndex => Option<SessionIndex>;

        /// True if the next session change will be a new era regardless of index.
        pub ForceEra get(fn force_era) config(): Forcing;

        /// Storage version of the pallet.
        ///
        /// This is set to v1.0.0 for new networks.
        StorageVersion build(|_: &GenesisConfig| Releases::V1_0_0): Releases;
    }
}

decl_event!(
    pub enum Event<T>
    where
        Balance = BalanceOf<T>,
    {
        /// The whole reward issued in that Era.
        /// (era_index: EraIndex, reward: Balance)
        WholeEraReward(EraIndex, Balance),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// Duplicate index.
        DuplicateIndex,
        /// Invalid era to reward.
        InvalidEraToReward,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Number of sessions per era.
        const SessionsPerEra: SessionIndex = T::SessionsPerEra::get();

        type Error = Error<T>;

        fn deposit_event() = default;

        fn on_runtime_upgrade() -> Weight {
            migrate::<T>();
            50_000
        }

        /// On finalize is called at after rotate session.
        fn on_finalize() {
            // Set the start of the first era.
            if let Some(mut active_era) = Self::active_era() {
                if active_era.start.is_none() {
                    active_era.start = Some(T::Time::now());
                    <ActiveEra<T>>::put(active_era);
                }
            }
        }

        // ----- Root calls.
        /// Force there to be no new eras indefinitely.
        ///
        /// # <weight>
        /// - No arguments.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedOperational(5_000)]
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
        #[weight = SimpleDispatchInfo::FixedOperational(5_000)]
        fn force_new_era(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceNew);
        }

        /// Force there to be a new era at the end of sessions indefinitely.
        ///
        /// # <weight>
        /// - One storage write
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedOperational(5_000)]
        fn force_new_era_always(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceAlways);
        }

        /// Set history_depth value.
        ///
        /// Origin must be root.
        #[weight = SimpleDispatchInfo::FixedOperational(500_000)]
        fn set_history_depth(origin, #[compact] new_history_depth: EraIndex) {
            ensure_root(origin)?;
            if let Some(current_era) = Self::current_era() {
                HistoryDepth::mutate(|history_depth| {
                    let last_kept = current_era.checked_sub(*history_depth).unwrap_or(0);
                    let new_last_kept = current_era.checked_sub(new_history_depth).unwrap_or(0);
                    for era_index in last_kept..new_last_kept {
                        Self::clear_era_information(era_index);
                    }
                    *history_depth = new_history_depth
                })
            }
        }
    }
}

fn migrate<T: Trait>() {
    // TODO: When runtime upgrade, migrate stroage.
    // if let Some(current_era) = CurrentEra::get() {
    //     let history_depth = HistoryDepth::get();
    //     for era in current_era.saturating_sub(history_depth)..=current_era {
    //         ErasStartSessionIndex::migrate_key_from_blake(era);
    //     }
    // }
}

impl<T: Trait> Module<T> {
    // MUTABLES (DANGEROUS)

    /// Plan a new session potentially trigger a new era.
    fn new_session(session_index: SessionIndex) -> Option<Vec<T::AccountId>> {
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

            match ForceEra::get() {
                Forcing::ForceNew => ForceEra::kill(),
                Forcing::ForceAlways => (),
                Forcing::NotForcing if era_length >= T::SessionsPerEra::get() => (),
                _ => return None,
            }

            Self::new_era(session_index)
        } else {
            // Set initial era
            Self::new_era(session_index)
        }
    }

    /// Start a session potentially starting an era.
    fn start_session(start_session: SessionIndex) {
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
    fn end_session(session_index: SessionIndex) {
        if let Some(active_era) = Self::active_era() {
            if let Some(next_active_era_start_session_index) =
                Self::eras_start_session_index(active_era.index + 1)
            {
                if next_active_era_start_session_index == session_index + 1 {
                    Self::end_era(active_era, session_index);
                }
            }
        }
    }

    /// * Increment `active_era.index`,
    /// * reset `active_era.start`,
    /// * update `BondedEras` and apply slashes.
    fn start_era(start_session: SessionIndex) {
        let active_era = <ActiveEra<T>>::mutate(|active_era| {
            let new_index = active_era.as_ref().map(|info| info.index + 1).unwrap_or(0);
            *active_era = Some(ActiveEraInfo {
                index: new_index,
                // Set new active era start in next `on_finalize`. To guarantee usage of `Time`
                start: None,
            });
            new_index
        });

        // let bonding_duration = T::BondingDuration::get();

        BondedEras::mutate(|bonded| {
            bonded.push((active_era, start_session));

            // if active_era > bonding_duration {
            //     let first_kept = active_era - bonding_duration;
            //
            //     // prune out everything that's from before the first-kept index.
            //     let n_to_prune = bonded.iter()
            //         .take_while(|&&(era_idx, _)| era_idx < first_kept)
            //         .count();
            //
            //     // kill slashing metadata.
            //     for (pruned_era, _) in bonded.drain(..n_to_prune) {
            //         slashing::clear_era_metadata::<T>(pruned_era);
            //     }
            //
            //     if let Some(&(_, first_session)) = bonded.first() {
            //         T::SessionInterface::prune_historical_up_to(first_session);
            //     }
            // }
        });
    }

    /// Compute payout for era.
    fn end_era(active_era: ActiveEraInfo<MomentOf<T>>, _session_index: SessionIndex) {
        // Note: active_era_start can be None if end era is called during genesis config.
        if let Some(active_era_start) = active_era.start {
            // The set of total amount of staking.
            let now = T::Time::now();
            let era_duration = now - active_era_start;

            if !era_duration.is_zero() {
                let total_payout = T::Currency::total_issuance();
                let for_dapps = T::GetForDappsStaking::get_era_staking_amount(&active_era.index);
                let for_security =
                    T::GetForSecurityStaking::get_era_staking_amount(&active_era.index);

                let (for_security_reward, for_dapps_rewards) =
                    T::ComputeTotalPayout::compute_total_payout(
                        total_payout,
                        era_duration.saturated_into::<u64>(),
                        for_security,
                        for_dapps,
                    );

                <ForSecurityEraReward<T>>::insert(active_era.index, for_security_reward);
                <ForDappsEraReward<T>>::insert(active_era.index, for_dapps_rewards);
            }
        }
    }

    /// Plan a new era. Return the potential new staking set.
    fn new_era(start_session_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        // Increment or set current era.
        let current_era = CurrentEra::get().map(|s| s + 1).unwrap_or(0);
        CurrentEra::put(current_era.clone());
        ErasStartSessionIndex::insert(&current_era, &start_session_index);

        // Clean old era information.
        if let Some(old_era) = current_era.checked_sub(Self::history_depth() + 1) {
            Self::clear_era_information(old_era);
        }

        // Return maybe validators.
        T::MaybeValidators::maybe_validators(current_era)
    }

    /// Clear all era information for given era.
    fn clear_era_information(era_index: EraIndex) {
        ErasStartSessionIndex::remove(era_index);
        <ForDappsEraReward<T>>::remove(era_index);
        <ForSecurityEraReward<T>>::remove(era_index);
    }
}

/// In this implementation `new_session(session)` must be called before `end_session(session-1)`
/// i.e. the new session must be planned before the ending of the previous session.
///
/// Once the first new_session is planned, all session must start and then end in order, though
/// some session can lag in between the newest session planned and the latest session started.
impl<T: Trait> SessionManager<T::AccountId> for Module<T> {
    fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        Self::new_session(new_index)
    }
    fn start_session(start_index: SessionIndex) {
        Self::start_session(start_index)
    }
    fn end_session(end_index: SessionIndex) {
        Self::end_session(end_index)
    }
}

/// In this implementation using validator and dapps rewards module.
impl<T: Trait> EraFinder<EraIndex, SessionIndex, MomentOf<T>> for Module<T> {
    fn bonded_eras() -> Vec<(EraIndex, SessionIndex)> {
        Self::bonded_eras()
    }
    fn current_era() -> Option<EraIndex> {
        Self::current_era()
    }
    fn active_era() -> Option<ActiveEraInfo<MomentOf<T>>> {
        Self::active_era()
    }
    fn eras_start_session_index(era: &EraIndex) -> Option<SessionIndex> {
        Self::eras_start_session_index(&era)
    }
}

/// Get the security rewards for validator module.
impl<T: Trait> ForSecurityEraRewardFinder<BalanceOf<T>> for Module<T> {
    fn for_security_era_reward(era: &EraIndex) -> Option<BalanceOf<T>> {
        Self::for_security_era_reward(&era)
    }
}

/// Get the dapps rewards for dapps staking module.
impl<T: Trait> ForDappsEraRewardFinder<BalanceOf<T>> for Module<T> {
    fn for_dapps_era_reward(era: &EraIndex) -> Option<BalanceOf<T>> {
        Self::for_dapps_era_reward(&era)
    }
}
