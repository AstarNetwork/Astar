// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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

//! # Inflation Handler Pallet
//!
//! ## Overview
//!
//! This pallet's main responsibility is handling inflation calculation & distribution.
//!
//! Inflation configuration is calculated periodically, according to the inflation parameters.
//! Based on this configuration, rewards are paid out - either per block or on demand.
//!
//! ## Cycles, Periods, Eras
//!
//! At the start of each cycle, the inflation configuration is recalculated.
//!
//! Cycle can be considered as a 'year' in the Astar network.
//! When cycle starts, inflation is calculated according to the total issuance at that point in time.
//! E.g. if 'yearly' inflation is set to be 7%, and total issuance is 200 ASTR, then the max inflation for that cycle will be 14 ASTR.
//!
//! Each cycle consists of one or more `periods`.
//! Periods are integral part of dApp staking protocol, allowing dApps to promote themselves, attract stakers and earn rewards.
//! At the end of each period, all stakes are reset, and dApps need to repeat the process.
//!
//! Each period consists of two subperiods: `Voting` and `Build&Earn`.
//! Length of these subperiods is expressed in eras. An `era` is the core _time unit_ in dApp staking protocol.
//! When an era ends, in `Build&Earn` subperiod, rewards for dApps are calculated & assigned.
//!
//! Era's length is expressed in blocks. E.g. an era can last for 7200 blocks, which is approximately 1 day for 12 second block time.
//!
//! `Build&Earn` subperiod length is expressed in eras. E.g. if `Build&Earn` subperiod lasts for 5 eras, it means that during that subperiod,
//! dApp rewards will be calculated & assigned 5 times in total. Also, 5 distinct eras will change during that subperiod. If e.g. `Build&Earn` started at era 100,
//! with 5 eras per `Build&Earn` subperiod, then the subperiod will end at era 105.
//!
//! `Voting` subperiod always comes before `Build&Earn` subperiod. Its length is also expressed in eras, although it has to be interpreted a bit differently.
//! Even though `Voting` can last for more than 1 era in respect of length, it always takes exactly 1 era.
//! What this means is that if `Voting` lasts for 3 eras, and each era lasts 7200 blocks, then `Voting` will last for 21600 blocks.
//! But unlike `Build&Earn` subperiod, `Voting` will only take up one 'numerical' era. So if `Voting` starts at era 110, it will end at era 11.
//!
//! #### Example
//! * Cycle length: 4 periods
//! * `Voting` length: 10 eras
//! * `Build&Earn` length: 81 eras
//! * Era length: 7200 blocks
//!
//! This would mean that cycle lasts for roughly 364 days (4 * (10 + 81)).
//!
//! ## Recalculation
//!
//! When new cycle begins, inflation configuration is recalculated according to the inflation parameters & total issuance at that point in time.
//! Based on the max inflation rate, rewards for different network actors are calculated.
//!
//! Some rewards are calculated to be paid out per block, while some are per era or per period.
//!
//! ## Rewards
//!
//! ### Collator & Treasury Rewards
//!
//! These are paid out at the beginning of each block & are fixed amounts.
//!
//! ### Staker Rewards
//!
//! Staker rewards are paid out per staker, _on-demand_.
//! However, reward pool for an era is calculated at the end of each era.
//!
//! `era_reward_pool = base_staker_reward_pool_per_era + adjustable_staker_reward_pool_per_era`
//!
//! While the base staker reward pool is fixed, the adjustable part is calculated according to the total value staked & the ideal staking rate.
//!
//! ### dApp Rewards
//!
//! dApp rewards are paid out per dApp, _on-demand_. The reward is decided by the dApp staking protocol, or the tier system to be more precise.
//! This pallet only provides the total reward pool for all dApps per era.
//!
//! # Interface
//!
//! ## StakingRewardHandler
//!
//! This pallet implements `StakingRewardHandler` trait, which is used by the dApp staking protocol to get reward pools & distribute rewards.
//!

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use astar_primitives::{
    dapp_staking::{
        CycleConfiguration, EraNumber, Observer as DappStakingObserver, StakingRewardHandler,
    },
    Balance,
};
use frame_support::{
    pallet_prelude::*,
    traits::{
        fungible::{Balanced, Credit, Inspect},
        tokens::Precision,
    },
    DefaultNoBound,
};
use frame_system::{ensure_root, pallet_prelude::*};
use serde::{Deserialize, Serialize};
use sp_runtime::{
    traits::{CheckedAdd, Zero},
    Perquintill,
};
use sp_std::marker::PhantomData;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(any(feature = "runtime-benchmarks"))]
pub mod benchmarking;

