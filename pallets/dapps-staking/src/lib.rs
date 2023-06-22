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

//! # Dapps Staking Pallet
//!
//! - [`Config`]
//!
//! ## Overview
//!
//! Pallet that implements dapps staking protocol.
//!
//! Dapps staking protocol is a completely decentralized & innovative approach to reward developers for their contribution to the Astar/Shiden ecosystem.
//! Stakers can pick a dapp and nominate it for rewards by locking their tokens. Dapps will be rewarded, based on the proportion of locked tokens.
//! Stakers are also rewarded, based on the total amount they've locked (invariant of the dapp they staked on).
//!
//! Rewards are accumulated throughout an **era** and when **era** finishes, both stakers and developers can claim their rewards for that era.
//! This is a continous process. Rewards can be claimed even for eras which are older than the last one (no limit at the moment).
//!
//! Reward claiming isn't automated since the whole process is done **on-chain** and is fully decentralized.
//! Both stakers and developers are responsible for claiming their own rewards.
//!
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//! - `register` - used to register a new contract for dapps staking
//! - `unregister` - used to unregister contract from dapps staking, making it ineligible for receiveing future rewards
//! - `withdraw_from_unregistered` - used by stakers to withdraw their stake from an unregistered contract (no unbonding period)
//! - `bond_and_stake` - basic call for nominating a dapp and locking stakers tokens into dapps staking
//! - `unbond_and_unstake` - removes nomination from the contract, starting the unbonding process for the unstaked funds
//! - `withdraw_unbonded` - withdraws all funds that have completed the unbonding period
//! - `nomination_transfer` - transfer nomination from one contract to another contract (avoids unbonding period)
//! - `claim_staker` - claims staker reward for a single era
//! - `claim_dapp` - claims dapp rewards for the specified era
//! - `force_new_era` - forces new era on the start of the next block
//! - `maintenance_mode` - enables or disables pallet maintenance mode
//! - `set_reward_destination` - sets reward destination for the staker rewards
//! - `set_contract_stake_info` - root-only call to set storage value (used for fixing corrupted data)
//! - `burn_stale_reward` - root-only call to burn unclaimed, stale rewards from unregistered contracts
//!
//! User is encouraged to refer to specific function implementations for more comprehensive documentation.
//!
//! ### Other
//!
//! - `on_initialize` - part of `Hooks` trait, it's important to call this per block since it handles reward snapshots and era advancement.
//! - `account_id` - returns pallet's account Id
//! - `ensure_pallet_enabled` - checks whether pallet is in maintenance mode or not and returns appropriate `Result`
//! - `rewards` - used to deposit staker and dapps rewards into dApps staking reward pool
//! - `tvl` - total value locked in dApps staking (might differ from total staked value)
//!
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Currency;
use frame_system::{self as system};
use parity_scale_codec::{Decode, Encode, HasCompact, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{AtLeast32BitUnsigned, Zero},
    RuntimeDebug,
};
use sp_std::{ops::Add, prelude::*};

pub mod pallet;
pub mod weights;

#[cfg(any(feature = "runtime-benchmarks"))]
pub mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod testing_utils;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_lib;

pub use pallet::pallet::*;
pub use weights::WeightInfo;

pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

// This represents the max assumed vector length that any storage item should have.
// In particular, this relates to `UnbondingInfo` and `StakerInfo`.
// In structs which are bound in size, `MaxEncodedLen` can just be derived but that's not the case for standard `vec`.
// To fix this 100% correctly, we'd need to do one of the following:
//
// - Use `BoundedVec` instead of `Vec` and do storage migration
// - Introduce a new type `S: Get<u32>` into the aforementioned structs and use it to inject max allowed size,
//   thus allowing us to correctly calculate max encoded len
//
// The issue with first approach is that it requires storage migration which we want to avoid
// unless it's really necessary. The issue with second approach is that it makes code much more
// difficult to work with since all of it will be ridden with injections of the `S` type.
//
// Since dApps staking has been stable for long time and there are plans to redesign & refactor it,
// doing neither of the above makes sense, timewise. So we use an assumption that vec length
// won't go over the following constant.
const MAX_ASSUMED_VEC_LEN: u32 = 10;

