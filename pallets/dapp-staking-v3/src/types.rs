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

use frame_support::{pallet_prelude::*, BoundedVec};
use frame_system::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::{
    traits::{AtLeast32BitUnsigned, Zero},
    Saturating,
};

use astar_primitives::Balance;

use crate::pallet::Config;

// Convenience type for `AccountLedger` usage.
pub type AccountLedgerFor<T> = AccountLedger<BlockNumberFor<T>, <T as Config>::MaxUnlockingChunks>;

/// Era number type
pub type EraNumber = u32;
/// Period number type
pub type PeriodNumber = u32;
/// Dapp Id type
pub type DAppId = u16;

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
}

// TODO: rename to SubperiodType? It would be less ambigious.
/// Distinct period types in dApp staking protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum PeriodType {
    /// Period during which the focus is on voting.
    Voting,
    /// Period during which dApps and stakers earn rewards.
    BuildAndEarn,
}

impl PeriodType {
    pub fn next(&self) -> Self {
        match self {
            PeriodType::Voting => PeriodType::BuildAndEarn,
            PeriodType::BuildAndEarn => PeriodType::Voting,
        }
    }
}

/// Wrapper type around current `PeriodType` and era number when it's expected to end.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct PeriodInfo {
    #[codec(compact)]
    pub number: PeriodNumber,
    pub period_type: PeriodType,
    #[codec(compact)]
    pub ending_era: EraNumber,
}

impl PeriodInfo {
    /// Create new instance of `PeriodInfo`
    pub fn new(number: PeriodNumber, period_type: PeriodType, ending_era: EraNumber) -> Self {
        Self {
            number,
            period_type,
            ending_era,
        }
    }

    /// `true` if the provided era belongs to the next period, `false` otherwise.
    /// It's only possible to provide this information for the `BuildAndEarn` period type.
    pub fn is_next_period(&self, era: EraNumber) -> bool {
        self.period_type == PeriodType::BuildAndEarn && self.ending_era <= era
    }
}

// TODO: doc
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct PeriodEndInfo {
    #[codec(compact)]
    pub bonus_reward_pool: Balance,
    #[codec(compact)]
    pub total_vp_stake: Balance,
    #[codec(compact)]
    pub final_era: EraNumber,
}

/// Force types to speed up the next era, and even period.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum ForcingTypes {
    /// Force the next era to start.
    NewEra,
    /// Force the current period phase to end, and new one to start
    NewEraAndPeriodPhase,
}

/// General information & state of the dApp staking protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct ProtocolState<BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen> {
    /// Ongoing era number.
    #[codec(compact)]
    pub era: EraNumber,
    /// Block number at which the next era should start.
    /// TODO: instead of abusing on-initialize and wasting block-space,
    /// I believe we should utilize `pallet-scheduler` to schedule the next era. Make an item for this.
    #[codec(compact)]
    pub next_era_start: BlockNumber,
    /// Ongoing period type and when is it expected to end.
    pub period_info: PeriodInfo,
    /// `true` if pallet is in maintenance mode (disabled), `false` otherwise.
    /// TODO: provide some configurable barrier to handle this on the runtime level instead? Make an item for this?
    pub maintenance: bool,
}

impl<BlockNumber> Default for ProtocolState<BlockNumber>
where
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen,
{
    fn default() -> Self {
        Self {
            era: 0,
            next_era_start: BlockNumber::from(1_u32),
            period_info: PeriodInfo {
                number: 0,
                period_type: PeriodType::Voting,
                ending_era: 2,
            },
            maintenance: false,
        }
    }
}