pub mod migration;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    /// The current storage version.
    pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    // Negative imbalance type of this pallet.
    pub(crate) type CreditOf<T> =
        Credit<<T as frame_system::Config>::AccountId, <T as Config>::Currency>;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Currency: Balanced<Self::AccountId, Balance = Balance>;

        /// Handler for 'per-block' payouts.
        type PayoutPerBlock: PayoutPerBlock<CreditOf<Self>>;

        /// Cycle ('year') configuration - covers periods, subperiods, eras & blocks.
        type CycleConfiguration: CycleConfiguration;

        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Inflation parameters have been force changed. This will have effect on the next inflation recalculation.
        InflationParametersForceChanged,
        /// Inflation recalculation has been forced.
        ForcedInflationRecalculation { config: InflationConfiguration },
        /// New inflation configuration has been set.
        NewInflationConfiguration { config: InflationConfiguration },
        /// Inflation decay factor has been updated.
        DecayFactorUpdated { decay_factor: Perquintill },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Sum of all parts must be one whole (100%).
        InvalidInflationParameters,
    }

    /// Active inflation configuration parameters.
    /// They describe current rewards, when inflation needs to be recalculated, etc.
    #[pallet::storage]
    #[pallet::whitelist_storage]
    pub type ActiveInflationConfig<T: Config> = StorageValue<_, InflationConfiguration, ValueQuery>;

    /// Static inflation parameters - used to calculate active inflation configuration at certain points in time.
    #[pallet::storage]
    pub type InflationParams<T: Config> = StorageValue<_, InflationParameters, ValueQuery>;

    /// Flag indicating whether on the first possible opportunity, recalculation of the inflation config should be done.
    #[pallet::storage]
    #[pallet::whitelist_storage]
    pub type DoRecalculation<T: Config> = StorageValue<_, EraNumber, OptionQuery>;

    #[pallet::genesis_config]
    #[derive(DefaultNoBound)]
    pub struct GenesisConfig<T> {
        pub params: InflationParameters,
        #[serde(skip)]
        pub _config: sp_std::marker::PhantomData<T>,
    }

    /// This should be executed **AFTER** other pallets that cause issuance to increase have been initialized.
    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            assert!(self.params.is_valid());

            let starting_era = 1;
            let config = Pallet::<T>::recalculate_inflation(starting_era);

            ActiveInflationConfig::<T>::put(config);
            InflationParams::<T>::put(self.params);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
            let mut weight = T::DbWeight::get().reads(1);
            let mut config = ActiveInflationConfig::<T>::get();

            if config.decay_rate != Perquintill::one() {
                config.decay_factor = config.decay_factor * config.decay_rate;
                ActiveInflationConfig::<T>::put(config);
                weight = weight.saturating_add(T::DbWeight::get().writes(1));
            }

            Self::payout_block_rewards(&config);

            // Benchmarks won't account for the whitelisted storage access so this needs to be added manually.
            // DoRecalculation - 1 DB read
            weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().reads(1));

            weight
        }

        fn on_finalize(_now: BlockNumberFor<T>) {
            // Recalculation is done at the block right before a new cycle starts.
            // This is to ensure all the rewards are paid out according to the new inflation configuration from next block.
            //
            // If this was done in `on_initialize`, collator & treasury would receive incorrect rewards for that one block.
            //
            // This should be done as late as possible, to ensure all operations that modify issuance are done.
            if let Some(next_era) = DoRecalculation::<T>::get() {
                let current_config = ActiveInflationConfig::<T>::get();
                let mut new_config = Self::recalculate_inflation(next_era);
                // preserve the current decay factor
                new_config.decay_factor = current_config.decay_factor;

                ActiveInflationConfig::<T>::put(new_config.clone());
                DoRecalculation::<T>::kill();

                Self::deposit_event(Event::<T>::NewInflationConfiguration { config: new_config });
            }

            // NOTE: weight of the `on_finalize` logic with recalculation has to be covered by the observer notify call.
        }

        fn integrity_test() {
            assert!(T::CycleConfiguration::periods_per_cycle() > 0);
            assert!(T::CycleConfiguration::eras_per_voting_subperiod() > 0);
            assert!(T::CycleConfiguration::eras_per_build_and_earn_subperiod() > 0);
            assert!(T::CycleConfiguration::blocks_per_era() > 0);
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Used to force-set the inflation parameters.
        /// The parameters must be valid, all parts summing up to one whole (100%), otherwise the call will fail.
        ///
        /// Must be called by `root` origin.
        ///
        /// Purpose of the call is testing & handling unforeseen circumstances.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::force_set_inflation_params())]
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

        /// Used to force inflation recalculation.
        /// This is done in the same way as it would be done in an appropriate block, but this call forces it.
        ///
        /// Must be called by `root` origin.
        ///
        /// Purpose of the call is testing & handling unforeseen circumstances.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::force_inflation_recalculation().saturating_add(T::DbWeight::get().writes(1)))]
        pub fn force_inflation_recalculation(
            origin: OriginFor<T>,
            next_era: EraNumber,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let current_config = ActiveInflationConfig::<T>::get();
            let mut new_config = Self::recalculate_inflation(next_era);
            // preserve the current decay factor
            new_config.decay_factor = current_config.decay_factor;
            ActiveInflationConfig::<T>::put(new_config.clone());

            Self::deposit_event(Event::<T>::ForcedInflationRecalculation { config: new_config });

            Ok(().into())
        }

        /// Re-adjust the existing inflation configuration using the current inflation parameters.
        ///
        /// It might seem similar to forcing the inflation recalculation, but it's not.
        /// This function adjusts the existing configuration, respecting the `max_emission` value used to calculate the current inflation config.
        /// (The 'force' approach uses the current total issuance)
        ///
        /// This call should be used in case inflation parameters have changed during the cycle, and the configuration should be adjusted now.
        ///
        /// NOTE:
        /// The call will do the best possible approximation of what the calculated max emission was at the moment when last inflation recalculation was done.
        /// But due to rounding losses, it's not possible to get the exact same value. As a consequence, repeated calls to this function
        /// might result in changes to the configuration, even though the inflation parameters haven't changed.
        /// However, since this function isn't supposed to be called often, and changes are minimal, this is acceptable.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::force_readjust_config().saturating_add(T::DbWeight::get().writes(1)))]
        pub fn force_readjust_config(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;

            let config = Self::readjusted_config();
            ActiveInflationConfig::<T>::put(config.clone());

            Self::deposit_event(Event::<T>::ForcedInflationRecalculation { config });

            Ok(().into())
        }

        /// Used to force-set the decay factor for reward payouts.
        ///
        /// Must be called by `root` origin.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::force_set_decay_factor())]
        pub fn force_set_decay_factor(
            origin: OriginFor<T>,
            decay_factor: Perquintill,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ActiveInflationConfig::<T>::mutate(|config| {
                config.decay_factor = decay_factor;
            });

            Self::deposit_event(Event::<T>::DecayFactorUpdated { decay_factor });

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Payout block rewards to the beneficiaries applying the decay factor.
        ///
        /// Return the total amount issued.
        fn payout_block_rewards(config: &InflationConfiguration) {
            let collator_rewards = config.decay_factor * config.collator_reward_per_block;
            let treasury_rewards = config.decay_factor * config.treasury_reward_per_block;

            let collator_amount = T::Currency::issue(collator_rewards);
            let treasury_amount = T::Currency::issue(treasury_rewards);

            T::PayoutPerBlock::collators(collator_amount);
            T::PayoutPerBlock::treasury(treasury_amount);
        }

        /// Recalculates the inflation based on the current total issuance & inflation parameters.
        ///
        /// Returns the new inflation configuration with default (no) decay factor.
        pub(crate) fn recalculate_inflation(next_era: EraNumber) -> InflationConfiguration {
            // Calculate max emission based on the current total issuance.
            let params = InflationParams::<T>::get();
            let total_issuance = T::Currency::total_issuance();
            let max_emission = params.max_inflation_rate * total_issuance;

            let recalculation_era =
                next_era.saturating_add(T::CycleConfiguration::eras_per_cycle());

            Self::new_config(recalculation_era, max_emission)
        }

        /// Re-adjust the existing inflation configuration using the current inflation parameters.
        ///
        /// It might seem similar to forcing the inflation recalculation, but it's not.
        /// This function adjusts the existing configuration, respecting the `max_emission` value used to calculate the current inflation config.
        /// (The 'force' approach uses the current total issuance)
        ///
        /// This call should be used in case inflation parameters have changed during the cycle, and the configuration should be adjusted now.
        pub(crate) fn readjusted_config() -> InflationConfiguration {
            // 1. First calculate the params needed to derive the `max_emission` value used to calculate the current inflation config.
            let config = ActiveInflationConfig::<T>::get();

            // Simple type conversion.
            let blocks_per_cycle = Balance::from(T::CycleConfiguration::blocks_per_cycle());
            let build_and_earn_eras_per_cycle =
                Balance::from(T::CycleConfiguration::build_and_earn_eras_per_cycle());
            let periods_per_cycle = Balance::from(T::CycleConfiguration::periods_per_cycle());

            // 2. Calculate reward pool amounts per cycle from the existing inflation configuration.
            let collator_reward_pool = config
                .collator_reward_per_block
                .saturating_mul(blocks_per_cycle);

            let treasury_reward_pool = config
                .treasury_reward_per_block
                .saturating_mul(blocks_per_cycle);

            let dapp_reward_pool = config
                .dapp_reward_pool_per_era
                .saturating_mul(build_and_earn_eras_per_cycle);

            let base_staker_reward_pool = config
                .base_staker_reward_pool_per_era
                .saturating_mul(build_and_earn_eras_per_cycle);
            let adjustable_staker_reward_pool = config
                .adjustable_staker_reward_pool_per_era
                .saturating_mul(build_and_earn_eras_per_cycle);

            let bonus_reward_pool = config
                .bonus_reward_pool_per_period
                .saturating_mul(periods_per_cycle);

            // 3. Sum up all values to get the old `max_emission` value.
            let max_emission = collator_reward_pool
                .saturating_add(treasury_reward_pool)
                .saturating_add(dapp_reward_pool)
                .saturating_add(base_staker_reward_pool)
                .saturating_add(adjustable_staker_reward_pool)
                .saturating_add(bonus_reward_pool);

            // 4. Calculate new inflation configuration
            let mut new_config = Self::new_config(config.recalculation_era, max_emission);
            new_config.decay_factor = config.decay_factor;
            new_config
        }

        // Calculate new inflation configuration, based on the provided `max_emission`.
        fn new_config(
            recalculation_era: EraNumber,
            max_emission: Balance,
        ) -> InflationConfiguration {
            let params = InflationParams::<T>::get();

            // Invalidated parameter, should be cleaned up in the future.
            // The reason for it's invalidity is because since we've entered the 2nd cycle, it's possible for total
            // issuance to exceed this cap if unclaimed rewards from previous cycle are claimed.
            //
            // In future upgrades, the storage scheme can be updated to completely clean this up.
            let issuance_safety_cap = Balance::MAX / 1000;

            // 1. Calculate distribution of max emission between different purposes.
            let treasury_emission = params.treasury_part * max_emission;
            let collators_emission = params.collators_part * max_emission;
            let dapps_emission = params.dapps_part * max_emission;
            let base_stakers_emission = params.base_stakers_part * max_emission;
            let adjustable_stakers_emission = params.adjustable_stakers_part * max_emission;
            let bonus_emission = params.bonus_part * max_emission;

            // 2. Calculate concrete rewards per block, era or period

            // 2.0 Convert all 'per cycle' values to the correct type (Balance).
            // Also include a safety check that none of the values is zero since this would cause a division by zero.
            // The configuration & integration tests must ensure this never happens, so the following code is just an additional safety measure.
            //
            // NOTE: Using `max(1)` to eliminate possibility of division by zero.
            // These values should never be 0 anyways, but this is just a safety measure.
            let blocks_per_cycle = Balance::from(T::CycleConfiguration::blocks_per_cycle().max(1));
            let build_and_earn_eras_per_cycle =
                Balance::from(T::CycleConfiguration::build_and_earn_eras_per_cycle().max(1));
            let periods_per_cycle =
                Balance::from(T::CycleConfiguration::periods_per_cycle().max(1));

            // 2.1. Collator & Treasury rewards per block
            let collator_reward_per_block = collators_emission.saturating_div(blocks_per_cycle);
            let treasury_reward_per_block = treasury_emission.saturating_div(blocks_per_cycle);

            // 2.2. dApp reward pool per era
            let dapp_reward_pool_per_era =
                dapps_emission.saturating_div(build_and_earn_eras_per_cycle);

            // 2.3. Staking reward pools per era
            let base_staker_reward_pool_per_era =
                base_stakers_emission.saturating_div(build_and_earn_eras_per_cycle);
            let adjustable_staker_reward_pool_per_era =
                adjustable_stakers_emission.saturating_div(build_and_earn_eras_per_cycle);

            // 2.4. Bonus reward pool per period
            let bonus_reward_pool_per_period = bonus_emission.saturating_div(periods_per_cycle);

            // 3. Prepare config & do sanity check of its values.
            let new_inflation_config = InflationConfiguration {
                recalculation_era,
                issuance_safety_cap,
                collator_reward_per_block,
                treasury_reward_per_block,
                dapp_reward_pool_per_era,
                base_staker_reward_pool_per_era,
                adjustable_staker_reward_pool_per_era,
                bonus_reward_pool_per_period,
                ideal_staking_rate: params.ideal_staking_rate,
                decay_rate: params.decay_rate,
                decay_factor: Perquintill::one(),
            };
            new_inflation_config.sanity_check();

            new_inflation_config
        }
    }

    impl<T: Config> DappStakingObserver for Pallet<T> {
        /// Informs the pallet that the next block will be the first block of a new era.
        fn block_before_new_era(new_era: EraNumber) -> Weight {
            let config = ActiveInflationConfig::<T>::get();
            if config.recalculation_era <= new_era {
                DoRecalculation::<T>::put(new_era);

                // Need to account for write into a single whitelisted storage item.
                T::WeightInfo::recalculation().saturating_add(T::DbWeight::get().writes(1))
            } else {
                Weight::zero()
            }
        }
    }

    impl<T: Config> StakingRewardHandler<T::AccountId> for Pallet<T> {
        fn staker_and_dapp_reward_pools(total_value_staked: Balance) -> (Balance, Balance) {
            let config = ActiveInflationConfig::<T>::get();
            let total_issuance = T::Currency::total_issuance();

            // First calculate the adjustable part of the staker reward pool, according to formula:
            // adjustable_part = max_adjustable_part * min(1, total_staked_percent / ideal_staked_percent)
            // (These operations are overflow & zero-division safe)
            let staked_ratio = Perquintill::from_rational(total_value_staked, total_issuance);
            let adjustment_factor = staked_ratio / config.ideal_staking_rate;

            let adjustable_part = adjustment_factor * config.adjustable_staker_reward_pool_per_era;
            let staker_reward_pool = config.decay_factor * config
                .base_staker_reward_pool_per_era
                .saturating_add(adjustable_part);
            let dapp_reward_pool = config.decay_factor * config.dapp_reward_pool_per_era;

            (staker_reward_pool, dapp_reward_pool)
        }

        fn bonus_reward_pool() -> Balance {
            let config = ActiveInflationConfig::<T>::get();
            config.decay_factor * config.bonus_reward_pool_per_period
        }

        fn payout_reward(account: &T::AccountId, reward: Balance) -> Result<(), ()> {
            // This can fail only if the amount is below existential deposit & the account doesn't exist,
            // or if the account has no provider references.
            // Another possibility is overflow, but if that happens, we already have a huge problem.
            //
            // In both cases, the reward is lost but this can be ignored since it's extremely unlikely
            // to appear and doesn't bring any real harm.
            let _ = T::Currency::deposit(account, reward, Precision::Exact);
            Ok(())
        }
    }
}