/// DApp State descriptor
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
enum DAppState {
    /// Contract is registered and active.
    Registered,
    /// Contract has been unregistered and is inactive.
    /// Claim for past eras and unbonding is still possible but no additional staking can be done.
    Unregistered(EraIndex),
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct DAppInfo<AccountId> {
    /// Developer (owner) account
    developer: AccountId,
    /// Current DApp State
    state: DAppState,
}

impl<AccountId> DAppInfo<AccountId> {
    /// Create new `DAppInfo` struct instance with the given developer and state `Registered`
    fn new(developer: AccountId) -> Self {
        Self {
            developer,
            state: DAppState::Registered,
        }
    }

    /// `true` if dApp has been unregistered, `false` otherwise
    fn is_unregistered(&self) -> bool {
        matches!(self.state, DAppState::Unregistered(_))
    }
}

/// Mode of era-forcing.
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum Forcing {
    /// Not forcing anything - just let whatever happen.
    NotForcing,
    /// Force a new era, then reset to `NotForcing` as soon as it is done.
    /// Note that this will force to trigger an election until a new era is triggered, if the
    /// election failed, the next session end will trigger a new election again, until success.
    ForceNew,
}

impl Default for Forcing {
    fn default() -> Self {
        Forcing::NotForcing
    }
}

/// A record of rewards allocated for stakers and dapps
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct RewardInfo<Balance: HasCompact + MaxEncodedLen> {
    /// Total amount of rewards for stakers in an era
    #[codec(compact)]
    pub stakers: Balance,
    /// Total amount of rewards for dapps in an era
    #[codec(compact)]
    pub dapps: Balance,
}

/// A record for total rewards and total amount staked for an era
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct EraInfo<Balance: HasCompact + MaxEncodedLen> {
    /// Total amount of earned rewards for an era
    pub rewards: RewardInfo<Balance>,
    /// Total staked amount in an era
    #[codec(compact)]
    pub staked: Balance,
    /// Total locked amount in an era
    #[codec(compact)]
    pub locked: Balance,
}

/// Used to split total EraPayout among contracts.
/// Each tuple (contract, era) has this structure.
/// This will be used to reward contracts developer and his stakers.
#[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ContractStakeInfo<Balance: HasCompact + MaxEncodedLen> {
    /// Total staked amount.
    #[codec(compact)]
    pub total: Balance,
    /// Total number of active stakers
    #[codec(compact)]
    number_of_stakers: u32,
    /// Indicates whether rewards were claimed for this era or not
    contract_reward_claimed: bool,
}

/// Storage value representing the current Dapps staking pallet storage version.
/// Used by `on_runtime_upgrade` to determine whether a storage migration is needed or not.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Version {
    V1_0_0,
    V2_0_0,
    V3_0_0,
    V4_0_0,
}

impl Default for Version {
    fn default() -> Self {
        Version::V4_0_0
    }
}

/// Used to represent how much was staked in a particular era.
/// E.g. `{staked: 1000, era: 5}` means that in era `5`, staked amount was 1000.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct EraStake<Balance: AtLeast32BitUnsigned + Copy + MaxEncodedLen> {
    /// Staked amount in era
    #[codec(compact)]
    staked: Balance,
    /// Staked era
    #[codec(compact)]
    era: EraIndex,
}

impl<Balance: AtLeast32BitUnsigned + Copy + MaxEncodedLen> EraStake<Balance> {
    /// Create a new instance of `EraStake` with given values
    fn new(staked: Balance, era: EraIndex) -> Self {
        Self { staked, era }
    }
}

