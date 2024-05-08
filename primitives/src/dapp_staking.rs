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
use sp_core::H160;
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
// Tier Rank type
pub type Rank = u8;

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
    fn number_of_slots(price: CurrencyAmount) -> u16;
}

/// Standard tier slots implementation, as proposed in the Tokenomics 2.0 document.
pub struct StandardTierSlots;
impl TierSlots for StandardTierSlots {
    fn number_of_slots(price: CurrencyAmount) -> u16 {
        let result: u64 = price.saturating_mul_int(1000_u64).saturating_add(50);
        result.unique_saturated_into()
    }
}

/// TierAndRank is wrapper around u8 to hold both tier and rank. u8 has 2 bytes (8bits) and they're using in this order `0xrank_tier`.
/// First 4 bits are used to hold rank and second 4 bits are used to hold tier.
/// i.e: 0xa1 will hold rank: 10 and tier: 1 (0xa1 & 0xf == 1; 0xa1 >> 4 == 10;)
#[derive(Copy, Clone, Encode, Decode, Eq, PartialEq, MaxEncodedLen, scale_info::TypeInfo)]
pub struct TierAndRank(u8);

impl TierAndRank {
    pub const MAX_RANK: u8 = 10;

    pub fn new(tier: TierId, rank: Rank) -> Result<Self, ArithmeticError> {
        if rank > Self::MAX_RANK || tier > 0xf {
            return Err(ArithmeticError::Overflow);
        }
        Ok(Self(rank << 4 | tier & 0x0f))
    }
    pub fn new_saturated(tier: TierId, rank: Rank) -> Self {
        Self(rank.min(Self::MAX_RANK) << 4 | tier.min(0xf) & 0x0f)
    }

    #[inline(always)]
    pub fn tier_id(&self) -> TierId {
        self.0 & 0x0f
    }

    #[inline(always)]
    pub fn rank(&self) -> Rank {
        (self.0 >> 4).min(Self::MAX_RANK)
    }

    #[inline(always)]
    pub fn destruct(&self) -> (TierId, Rank) {
        (self.tier_id(), self.rank())
    }
}

impl core::fmt::Debug for TierAndRank {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TierAndRank")
            .field("tier", &self.tier_id())
            .field("rank", &self.rank())
            .finish()
    }
}

impl TierAndRank {
    pub fn find_rank(lower_bound: Balance, upper_bound: Balance, stake_amount: Balance) -> Rank {
        if upper_bound.is_zero() {
            return 0;
        }
        let rank_threshold = upper_bound
            .saturating_sub(lower_bound)
            .saturating_div(TierAndRank::MAX_RANK.into());
        if rank_threshold.is_zero() {
            0
        } else {
            <Balance as TryInto<u8>>::try_into(
                stake_amount
                    .saturating_sub(lower_bound.saturating_add(1))
                    .saturating_div(rank_threshold),
            )
            .unwrap_or_default()
            .min(TierAndRank::MAX_RANK)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_and_rank() {
        let t = TierAndRank::new(0, 0).unwrap();
        assert_eq!(t.destruct(), (0, 0));

        let t = TierAndRank::new(15, 10).unwrap();
        assert_eq!(t.destruct(), (15, 10));

        assert_eq!(TierAndRank::new(16, 10), Err(ArithmeticError::Overflow));
        assert_eq!(TierAndRank::new(15, 11), Err(ArithmeticError::Overflow));

        let t = TierAndRank::new_saturated(0, 0);
        assert_eq!(t.destruct(), (0, 0));

        let t = TierAndRank::new_saturated(1, 1);
        assert_eq!(t.destruct(), (1, 1));

        let t = TierAndRank::new_saturated(3, 15);
        assert_eq!(t.destruct(), (3, 10));

        // max value for tier and rank
        let t = TierAndRank::new_saturated(16, 16);
        assert_eq!(t.destruct(), (15, 10));
    }

    #[test]
    fn find_rank() {
        assert_eq!(TierAndRank::find_rank(0, 0, 0), 0);
        assert_eq!(TierAndRank::find_rank(0, 100, 10), 0);
        assert_eq!(TierAndRank::find_rank(0, 100, 49), 4);
        assert_eq!(TierAndRank::find_rank(0, 100, 50), 4);
        assert_eq!(TierAndRank::find_rank(0, 100, 51), 5);
        assert_eq!(TierAndRank::find_rank(0, 100, 100), 9);
        assert_eq!(TierAndRank::find_rank(0, 100, 101), 10);

        assert_eq!(TierAndRank::find_rank(100, 100, 100), 0);
        assert_eq!(TierAndRank::find_rank(200, 100, 100), 0);
    }
}