impl<BlockNumber> ProtocolState<BlockNumber>
where
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen,
{
    /// Current period type.
    pub fn period_type(&self) -> PeriodType {
        self.period_info.period_type
    }

    /// Current period number.
    pub fn period_number(&self) -> PeriodNumber {
        self.period_info.number
    }

    /// Ending era of current period
    pub fn ending_era(&self) -> EraNumber {
        self.period_info.ending_era
    }

    /// Checks whether a new era should be triggered, based on the provided `BlockNumber` argument
    /// or possibly other protocol state parameters.
    pub fn is_new_era(&self, now: BlockNumber) -> bool {
        self.next_era_start <= now
    }

    // TODO: rename this into something better?
    /// Triggers the next period type, updating appropriate parameters.
    pub fn next_period_type(&mut self, ending_era: EraNumber, next_era_start: BlockNumber) {
        let period_number = if self.period_type() == PeriodType::BuildAndEarn {
            self.period_number().saturating_add(1)
        } else {
            self.period_number()
        };

        self.period_info = PeriodInfo {
            number: period_number,
            period_type: self.period_type().next(),
            ending_era,
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

/// How much was unlocked in some block.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct UnlockingChunk<BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen + Copy> {
    #[codec(compact)]
    pub amount: Balance,
    #[codec(compact)]
    pub unlock_block: BlockNumber,
}

impl<BlockNumber> Default for UnlockingChunk<BlockNumber>
where
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
{
    fn default() -> Self {
        Self {
            amount: Balance::zero(),
            unlock_block: BlockNumber::zero(),
        }
    }
}

/// General info about user's stakes
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo)]
#[scale_info(skip_type_params(UnlockingLen))]
pub struct AccountLedger<
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    UnlockingLen: Get<u32>,
> {
    /// How much active locked amount an account has.
    pub locked: Balance,
    /// How much started unlocking on a certain block
    pub unlocking: BoundedVec<UnlockingChunk<BlockNumber>, UnlockingLen>,
    /// How much user has/had staked in a particular era.
    pub staked: StakeAmount,
    /// Helper staked amount to keep track of future era stakes.
    /// Both `stake` and `staked_future` must ALWAYS refer to the same period.
    pub staked_future: Option<StakeAmount>,
    /// TODO
    pub staker_rewards_claimed: bool,
    /// TODO
    pub bonus_reward_claimed: bool,
}

impl<BlockNumber, UnlockingLen> Default for AccountLedger<BlockNumber, UnlockingLen>
where
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    UnlockingLen: Get<u32>,
{
    fn default() -> Self {
        Self {
            locked: Balance::zero(),
            unlocking: BoundedVec::<UnlockingChunk<BlockNumber>, UnlockingLen>::default(),
            staked: StakeAmount::default(),
            staked_future: None,
            staker_rewards_claimed: false,
            bonus_reward_claimed: false,
        }
    }
}

