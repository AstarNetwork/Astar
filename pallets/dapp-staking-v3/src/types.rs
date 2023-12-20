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

//! # dApp Staking Module Types
//!
//! Contains various types, structs & enums used by the dApp staking implementation.
//! The main purpose of this is to abstract complexity away from the extrinsic call implementation,
//! and even more importantly to make the code more testable.
//!
//! # Overview
//!
//! The following is a high level overview of the implemented structs, enums & types.
//! For details, please refer to the documentation and code of each individual type.
//!
//! ## General Protocol Information
//!
//! * `EraNumber` - numeric Id of an era.
//! * `PeriodNumber` - numeric Id of a period.
//! * `Subperiod` - an enum describing which subperiod is active in the current period.
//! * `PeriodInfo` - contains information about the ongoing period, like period number, current subperiod and when will the current subperiod end.
//! * `PeriodEndInfo` - contains information about a finished past period, like the final era of the period, total amount staked & bonus reward pool.
//! * `ProtocolState` - contains the most general protocol state info: current era number, block when the era ends, ongoing period info, and whether protocol is in maintenance mode.
//!
//! ## DApp Information
//!
//! * `DAppId` - a compact unique numeric Id of a dApp.
//! * `DAppInfo` - contains general information about a dApp, like owner and reward beneficiary, Id and state.
//! * `ContractStakeAmount` - contains information about how much is staked on a particular contract.
//!
//! ## Staker Information
//!
//! * `UnlockingChunk` - describes some amount undergoing the unlocking process.
//! * `StakeAmount` - contains information about the staked amount in a particular era, and period.
//! * `AccountLedger` - keeps track of total locked & staked balance, unlocking chunks and number of stake entries.
//! * `SingularStakingInfo` - contains information about a particular staker's stake on a specific smart contract. Used to track loyalty.
//!
//! ## Era Information
//!
//! * `EraInfo` - contains information about the ongoing era, like how much is locked & staked.
//! * `EraReward` - contains information about a finished era, like reward pools and total staked amount.
//! * `EraRewardSpan` - a composite of multiple `EraReward` objects, used to describe a range of finished eras.
//!
//! ## Tier Information
//!
//! * `TierThreshold` - an enum describing tier entry thresholds.
//! * `TierParameters` - contains static information about tiers, like init thresholds, reward & slot distribution.
//! * `TiersConfiguration` - contains dynamic information about tiers, derived from `TierParameters` and onchain data.
//! * `DAppTier` - a compact struct describing a dApp's tier.
//! * `DAppTierRewards` - composite of `DAppTier` objects, describing the entire reward distribution for a particular era.
//!

use frame_support::{pallet_prelude::*, BoundedVec};
use parity_scale_codec::{Decode, Encode};
use sp_arithmetic::fixed_point::FixedU64;
use sp_runtime::{
    traits::{CheckedAdd, UniqueSaturatedInto, Zero},
    FixedPointNumber, Permill, Saturating,
};
pub use sp_std::{fmt::Debug, vec::Vec};

use astar_primitives::{Balance, BlockNumber};

use crate::pallet::Config;

// Convenience type for `AccountLedger` usage.
pub type AccountLedgerFor<T> = AccountLedger<<T as Config>::MaxUnlockingChunks>;

// Convenience type for `DAppTierRewards` usage.
pub type DAppTierRewardsFor<T> =
    DAppTierRewards<<T as Config>::MaxNumberOfContracts, <T as Config>::NumberOfTiers>;

// Convenience type for `EraRewardSpan` usage.
pub type EraRewardSpanFor<T> = EraRewardSpan<<T as Config>::EraRewardSpanLength>;

// Convenience type for `DAppInfo` usage.
pub type DAppInfoFor<T> = DAppInfo<<T as frame_system::Config>::AccountId>;

/// Era number type
pub type EraNumber = u32;
/// Period number type
pub type PeriodNumber = u32;
/// Dapp Id type
pub type DAppId = u16;
/// Tier Id type
pub type TierId = u8;

/// Simple enum representing errors possible when using sparse bounded vector.
#[derive(Debug, PartialEq, Eq)]
pub enum AccountLedgerError {
    /// Old or future era values cannot be added.
    InvalidEra,
    /// Bounded storage capacity exceeded.
    NoCapacity,
    /// Invalid period specified.
    InvalidPeriod,
    /// Stake amount is to large in respect to what's available.
    UnavailableStakeFunds,
    /// Unstake amount is to large in respect to what's staked.
    UnstakeAmountLargerThanStake,
    /// Nothing to claim.
    NothingToClaim,
    /// Rewards have already been claimed
    AlreadyClaimed,
    /// Attempt to crate the iterator failed due to incorrect data.
    InvalidIterator,
}

/// Distinct subperiods in dApp staking protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum Subperiod {
    /// Subperiod during which the focus is on voting.
    Voting,
    /// Subperiod during which dApps and stakers earn rewards.
    BuildAndEarn,
}

impl Subperiod {
    /// Next subperiod, after `self`.
    pub fn next(&self) -> Self {
        match self {
            Subperiod::Voting => Subperiod::BuildAndEarn,
            Subperiod::BuildAndEarn => Subperiod::Voting,
        }
    }
}

/// Info about the ongoing period.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct PeriodInfo {
    /// Period number.
    #[codec(compact)]
    pub number: PeriodNumber,
    /// Subperiod type.
    pub subperiod: Subperiod,
    /// Era in which the new subperiod starts.
    #[codec(compact)]
    pub next_subperiod_start_era: EraNumber,
}

impl PeriodInfo {
    /// `true` if the provided era belongs to the next period, `false` otherwise.
    /// It's only possible to provide this information correctly for the ongoing `BuildAndEarn` subperiod.
    pub fn is_next_period(&self, era: EraNumber) -> bool {
        self.subperiod == Subperiod::BuildAndEarn && self.next_subperiod_start_era <= era
    }
}

/// Information describing relevant information for a finished period.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct PeriodEndInfo {
    /// Bonus reward pool allocated for 'loyal' stakers
    #[codec(compact)]
    pub bonus_reward_pool: Balance,
    /// Total amount staked (remaining) from the voting period.
    #[codec(compact)]
    pub total_vp_stake: Balance,
    /// Final era, inclusive, in which the period ended.
    #[codec(compact)]
    pub final_era: EraNumber,
}

/// Force types to speed up the next era, and even period.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum ForcingType {
    /// Force the next era to start.
    Era,
    /// Force the current subperiod to end, and new one to start. It will also force a new era to start.
    Subperiod,
}

/// General information & state of the dApp staking protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct ProtocolState {
    /// Ongoing era number.
    #[codec(compact)]
    pub era: EraNumber,
    /// Block number at which the next era should start.
    #[codec(compact)]
    pub next_era_start: BlockNumber,
    /// Information about the ongoing period.
    pub period_info: PeriodInfo,
    /// `true` if pallet is in maintenance mode (disabled), `false` otherwise.
    pub maintenance: bool,
}

impl Default for ProtocolState {
    fn default() -> Self {
        Self {
            era: 0,
            next_era_start: 1,
            period_info: PeriodInfo {
                number: 0,
                subperiod: Subperiod::Voting,
                next_subperiod_start_era: 2,
            },
            maintenance: false,
        }
    }
}

impl ProtocolState {
    /// Current subperiod.
    pub fn subperiod(&self) -> Subperiod {
        self.period_info.subperiod
    }

    /// Current period number.
    pub fn period_number(&self) -> PeriodNumber {
        self.period_info.number
    }