/// Used to provide a compact and bounded storage for information about stakes in unclaimed eras.
///
/// In order to avoid creating a separate storage entry for each `(staker, contract, era)` triplet,
/// this struct is used to provide a more memory efficient solution.
///
/// Basic idea is to store `EraStake` structs into a vector from which a complete
/// picture of **unclaimed eras** and stakes can be constructed.
///
/// # Example
/// For simplicity, the following example will represent `EraStake` using `<era, stake>` notation.
/// Let us assume we have the following vector in `StakerInfo` struct.
///
/// `[<5, 1000>, <6, 1500>, <8, 2100>, <9, 0>, <11, 500>]`
///
/// This tells us which eras are unclaimed and how much it was staked in each era.
/// The interpretation is the following:
/// 1. In era **5**, staked amount was **1000** (interpreted from `<5, 1000>`)
/// 2. In era **6**, staker staked additional **500**, increasing total staked amount to **1500**
/// 3. No entry for era **7** exists which means there were no changes from the former entry.
///    This means that in era **7**, staked amount was also **1500**
/// 4. In era **8**, staker staked an additional **600**, increasing total stake to **2100**
/// 5. In era **9**, staker unstaked everything from the contract (interpreted from `<9, 0>`)
/// 6. No changes were made in era **10** so we can interpret this same as the previous entry which means **0** staked amount.
/// 7. In era **11**, staker staked **500** on the contract, making his stake active again after 2 eras of inactivity.
///
/// **NOTE:** It is important to understand that staker **DID NOT** claim any rewards during this period.
///
#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct StakerInfo<Balance: AtLeast32BitUnsigned + Copy + MaxEncodedLen> {
    // Size of this list would be limited by a configurable constant
    stakes: Vec<EraStake<Balance>>,
}

impl<Balance: AtLeast32BitUnsigned + Copy + MaxEncodedLen> MaxEncodedLen for StakerInfo<Balance> {
    // This is just an assumption, will be calculated properly in the future. See the comment for `MAX_ASSUMED_VEC_LEN`.
    fn max_encoded_len() -> usize {
        parity_scale_codec::Compact(MAX_ASSUMED_VEC_LEN)
            .encoded_size()
            .saturating_add(
                (MAX_ASSUMED_VEC_LEN as usize)
                    .saturating_mul(EraStake::<Balance>::max_encoded_len()),
            )
    }
}

impl<Balance: AtLeast32BitUnsigned + Copy + MaxEncodedLen> StakerInfo<Balance> {
    /// `true` if no active stakes and unclaimed eras exist, `false` otherwise
    fn is_empty(&self) -> bool {
        self.stakes.is_empty()
    }

    /// number of `EraStake` chunks
    fn len(&self) -> u32 {
        self.stakes.len() as u32
    }

    /// Stakes some value in the specified era.
    ///
    /// User should ensure that given era is either equal or greater than the
    /// latest available era in the staking info.
    ///
    /// # Example
    ///
    /// The following example demonstrates how internal vector changes when `stake` is called:
    ///
    /// `stakes: [<5, 1000>, <7, 1300>]`
    /// * `stake(7, 100)` will result in `[<5, 1000>, <7, 1400>]`
    /// * `stake(9, 200)` will result in `[<5, 1000>, <7, 1400>, <9, 1600>]`
    ///
    fn stake(&mut self, current_era: EraIndex, value: Balance) -> Result<(), &str> {
        if let Some(era_stake) = self.stakes.last_mut() {
            if era_stake.era > current_era {
                return Err("Unexpected era");
            }

            let new_stake_value = era_stake.staked.saturating_add(value);

            if current_era == era_stake.era {
                *era_stake = EraStake::new(new_stake_value, current_era)
            } else {
                self.stakes
                    .push(EraStake::new(new_stake_value, current_era))
            }
        } else {
            self.stakes.push(EraStake::new(value, current_era));
        }

        Ok(())
    }

