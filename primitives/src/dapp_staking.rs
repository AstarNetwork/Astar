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

use super::{oracle::CurrencyAmount, Balance, BlockNumber};

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};

use frame_support::pallet_prelude::{RuntimeDebug, Weight};
use sp_arithmetic::ArithmeticError;
use sp_core::{DecodeWithMemTracking, H160};
use sp_runtime::{
    traits::{UniqueSaturatedInto, Zero},
    FixedPointNumber,
};
use sp_std::hash::Hash;

/// Era number type
pub type EraNumber = u32;
/// Period number type
pub type PeriodNumber = u32;
/// Dapp Id type
pub type DAppId = u16;
/// Tier Id type
pub type TierId = u8;
/// Tier Rank type
pub type Rank = u8;

/// Maximum encodable rank (4 bits)
pub const MAX_ENCODED_RANK: u32 = 0x0f;
/// Maximum encodable tier (4 bits)
pub const MAX_ENCODED_TIER: u8 = 0x0f;

/// Configuration for cycles, periods, subperiods & eras.
///
/// * `cycle` - Time unit similar to 'year' in the real world. Consists of one or more periods. At the beginning of each cycle, inflation is recalculated.
/// * `period` - Period consists of two distinct subperiods: `Voting` & `Build&Earn`. They are integral parts of dApp staking.
///              Length is expressed in standard eras or just _eras_.
/// * `era` - Era is the basic time unit in the dApp staking protocol. At the end of each era, reward pools for stakers & dApps are calculated.
///           Era length is expressed in blocks.
pub trait CycleConfiguration {
    /// How many different periods are there in a cycle (a 'year').
    ///
    /// This value has to be at least 1.
    fn periods_per_cycle() -> PeriodNumber;

    /// For how many standard era lengths does the voting subperiod last.
    ///
    /// This value has to be at least 1.
    fn eras_per_voting_subperiod() -> EraNumber;

    /// How many standard eras are there in the build&earn subperiod.
    ///
    /// This value has to be at least 1.
    fn eras_per_build_and_earn_subperiod() -> EraNumber;

    /// How many blocks are there per standard era.
    ///
    /// This value has to be at least 1.
    fn blocks_per_era() -> BlockNumber;

    /// For how many standard era lengths does the period last.
    fn period_in_era_lengths() -> EraNumber {
        Self::eras_per_voting_subperiod().saturating_add(Self::eras_per_build_and_earn_subperiod())
    }

    /// For how many standard era lengths does the cycle (a 'year') last.
    fn cycle_in_era_lengths() -> EraNumber {
        Self::period_in_era_lengths().saturating_mul(Self::periods_per_cycle())
    }

    /// How many blocks are there per cycle (a 'year').
    fn blocks_per_cycle() -> BlockNumber {
        Self::blocks_per_era().saturating_mul(Self::cycle_in_era_lengths())
    }

    /// For how many standard era lengths do all the build&earn subperiods in a cycle last.
    fn build_and_earn_eras_per_cycle() -> EraNumber {
        Self::eras_per_build_and_earn_subperiod().saturating_mul(Self::periods_per_cycle())
    }

    /// How many distinct eras are there in a single period.
    fn eras_per_period() -> EraNumber {
        Self::eras_per_build_and_earn_subperiod().saturating_add(1)
    }

    /// How many distinct eras are there in a cycle.
    fn eras_per_cycle() -> EraNumber {
        Self::eras_per_period().saturating_mul(Self::periods_per_cycle())
    }
}

/// Trait for observers (listeners) of various events related to dApp staking protocol.
pub trait Observer {
    /// Called in the block right before the next era starts.
    ///
    /// Returns the weight consumed by the call.
    ///
    /// # Arguments
    /// * `next_era` - Era number of the next era.
    fn block_before_new_era(_next_era: EraNumber) -> Weight {
        Weight::zero()
    }
}

impl Observer for () {}

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
    fn payout_reward(beneficiary: &AccountId, reward: Balance) -> Result<(), ()>;
}