    /// Ending era of current period
    pub fn next_subperiod_start_era(&self) -> EraNumber {
        self.period_info.next_subperiod_start_era
    }

    /// Checks whether a new era should be triggered, based on the provided _current_ block number argument
    /// or possibly other protocol state parameters.
    pub fn is_new_era(&self, now: BlockNumber) -> bool {
        self.next_era_start <= now
    }

    /// Triggers the next subperiod, updating appropriate parameters.
    pub fn advance_to_next_subperiod(
        &mut self,
        next_subperiod_start_era: EraNumber,
        next_era_start: BlockNumber,
    ) {
        let period_number = if self.subperiod() == Subperiod::BuildAndEarn {
            self.period_number().saturating_add(1)
        } else {
            self.period_number()
        };

        self.period_info = PeriodInfo {
            number: period_number,
            subperiod: self.subperiod().next(),
            next_subperiod_start_era,
        };
        self.next_era_start = next_era_start;
    }
}

/// State in which some dApp is in.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum DAppState {
    /// dApp is registered and active.
    Registered,
    /// dApp has been unregistered in the contained era
    Unregistered(#[codec(compact)] EraNumber),
}

/// General information about dApp.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct DAppInfo<AccountId> {
    /// Owner of the dApp, default reward beneficiary.
    pub owner: AccountId,
    /// dApp's unique identifier in dApp staking.
    #[codec(compact)]
    pub id: DAppId,
    /// Current state of the dApp.
    pub state: DAppState,
    // If `None`, rewards goes to the developer account, otherwise to the account Id in `Some`.
    pub reward_destination: Option<AccountId>,
}

impl<AccountId> DAppInfo<AccountId> {
    /// Reward destination account for this dApp.
    pub fn reward_beneficiary(&self) -> &AccountId {
        match &self.reward_destination {
            Some(account_id) => account_id,
            None => &self.owner,
        }
    }

    /// `true` if dApp is registered, `false` otherwise.
    pub fn is_registered(&self) -> bool {
        self.state == DAppState::Registered
    }
}

/// How much was unlocked in some block.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct UnlockingChunk {
    /// Amount undergoing the unlocking period.
    #[codec(compact)]
    pub amount: Balance,
    /// Block in which the unlocking period is finished for this chunk.
    #[codec(compact)]
    pub unlock_block: BlockNumber,
}

impl Default for UnlockingChunk {
    fn default() -> Self {
        Self {
            amount: Balance::zero(),
            unlock_block: BlockNumber::zero(),
        }
    }
}

/// General info about an account's lock & stakes.
///
/// ## Overview
///
/// The most complex part about this type are the `staked` and `staked_future` fields.
/// To understand why the two fields exist and how they are used, it's important to consider some facts:
/// * when an account _stakes_, the staked amount is only eligible for rewards from the next era
/// * all stakes are reset when a period ends - but this is done in a lazy fashion, account ledgers aren't directly updated
/// * `stake` and `unstake` operations are allowed only if the account has claimed all pending rewards
///
/// In order to keep track of current era stake, and _next era_ stake, two fields are needed.
/// Since it's not allowed to stake/unstake if there are pending rewards, it's guaranteed that the `staked` and `staked_future` eras are **always consecutive**.
/// In order to understand if _stake_ is still valid, it's enough to check the `period` field of either `staked` or `staked_future`.
///
/// ## Example
///
/// ### Scenario 1
///
/// * current era is **20**, and current period is **1**
/// * `staked` is equal to: `{ voting: 100, build_and_earn: 50, era: 5, period: 1 }`
/// * `staked_future` is equal to: `{ voting: 100, build_and_earn: 100, era: 6, period: 1 }`
///
/// The correct way to interpret this is:
/// * account had staked **150** in total in era 5
/// * account had increased their stake to **200** in total in era 6
/// * since then, era 6, account hadn't staked or unstaked anything or hasn't claimed any rewards
/// * since we're in era **20** and period is still **1**, the account's stake for eras **7** to **20** is still **200**
///
/// ### Scenario 2
///
/// * current era is **20**, and current period is **1**
/// * `staked` is equal to: `{ voting: 0, build_and_earn: 0, era: 0, period: 0 }`
/// * `staked_future` is equal to: `{ voting: 0, build_and_earn: 350, era: 13, period: 1 }`
///
/// The correct way to interpret this is:
/// * `staked` entry is _empty_
/// * account had called `stake` during era 12, and staked **350** for the next era
/// * account hadn't staked, unstaked or claimed rewards since then
/// * since we're in era **20** and period is still **1**, the account's stake for eras **13** to **20** is still **350**
///
/// ### Scenario 3
///
/// * current era is **30**, and current period is **2**
/// * period **1** ended after era **24**, and period **2** started in era **25**
/// * `staked` is equal to: `{ voting: 100, build_and_earn: 300, era: 20, period: 1 }`
/// * `staked_future` is equal to `None`
///
/// The correct way to interpret this is:
/// * in era **20**, account had claimed rewards for the past eras, so only the `staked` entry remained
/// * since then, account hadn't staked, unstaked or claimed rewards
/// * period 1 ended in era **24**, which means that after that era, the `staked` entry is no longer valid
/// * account had staked **400** in total from era **20** up to era **24** (inclusive)
/// * account's stake in era **25** is **zero**
///
#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    RuntimeDebugNoBound,
    PartialEqNoBound,
    EqNoBound,
    CloneNoBound,
    TypeInfo,
)]
#[scale_info(skip_type_params(UnlockingLen))]
pub struct AccountLedger<UnlockingLen: Get<u32>> {
    /// How much active locked amount an account has. This can be used for staking.
    #[codec(compact)]
    pub locked: Balance,
    /// Vector of all the unlocking chunks. This is also considered _locked_ but cannot be used for staking.
    pub unlocking: BoundedVec<UnlockingChunk, UnlockingLen>,
    /// Primary field used to store how much was staked in a particular era.
    pub staked: StakeAmount,
    /// Secondary field used to store 'stake' information for the 'next era'.
    /// This is needed since stake amount is only applicable from the next era after it's been staked.
    ///
    /// Both `stake` and `staked_future` must ALWAYS refer to the same period.
    /// If `staked_future` is `Some`, it will always be **EXACTLY** one era after the `staked` field era.
    pub staked_future: Option<StakeAmount>,
    /// Number of contract stake entries in storage.
    #[codec(compact)]
    pub contract_stake_count: u32,
}

impl<UnlockingLen> Default for AccountLedger<UnlockingLen>
where
    UnlockingLen: Get<u32>,
{
    fn default() -> Self {
        Self {
            locked: Balance::zero(),
            unlocking: BoundedVec::default(),
            staked: StakeAmount::default(),
            staked_future: None,
            contract_stake_count: Zero::zero(),
        }
    }
}

