// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

//! TODO

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use astar_primitives::{Balance, BlockNumber};
use frame_support::{
    pallet_prelude::*,
    traits::{Currency, OnTimestampSet},
};
use frame_system::{ensure_root, pallet_prelude::*};
use sp_runtime::{traits::CheckedAdd, Perquintill, Saturating};

#[cfg(any(feature = "runtime-benchmarks"))]
pub mod benchmarking;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {

    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    // Negative imbalance type of this pallet.
    pub(crate) type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::NegativeImbalance;

    #[pallet::config]
    pub trait Config: frame_system::Config<BlockNumber = BlockNumber> {
        /// The currency trait.
        /// This has been soft-deprecated but it still needs to be used here in order to access `NegativeImbalance`
        // which is defined in the currency trait.
        type Currency: Currency<Self::AccountId, Balance = Balance>;

        /// Handler for 'per-block' payouts.
        type PayoutPerBlock: PayoutPerBlock<NegativeImbalanceOf<Self>>;

        /// Cycle ('year') configuration - covers periods, subperiods, eras & blocks.
        type CycleConfiguration: CycleConfiguration;

        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Inflation parameters have been force changed. This will have effect on the next inflation recalculation.
        InflationParametersForceChanged,
        /// Inflation configuration has been force changed. This will have an immediate effect from this block.
        InflationConfigurationForceChanged { config: InflationConfiguration },
        /// Inflation recalculation has been forced.
        ForcedInflationRecalculation { config: InflationConfiguration },
        /// New inflation configuration has been set.
        NewInflationConfiguration { config: InflationConfiguration },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Sum of all parts must be one whole (100%).
        InvalidInflationParameters,
    }

    /// Active inflation configuration parameteres.
    /// They describe current rewards, when inflation needs to be recalculated, etc.
    #[pallet::storage]
    #[pallet::whitelist_storage]
    pub type InflationConfig<T: Config> = StorageValue<_, InflationConfiguration, ValueQuery>;

    /// Static inflation parameters - used to calculate active inflation configuration at certain points in time.
    #[pallet::storage]
    pub type InflationParams<T: Config> = StorageValue<_, InflationParameters, ValueQuery>;

    /// Used to keep track of the approved & issued issuance.
    #[pallet::storage]
    #[pallet::whitelist_storage]
    pub type SafetyInflationTracker<T: Config> = StorageValue<_, InflationTracker, ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumber> for Pallet<T> {
        fn on_initialize(now: BlockNumber) -> Weight {
            // Need to account for weight consumed in `on_timestamp` & `on_finalize`.
            if Self::is_recalculation_in_next_block(now, &InflationConfig::<T>::get()) {
                Weight::from_parts(0, 0)
            } else {
                Weight::from_parts(0, 0)
            }
        }

        fn on_finalize(now: BlockNumber) {
            // Recalculation is done at the block right before the re-calculation is supposed to happen.
            // This is to ensure all the rewards are paid out according to the new inflation configuration from next block.
            //
            // If this was done in `on_initialize`, collator & treasury would receive incorrect rewards for that one block.
            // That's not a big problem, but it would be wrong!
            if Self::is_recalculation_in_next_block(now, &InflationConfig::<T>::get()) {
                let (max_emission, config) = Self::recalculate_inflation(now);
                InflationConfig::<T>::put(config.clone());

                SafetyInflationTracker::<T>::mutate(|tracker| {
                    tracker.cap.saturating_accrue(max_emission);
                });

                Self::deposit_event(Event::<T>::NewInflationConfiguration { config });
            }
        }
    }

