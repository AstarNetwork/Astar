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
//! Periods are integral part of dApp staking protocol, allowing dApps to promotove themselves, attract stakers and earn rewards.
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
//! ### Staker & Treasury Rewards
//!
//! These are paid out at the begininng of each block & are fixed amounts.
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
    dapp_staking::{CycleConfiguration, StakingRewardHandler},
    Balance, BlockNumber,
};
use frame_support::{
    pallet_prelude::*,
    traits::{Currency, GetStorageVersion, OnRuntimeUpgrade},
};
use frame_system::{ensure_root, pallet_prelude::*};
use sp_runtime::{traits::CheckedAdd, Perquintill};
use sp_std::marker::PhantomData;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(any(feature = "runtime-benchmarks"))]
pub mod benchmarking;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {

    use super::*;

    /// The current storage version.
    pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    // Negative imbalance type of this pallet.
    pub(crate) type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::NegativeImbalance;

    #[pallet::config]
    pub trait Config: frame_system::Config<BlockNumber = BlockNumber> {
        /// The currency trait.
        /// This has been soft-deprecated but it still needs to be used here in order to access `NegativeImbalance`
        /// which is defined in the currency trait.
        type Currency: Currency<Self::AccountId, Balance = Balance>;

        /// Handler for 'per-block' payouts.
        type PayoutPerBlock: PayoutPerBlock<NegativeImbalanceOf<Self>>;

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
    pub type ActiveInflationConfig<T: Config> = StorageValue<_, InflationConfiguration, ValueQuery>;

    /// Static inflation parameters - used to calculate active inflation configuration at certain points in time.
    #[pallet::storage]
    pub type InflationParams<T: Config> = StorageValue<_, InflationParameters, ValueQuery>;

    #[pallet::genesis_config]
    #[cfg_attr(feature = "std", derive(Default))]
    pub struct GenesisConfig {
        pub params: InflationParameters,
    }

    /// This should be executed **AFTER** other pallets that cause issuance to increase have been initialized.
    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            assert!(self.params.is_valid());

            let now = frame_system::Pallet::<T>::block_number();
            let config = Pallet::<T>::recalculate_inflation(now);