impl<UnlockingLen> AccountLedger<UnlockingLen>
where
    UnlockingLen: Get<u32>,
{
    /// Empty if no locked/unlocking/staked info exists.
    pub fn is_empty(&self) -> bool {
        self.locked.is_zero()
            && self.unlocking.is_empty()
            && self.staked.total().is_zero()
            && self.staked_future.is_none()
    }

    /// Returns active locked amount.
    /// If `zero`, means that associated account hasn't got any active locked funds.
    ///
    /// It is possible that some funds are undergoing the unlocking period, but they aren't considered active in that case.
    pub fn active_locked_amount(&self) -> Balance {
        self.locked
    }

    /// Returns unlocking amount.
    /// If `zero`, means that associated account hasn't got any unlocking chunks.
    pub fn unlocking_amount(&self) -> Balance {
        self.unlocking.iter().fold(Balance::zero(), |sum, chunk| {
            sum.saturating_add(chunk.amount)
        })
    }

    /// Total locked amount by the user.
    /// Includes both active locked amount & unlocking amount.
    pub fn total_locked_amount(&self) -> Balance {
        self.active_locked_amount()
            .saturating_add(self.unlocking_amount())
    }

    /// Adds the specified amount to the total locked amount.
    pub fn add_lock_amount(&mut self, amount: Balance) {
        self.locked.saturating_accrue(amount);
    }

    /// Subtracts the specified amount of the total locked amount.
    pub fn subtract_lock_amount(&mut self, amount: Balance) {
        self.locked.saturating_reduce(amount);
    }

    /// Adds the specified amount to the unlocking chunks.
    ///
    /// If entry for the specified block already exists, it's updated.
    ///
    /// If entry for the specified block doesn't exist, it's created and insertion is attempted.
    /// In case vector has no more capacity, error is returned, and whole operation is a noop.
    pub fn add_unlocking_chunk(
        &mut self,
        amount: Balance,
        unlock_block: BlockNumber,
    ) -> Result<(), AccountLedgerError> {
        if amount.is_zero() {
            return Ok(());
        }

        let idx = self
            .unlocking
            .binary_search_by(|chunk| chunk.unlock_block.cmp(&unlock_block));

        match idx {
            Ok(idx) => {
                self.unlocking[idx].amount.saturating_accrue(amount);
            }
            Err(idx) => {
                let new_unlocking_chunk = UnlockingChunk {
                    amount,
                    unlock_block,
                };
                self.unlocking
                    .try_insert(idx, new_unlocking_chunk)
                    .map_err(|_| AccountLedgerError::NoCapacity)?;
            }
        }

        Ok(())
    }

    /// Amount available for unlocking.
    pub fn unlockable_amount(&self, current_period: PeriodNumber) -> Balance {
        self.active_locked_amount()
            .saturating_sub(self.staked_amount(current_period))
    }

    /// Claims all of the fully unlocked chunks, and returns the total claimable amount.
    pub fn claim_unlocked(&mut self, current_block_number: BlockNumber) -> Balance {
        let mut total = Balance::zero();

        self.unlocking.retain(|chunk| {
            if chunk.unlock_block <= current_block_number {
                total.saturating_accrue(chunk.amount);
                false
            } else {
                true
            }
        });

        total
    }

    /// Consumes all of the unlocking chunks, and returns the total amount being unlocked.
    pub fn consume_unlocking_chunks(&mut self) -> Balance {
        let amount = self.unlocking.iter().fold(Balance::zero(), |sum, chunk| {
            sum.saturating_add(chunk.amount)
        });
        self.unlocking = Default::default();

        amount
    }

    /// Amount that is available for staking.
    ///
    /// This is equal to the total active locked amount, minus the staked amount already active.
    pub fn stakeable_amount(&self, active_period: PeriodNumber) -> Balance {
        self.active_locked_amount()
            .saturating_sub(self.staked_amount(active_period))
    }

    /// Amount that is staked, in respect to currently active period.
    pub fn staked_amount(&self, active_period: PeriodNumber) -> Balance {
        // First check the 'future' entry, afterwards check the 'first' entry
        match self.staked_future {
            Some(stake_amount) if stake_amount.period == active_period => stake_amount.total(),
            _ => match self.staked {
                stake_amount if stake_amount.period == active_period => stake_amount.total(),
                _ => Balance::zero(),
            },
        }
    }

    /// How much is staked for the specified subperiod, in respect to the specified era.
    pub fn staked_amount_for_type(&self, subperiod: Subperiod, period: PeriodNumber) -> Balance {
        // First check the 'future' entry, afterwards check the 'first' entry
        match self.staked_future {
            Some(stake_amount) if stake_amount.period == period => stake_amount.for_type(subperiod),
            _ => match self.staked {
                stake_amount if stake_amount.period == period => stake_amount.for_type(subperiod),
                _ => Balance::zero(),
            },
        }
    }

    /// Check for stake/unstake operation era & period arguments.
    ///
    /// Ensures that the provided era & period are valid according to the current ledger state.
    fn stake_unstake_argument_check(
        &self,
        current_era: EraNumber,
        current_period_info: &PeriodInfo,
    ) -> Result<(), AccountLedgerError> {
        if !self.staked.is_empty() {
            // In case entry for the current era exists, it must match the era exactly.
            if self.staked.era != current_era {
                return Err(AccountLedgerError::InvalidEra);
            }
            if self.staked.period != current_period_info.number {
                return Err(AccountLedgerError::InvalidPeriod);
            }
            // In case it doesn't (i.e. first time staking), then the future era must either be the current or the next era.
        } else if let Some(stake_amount) = self.staked_future {
            if stake_amount.era != current_era.saturating_add(1) && stake_amount.era != current_era
            {
                return Err(AccountLedgerError::InvalidEra);
            }
            if stake_amount.period != current_period_info.number {
                return Err(AccountLedgerError::InvalidPeriod);
            }
        }
        Ok(())
    }

    /// Adds the specified amount to total staked amount, if possible.
    ///
    /// Staking can only be done for the ongoing period, and era.
    /// 1. The `period` requirement enforces staking in the ongoing period.
    /// 2. The `era` requirement enforces staking in the ongoing era.
    ///
    /// The 2nd condition is needed to prevent stakers from building a significant history of stakes,
    /// without claiming the rewards. So if a historic era exists as an entry, stakers will first need to claim
    /// the pending rewards, before they can stake again.
    ///
    /// Additionally, the staked amount must not exceed what's available for staking.
    pub fn add_stake_amount(
        &mut self,
        amount: Balance,
        current_era: EraNumber,
        current_period_info: PeriodInfo,
    ) -> Result<(), AccountLedgerError> {
        if amount.is_zero() {
            return Ok(());
        }

        self.stake_unstake_argument_check(current_era, &current_period_info)?;

        if self.stakeable_amount(current_period_info.number) < amount {
            return Err(AccountLedgerError::UnavailableStakeFunds);
        }

        // Update existing entry if it exists, otherwise create it.
        match self.staked_future.as_mut() {
            Some(stake_amount) => {
                stake_amount.add(amount, current_period_info.subperiod);
            }
            None => {
                let mut stake_amount = self.staked;
                stake_amount.era = current_era.saturating_add(1);
                stake_amount.period = current_period_info.number;
                stake_amount.add(amount, current_period_info.subperiod);
                self.staked_future = Some(stake_amount);
            }
        }

        Ok(())
    }

    /// Subtracts the specified amount from the total staked amount, if possible.
    ///
    /// Unstake can only be called if the entry for the current era exists.
    /// In case historic entry exists, rewards first need to be claimed, before unstaking is possible.
    /// Similar as with stake functionality, this is to prevent staker from building a significant history of stakes.
    pub fn unstake_amount(
        &mut self,
        amount: Balance,
        current_era: EraNumber,
        current_period_info: PeriodInfo,
    ) -> Result<(), AccountLedgerError> {
        if amount.is_zero() {
            return Ok(());
        }

        self.stake_unstake_argument_check(current_era, &current_period_info)?;

        // User must be precise with their unstake amount.
        if self.staked_amount(current_period_info.number) < amount {
            return Err(AccountLedgerError::UnstakeAmountLargerThanStake);
        }

        self.staked.subtract(amount, current_period_info.subperiod);

        // Convenience cleanup
        if self.staked.is_empty() {
            self.staked = Default::default();
        }

        if let Some(mut stake_amount) = self.staked_future {
            stake_amount.subtract(amount, current_period_info.subperiod);

            self.staked_future = if stake_amount.is_empty() {
                None
            } else {
                Some(stake_amount)
            };
        }

        Ok(())
    }

    /// Period for which account has staking information or `None` if no staking information exists.
    pub fn staked_period(&self) -> Option<PeriodNumber> {
        if self.staked.is_empty() {
            self.staked_future.map(|stake_amount| stake_amount.period)
        } else {
            Some(self.staked.period)
        }
    }

    /// Earliest era for which the account has staking information or `None` if no staking information exists.
    pub fn earliest_staked_era(&self) -> Option<EraNumber> {
        if self.staked.is_empty() {
            self.staked_future.map(|stake_amount| stake_amount.era)
        } else {
            Some(self.staked.era)
        }
    }

    /// Cleanup staking information if it has expired.
    ///
    /// # Args
    /// `valid_threshold_period` - last period for which entries can still be considered valid.
    ///
    /// `true` if any change was made, `false` otherwise.
    pub fn maybe_cleanup_expired(&mut self, valid_threshold_period: PeriodNumber) -> bool {
        match self.staked_period() {
            Some(staked_period) if staked_period < valid_threshold_period => {
                self.staked = Default::default();
                self.staked_future = None;
                true
            }
            _ => false,
        }
    }

    /// 'Claim' rewards up to the specified era.
    /// Returns an iterator over the `(era, amount)` pairs, where `amount`
    /// describes the staked amount eligible for reward in the appropriate era.
    ///
    /// If `period_end` is provided, it's used to determine whether all applicable chunks have been claimed.
    pub fn claim_up_to_era(
        &mut self,
        era: EraNumber,
        period_end: Option<EraNumber>,
    ) -> Result<EraStakePairIter, AccountLedgerError> {
        // Main entry exists, but era isn't 'in history'
        if !self.staked.is_empty() {
            ensure!(era >= self.staked.era, AccountLedgerError::NothingToClaim);
        } else if let Some(stake_amount) = self.staked_future {
            // Future entry exists, but era isn't 'in history'
            ensure!(era >= stake_amount.era, AccountLedgerError::NothingToClaim);
        }

        // There are multiple options:
        // 1. We only have future entry, no current entry
        // 2. We have both current and future entry, but are only claiming 1 era
        // 3. We have both current and future entry, and are claiming multiple eras
        // 4. We only have current entry, no future entry
        let (span, maybe_first) = if let Some(stake_amount) = self.staked_future {
            if self.staked.is_empty() {
                ((stake_amount.era, era, stake_amount.total()), None)
            } else if self.staked.era == era {
                ((era, era, self.staked.total()), None)
            } else {
                (
                    (stake_amount.era, era, stake_amount.total()),
                    Some((self.staked.era, self.staked.total())),
                )
            }
        } else {
            ((self.staked.era, era, self.staked.total()), None)
        };

        let result = EraStakePairIter::new(span, maybe_first)
            .map_err(|_| AccountLedgerError::InvalidIterator)?;

        // Rollover future to 'current' stake amount
        if let Some(stake_amount) = self.staked_future.take() {
            self.staked = stake_amount;
        }
        self.staked.era = era.saturating_add(1);

        // Make sure to clean up the entries if all rewards for the period have been claimed.
        match period_end {
            Some(period_end_era) if era >= period_end_era => {
                self.staked = Default::default();
                self.staked_future = None;
            }
            _ => (),
        }

        Ok(result)
    }
}