/// Configuration of the inflation.
/// Contains information about rewards, when inflation is recalculated, etc.
#[derive(Encode, Decode, MaxEncodedLen, Default, Copy, Clone, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct InflationConfiguration {
    /// Era number at which the inflation configuration must be recalculated, based on the total issuance at that block.
    #[codec(compact)]
    pub recalculation_era: EraNumber,
    /// Maximum amount of issuance we can have during this cycle.
    #[codec(compact)]
    pub issuance_safety_cap: Balance,
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
    /// Adjustable staker rewards, based on the total value staked.
    /// This is provided to the stakers according to formula: 'pool * min(1, total_staked / ideal_staked)'.
    #[codec(compact)]
    pub adjustable_staker_reward_pool_per_era: Balance,
    /// Bonus reward pool per period, for eligible stakers.
    #[codec(compact)]
    pub bonus_reward_pool_per_period: Balance,
    /// The ideal staking rate, in respect to total issuance.
    /// Used to derive exact amount of adjustable staker rewards.
    #[codec(compact)]
    pub ideal_staking_rate: Perquintill,
    /// Per-block decay rate applied to the decay factor.
    /// A value of `Perquintill::one()` means no decay.
    #[codec(compact)]
    pub decay_rate: Perquintill,
    /// Compounded decay multiplied into rewards when they are actually paid.
    /// A value of `Perquintill::one()` means no decay.
    #[codec(compact)]
    pub decay_factor: Perquintill,
}