    /// Unstakes some value in the specified era.
    ///
    /// User should ensure that given era is either equal or greater than the
    /// latest available era in the staking info.
    ///
    /// # Example 1
    ///
    /// `stakes: [<5, 1000>, <7, 1300>]`
    /// * `unstake(7, 100)` will result in `[<5, 1000>, <7, 1200>]`
    /// * `unstake(9, 400)` will result in `[<5, 1000>, <7, 1200>, <9, 800>]`
    /// * `unstake(10, 800)` will result in `[<5, 1000>, <7, 1200>, <9, 800>, <10, 0>]`
    ///
    /// # Example 2
    ///
    /// `stakes: [<5, 1000>]`
    /// * `unstake(5, 1000)` will result in `[]`
    ///
    /// Note that if no unclaimed eras remain, vector will be cleared.
    ///
    fn unstake(&mut self, current_era: EraIndex, value: Balance) -> Result<(), &str> {
        if let Some(era_stake) = self.stakes.last_mut() {
            if era_stake.era > current_era {
                return Err("Unexpected era");
            }

            let new_stake_value = era_stake.staked.saturating_sub(value);
            if current_era == era_stake.era {
                *era_stake = EraStake::new(new_stake_value, current_era)
            } else {
                self.stakes
                    .push(EraStake::new(new_stake_value, current_era))
            }

            // Removes unstaked values if they're no longer valid for comprehension
            if !self.stakes.is_empty() && self.stakes[0].staked.is_zero() {
                self.stakes.remove(0);
            }
        }

        Ok(())
    }

    /// `Claims` the oldest era available for claiming.
    /// In case valid era exists, returns `(claim era, staked amount)` tuple.
    /// If no valid era exists, returns `(0, 0)` tuple.
    ///
    /// # Example
    ///
    /// The following example will demonstrate how the internal vec changes when `claim` is called consecutively.
    ///
    /// `stakes: [<5, 1000>, <7, 1300>, <8, 0>, <15, 3000>]`
    ///
    /// 1. `claim()` will return `(5, 1000)`
    ///     Internal vector is modified to `[<6, 1000>, <7, 1300>, <8, 0>, <15, 3000>]`
    ///
    /// 2. `claim()` will return `(6, 1000)`.
    ///    Internal vector is modified to `[<7, 1300>, <8, 0>, <15, 3000>]`
    ///
    /// 3. `claim()` will return `(7, 1300)`.
    ///    Internal vector is modified to `[<15, 3000>]`
    ///    Note that `0` staked period is discarded since nothing can be claimed there.
    ///
    /// 4. `claim()` will return `(15, 3000)`.
    ///    Internal vector is modified to `[16, 3000]`
    ///
    /// Repeated calls would continue to modify vector following the same rule as in *4.*
    ///
    fn claim(&mut self) -> (EraIndex, Balance) {
        if let Some(era_stake) = self.stakes.first() {
            let era_stake = *era_stake;

            if self.stakes.len() == 1 || self.stakes[1].era > era_stake.era + 1 {
                self.stakes[0] = EraStake {
                    staked: era_stake.staked,
                    era: era_stake.era.saturating_add(1),
                }
            } else {
                // in case: self.stakes[1].era == era_stake.era + 1
                self.stakes.remove(0);
            }

            // Removes unstaked values if they're no longer valid for comprehension
            if !self.stakes.is_empty() && self.stakes[0].staked.is_zero() {
                self.stakes.remove(0);
            }

            (era_stake.era, era_stake.staked)
        } else {
            (0, Zero::zero())
        }
    }

    /// Latest staked value.
    /// E.g. if staker is fully unstaked, this will return `Zero`.
    /// Otherwise returns a non-zero balance.
    pub fn latest_staked_value(&self) -> Balance {
        self.stakes.last().map_or(Zero::zero(), |x| x.staked)
    }
}

/// Represents an balance amount undergoing the unbonding process.
/// Since unbonding takes time, it's important to keep track of when and how much was unbonded.
#[derive(
    Clone, Copy, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub struct UnlockingChunk<Balance: MaxEncodedLen> {
    /// Amount being unlocked
    #[codec(compact)]
    amount: Balance,
    /// Era in which the amount will become unlocked and can be withdrawn.
    #[codec(compact)]
    unlock_era: EraIndex,
}

impl<Balance> UnlockingChunk<Balance>
where
    Balance: Add<Output = Balance> + Copy + MaxEncodedLen,
{
    // Adds the specified amount to this chunk
    fn add_amount(&mut self, amount: Balance) {
        self.amount = self.amount + amount
    }
}

/// Contains unlocking chunks.
/// This is a convenience struct that provides various utility methods to help with unbonding handling.
#[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct UnbondingInfo<Balance: AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen> {
    // Vector of unlocking chunks. Sorted in ascending order in respect to unlock_era.
    unlocking_chunks: Vec<UnlockingChunk<Balance>>,
}