/// Helper internal struct for iterating over `(era, stake amount)` pairs.
///
/// Due to how `AccountLedger` is implemented, few scenarios are possible when claming rewards:
///
/// 1. `staked` has some amount, `staked_future` is `None`
///   * `maybe_first` is `None`, span describes the entire range
/// 2. `staked` has nothing, `staked_future` is some and has some amount
///   * `maybe_first` is `None`, span describes the entire range
/// 3. `staked` has some amount, `staked_future` has some amount
///   * `maybe_first` is `Some` and covers the `staked` entry, span describes the entire range except the first pair.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EraStakePairIter {
    /// Denotes whether the first entry is different than the others.
    maybe_first: Option<(EraNumber, Balance)>,
    /// Starting era of the span.
    start_era: EraNumber,
    /// Ending era of the span.
    end_era: EraNumber,
    /// Staked amount in the span.
    amount: Balance,
}

impl EraStakePairIter {
    /// Create new iterator struct for `(era, staked amount)` pairs.
    pub fn new(
        span: (EraNumber, EraNumber, Balance),
        maybe_first: Option<(EraNumber, Balance)>,
    ) -> Result<Self, ()> {
        // First era must be smaller or equal to the last era.
        if span.0 > span.1 {
            return Err(());
        }
        // If 'maybe_first' is defined, it must exactly match the `span.0 - 1` era value.
        match maybe_first {
            Some((era, _)) if span.0.saturating_sub(era) != 1 => {
                return Err(());
            }
            _ => (),
        }

        Ok(Self {
            maybe_first,
            start_era: span.0,
            end_era: span.1,
            amount: span.2,
        })
    }
}

impl Iterator for EraStakePairIter {
    type Item = (EraNumber, Balance);

    fn next(&mut self) -> Option<Self::Item> {
        // Fist cover the scenario where we have a unique first value
        if let Some((era, amount)) = self.maybe_first.take() {
            return Some((era, amount));
        }

        // Afterwards, just keep returning the same amount for different eras
        if self.start_era <= self.end_era {
            let value = (self.start_era, self.amount);
            self.start_era.saturating_inc();
            return Some(value);
        } else {
            None
        }
    }
}

/// Describes stake amount in an particular era/period.
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct StakeAmount {
    /// Amount of staked funds accounting for the voting period.
    #[codec(compact)]
    pub voting: Balance,
    /// Amount of staked funds accounting for the build&earn period.
    #[codec(compact)]
    pub build_and_earn: Balance,
    /// Era to which this stake amount refers to.
    #[codec(compact)]
    pub era: EraNumber,
    /// Period to which this stake amount refers to.
    #[codec(compact)]
    pub period: PeriodNumber,
}

impl StakeAmount {
    /// `true` if nothing is staked, `false` otherwise
    pub fn is_empty(&self) -> bool {
        self.voting.is_zero() && self.build_and_earn.is_zero()
    }

    /// Total amount staked in both subperiods.
    pub fn total(&self) -> Balance {
        self.voting.saturating_add(self.build_and_earn)
    }

    /// Amount staked for the specified subperiod.
    pub fn for_type(&self, subperiod: Subperiod) -> Balance {
        match subperiod {
            Subperiod::Voting => self.voting,
            Subperiod::BuildAndEarn => self.build_and_earn,
        }
    }

    /// Stake the specified `amount` for the specified `subperiod`.
    pub fn add(&mut self, amount: Balance, subperiod: Subperiod) {
        match subperiod {
            Subperiod::Voting => self.voting.saturating_accrue(amount),
            Subperiod::BuildAndEarn => self.build_and_earn.saturating_accrue(amount),
        }
    }