impl<BlockNumber, UnlockingLen> AccountLedger<BlockNumber, UnlockingLen>
where
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
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

    pub fn staked_amount_for_type(
        &self,
        period_type: PeriodType,
        active_period: PeriodNumber,
    ) -> Balance {
        // First check the 'future' entry, afterwards check the 'first' entry
        match self.staked_future {
            Some(stake_amount) if stake_amount.period == active_period => {
                stake_amount.for_type(period_type)
            }
            _ => match self.staked {
                stake_amount if stake_amount.period == active_period => {
                    stake_amount.for_type(period_type)
                }
                _ => Balance::zero(),
            },
        }
    }

    // TODO: update this
    /// Adds the specified amount to total staked amount, if possible.
    ///
    /// Staking is only allowed if one of the two following conditions is met:
    /// 1. Staker is staking again in the period in which they already staked.
    /// 2. Staker is staking for the first time in this period, and there are no staking chunks from the previous eras.
    ///
    /// Additonally, the staked amount must not exceed what's available for staking.
    pub fn add_stake_amount(
        &mut self,
        amount: Balance,
        era: EraNumber,
        current_period_info: PeriodInfo,
    ) -> Result<(), AccountLedgerError> {
        if amount.is_zero() {
            return Ok(());
        }

        // TODO: maybe the check can be nicer?
        if !self.staked.is_empty() {
            // In case entry for the current era exists, it must match the era exactly.
            if self.staked.era != era {
                return Err(AccountLedgerError::InvalidEra);
            }
            if self.staked.period != current_period_info.number {
                return Err(AccountLedgerError::InvalidPeriod);
            }
            // In case it doesn't (i.e. first time staking), then the future era must match exactly
            // one era after the one provided via argument.
        } else if let Some(stake_amount) = self.staked_future {
            if stake_amount.era != era + 1 {
                return Err(AccountLedgerError::InvalidEra);
            }
            if stake_amount.period != current_period_info.number {
                return Err(AccountLedgerError::InvalidPeriod);
            }
        }

        if self.stakeable_amount(current_period_info.number) < amount {
            return Err(AccountLedgerError::UnavailableStakeFunds);
        }

        // Update existing entry if it exists, otherwise create it.
        match self.staked_future.as_mut() {
            Some(stake_amount) => {
                stake_amount.add(amount, current_period_info.period_type);
            }
            None => {
                let mut stake_amount = self.staked;
                stake_amount.era = era + 1;
                stake_amount.period = current_period_info.number;
                stake_amount.add(amount, current_period_info.period_type);
                self.staked_future = Some(stake_amount);
            }
        }

        Ok(())
    }

    /// Subtracts the specified amount from the total staked amount, if possible.
    ///
    /// Unstaking will reduce total stake for the current era, and next era(s).
    /// The specified amount must not exceed what's available for staking.
    pub fn unstake_amount(
        &mut self,
        amount: Balance,
        era: EraNumber,
        current_period_info: PeriodInfo,
    ) -> Result<(), AccountLedgerError> {
        if amount.is_zero() {
            return Ok(());
        }

        // TODO: maybe the check can be nicer? (and not duplicated?)
        if !self.staked.is_empty() {
            // In case entry for the current era exists, it must match the era exactly.
            if self.staked.era != era {
                return Err(AccountLedgerError::InvalidEra);
            }
            if self.staked.period != current_period_info.number {
                return Err(AccountLedgerError::InvalidPeriod);
            }
            // In case it doesn't (i.e. first time staking), then the future era must match exactly
            // one era after the one provided via argument.
        } else if let Some(stake_amount) = self.staked_future {
            if stake_amount.era != era + 1 {
                return Err(AccountLedgerError::InvalidEra);
            }
            if stake_amount.period != current_period_info.number {
                return Err(AccountLedgerError::InvalidPeriod);
            }
        }

        // User must be precise with their unstake amount.
        if self.staked_amount(current_period_info.number) < amount {
            return Err(AccountLedgerError::UnstakeAmountLargerThanStake);
        }

        self.staked
            .subtract(amount, current_period_info.period_type);
        // Convenience cleanup
        if self.staked.is_empty() {
            self.staked = Default::default();
        }
        if let Some(mut stake_amount) = self.staked_future {
            stake_amount.subtract(amount, current_period_info.period_type);

            self.staked_future = if stake_amount.is_empty() {
                None
            } else {
                Some(stake_amount)
            };
        }

        Ok(())
    }

    /// Claim up stake chunks up to the specified `era`.
    /// Returns the vector describing claimable chunks.
    ///
    /// If `period_end` is provided, it's used to determine whether all applicable chunks have been claimed.
    pub fn claim_up_to_era(
        &mut self,
        era: EraNumber,
        period_end: Option<EraNumber>,
    ) -> Result<(EraNumber, EraNumber, Balance), AccountLedgerError> {
        // TODO: the check also needs to ensure that future entry is covered!!!
        // TODO2: the return type won't work since we can have 2 distinct values - one from staked, one from staked_future
        if era <= self.staked.era || self.staked.total().is_zero() {
            return Err(AccountLedgerError::NothingToClaim);
        }

        let result = (self.staked.era, era, self.staked.total());

        // Update latest 'staked' era
        self.staked.era = era;

        // Make sure to clean
        match period_end {
            Some(ending_era) if era >= ending_era => {
                self.staker_rewards_claimed = true;
                self.staked = Default::default();
                self.staked_future = None;
            }
            _ => (),
        }

        Ok(result)
    }
}