    impl<Moment, T: Config> OnTimestampSet<Moment> for Pallet<T> {
        fn on_timestamp_set(_moment: Moment) {
            let amount = Self::payout_block_rewards();

            // Update the tracker, but no check whether an overflow has happened.
            // This can modified if needed, but these amounts are supposed to be small &
            // collators need to be paid for producing the block.
            // TODO: potential discussion topic for the review!
            SafetyInflationTracker::<T>::mutate(|tracker| {
                tracker.issued.saturating_accrue(amount);
            });
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Used to force-set the inflation parameters.
        /// The parameters must be valid, all parts summing up to one whole (100%), otherwise the call will fail.
        ///
        /// Must be called by `root` origin.
        ///
        /// Purpose of the call is testing & handling unforseen circumstances.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(0, 0))]
        pub fn force_set_inflation_params(
            origin: OriginFor<T>,
            params: InflationParameters,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(params.is_valid(), Error::<T>::InvalidInflationParameters);
            InflationParams::<T>::put(params);

            Self::deposit_event(Event::<T>::InflationParametersForceChanged);

            Ok(().into())
        }

        /// Used to force-set the inflation configuration.
        /// The parameters aren't checked for validity, since essentially anything can be valid.
        ///
        /// Must be called by `root` origin.
        ///
        /// Purpose of the call is testing & handling unforseen circumstances.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(0, 0))]
        pub fn force_set_inflation_config(
            origin: OriginFor<T>,
            config: InflationConfiguration,
        ) -> DispatchResult {
            ensure_root(origin)?;

            InflationConfig::<T>::put(config.clone());

            Self::deposit_event(Event::<T>::InflationConfigurationForceChanged { config });

            Ok(().into())
        }

        /// Used to force inflation recalculation.
        /// This is done in the same way as it would be done in an appropriate block, but this call forces it.
        ///
        /// Must be called by `root` origin.
        ///
        /// Purpose of the call is testing & handling unforseen circumstances.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(0, 0))]
        pub fn force_inflation_recalculation(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;

            let (max_emission, config) =
                Self::recalculate_inflation(frame_system::Pallet::<T>::block_number());

            InflationConfig::<T>::put(config.clone());

            SafetyInflationTracker::<T>::mutate(|tracker| {
                tracker.cap.saturating_accrue(max_emission);
            });

            Self::deposit_event(Event::<T>::ForcedInflationRecalculation { config });

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Used to check if inflation recalculation is supposed to happen on the next block.
        fn is_recalculation_in_next_block(
            now: BlockNumber,
            config: &InflationConfiguration,
        ) -> bool {
            config.recalculation_block.saturating_sub(now) <= 1
        }

        /// Payout block rewards to the beneficiaries.
        ///
        /// Return the total amount issued.
        fn payout_block_rewards() -> Balance {
            let config = InflationConfig::<T>::get();

            let collator_amount = T::Currency::issue(config.collator_reward_per_block);
            let treasury_amount = T::Currency::issue(config.treasury_reward_per_block);

            T::PayoutPerBlock::collators(collator_amount);
            T::PayoutPerBlock::treasury(treasury_amount);

            config.collator_reward_per_block + config.treasury_reward_per_block
        }

        /// Recalculates the inflation based on the total issuance & inflation parameters.
        ///
        /// Returns the maximum total emission for the cycle, and the new inflation configuration.
        pub(crate) fn recalculate_inflation(now: BlockNumber) -> (Balance, InflationConfiguration) {
            let params = InflationParams::<T>::get();
            let total_issuance = T::Currency::total_issuance();

            // 1. Calculate maximum emission over the period before the next recalculation.
            let max_emission = params.max_inflation_rate * total_issuance;

            // 2. Calculate distribution of max emission between different purposes.
            let treasury_emission = params.treasury_part * max_emission;
            let collators_emission = params.collators_part * max_emission;
            let dapps_emission = params.dapps_part * max_emission;
            let base_stakers_emission = params.base_stakers_part * max_emission;
            let adjustable_stakers_emission = params.adjustable_stakers_part * max_emission;
            let bonus_emission = params.bonus_part * max_emission;

            // 3. Calculate concrete rewards per blocl, era or period

            // 3.1. Collator & Treausry rewards per block
            let collator_reward_per_block =
                collators_emission / Balance::from(T::CycleConfiguration::blocks_per_cycle());
            let treasury_reward_per_block =
                treasury_emission / Balance::from(T::CycleConfiguration::blocks_per_cycle());

            // 3.2. dApp reward pool per era
            let dapp_reward_pool_per_era = dapps_emission
                / Balance::from(T::CycleConfiguration::build_and_earn_eras_per_cycle());

            // 3.3. Staking reward pools per era
            let base_staker_reward_pool_per_era = base_stakers_emission
                / Balance::from(T::CycleConfiguration::build_and_earn_eras_per_cycle());
            let adjustable_staker_reward_pool_per_era = adjustable_stakers_emission
                / Balance::from(T::CycleConfiguration::build_and_earn_eras_per_cycle());

            // 3.4. Bonus reward pool per period
            let bonus_reward_pool_per_period =
                bonus_emission / Balance::from(T::CycleConfiguration::periods_per_cycle());

            // 4. Block at which the inflation must be recalculated.
            let recalculation_block = now.saturating_add(T::CycleConfiguration::blocks_per_cycle());

            // 5. Return calculated values
            (
                max_emission,
                InflationConfiguration {
                    recalculation_block,
                    collator_reward_per_block,
                    treasury_reward_per_block,
                    dapp_reward_pool_per_era,
                    base_staker_reward_pool_per_era,
                    adjustable_staker_reward_pool_per_era,
                    bonus_reward_pool_per_period,
                    ideal_staking_rate: params.ideal_staking_rate,
                },
            )
        }
    }

    impl<T: Config> StakingRewardHandler<T::AccountId> for Pallet<T> {
        fn staker_and_dapp_reward_pools(total_value_staked: Balance) -> (Balance, Balance) {
            let config = InflationConfig::<T>::get();

            // First calculate the adjustable part of the staker reward pool, according to formula:
            // adjustable_part = max_adjustable_part * min(1, total_staked_percent / ideal_staked_percent)
            let total_issuance = T::Currency::total_issuance();

            // These operations are overflow & zero-division safe.
            let staked_ratio = Perquintill::from_rational(total_value_staked, total_issuance);
            let adjustment_factor = staked_ratio / config.ideal_staking_rate;

            let adjustable_part = adjustment_factor * config.adjustable_staker_reward_pool_per_era;
            let staker_reward_pool = config
                .base_staker_reward_pool_per_era
                .saturating_add(adjustable_part);

            (staker_reward_pool, config.dapp_reward_pool_per_era)
        }

        fn bonus_reward_pool() -> Balance {
            InflationConfig::<T>::get().bonus_reward_pool_per_period
        }

        fn payout_reward(reward: Balance, account: &T::AccountId) -> Result<(), ()> {
            let mut tracker = SafetyInflationTracker::<T>::get();

            // This is a safety measure to prevent excessive minting.
            // TODO: discuss this in review with the team. Is it strict enough? Should we use a different approach?
            tracker.issued.saturating_accrue(reward);
            ensure!(tracker.issued <= tracker.cap, ());
            SafetyInflationTracker::<T>::put(tracker);

            // This can fail only if the amount is below existential deposit & the account doesn't exist,
            // or if the account has no provider references.
            // In both cases, the reward is lost but this can be ignored since it's extremelly unlikely
            // to appear and doesn't bring any real harm.
            T::Currency::deposit_creating(account, reward);
            Ok(())
        }
    }
}

/// Configuration of the inflation.
/// Contains information about rewards, when inflation is recalculated, etc.
#[derive(Encode, Decode, MaxEncodedLen, Default, Copy, Clone, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct InflationConfiguration {
    /// Block number at which the inflation must be recalculated, based on the total issuance at that block.
    #[codec(compact)]
    pub recalculation_block: BlockNumber,
    /// Reward for collator who produced the block. Always deposited the collator in full.
    #[codec(compact)]
    pub collator_reward_per_block: Balance,
    /// Part of the inflation going towards the treasury. Always deposited in full.
    #[codec(compact)]
    pub treasury_reward_per_block: Balance,
    /// dApp reward pool per era - based on this the tier rewards are calculated.
    /// There's no guarantee that this whole amount will be minted & distributed.
    #[codec(compact)]
    pub dapp_reward_pool_per_era: Balance,
    /// Base staker reward pool per era - this is always provided to stakers, regardless of the total value staked.
    #[codec(compact)]
    pub base_staker_reward_pool_per_era: Balance,
    /// Adjustabke staker rewards, based on the total value staked.
    /// This is provided to the stakers according to formula: 'pool * min(1, total_staked / ideal_staked)'.
    #[codec(compact)]
    pub adjustable_staker_reward_pool_per_era: Balance,
    /// Bonus reward pool per period, for loyal stakers.
    #[codec(compact)]
    pub bonus_reward_pool_per_period: Balance,
    /// The ideal staking rate, in respect to total issuance.
    /// Used to derive exact amount of adjustable staker rewards.
    #[codec(compact)]
    pub ideal_staking_rate: Perquintill,
}

/// Inflation parameters.
///
/// The parts of the inflation that go towards different purposes must add up to exactly 100%.
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Default, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct InflationParameters {
    /// Maximum possible inflation rate, based on the total issuance at some point in time.
    /// From this value, all the other inflation parameters are derived.
    #[codec(compact)]
    pub max_inflation_rate: Perquintill,
    /// How much of the inflation in total goes towards the treasury.
    #[codec(compact)]
    pub treasury_part: Perquintill,
    /// How much of the inflation in total goes towards collators.
    #[codec(compact)]
    pub collators_part: Perquintill,
    /// How much of the inflation in total goes towards dApp rewards (tier rewards).
    #[codec(compact)]
    pub dapps_part: Perquintill,
    /// How much of the inflation in total goes towards base staker rewards.
    #[codec(compact)]
    pub base_stakers_part: Perquintill,
    /// How much of the inflation in total can go towards adjustable staker rewards.
    /// These rewards are adjusted based on the total value staked.
    #[codec(compact)]
    pub adjustable_stakers_part: Perquintill,
    /// How much of the inflation in total goes towards bonus staker rewards (loyalty rewards).
    #[codec(compact)]
    pub bonus_part: Perquintill,
    /// The ideal staking rate, in respect to total issuance.
    /// Used to derive exact amount of adjustable staker rewards.
    #[codec(compact)]
    pub ideal_staking_rate: Perquintill,
}

impl InflationParameters {
    /// `true` if sum of all percentages is `one whole`, `false` otherwise.
    pub fn is_valid(&self) -> bool {
        let variables = [
            &self.treasury_part,
            &self.collators_part,
            &self.dapps_part,
            &self.base_stakers_part,
            &self.adjustable_stakers_part,
            &self.bonus_part,
        ];

        variables
            .iter()
            .fold(Some(Perquintill::zero()), |acc, part| {
                if let Some(acc) = acc {
                    acc.checked_add(*part)
                } else {
                    None
                }
            })
            == Some(Perquintill::one())
    }
}

/// A safety-measure to ensure we never issue more inflation than we are supposed to.
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Default, Debug, PartialEq, Eq, TypeInfo)]
pub struct InflationTracker {
    /// The amount of inflation 'approved' for issuance so far.
    #[codec(compact)]
    cap: Balance,
    /// The amount of inflation issued so far.
    /// Must never exceed the `cap`.
    #[codec(compact)]
    issued: Balance,
}