    /// Unstake the specified `amount` for the specified `subperiod`.
    ///
    /// In case subperiod is `Voting`, the amount is subtracted from the voting period.
    ///
    /// In case subperiod is `Build&Earn`, the amount is first subtracted from the
    /// build&earn amount, and any rollover is subtracted from the voting period.
    pub fn subtract(&mut self, amount: Balance, subperiod: Subperiod) {
        match subperiod {
            Subperiod::Voting => self.voting.saturating_reduce(amount),
            Subperiod::BuildAndEarn => {
                if self.build_and_earn >= amount {
                    self.build_and_earn.saturating_reduce(amount);
                } else {
                    // Rollover from build&earn to voting, is guaranteed to be larger than zero due to previous check
                    // E.g. voting = 10, build&earn = 5, amount = 7
                    // underflow = build&earn - amount = 5 - 7 = -2
                    // voting = 10 - 2 = 8
                    // build&earn = 0
                    let remainder = amount.saturating_sub(self.build_and_earn);
                    self.build_and_earn = Balance::zero();
                    self.voting.saturating_reduce(remainder);
                }
            }
        }
    }
}

/// Info about current era, including the rewards, how much is locked, unlocking, etc.
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct EraInfo {
    /// How much balance is locked in dApp staking.
    /// Does not include the amount that is undergoing the unlocking period.
    #[codec(compact)]
    pub total_locked: Balance,
    /// How much balance is undergoing unlocking process.
    /// This amount still counts into locked amount.
    #[codec(compact)]
    pub unlocking: Balance,
    /// Stake amount valid for the ongoing era.
    pub current_stake_amount: StakeAmount,
    /// Stake amount valid from the next era.
    pub next_stake_amount: StakeAmount,
}

impl EraInfo {
    /// Update with the new amount that has just been locked.
    pub fn add_locked(&mut self, amount: Balance) {
        self.total_locked.saturating_accrue(amount);
    }

    /// Update with the new amount that has just started undergoing the unlocking period.
    pub fn unlocking_started(&mut self, amount: Balance) {
        self.total_locked.saturating_reduce(amount);
        self.unlocking.saturating_accrue(amount);
    }

    /// Update with the new amount that has been removed from unlocking.
    pub fn unlocking_removed(&mut self, amount: Balance) {
        self.unlocking.saturating_reduce(amount);
    }

    /// Add the specified `amount` to the appropriate stake amount, based on the `Subperiod`.
    pub fn add_stake_amount(&mut self, amount: Balance, subperiod: Subperiod) {
        self.next_stake_amount.add(amount, subperiod);
    }

    /// Subtract the specified `amount` from the appropriate stake amount, based on the `Subperiod`.
    pub fn unstake_amount(&mut self, amount: Balance, subperiod: Subperiod) {
        self.current_stake_amount.subtract(amount, subperiod);
        self.next_stake_amount.subtract(amount, subperiod);
    }

    /// Total staked amount in this era.
    pub fn total_staked_amount(&self) -> Balance {
        self.current_stake_amount.total()
    }

    /// Staked amount of specified `type` in this era.
    pub fn staked_amount(&self, subperiod: Subperiod) -> Balance {
        self.current_stake_amount.for_type(subperiod)
    }

    /// Total staked amount in the next era.
    pub fn total_staked_amount_next_era(&self) -> Balance {
        self.next_stake_amount.total()
    }

    /// Staked amount of specifeid `type` in the next era.
    pub fn staked_amount_next_era(&self, subperiod: Subperiod) -> Balance {
        self.next_stake_amount.for_type(subperiod)
    }

    /// Updates `Self` to reflect the transition to the next era.
    ///
    ///  ## Args
    /// `next_subperiod` - `None` if no subperiod change, `Some(type)` if `type` is starting from the next era.
    pub fn migrate_to_next_era(&mut self, next_subperiod: Option<Subperiod>) {
        match next_subperiod {
            // If next era marks start of new voting period period, it means we're entering a new period
            Some(Subperiod::Voting) => {
                for stake_amount in [&mut self.current_stake_amount, &mut self.next_stake_amount] {
                    stake_amount.voting = Zero::zero();
                    stake_amount.build_and_earn = Zero::zero();
                    stake_amount.era.saturating_inc();
                    stake_amount.period.saturating_inc();
                }
            }
            Some(Subperiod::BuildAndEarn) | None => {
                self.current_stake_amount = self.next_stake_amount;
                self.next_stake_amount.era.saturating_inc();
            }
        };
    }
}

/// Information about how much a particular staker staked on a particular smart contract.
///
/// Keeps track of amount staked in the 'voting period', as well as 'build&earn period'.
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct SingularStakingInfo {
    /// Staked amount
    staked: StakeAmount,
    /// Indicates whether a staker is a loyal staker or not.
    loyal_staker: bool,
}

impl SingularStakingInfo {
    /// Creates new instance of the struct.
    ///
    /// ## Args
    ///
    /// `period` - period number for which this entry is relevant.
    /// `subperiod` - subperiod during which this entry is created.
    pub fn new(period: PeriodNumber, subperiod: Subperiod) -> Self {
        Self {
            staked: StakeAmount {
                voting: Balance::zero(),
                build_and_earn: Balance::zero(),
                era: 0,
                period,
            },
            // Loyalty staking is only possible if stake is first made during the voting period.
            loyal_staker: subperiod == Subperiod::Voting,
        }
    }

    /// Stake the specified amount on the contract, for the specified subperiod.
    pub fn stake(&mut self, amount: Balance, current_era: EraNumber, subperiod: Subperiod) {
        self.staked.add(amount, subperiod);
        // Stake is only valid from the next era so we keep it consistent here
        self.staked.era = current_era.saturating_add(1);
    }

    /// Unstakes some of the specified amount from the contract.
    ///
    /// In case the `amount` being unstaked is larger than the amount staked in the `voting period`,
    /// and `voting period` has passed, this will remove the _loyalty_ flag from the staker.
    ///
    /// Returns the amount that was unstaked from the `voting period` stake, and from the `build&earn period` stake.
    pub fn unstake(
        &mut self,
        amount: Balance,
        current_era: EraNumber,
        subperiod: Subperiod,
    ) -> (Balance, Balance) {
        let snapshot = self.staked;

        self.staked.subtract(amount, subperiod);
        // Keep the latest era for which the entry is valid
        self.staked.era = self.staked.era.max(current_era);

        self.loyal_staker = self.loyal_staker
            && (subperiod == Subperiod::Voting
                || subperiod == Subperiod::BuildAndEarn && self.staked.voting == snapshot.voting);

        // Amount that was unstaked
        (
            snapshot.voting.saturating_sub(self.staked.voting),
            snapshot
                .build_and_earn
                .saturating_sub(self.staked.build_and_earn),
        )
    }

    /// Total staked on the contract by the user. Both subperiod stakes are included.
    pub fn total_staked_amount(&self) -> Balance {
        self.staked.total()
    }

    /// Returns amount staked in the specified period.
    pub fn staked_amount(&self, subperiod: Subperiod) -> Balance {
        self.staked.for_type(subperiod)
    }

    /// If `true` staker has staked during voting period and has never reduced their sta
    pub fn is_loyal(&self) -> bool {
        self.loyal_staker
    }

    /// Period for which this entry is relevant.
    pub fn period_number(&self) -> PeriodNumber {
        self.staked.period
    }

    /// Era in which the entry was last time updated
    pub fn era(&self) -> EraNumber {
        self.staked.era
    }

    /// `true` if no stake exists, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.staked.is_empty()
    }
}

