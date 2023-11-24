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
use frame_support::pallet_prelude::*;
use frame_support::{
    log,
    traits::{Currency, Get, Imbalance, OnTimestampSet},
};
use frame_system::{ensure_root, pallet_prelude::*};
use sp_runtime::{
    traits::{CheckedAdd, Zero},
    Perquintill,
};
use sp_std::vec;

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
    pub trait Config: frame_system::Config {
        // TODO: modify this so it doesn't use deprecated trait?
        /// The currency trait.
        type Currency: Currency<Self::AccountId, Balance = Balance>;

        /// Handler for 'per-block' payouts.
        type PayoutPerBlock: PayoutPerBlock<NegativeImbalanceOf<Self>>;

        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Distribution configuration has been updated.
        DistributionConfigurationChanged,
    }

    /// Active inflation configuration parameteres.
    /// They describe current rewards, when inflation needs to be recalculated, etc.
    #[pallet::storage]
    pub type InflationConfig<T: Config> = StorageValue<_, InflationConfiguration, ValueQuery>;

    /// Static inflation parameters - used to calculate active inflation configuration at certain points in time.
    #[pallet::storage]
    pub type InflationParams<T: Config> = StorageValue<_, InflationParameters, ValueQuery>;

    impl<Moment, T: Config> OnTimestampSet<Moment> for Pallet<T> {
        fn on_timestamp_set(_moment: Moment) {
            Self::payout_block_rewards();
        }
    }

    impl<T: Config> Pallet<T> {
        /// Payout block rewards to the beneficiaries.
        pub(crate) fn payout_block_rewards() {
            let config = InflationConfig::<T>::get();

            let collator_amount = T::Currency::issue(config.collator_reward_per_block);
            let treasury_amount = T::Currency::issue(config.treasury_reward_per_block);

            T::PayoutPerBlock::collators(collator_amount);
            T::PayoutPerBlock::treasury(treasury_amount);

            // TODO: benchmark this and include it into on_initialize weight cost
        }
    }
}

/// Configuration of the inflation.
/// Contains information about rewards, when inflation is recalculated, etc.
#[derive(Encode, Decode, MaxEncodedLen, Default, Copy, Clone, Debug, PartialEq, Eq, TypeInfo)]
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
}

/// Inflation parameters.
///
/// The parts of the inflation that go towards different purposes must add up to exactly 100%.
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Default, Debug, PartialEq, Eq, TypeInfo)]
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

/// Defines functions used to payout the beneficiaries of block rewards
pub trait PayoutPerBlock<Imbalance> {
    /// Payout reward to the treasury.
    fn treasury(reward: Imbalance);

    /// Payout reward to the collator responsible for producing the block.
    fn collators(reward: Imbalance);
}