impl InflationConfiguration {
    /// Sanity check that does rudimentary checks on the configuration and prints warnings if something is unexpected.
    ///
    /// There are no strict checks, since the configuration values aren't strictly bounded like those of the parameters.
    pub fn sanity_check(&self) {
        if self.collator_reward_per_block.is_zero() {
            log::warn!("Collator reward per block is zero. If this is not expected, please report this to Astar team.");
        }
        if self.treasury_reward_per_block.is_zero() {
            log::warn!("Treasury reward per block is zero. If this is not expected, please report this to Astar team.");
        }
        if self.dapp_reward_pool_per_era.is_zero() {
            log::warn!("dApp reward pool per era is zero. If this is not expected, please report this to Astar team.");
        }
        if self.base_staker_reward_pool_per_era.is_zero() {
            log::warn!("Base staker reward pool per era is zero.  If this is not expected, please report this to Astar team.");
        }
        if self.adjustable_staker_reward_pool_per_era.is_zero() {
            log::warn!("Adjustable staker reward pool per era is zero.  If this is not expected, please report this to Astar team.");
        }
        if self.bonus_reward_pool_per_period.is_zero() {
            log::warn!("Bonus reward pool per period is zero.  If this is not expected, please report this to Astar team.");
        }
    }
}

/// Inflation parameters.
///
/// The parts of the inflation that go towards different purposes must add up to exactly 100%.
#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    TypeInfo,
    Serialize,
    Deserialize,
)]
pub struct InflationParameters {
    /// Maximum possible inflation rate, based on the total issuance at some point in time.
    /// From this value, all the other inflation parameters are derived.
    #[codec(compact)]
    pub max_inflation_rate: Perquintill,
    /// Portion of the inflation that goes towards the treasury.
    #[codec(compact)]
    pub treasury_part: Perquintill,
    /// Portion of the inflation that goes towards collators.
    #[codec(compact)]
    pub collators_part: Perquintill,
    /// Portion of the inflation that goes towards dApp rewards (tier rewards).
    #[codec(compact)]
    pub dapps_part: Perquintill,
    /// Portion of the inflation that goes towards base staker rewards.
    #[codec(compact)]
    pub base_stakers_part: Perquintill,
    /// Portion of the inflation that can go towards the adjustable staker rewards.
    /// These rewards are adjusted based on the total value staked.
    #[codec(compact)]
    pub adjustable_stakers_part: Perquintill,
    /// Portion of the inflation that goes towards bonus staker rewards (loyalty rewards).
    #[codec(compact)]
    pub bonus_part: Perquintill,
    /// The ideal staking rate, in respect to total issuance.
    /// Used to derive exact amount of adjustable staker rewards.
    #[codec(compact)]
    pub ideal_staking_rate: Perquintill,
    /// Per-block decay rate applied to all reward pools and per-block rewards.
    /// A value of `Perquintill::one()` means no decay.
    #[codec(compact)]
    pub decay_rate: Perquintill,
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

// Default inflation parameters, just to make sure genesis builder is happy
impl Default for InflationParameters {
    fn default() -> Self {
        Self {
            max_inflation_rate: Perquintill::from_percent(7),
            treasury_part: Perquintill::from_percent(5),
            collators_part: Perquintill::from_percent(3),
            dapps_part: Perquintill::from_percent(20),
            base_stakers_part: Perquintill::from_percent(25),
            adjustable_stakers_part: Perquintill::from_percent(35),
            bonus_part: Perquintill::from_percent(12),
            ideal_staking_rate: Perquintill::from_percent(50),
            decay_rate: Perquintill::one(),
        }
    }
}

/// Defines functions used to payout the beneficiaries of block rewards
pub trait PayoutPerBlock<Imbalance> {
    /// Payout reward to the treasury.
    fn treasury(reward: Imbalance);

    /// Payout reward to the collator responsible for producing the block.
    fn collators(reward: Imbalance);
}