// TODO
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
    /// Create new instance of `StakeAmount` with specified `voting` and `build_and_earn` amounts.
    pub fn new(
        voting: Balance,
        build_and_earn: Balance,
        era: EraNumber,
        period: PeriodNumber,
    ) -> Self {
        Self {
            voting,
            build_and_earn,
            era,
            period,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.voting.is_zero() && self.build_and_earn.is_zero()
    }

    /// Total amount staked in both period types.
    pub fn total(&self) -> Balance {
        self.voting.saturating_add(self.build_and_earn)
    }

    /// Amount staked for the specified period type.
    pub fn for_type(&self, period_type: PeriodType) -> Balance {
        match period_type {
            PeriodType::Voting => self.voting,
            PeriodType::BuildAndEarn => self.build_and_earn,
        }
    }

    /// Stake the specified `amount` for the specified `period_type`.
    pub fn add(&mut self, amount: Balance, period_type: PeriodType) {
        match period_type {
            PeriodType::Voting => self.voting.saturating_accrue(amount),
            PeriodType::BuildAndEarn => self.build_and_earn.saturating_accrue(amount),
        }
    }

    /// Unstake the specified `amount` for the specified `period_type`.
    ///
    /// In case period type is `Voting`, the amount is subtracted from the voting period.
    ///
    /// In case period type is `Build&Earn`, the amount is first subtracted from the
    /// build&earn amount, and any rollover is subtracted from the voting period.
    pub fn subtract(&mut self, amount: Balance, period_type: PeriodType) {
        match period_type {
            PeriodType::Voting => self.voting.saturating_reduce(amount),
            PeriodType::BuildAndEarn => {
                if self.build_and_earn >= amount {
                    self.build_and_earn.saturating_reduce(amount);
                } else {
                    // Rollover from build&earn to voting, is guaranteed to be larger than zero due to previous check
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
    /// How much balance is considered to be locked in the current era.
    /// This value influences the reward distribution.
    #[codec(compact)]
    pub active_era_locked: Balance,
    /// How much balance is locked in dApp staking, in total.
    /// For rewards, this amount isn't relevant for the current era, but only from the next one.
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
        self.active_era_locked.saturating_reduce(amount);
        self.total_locked.saturating_reduce(amount);
        self.unlocking.saturating_accrue(amount);
    }

    /// Update with the new amount that has been removed from unlocking.
    pub fn unlocking_removed(&mut self, amount: Balance) {
        self.unlocking.saturating_reduce(amount);
    }

    /// Add the specified `amount` to the appropriate stake amount, based on the `PeriodType`.
    pub fn add_stake_amount(&mut self, amount: Balance, period_type: PeriodType) {
        self.next_stake_amount.add(amount, period_type);
    }

    /// Subtract the specified `amount` from the appropriate stake amount, based on the `PeriodType`.
    pub fn unstake_amount(&mut self, amount: Balance, period_type: PeriodType) {
        self.current_stake_amount.subtract(amount, period_type);
        self.next_stake_amount.subtract(amount, period_type);
    }

    /// Total staked amount in this era.
    pub fn total_staked_amount(&self) -> Balance {
        self.current_stake_amount.total()
    }

    /// Staked amount of specified `type` in this era.
    pub fn staked_amount(&self, period_type: PeriodType) -> Balance {
        self.current_stake_amount.for_type(period_type)
    }

    /// Total staked amount in the next era.
    pub fn total_staked_amount_next_era(&self) -> Balance {
        self.next_stake_amount.total()
    }

    /// Staked amount of specifeid `type` in the next era.
    pub fn staked_amount_next_era(&self, period_type: PeriodType) -> Balance {
        self.next_stake_amount.for_type(period_type)
    }

    /// Updates `Self` to reflect the transition to the next era.
    ///
    ///  ## Args
    /// `next_period_type` - `None` if no period type change, `Some(type)` if `type` is starting from the next era.
    pub fn migrate_to_next_era(&mut self, next_period_type: Option<PeriodType>) {
        self.active_era_locked = self.total_locked;
        match next_period_type {
            // If next era marks start of new voting period period, it means we're entering a new period
            Some(PeriodType::Voting) => {
                self.current_stake_amount = Default::default();
                self.next_stake_amount = Default::default();
            }
            Some(PeriodType::BuildAndEarn) | None => {
                self.current_stake_amount = self.next_stake_amount;
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
    /// `period_type` - period type during which this entry is created.
    pub fn new(period: PeriodNumber, period_type: PeriodType) -> Self {
        Self {
            // TODO: one drawback here is using the struct which has `era` as the field - it's not needed here. Should I add a special struct just for this?
            staked: StakeAmount::new(Balance::zero(), Balance::zero(), 0, period),
            // Loyalty staking is only possible if stake is first made during the voting period.
            loyal_staker: period_type == PeriodType::Voting,
        }
    }

    /// Stake the specified amount on the contract, for the specified period type.
    pub fn stake(&mut self, amount: Balance, period_type: PeriodType) {
        self.staked.add(amount, period_type);
    }

    /// Unstakes some of the specified amount from the contract.
    ///
    /// In case the `amount` being unstaked is larger than the amount staked in the `voting period`,
    /// and `voting period` has passed, this will remove the _loyalty_ flag from the staker.
    ///
    /// Returns the amount that was unstaked from the `voting period` stake, and from the `build&earn period` stake.
    pub fn unstake(&mut self, amount: Balance, period_type: PeriodType) -> (Balance, Balance) {
        let snapshot = self.staked;

        self.staked.subtract(amount, period_type);

        self.loyal_staker = self.loyal_staker
            && (period_type == PeriodType::Voting
                || period_type == PeriodType::BuildAndEarn
                    && self.staked.voting == snapshot.voting);

        // Amount that was unstaked
        (
            snapshot.voting.saturating_sub(self.staked.voting),
            snapshot
                .build_and_earn
                .saturating_sub(self.staked.build_and_earn),
        )
    }

    /// Total staked on the contract by the user. Both period type stakes are included.
    pub fn total_staked_amount(&self) -> Balance {
        self.staked.total()
    }

    /// Returns amount staked in the specified period.
    pub fn staked_amount(&self, period_type: PeriodType) -> Balance {
        self.staked.for_type(period_type)
    }

    /// If `true` staker has staked during voting period and has never reduced their sta
    pub fn is_loyal(&self) -> bool {
        self.loyal_staker
    }

    /// Period for which this entry is relevant.
    pub fn period_number(&self) -> PeriodNumber {
        self.staked.period
    }

    /// `true` if no stake exists, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.staked.is_empty()
    }
}

const STAKING_SERIES_HISTORY: u32 = 3;

/// Composite type that holds information about how much was staked on a contract during some past eras & periods, including the current era & period.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct ContractStakeAmountSeries(BoundedVec<StakeAmount, ConstU32<STAKING_SERIES_HISTORY>>);
impl ContractStakeAmountSeries {
    /// Helper function to create a new instance of `ContractStakeAmountSeries`.
    #[cfg(test)]
    pub fn new(inner: Vec<StakeAmount>) -> Self {
        Self(BoundedVec::try_from(inner).expect("Test should ensure this is always valid"))
    }

    /// Returns inner `Vec` of `StakeAmount` instances. Useful for testing.
    #[cfg(test)]
    pub fn inner(&self) -> Vec<StakeAmount> {
        self.0.clone().into_inner()
    }

    /// Length of the series.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// `true` if series is empty, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the `StakeAmount` type for the specified era & period, if it exists.
    pub fn get(&self, era: EraNumber, period: PeriodNumber) -> Option<StakeAmount> {
        let idx = self
            .0
            .binary_search_by(|stake_amount| stake_amount.era.cmp(&era));

        // There are couple of distinct scenarios:
        // 1. Era exists, so we just return it.
        // 2. Era doesn't exist, and ideal index is zero, meaning there's nothing in history that would cover this era.
        // 3. Era doesn't exist, and ideal index is greater than zero, meaning we can potentially use one of the previous entries to derive the information.
        // 3.1. In case periods are matching, we return that value.
        // 3.2. In case periods aren't matching, we return `None` since stakes don't carry over between periods.
        match idx {
            Ok(idx) => self.0.get(idx).map(|x| *x),
            Err(ideal_idx) => {
                if ideal_idx.is_zero() {
                    None
                } else {
                    match self.0.get(ideal_idx - 1) {
                        Some(info) if info.period == period => {
                            let mut info = *info;
                            info.era = era;
                            Some(info)
                        }
                        _ => None,
                    }
                }
            }
        }
    }

    /// Total staked amount on the contract, in the active period.
    pub fn total_staked_amount(&self, active_period: PeriodNumber) -> Balance {
        match self.0.last() {
            Some(stake_amount) if stake_amount.period == active_period => stake_amount.total(),
            _ => Balance::zero(),
        }
    }

    /// Staked amount on the contract, for specified period type, in the active period.
    pub fn staked_amount(&self, period: PeriodNumber, period_type: PeriodType) -> Balance {
        match self.0.last() {
            Some(stake_amount) if stake_amount.period == period => {
                stake_amount.for_type(period_type)
            }
            _ => Balance::zero(),
        }
    }

    /// Stake the specified `amount` on the contract, for the specified `period_type` and `era`.
    pub fn stake(
        &mut self,
        amount: Balance,
        period_info: PeriodInfo,
        era: EraNumber,
    ) -> Result<(), ()> {
        // Defensive check to ensure we don't end up in a corrupted state. Should never happen.
        if let Some(stake_amount) = self.0.last() {
            if stake_amount.era > era || stake_amount.period > period_info.number {
                return Err(());
            }
        }

        // Get the most relevant `StakeAmount` instance
        let mut stake_amount = if let Some(stake_amount) = self.0.last() {
            if stake_amount.era == era {
                // Era matches, so we just update the last element.
                let stake_amount = *stake_amount;
                let _ = self.0.pop();
                stake_amount
            } else if stake_amount.period == period_info.number {
                // Periods match so we should 'copy' the last element to get correct staking amount
                let mut temp = *stake_amount;
                temp.era = era;
                temp
            } else {
                // It's a new period, so we need a completely new instance
                StakeAmount::new(Balance::zero(), Balance::zero(), era, period_info.number)
            }
        } else {
            // It's a new period, so we need a completely new instance
            StakeAmount::new(Balance::zero(), Balance::zero(), era, period_info.number)
        };

        // Update the stake amount
        stake_amount.add(amount, period_info.period_type);

        // This should be infalible due to previous checks that ensure we don't end up overflowing the vector.
        self.prune();
        self.0.try_push(stake_amount).map_err(|_| ())
    }

    /// Unstake the specified `amount` from the contract, for the specified `period_type` and `era`.
    pub fn unstake(
        &mut self,
        amount: Balance,
        period_info: PeriodInfo,
        era: EraNumber,
    ) -> Result<(), ()> {
        // TODO: look into refactoring/optimizing this - right now it's a bit complex.

        // Defensive check to ensure we don't end up in a corrupted state. Should never happen.
        if let Some(stake_amount) = self.0.last() {
            // It's possible last element refers to the upcoming era, hence the "-1" on the 'era'.
            if stake_amount.era.saturating_sub(1) > era || stake_amount.period > period_info.number
            {
                return Err(());
            }
        } else {
            // Vector is empty, should never happen.
            return Err(());
        }

        // 1st step - remove the last element IFF it's for the next era.
        // Unstake the requested amount from it.
        let last_era_info = match self.0.last() {
            Some(stake_amount) if stake_amount.era == era.saturating_add(1) => {
                let mut stake_amount = *stake_amount;
                stake_amount.subtract(amount, period_info.period_type);
                let _ = self.0.pop();
                Some(stake_amount)
            }
            _ => None,
        };

        // 2nd step - 3 options:
        // 1. - last element has a matching era so we just update it.
        // 2. - last element has a past era and matching period, so we'll create a new entry based on it.
        // 3. - last element has a past era and past period, meaning it's invalid.
        let second_last_era_info = if let Some(stake_amount) = self.0.last_mut() {
            if stake_amount.era == era {
                stake_amount.subtract(amount, period_info.period_type);
                None
            } else if stake_amount.period == period_info.number {
                let mut new_entry = *stake_amount;
                new_entry.subtract(amount, period_info.period_type);
                new_entry.era = era;
                Some(new_entry)
            } else {
                None
            }
        } else {
            None
        };

        // 3rd step - push the new entries, if they exist.
        if let Some(info) = second_last_era_info {
            self.prune();
            self.0.try_push(info).map_err(|_| ())?;
        }
        if let Some(info) = last_era_info {
            self.prune();
            self.0.try_push(info).map_err(|_| ())?;
        }

        Ok(())
    }

    /// Used to remove past entries, in case vector is full.
    fn prune(&mut self) {
        // Prune the oldest entry if we have more than the limit
        if self.0.len() == STAKING_SERIES_HISTORY as usize {
            // TODO: this can be perhaps optimized so we prune entries which are very old.
            // However, this makes the code more complex & more error prone.
            // If kept like this, we always make sure we cover the history, and we never exceed it.
            self.0.remove(0);
        }
    }
}

/// Information required for staker reward payout for a particular era.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct EraReward {
    /// Total reward pool for staker rewards
    #[codec(compact)]
    staker_reward_pool: Balance,
    /// Total amount which was staked at the end of an era
    #[codec(compact)]
    staked: Balance,
}

impl EraReward {
    /// Create new instance of `EraReward` with specified `staker_reward_pool` and `staked` amounts.
    pub fn new(staker_reward_pool: Balance, staked: Balance) -> Self {
        Self {
            staker_reward_pool,
            staked,
        }
    }

    /// Total reward pool for staker rewards.
    pub fn staker_reward_pool(&self) -> Balance {
        self.staker_reward_pool
    }

    /// Total amount which was staked at the end of an era.
    pub fn staked(&self) -> Balance {
        self.staked
    }
}

#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum EraRewardSpanError {
    /// Provided era is invalid. Must be exactly one era after the last one in the span.
    InvalidEra,
    /// Span has no more capacity for additional entries.
    NoCapacity,
}

/// Used to efficiently store era span information.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo)]
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