/// Composite type that holds information about how much was staked on a contract in up to two distinct eras.
///
/// This is needed since 'stake' operation only makes the staked amount valid from the next era.
/// In a situation when `stake` is called in era `N`, the staked amount is valid from era `N+1`, hence the need for 'future' entry.
///
/// **NOTE:** The 'future' entry term is only valid in the era when `stake` is called. It's possible contract stake isn't changed in consecutive eras,
/// so we might end up in a situation where era is `N + 10` but `staked` entry refers to era `N` and `staked_future` entry refers to era `N+1`.
/// This is still valid since these values are expected to be updated lazily.
#[derive(Encode, Decode, MaxEncodedLen, RuntimeDebug, PartialEq, Eq, Clone, TypeInfo, Default)]
pub struct ContractStakeAmount {
    /// Staked amount in the 'current' era.
    pub staked: StakeAmount,
    /// Staked amount in the next or 'future' era.
    pub staked_future: Option<StakeAmount>,
}

impl ContractStakeAmount {
    /// `true` if series is empty, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.staked.is_empty() && self.staked_future.is_none()
    }

    /// Latest period for which stake entry exists.
    pub fn latest_stake_period(&self) -> Option<PeriodNumber> {
        if let Some(stake_amount) = self.staked_future {
            Some(stake_amount.period)
        } else if !self.staked.is_empty() {
            Some(self.staked.period)
        } else {
            None
        }
    }

    /// Latest era for which stake entry exists.
    pub fn latest_stake_era(&self) -> Option<EraNumber> {
        if let Some(stake_amount) = self.staked_future {
            Some(stake_amount.era)
        } else if !self.staked.is_empty() {
            Some(self.staked.era)
        } else {
            None
        }
    }

    /// Returns the `StakeAmount` type for the specified era & period, if it exists.
    pub fn get(&self, era: EraNumber, period: PeriodNumber) -> Option<StakeAmount> {
        let mut maybe_result = match (self.staked, self.staked_future) {
            (_, Some(staked_future)) if staked_future.era <= era => {
                if staked_future.period == period {
                    Some(staked_future)
                } else {
                    None
                }
            }
            (staked, _) if staked.era <= era && staked.period == period => Some(staked),
            _ => None,
        };

        if let Some(result) = maybe_result.as_mut() {
            result.era = era;
        }

        maybe_result
    }

    /// Total staked amount on the contract, in the active period.
    pub fn total_staked_amount(&self, active_period: PeriodNumber) -> Balance {
        match (self.staked, self.staked_future) {
            (_, Some(staked_future)) if staked_future.period == active_period => {
                staked_future.total()
            }
            (staked, _) if staked.period == active_period => staked.total(),
            _ => Balance::zero(),
        }
    }

    /// Staked amount on the contract, for specified subperiod, in the active period.
    pub fn staked_amount(&self, active_period: PeriodNumber, subperiod: Subperiod) -> Balance {
        match (self.staked, self.staked_future) {
            (_, Some(staked_future)) if staked_future.period == active_period => {
                staked_future.for_type(subperiod)
            }
            (staked, _) if staked.period == active_period => staked.for_type(subperiod),
            _ => Balance::zero(),
        }
    }

    /// Stake the specified `amount` on the contract, for the specified `subperiod` and `era`.
    pub fn stake(&mut self, amount: Balance, period_info: PeriodInfo, current_era: EraNumber) {
        let stake_era = current_era.saturating_add(1);

        match self.staked_future.as_mut() {
            // Future entry matches the era, just updated it and return
            Some(stake_amount) if stake_amount.era == stake_era => {
                stake_amount.add(amount, period_info.subperiod);
                return;
            }
            // Future entry has older era, but periods match so overwrite the 'current' entry with it
            Some(stake_amount) if stake_amount.period == period_info.number => {
                self.staked = *stake_amount;
            }
            // Otherwise do nothing
            _ => (),
        }

        // Prepare new entry
        let mut new_entry = match self.staked {
            // 'current' entry period matches so we use it as base for the new entry
            stake_amount if stake_amount.period == period_info.number => stake_amount,
            // otherwise just create a dummy new entry
            _ => Default::default(),
        };
        new_entry.add(amount, period_info.subperiod);
        new_entry.era = stake_era;
        new_entry.period = period_info.number;

        self.staked_future = Some(new_entry);

        // Convenience cleanup
        if self.staked.period < period_info.number {
            self.staked = Default::default();
        }
    }

    /// Unstake the specified `amount` from the contract, for the specified `subperiod` and `era`.
    pub fn unstake(&mut self, amount: Balance, period_info: PeriodInfo, current_era: EraNumber) {
        // First align entries - we only need to keep track of the current era, and the next one
        match self.staked_future {
            // Future entry exists, but it covers current or older era.
            Some(stake_amount)
                if stake_amount.era <= current_era && stake_amount.period == period_info.number =>
            {
                self.staked = stake_amount;
                self.staked.era = current_era;
                self.staked_future = None;
            }
            _ => (),
        }

        // Current entry is from the right period, but older era. Shift it to the current era.
        if self.staked.era < current_era && self.staked.period == period_info.number {
            self.staked.era = current_era;
        }

        // Subtract both amounts
        self.staked.subtract(amount, period_info.subperiod);
        if let Some(stake_amount) = self.staked_future.as_mut() {
            stake_amount.subtract(amount, period_info.subperiod);
        }

        // Conevnience cleanup
        if self.staked.is_empty() {
            self.staked = Default::default();
        }
        if let Some(stake_amount) = self.staked_future {
            if stake_amount.is_empty() {
                self.staked_future = None;
            }
        }
    }
}

/// Information required for staker reward payout for a particular era.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct EraReward {
    /// Total reward pool for staker rewards
    #[codec(compact)]
    pub staker_reward_pool: Balance,
    /// Total amount which was staked at the end of an era
    #[codec(compact)]
    pub staked: Balance,
    /// Total reward pool for dApp rewards
    #[codec(compact)]
    pub dapp_reward_pool: Balance,
}

#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum EraRewardSpanError {
    /// Provided era is invalid. Must be exactly one era after the last one in the span.
    InvalidEra,
    /// Span has no more capacity for additional entries.
    NoCapacity,
}

/// Used to efficiently store era span information.
#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    RuntimeDebugNoBound,
    PartialEqNoBound,
    EqNoBound,
    CloneNoBound,
    TypeInfo,
)]
#[scale_info(skip_type_params(SL))]
pub struct EraRewardSpan<SL: Get<u32>> {
    /// Span of EraRewardInfo entries.
    span: BoundedVec<EraReward, SL>,
    /// The first era in the span.
    #[codec(compact)]
    first_era: EraNumber,
    /// The final era in the span.
    #[codec(compact)]
    last_era: EraNumber,
}