impl<Balance: AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen> MaxEncodedLen
    for UnbondingInfo<Balance>
{
    // This is just an assumption, will be calculated properly in the future. See the comment for `MAX_ASSUMED_VEC_LEN`.
    fn max_encoded_len() -> usize {
        parity_scale_codec::Compact(MAX_ASSUMED_VEC_LEN)
            .encoded_size()
            .saturating_add(
                (MAX_ASSUMED_VEC_LEN as usize)
                    .saturating_mul(UnlockingChunk::<Balance>::max_encoded_len()),
            )
    }
}

impl<Balance> UnbondingInfo<Balance>
where
    Balance: AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen,
{
    /// Returns total number of unlocking chunks.
    fn len(&self) -> u32 {
        self.unlocking_chunks.len() as u32
    }

    /// True if no unlocking chunks exist, false otherwise.
    fn is_empty(&self) -> bool {
        self.unlocking_chunks.is_empty()
    }

    /// Returns sum of all unlocking chunks.
    fn sum(&self) -> Balance {
        self.unlocking_chunks
            .iter()
            .map(|chunk| chunk.amount)
            .reduce(|c1, c2| c1 + c2)
            .unwrap_or_default()
    }

    /// Adds a new unlocking chunk to the vector, preserving the unlock_era based ordering.
    fn add(&mut self, chunk: UnlockingChunk<Balance>) {
        // It is possible that the unbonding period changes so we need to account for that
        match self
            .unlocking_chunks
            .binary_search_by(|x| x.unlock_era.cmp(&chunk.unlock_era))
        {
            // Merge with existing chunk if unlock_eras match
            Ok(pos) => self.unlocking_chunks[pos].add_amount(chunk.amount),
            // Otherwise insert where it should go. Note that this will in almost all cases return the last index.
            Err(pos) => self.unlocking_chunks.insert(pos, chunk),
        }
    }

    /// Partitions the unlocking chunks into two groups:
    ///
    /// First group includes all chunks which have unlock era lesser or equal to the specified era.
    /// Second group includes all the rest.
    ///
    /// Order of chunks is preserved in the two new structs.
    fn partition(self, era: EraIndex) -> (Self, Self) {
        let (matching_chunks, other_chunks): (
            Vec<UnlockingChunk<Balance>>,
            Vec<UnlockingChunk<Balance>>,
        ) = self
            .unlocking_chunks
            .iter()
            .partition(|chunk| chunk.unlock_era <= era);

        (
            Self {
                unlocking_chunks: matching_chunks,
            },
            Self {
                unlocking_chunks: other_chunks,
            },
        )
    }

    #[cfg(test)]
    /// Return clone of the internal vector. Should only be used for testing.
    fn vec(&self) -> Vec<UnlockingChunk<Balance>> {
        self.unlocking_chunks.clone()
    }
}

/// Instruction on how to handle reward payout for stakers.
/// In order to make staking more competitive, majority of stakers will want to
/// automatically restake anything they earn.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum RewardDestination {
    /// Rewards are transferred to stakers free balance without any further action.
    FreeBalance,
    /// Rewards are transferred to stakers balance and are immediately re-staked
    /// on the contract from which the reward was received.
    StakeBalance,
}

impl Default for RewardDestination {
    fn default() -> Self {
        RewardDestination::StakeBalance
    }
}

/// Contains information about account's locked & unbonding balances.
#[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct AccountLedger<Balance: AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen> {
    /// Total balance locked.
    #[codec(compact)]
    pub locked: Balance,
    /// Information about unbonding chunks.
    unbonding_info: UnbondingInfo<Balance>,
    /// Instruction on how to handle reward payout
    reward_destination: RewardDestination,
}

impl<Balance: AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen> AccountLedger<Balance> {
    /// `true` if ledger is empty (no locked funds, no unbonding chunks), `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.locked.is_zero() && self.unbonding_info.is_empty()
    }

    /// Configured reward destination
    pub fn reward_destination(&self) -> RewardDestination {
        self.reward_destination
    }
}