/// Trait defining the interface for dApp staking `smart contract types` handler.
///
/// It can be used to create a representation of the specified smart contract instance type.
pub trait SmartContractHandle<AccountId> {
    /// Create a new smart contract representation for the specified EVM address.
    fn evm(address: H160) -> Self;
    /// Create a new smart contract representation for the specified Wasm address.
    fn wasm(address: AccountId) -> Self;
}

/// Multi-VM pointer to smart contract instance.
#[derive(
    PartialEq,
    Eq,
    Copy,
    Clone,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    MaxEncodedLen,
    Hash,
    scale_info::TypeInfo,
)]
pub enum SmartContract<AccountId> {
    /// EVM smart contract instance.
    Evm(H160),
    /// Wasm smart contract instance.
    Wasm(AccountId),
}

impl<AccountId> SmartContractHandle<AccountId> for SmartContract<AccountId> {
    fn evm(address: H160) -> Self {
        Self::Evm(address)
    }

    fn wasm(address: AccountId) -> Self {
        Self::Wasm(address)
    }
}

/// Used to check whether an account is allowed to participate in dApp staking or not.
pub trait AccountCheck<AccountId> {
    /// `true` if the account is allowed to stake, `false` otherwise.
    fn allowed_to_stake(account: &AccountId) -> bool;
}

impl<AccountId> AccountCheck<AccountId> for () {
    fn allowed_to_stake(_account: &AccountId) -> bool {
        true
    }
}

/// Trait for calculating the total number of tier slots for the given price.
pub trait TierSlots {
    /// Returns the total number of tier slots for the given price.
    ///
    /// # Arguments
    /// * `price` - price (e.g. moving average over some time period) of the native currency.
    /// * `args` - arguments, `a` & `b`, for the linear equation `number_of_slots = a * price + b`.
    ///
    /// Returns the total number of tier slots.
    fn number_of_slots(price: CurrencyAmount, args: (u64, u64)) -> u16;
}

/// Standard tier slots implementation, as proposed in the Tokenomics 2.0 document.
pub struct StandardTierSlots;
impl TierSlots for StandardTierSlots {
    fn number_of_slots(price: CurrencyAmount, args: (u64, u64)) -> u16 {
        let result: u64 = price.saturating_mul_int(args.0).saturating_add(args.1);
        result.unique_saturated_into()
    }
}

/// Standard tier slots arguments.
/// Initially decided for Astar, during the Tokenomics 2.0 work.
pub const STANDARD_TIER_SLOTS_ARGS: (u64, u64) = (1000, 50);
pub const FIXED_TIER_SLOTS_ARGS: (u64, u64) = (0, 16);

/// RankedTier is wrapper around u8 to hold both tier and rank. u8 has 2 bytes (8bits) and they're using in this order `0xrank_tier`.
/// First 4 bits are used to hold rank and second 4 bits are used to hold tier.
/// i.e: 0xa1 will hold rank: 10 and tier: 1 (0xa1 & 0xf == 1; 0xa1 >> 4 == 10;)
#[derive(Copy, Clone, Encode, Decode, Eq, PartialEq, MaxEncodedLen, scale_info::TypeInfo)]
pub struct RankedTier(u8);

impl RankedTier {
    /// Validate max_rank fits in 4 bits
    /// Returns Err(ArithmeticError::Overflow) if max value is not respected.
    #[inline(always)]
    fn validate_max_rank(max_rank: Rank) -> Result<(), ArithmeticError> {
        if (max_rank as u32) > MAX_ENCODED_RANK {
            Err(ArithmeticError::Overflow)
        } else {
            Ok(())
        }
    }

    /// Create new encoded RankedTier from tier and rank.
    /// `max_rank` defines how many ranks this tier supports.
    /// Returns Err(ArithmeticError::Overflow) if max values are not respected.
    pub fn new(tier: TierId, rank: Rank, max_rank: Rank) -> Result<Self, ArithmeticError> {
        Self::validate_max_rank(max_rank)?;

        if tier > MAX_ENCODED_TIER || rank > max_rank {
            return Err(ArithmeticError::Overflow);
        }

        Ok(Self((rank << 4) | (tier & 0x0f)))
    }