impl<SL> EraRewardSpan<SL>
where
    SL: Get<u32>,
{
    /// Create new instance of the `EraRewardSpan`
    pub fn new() -> Self {
        Self {
            span: Default::default(),
            first_era: 0,
            last_era: 0,
        }
    }

    /// First era covered in the span.
    pub fn first_era(&self) -> EraNumber {
        self.first_era
    }

    /// Last era covered in the span
    pub fn last_era(&self) -> EraNumber {
        self.last_era
    }

    /// Span length.
    pub fn len(&self) -> usize {
        self.span.len()
    }

    /// `true` if span is empty, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.span.is_empty()
    }

    /// Push new `EraReward` entry into the span.
    /// If span is non-empty, the provided `era` must be exactly one era after the last one in the span.
    pub fn push(
        &mut self,
        era: EraNumber,
        era_reward: EraReward,
    ) -> Result<(), EraRewardSpanError> {
        // First entry, no checks, just set eras to the provided value.
        if self.span.is_empty() {
            self.first_era = era;
            self.last_era = era;
            self.span
                .try_push(era_reward)
                // Defensive check, should never happen since it means capacity is 'zero'.
                .map_err(|_| EraRewardSpanError::NoCapacity)
        } else {
            // Defensive check to ensure next era rewards refers to era after the last one in the span.
            if era != self.last_era.saturating_add(1) {
                return Err(EraRewardSpanError::InvalidEra);
            }

            self.last_era = era;
            self.span
                .try_push(era_reward)
                .map_err(|_| EraRewardSpanError::NoCapacity)
        }
    }

    /// Get the `EraReward` entry for the specified `era`.
    ///
    /// In case `era` is not covered by the span, `None` is returned.
    pub fn get(&self, era: EraNumber) -> Option<&EraReward> {
        match era.checked_sub(self.first_era()) {
            Some(index) => self.span.get(index as usize),
            None => None,
        }
    }
}

/// Description of tier entry requirement.
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum TierThreshold {
    /// Entry into tier is mandated by minimum amount of staked funds.
    /// Value is fixed, and is not expected to change in between periods.
    FixedTvlAmount { amount: Balance },
    /// Entry into tier is mandated by minimum amount of staked funds.
    /// Value is expected to dynamically change in-between periods, depending on the system parameters.
    /// The `amount` should never go below the `minimum_amount`.
    DynamicTvlAmount {
        amount: Balance,
        minimum_amount: Balance,
    },
}

impl TierThreshold {
    /// Used to check if stake amount satisfies the threshold or not.
    pub fn is_satisfied(&self, stake: Balance) -> bool {
        match self {
            Self::FixedTvlAmount { amount } => stake >= *amount,
            Self::DynamicTvlAmount { amount, .. } => stake >= *amount,
        }
    }

    /// Return threshold for the tier.
    pub fn threshold(&self) -> Balance {
        match self {
            Self::FixedTvlAmount { amount } => *amount,
            Self::DynamicTvlAmount { amount, .. } => *amount,
        }
    }
}

/// Top level description of tier slot parameters used to calculate tier configuration.
#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    RuntimeDebugNoBound,
    PartialEqNoBound,
    EqNoBound,
    CloneNoBound,
    TypeInfo,
)]
#[scale_info(skip_type_params(NT))]
pub struct TierParameters<NT: Get<u32>> {
    /// Reward distribution per tier, in percentage.
    /// First entry refers to the first tier, and so on.
    /// The sum of all values must not exceed 100%.
    /// In case it is less, portion of rewards will never be distributed.
    pub reward_portion: BoundedVec<Permill, NT>,
    /// Distribution of number of slots per tier, in percentage.
    /// First entry refers to the first tier, and so on.
    /// The sum of all values must not exceed 100%.
    /// In case it is less, slot capacity will never be fully filled.
    pub slot_distribution: BoundedVec<Permill, NT>,
    /// Requirements for entry into each tier.
    /// First entry refers to the first tier, and so on.
    pub tier_thresholds: BoundedVec<TierThreshold, NT>,
}

impl<NT: Get<u32>> TierParameters<NT> {
    /// Check if configuration is valid.
    /// All vectors are expected to have exactly the amount of entries as `number_of_tiers`.
    pub fn is_valid(&self) -> bool {
        // Reward portions sum should not exceed 100%.
        if self
            .reward_portion
            .iter()
            .fold(Some(Permill::zero()), |acc, permill| match acc {
                Some(acc) => acc.checked_add(permill),
                None => None,
            })
            .is_none()
        {
            return false;
        }

        // Slot distribution sum should not exceed 100%.
        if self
            .slot_distribution
            .iter()
            .fold(Some(Permill::zero()), |acc, permill| match acc {
                Some(acc) => acc.checked_add(permill),
                None => None,
            })
            .is_none()
        {
            return false;
        }

        let number_of_tiers: usize = NT::get() as usize;
        number_of_tiers == self.reward_portion.len()
            && number_of_tiers == self.slot_distribution.len()
            && number_of_tiers == self.tier_thresholds.len()
    }
}

impl<NT: Get<u32>> Default for TierParameters<NT> {
    fn default() -> Self {
        Self {
            reward_portion: BoundedVec::default(),
            slot_distribution: BoundedVec::default(),
            tier_thresholds: BoundedVec::default(),
        }
    }
}

/// Configuration of dApp tiers.
#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    RuntimeDebugNoBound,
    PartialEqNoBound,
    EqNoBound,
    CloneNoBound,
    TypeInfo,
)]
#[scale_info(skip_type_params(NT))]
pub struct TiersConfiguration<NT: Get<u32>> {
    /// Total number of slots.
    #[codec(compact)]
    pub number_of_slots: u16,
    /// Number of slots per tier.
    /// First entry refers to the first tier, and so on.
    pub slots_per_tier: BoundedVec<u16, NT>,
    /// Reward distribution per tier, in percentage.
    /// First entry refers to the first tier, and so on.
    /// The sum of all values must be exactly equal to 1.
    pub reward_portion: BoundedVec<Permill, NT>,
    /// Requirements for entry into each tier.
    /// First entry refers to the first tier, and so on.
    pub tier_thresholds: BoundedVec<TierThreshold, NT>,
}

impl<NT: Get<u32>> Default for TiersConfiguration<NT> {
    fn default() -> Self {
        Self {
            number_of_slots: 0,
            slots_per_tier: BoundedVec::default(),
            reward_portion: BoundedVec::default(),
            tier_thresholds: BoundedVec::default(),
        }
    }
}

// Some TODOs:
// Some parts regarding tiers should be refactored.
// * There's no need to keep thresholds in two separate storage items since the calculation can always be done compared to the
//    anchor value of 5 cents. This still needs to be checked & investigated, but it's worth a try.

impl<NT: Get<u32>> TiersConfiguration<NT> {
    /// Check if parameters are valid.
    pub fn is_valid(&self) -> bool {
        let number_of_tiers: usize = NT::get() as usize;
        number_of_tiers == self.slots_per_tier.len()
            // All vecs length must match number of tiers.
            && number_of_tiers == self.reward_portion.len()
            && number_of_tiers == self.tier_thresholds.len()
            // Total number of slots must match the sum of slots per tier. 
            && self.slots_per_tier.iter().fold(0, |acc, x| acc + x) == self.number_of_slots
    }