            ActiveInflationConfig::<T>::put(config);
            InflationParams::<T>::put(self.params);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumber> for Pallet<T> {
        fn on_initialize(now: BlockNumber) -> Weight {
            Self::payout_block_rewards();

            let recalculation_weight =
                if Self::is_recalculation_in_next_block(now, &ActiveInflationConfig::<T>::get()) {
                    T::WeightInfo::hook_with_recalculation()
                } else {
                    T::WeightInfo::hook_without_recalculation()
                };

            // Benchmarks won't acount for whitelisted storage access so this needs to be added manually.
            //
            // ActiveInflationConfig - 1 DB read
            let whitelisted_weight = <T as frame_system::Config>::DbWeight::get().reads(1);

            recalculation_weight.saturating_add(whitelisted_weight)
        }

        fn on_finalize(now: BlockNumber) {
            // Recalculation is done at the block right before the re-calculation is supposed to happen.
            // This is to ensure all the rewards are paid out according to the new inflation configuration from next block.
            //
            // If this was done in `on_initialize`, collator & treasury would receive incorrect rewards for that one block.
            //
            // This should be done as late as possible, to ensure all operations that modify issuance are done.
            if Self::is_recalculation_in_next_block(now, &ActiveInflationConfig::<T>::get()) {
                let config = Self::recalculate_inflation(now);
                ActiveInflationConfig::<T>::put(config.clone());

                Self::deposit_event(Event::<T>::NewInflationConfiguration { config });
            }
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
        /// Purpose of the call is testing & handling unforseen circumstances.
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
        /// Purpose of the call is testing & handling unforseen circumstances.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::force_inflation_recalculation())]
        pub fn force_inflation_recalculation(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;

            let config = Self::recalculate_inflation(frame_system::Pallet::<T>::block_number());

            ActiveInflationConfig::<T>::put(config.clone());

            Self::deposit_event(Event::<T>::ForcedInflationRecalculation { config });

            Ok(().into())
        }

        /// Used to force-set the inflation configuration.
        /// The parameters aren't checked for validity, since essentially anything can be valid.
        ///
        /// Must be called by `root` origin.
        ///
        /// Purpose of the call is testing & handling unforseen circumstances.
        ///
        /// **NOTE:** and a TODO, remove this before deploying on mainnet.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::force_set_inflation_config())]
        pub fn force_set_inflation_config(
            origin: OriginFor<T>,
            config: InflationConfiguration,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ActiveInflationConfig::<T>::put(config.clone());

            Self::deposit_event(Event::<T>::InflationConfigurationForceChanged { config });

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Used to check if inflation recalculation is supposed to happen on the next block.
        ///
        /// This will be true even if recalculation is overdue, e.g. it should have happened in the current or older block.
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
            let config = ActiveInflationConfig::<T>::get();

            let collator_amount = T::Currency::issue(config.collator_reward_per_block);
            let treasury_amount = T::Currency::issue(config.treasury_reward_per_block);

            T::PayoutPerBlock::collators(collator_amount);
            T::PayoutPerBlock::treasury(treasury_amount);

            config.collator_reward_per_block + config.treasury_reward_per_block
        }

        /// Recalculates the inflation based on the total issuance & inflation parameters.
        ///
        /// Returns the new inflation configuration.
        pub(crate) fn recalculate_inflation(now: BlockNumber) -> InflationConfiguration {
            let params = InflationParams::<T>::get();
            let total_issuance = T::Currency::total_issuance();

            // 1. Calculate maximum emission over the period before the next recalculation.
            let max_emission = params.max_inflation_rate * total_issuance;
            let issuance_safety_cap = total_issuance.saturating_add(max_emission);

            // 2. Calculate distribution of max emission between different purposes.
            let treasury_emission = params.treasury_part * max_emission;
            let collators_emission = params.collators_part * max_emission;
            let dapps_emission = params.dapps_part * max_emission;
            let base_stakers_emission = params.base_stakers_part * max_emission;
            let adjustable_stakers_emission = params.adjustable_stakers_part * max_emission;
            let bonus_emission = params.bonus_part * max_emission;

            // 3. Calculate concrete rewards per block, era or period

            // 3.0 Convert all 'per cycle' values to the correct type (Balance).
            // Also include a safety check that none of the values is zero since this would cause a division by zero.
            // The configuration & integration tests must ensure this never happens, so the following code is just an additional safety measure.
            let blocks_per_cycle = match T::CycleConfiguration::blocks_per_cycle() {
                0 => Balance::MAX,
                blocks_per_cycle => Balance::from(blocks_per_cycle),
            };

            let build_and_earn_eras_per_cycle =
                match T::CycleConfiguration::build_and_earn_eras_per_cycle() {
                    0 => Balance::MAX,
                    build_and_earn_eras_per_cycle => Balance::from(build_and_earn_eras_per_cycle),
                };

            let periods_per_cycle = match T::CycleConfiguration::periods_per_cycle() {
                0 => Balance::MAX,
                periods_per_cycle => Balance::from(periods_per_cycle),
            };

            // 3.1. Collator & Treausry rewards per block
            let collator_reward_per_block = collators_emission / blocks_per_cycle;
            let treasury_reward_per_block = treasury_emission / blocks_per_cycle;

            // 3.2. dApp reward pool per era
            let dapp_reward_pool_per_era = dapps_emission / build_and_earn_eras_per_cycle;

            // 3.3. Staking reward pools per era
            let base_staker_reward_pool_per_era =
                base_stakers_emission / build_and_earn_eras_per_cycle;
            let adjustable_staker_reward_pool_per_era =
                adjustable_stakers_emission / build_and_earn_eras_per_cycle;

            // 3.4. Bonus reward pool per period
            let bonus_reward_pool_per_period = bonus_emission / periods_per_cycle;

            // 4. Block at which the inflation must be recalculated.
            let recalculation_block = now.saturating_add(T::CycleConfiguration::blocks_per_cycle());

            // 5. Return calculated values
            InflationConfiguration {
                recalculation_block,
                issuance_safety_cap,
                collator_reward_per_block,
                treasury_reward_per_block,
                dapp_reward_pool_per_era,
                base_staker_reward_pool_per_era,
                adjustable_staker_reward_pool_per_era,
                bonus_reward_pool_per_period,
                ideal_staking_rate: params.ideal_staking_rate,
            }
        }

        /// Check if payout cap limit would be reached after payout.
        fn is_payout_cap_limit_exceeded(payout: Balance) -> bool {
            let config = ActiveInflationConfig::<T>::get();
            let total_issuance = T::Currency::total_issuance();

            let new_issuance = total_issuance.saturating_add(payout);

            if new_issuance > config.issuance_safety_cap {
                log::error!("Issuance cap has been exceeded. Please report this issue ASAP!");
            }

            // Allow for 1% safety cap overflow, to prevent bad UX for users in case of rounding errors.
            // This will be removed in the future once we know everything is working as expected.
            let relaxed_issuance_safety_cap = config
                .issuance_safety_cap
                .saturating_mul(101)
                .saturating_div(100);

            new_issuance > relaxed_issuance_safety_cap
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
            let staker_reward_pool = config
                .base_staker_reward_pool_per_era
                .saturating_add(adjustable_part);

            (staker_reward_pool, config.dapp_reward_pool_per_era)
        }

        fn bonus_reward_pool() -> Balance {
            ActiveInflationConfig::<T>::get().bonus_reward_pool_per_period
        }

        fn payout_reward(account: &T::AccountId, reward: Balance) -> Result<(), ()> {
            // This is a safety measure to prevent excessive minting.
            ensure!(!Self::is_payout_cap_limit_exceeded(reward), ());

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
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
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

/// `OnRuntimeUpgrade` logic for integrating this pallet into the live network.
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;
pub struct PalletInflationInitConfig<T, P>(PhantomData<(T, P)>);
impl<T: Config, P: Get<InflationParameters>> OnRuntimeUpgrade for PalletInflationInitConfig<T, P> {
    fn on_runtime_upgrade() -> Weight {
        if Pallet::<T>::on_chain_storage_version() >= STORAGE_VERSION {
            return T::DbWeight::get().reads(1);
        }

        // 1. Get & set inflation parameters
        let inflation_params = P::get();
        InflationParams::<T>::put(inflation_params.clone());

        // 2. Calculation inflation config, set it & depossit event
        let now = frame_system::Pallet::<T>::block_number();
        let config = Pallet::<T>::recalculate_inflation(now);
        ActiveInflationConfig::<T>::put(config.clone());

        Pallet::<T>::deposit_event(Event::<T>::NewInflationConfiguration { config });

        // 3. Set version
        STORAGE_VERSION.put::<Pallet<T>>();

        log::info!("Inflation pallet storage initialized.");

        T::WeightInfo::hook_with_recalculation()
            .saturating_add(T::DbWeight::get().reads_writes(1, 2))
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
        assert_eq!(Pallet::<T>::on_chain_storage_version(), STORAGE_VERSION);
        assert!(InflationParams::<T>::get().is_valid());

        Ok(())
    }
}