/// Defines functions used to payout the beneficiaries of block rewards
pub trait PayoutPerBlock<Imbalance> {
    /// Payout reward to the treasury.
    fn treasury(reward: Imbalance);

    /// Payout reward to the collator responsible for producing the block.
    fn collators(reward: Imbalance);
}

// TODO: This should be moved to primitives.
// TODO2: However this ends up looking in the end, we should not duplicate these parameters in the runtime.
//        Both the dApp staking & inflation pallet should use the same source.
pub trait CycleConfiguration {
    /// How many different periods are there in a cycle (a 'year').
    fn periods_per_cycle() -> u32;

    /// For how many standard era lengths does the voting subperiod last.
    fn eras_per_voting_subperiod() -> u32;

    /// How many standard eras are there in the build&earn subperiod.
    fn eras_per_build_and_earn_subperiod() -> u32;

    /// How many blocks are there per standard era.
    fn blocks_per_era() -> u32;

    /// For how many standard era lengths does the period last.
    fn eras_per_period() -> u32 {
        Self::eras_per_voting_subperiod().saturating_add(Self::eras_per_build_and_earn_subperiod())
    }

    /// For how many standard era lengths does the cylce (a 'year') last.
    fn eras_per_cycle() -> u32 {
        Self::eras_per_period().saturating_mul(Self::periods_per_cycle())
    }

    /// How many blocks are there per cycle (a 'year').
    fn blocks_per_cycle() -> u32 {
        Self::blocks_per_era().saturating_mul(Self::eras_per_cycle())
    }

    /// For how many standard era lengths do all the build&earn subperiods in a cycle last.    
    fn build_and_earn_eras_per_cycle() -> u32 {
        Self::eras_per_build_and_earn_subperiod().saturating_mul(Self::periods_per_cycle())
    }
}

// TODO: This should be moved to primitives.
/// Interface for staking reward handler.
///
/// Provides reward pool values for stakers - normal & bonus rewards, as well as dApp reward pool.
/// Also provides a safe function for paying out rewards.
pub trait StakingRewardHandler<AccountId> {
    /// Returns the staker reward pool & dApp reward pool for an era.
    ///
    /// The total staker reward pool is dynamic and depends on the total value staked.
    fn staker_and_dapp_reward_pools(total_value_staked: Balance) -> (Balance, Balance);

    /// Returns the bonus reward pool for a period.
    fn bonus_reward_pool() -> Balance;

    /// Attempts to pay out the rewards to the beneficiary.
    fn payout_reward(reward: Balance, beneficiary: &AccountId) -> Result<(), ()>;
}