    /// Calculate new `TiersConfiguration`, based on the old settings, current native currency price and tier configuration.
    pub fn calculate_new(&self, native_price: FixedU64, params: &TierParameters<NT>) -> Self {
        // It must always be at least 1 slot.
        let new_number_of_slots = Self::calculate_number_of_slots(native_price).max(1);

        // Calculate how much each tier gets slots.
        let new_slots_per_tier: Vec<u16> = params
            .slot_distribution
            .clone()
            .into_inner()
            .iter()
            .map(|percent| *percent * new_number_of_slots as u128)
            .map(|x| x.unique_saturated_into())
            .collect();
        let new_slots_per_tier =
            BoundedVec::<u16, NT>::try_from(new_slots_per_tier).unwrap_or_default();

        // Update tier thresholds.
        // In case number of slots increase, we decrease thresholds required to enter the tier.
        // In case number of slots decrease, we increase the threshold required to enter the tier.
        //
        // According to formula: %_threshold = (100% / (100% - delta_%_slots) - 1) * 100%
        //
        // where delta_%_slots is simply: (old_num_slots - new_num_slots) / old_num_slots
        //
        // When these entries are put into the threshold formula, we get:
        // = 1 / ( 1 - (old_num_slots - new_num_slots) / old_num_slots ) - 1
        // = 1 / ( new / old) - 1
        // = old / new - 1
        // = (old - new) / new
        //
        // This number can be negative. In order to keep all operations in unsigned integer domain,
        // formulas are adjusted like:
        //
        // 1. Number of slots has increased, threshold is expected to decrease
        // %_threshold = (new_num_slots - old_num_slots) / new_num_slots
        // new_threshold = old_threshold * (1 - %_threshold)
        //
        // 2. Number of slots has decreased, threshold is expected to increase
        // %_threshold = (old_num_slots - new_num_slots) / new_num_slots
        // new_threshold = old_threshold * (1 + %_threshold)
        //
        let new_tier_thresholds = if new_number_of_slots > self.number_of_slots {
            let delta_threshold_decrease = FixedU64::from_rational(
                (new_number_of_slots - self.number_of_slots).into(),
                new_number_of_slots.into(),
            );

            let mut new_tier_thresholds = self.tier_thresholds.clone();
            new_tier_thresholds
                .iter_mut()
                .for_each(|threshold| match threshold {
                    TierThreshold::DynamicTvlAmount {
                        amount,
                        minimum_amount,
                    } => {
                        *amount = amount
                            .saturating_sub(delta_threshold_decrease.saturating_mul_int(*amount));
                        *amount = *amount.max(minimum_amount);
                    }
                    _ => (),
                });

            new_tier_thresholds
        } else if new_number_of_slots < self.number_of_slots {
            let delta_threshold_increase = FixedU64::from_rational(
                (self.number_of_slots - new_number_of_slots).into(),
                new_number_of_slots.into(),
            );

            let mut new_tier_thresholds = self.tier_thresholds.clone();
            new_tier_thresholds
                .iter_mut()
                .for_each(|threshold| match threshold {
                    TierThreshold::DynamicTvlAmount { amount, .. } => {
                        *amount = amount
                            .saturating_add(delta_threshold_increase.saturating_mul_int(*amount));
                    }
                    _ => (),
                });

            new_tier_thresholds
        } else {
            self.tier_thresholds.clone()
        };

        Self {
            number_of_slots: new_number_of_slots,
            slots_per_tier: new_slots_per_tier,
            reward_portion: params.reward_portion.clone(),
            tier_thresholds: new_tier_thresholds,
        }
    }

    /// Calculate number of slots, based on the provided native token price.
    pub fn calculate_number_of_slots(native_price: FixedU64) -> u16 {
        // floor(1000 x price + 50), formula proposed in Tokenomics 2.0 document.
        let result: u64 = native_price.saturating_mul_int(1000).saturating_add(50);

        result.unique_saturated_into()
    }
}

/// Used to represent into which tier does a particular dApp fall into.
///
/// In case tier Id is `None`, it means that either reward was already claimed, or dApp is not eligible for rewards.
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo)]
pub struct DAppTier {
    /// Unique dApp id in dApp staking protocol.
    #[codec(compact)]
    pub dapp_id: DAppId,
    /// `Some(tier_id)` if dApp belongs to tier and has unclaimed rewards, `None` otherwise.
    pub tier_id: Option<TierId>,
}

/// Information about all of the dApps that got into tiers, and tier rewards
#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    RuntimeDebugNoBound,
    PartialEqNoBound,
    EqNoBound,
    CloneNoBound,
    TypeInfo,
)]
#[scale_info(skip_type_params(MD, NT))]
pub struct DAppTierRewards<MD: Get<u32>, NT: Get<u32>> {
    /// DApps and their corresponding tiers (or `None` if they have been claimed in the meantime)
    pub dapps: BoundedVec<DAppTier, MD>,
    /// Rewards for each tier. First entry refers to the first tier, and so on.
    pub rewards: BoundedVec<Balance, NT>,
    /// Period during which this struct was created.
    #[codec(compact)]
    pub period: PeriodNumber,
}

impl<MD: Get<u32>, NT: Get<u32>> Default for DAppTierRewards<MD, NT> {
    fn default() -> Self {
        Self {
            dapps: BoundedVec::default(),
            rewards: BoundedVec::default(),
            period: 0,
        }
    }
}

impl<MD: Get<u32>, NT: Get<u32>> DAppTierRewards<MD, NT> {
    /// Attempt to construct `DAppTierRewards` struct.
    /// If the provided arguments exceed the allowed capacity, return an error.
    pub fn new(
        dapps: Vec<DAppTier>,
        rewards: Vec<Balance>,
        period: PeriodNumber,
    ) -> Result<Self, ()> {
        // Sort by dApp ID, in ascending order (unstable sort should be faster, and stability is "guaranteed" due to lack of duplicated Ids).
        let mut dapps = dapps;
        dapps.sort_unstable_by(|first, second| first.dapp_id.cmp(&second.dapp_id));

        let dapps = BoundedVec::try_from(dapps).map_err(|_| ())?;
        let rewards = BoundedVec::try_from(rewards).map_err(|_| ())?;
        Ok(Self {
            dapps,
            rewards,
            period,
        })
    }

    /// Consume reward for the specified dapp id, returning its amount and tier Id.
    /// In case dapp isn't applicable for rewards, or they have already been consumed, returns `None`.
    pub fn try_claim(&mut self, dapp_id: DAppId) -> Result<(Balance, TierId), DAppTierError> {
        // Check if dApp Id exists.
        let dapp_idx = self
            .dapps
            .binary_search_by(|entry| entry.dapp_id.cmp(&dapp_id))
            .map_err(|_| DAppTierError::NoDAppInTiers)?;

        match self.dapps.get_mut(dapp_idx) {
            Some(dapp_tier) => {
                if let Some(tier_id) = dapp_tier.tier_id.take() {
                    Ok((
                        self.rewards
                            .get(tier_id as usize)
                            .map_or(Balance::zero(), |x| *x),
                        tier_id,
                    ))
                } else {
                    Err(DAppTierError::RewardAlreadyClaimed)
                }
            }
            // unreachable code, at this point it was proved that index exists
            _ => Err(DAppTierError::InternalError),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DAppTierError {
    /// Specified dApp Id doesn't exist in any tier.
    NoDAppInTiers,
    /// Reward has already been claimed for this dApp.
    RewardAlreadyClaimed,
    /// Internal, unexpected error occured.
    InternalError,
}

/// Describes which entries are next in line for cleanup.
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct CleanupMarker {
    /// Era reward span index that should be checked & cleaned up next.
    #[codec(compact)]
    pub era_reward_index: EraNumber,
    /// dApp tier rewards index that should be checked & cleaned up next.
    #[codec(compact)]
    pub dapp_tiers_index: EraNumber,
}

///////////////////////////////////////////////////////////////////////
////////////   MOVE THIS TO SOME PRIMITIVES CRATE LATER    ////////////
///////////////////////////////////////////////////////////////////////

/// Interface for fetching price of the native token.
///
/// TODO: discussion about below
/// The assumption is that the underlying implementation will ensure
/// this price is averaged and/or weighted over a certain period of time.
/// Alternative is to provide e.g. number of blocks for which an approximately averaged value is needed,
/// and let the underlying implementation take care converting block range into value best represening it.
pub trait PriceProvider {
    /// Get the price of the native token.
    fn average_price() -> FixedU64;
}