    /// Create new encoded RankedTier from tier and rank with saturation.
    pub fn new_saturated(tier: TierId, rank: Rank, max_rank: Rank) -> Self {
        Self((rank.min(max_rank) << 4) | (tier.min(0x0f) & 0x0f))
    }

    #[inline(always)]
    pub fn tier(&self) -> TierId {
        self.0 & MAX_ENCODED_TIER
    }

    #[inline(always)]
    pub fn rank(&self) -> Rank {
        (self.0 >> 4).min(MAX_ENCODED_RANK as u8)
    }

    #[inline(always)]
    pub fn deconstruct(&self) -> (TierId, Rank) {
        (self.tier(), self.rank())
    }
}

impl core::fmt::Debug for RankedTier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RankedTier")
            .field("tier", &self.tier())
            .field("rank", &self.rank())
            .finish()
    }
}

impl RankedTier {
    /// Find rank based on lower/upper bounds, stake amount and number of ranks.
    /// Delta between upper and lower bound is divided in `max_rank` and will increase rank
    /// by one for each threshold staked amount will reach.
    ///
    /// `max_rank` is the maximum rank for the tier (≤ 15).
    pub fn find_rank(
        lower_bound: Balance,
        upper_bound: Balance,
        stake_amount: Balance,
        max_rank: Rank,
    ) -> Rank {
        if upper_bound.is_zero() || max_rank == 0 || (max_rank as u32) > MAX_ENCODED_RANK {
            return 0;
        }

        let rank_threshold = upper_bound
            .saturating_sub(lower_bound)
            .saturating_div(max_rank.into());
        if rank_threshold.is_zero() {
            return 0;
        }

        <Balance as TryInto<u8>>::try_into(
            stake_amount
                .saturating_sub(lower_bound)
                .saturating_div(rank_threshold),
        )
        .unwrap_or_default()
        .min(max_rank)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_RANK: u8 = 10;

    #[test]
    fn tier_and_rank() {
        let t = RankedTier::new(0, 0, MAX_RANK).unwrap();
        assert_eq!(t.deconstruct(), (0, 0));

        let t = RankedTier::new(15, 10, MAX_RANK).unwrap();
        assert_eq!(t.deconstruct(), (15, 10));

        assert_eq!(
            RankedTier::new(16, 10, MAX_RANK),
            Err(ArithmeticError::Overflow)
        );
        assert_eq!(
            RankedTier::new(15, 11, MAX_RANK),
            Err(ArithmeticError::Overflow)
        );

        let t = RankedTier::new_saturated(0, 0, MAX_RANK);
        assert_eq!(t.deconstruct(), (0, 0));

        let t = RankedTier::new_saturated(1, 1, MAX_RANK);
        assert_eq!(t.deconstruct(), (1, 1));

        let t = RankedTier::new_saturated(3, 15, MAX_RANK);
        assert_eq!(t.deconstruct(), (3, 10));

        // max value for tier and rank
        let t = RankedTier::new_saturated(16, 16, MAX_RANK);
        assert_eq!(t.deconstruct(), (15, 10));
    }

    #[test]
    fn find_rank() {
        assert_eq!(RankedTier::find_rank(0, 0, 0, MAX_RANK), 0);
        assert_eq!(RankedTier::find_rank(0, 100, 9, MAX_RANK), 0);
        assert_eq!(RankedTier::find_rank(0, 100, 10, MAX_RANK), 1);
        assert_eq!(RankedTier::find_rank(0, 100, 49, MAX_RANK), 4);
        assert_eq!(RankedTier::find_rank(0, 100, 50, MAX_RANK), 5);
        assert_eq!(RankedTier::find_rank(0, 100, 51, MAX_RANK), 5);
        assert_eq!(RankedTier::find_rank(0, 100, 101, MAX_RANK), 10);

        assert_eq!(RankedTier::find_rank(100, 100, 100, MAX_RANK), 0);
        assert_eq!(RankedTier::find_rank(200, 100, 100, MAX_RANK), 0);
    }

    #[test]
    fn different_max_ranks_work() {
        assert_eq!(RankedTier::find_rank(0, 100, 100, 5), 5);
        assert_eq!(RankedTier::find_rank(0, 100, 100, 15), 15);
    }
}
