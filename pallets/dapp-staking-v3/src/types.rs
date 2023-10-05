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

// TODO: instead of using `pub` visiblity for fields, either use `pub(crate)` or add dedicated methods for accessing them.

/// Convenience type for `AccountLedger` usage.
pub type AccountLedgerFor<T> = AccountLedger<
    BlockNumberFor<T>,
    <T as Config>::MaxUnlockingChunks,
    <T as Config>::MaxStakingChunks,
>;

/// Era number type
pub type EraNumber = u32;
/// Period number type
pub type PeriodNumber = u32;
/// Dapp Id type
pub type DAppId = u16;

// TODO: perhaps this trait is not needed and instead of having 2 separate '___Chunk' types, we can have just one?
/// Trait for types that can be used as a pair of amount & era.
pub trait AmountEraPair: MaxEncodedLen + Default + Copy {
    /// Balance amount used somehow during the accompanied era.
    fn get_amount(&self) -> Balance;
    /// Era acting as timestamp for the accompanied amount.
    fn get_era(&self) -> EraNumber;
    // Sets the era to the specified value.
    fn set_era(&mut self, era: EraNumber);
    /// Increase the total amount by the specified increase, saturating at the maximum value.
    fn saturating_accrue(&mut self, increase: Balance);
    /// Reduce the total amount by the specified reduction, saturating at the minumum value.
    fn saturating_reduce(&mut self, reduction: Balance);
}

/// Simple enum representing errors possible when using sparse bounded vector.
#[derive(Debug, PartialEq, Eq)]
pub enum AccountLedgerError {
    /// Old era values cannot be added.
    OldEra,
    /// Bounded storage capacity exceeded.
    NoCapacity,
    /// Invalid period specified.
    InvalidPeriod,
    /// Stake amount is to large in respect to what's available.
    UnavailableStakeFunds,
}

/// Helper struct for easier manipulation of sparse <amount, era> pairs.
///
/// The struct guarantees the following:
/// -----------------------------------
/// 1. The vector is always sorted by era, in ascending order.
/// 2. There are no two consecutive zero chunks.
/// 3. There are no two chunks with the same era.
/// 4. The vector is always bounded by the specified maximum length.
///
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo)]
#[scale_info(skip_type_params(ML))]
pub struct SparseBoundedAmountEraVec<P: AmountEraPair, ML: Get<u32>>(pub BoundedVec<P, ML>);

impl<P, ML> SparseBoundedAmountEraVec<P, ML>
where
    P: AmountEraPair,
    ML: Get<u32>,
{
    /// Create new instance
    pub fn new() -> Self {
        Self(BoundedVec::<P, ML>::default())
    }

    /// Places the specified <amount, era> pair into the vector, in an appropriate place.
    ///
    /// There are two possible successful scenarios:
    /// 1. If entry for the specified era already exists, it's updated.
    ///    [(100, 1)] -- add_amount(50, 1) --> [(150, 1)]
    ///
    /// 2. If entry for the specified era doesn't exist, it's created and insertion is attempted.
    ///    [(100, 1)] -- add_amount(50, 2) --> [(100, 1), (150, 2)]
    ///
    /// In case vector has no more capacity, error is returned, and whole operation is a noop.
    pub fn add_amount(
        &mut self,
        amount: Balance,
        era: EraNumber,
    ) -> Result<(), AccountLedgerError> {
        if amount.is_zero() {
            return Ok(());
        }

        let mut chunk = if let Some(&chunk) = self.0.last() {
            ensure!(chunk.get_era() <= era, AccountLedgerError::OldEra);
            chunk
        } else {
            P::default()
        };

        chunk.saturating_accrue(amount);

        if chunk.get_era() == era && !self.0.is_empty() {
            if let Some(last) = self.0.last_mut() {
                *last = chunk;
            }
        } else {
            chunk.set_era(era);
            self.0
                .try_push(chunk)
                .map_err(|_| AccountLedgerError::NoCapacity)?;
        }

        Ok(())
    }

    /// Subtracts the specified amount of the total locked amount, if possible.
    ///
    /// There are multiple success scenarios/rules:
    /// 1. If entry for the specified era already exists, it's updated.
    ///    a. [(100, 1)] -- subtract_amount(50, 1) --> [(50, 1)]
    ///    b. [(100, 1)] -- subtract_amount(100, 1) --> []
    ///
    /// 2. All entries following the specified era will have their amount reduced as well.
    ///    [(100, 1), (150, 2)] -- subtract_amount(50, 1) --> [(50, 1), (100, 2)]
    ///
    /// 3. If entry for the specified era doesn't exist, it's created and insertion is attempted.
    ///    [(100, 1), (200, 3)] -- subtract_amount(100, 2) --> [(100, 1), (0, 2), (100, 3)]
    ///
    /// 4. No two consecutive zero chunks are allowed.
    ///   [(100, 1), (0, 2), (100, 3), (200, 4)] -- subtract_amount(100, 3) --> [(100, 1), (0, 2), (100, 4)]
    ///
    /// In case vector has no more capacity, error is returned, and whole operation is a noop.
    pub fn subtract_amount(
        &mut self,
        amount: Balance,
        era: EraNumber,
    ) -> Result<(), AccountLedgerError> {
        if amount.is_zero() || self.0.is_empty() {
            return Ok(());
        }
        // TODO: this method can surely be optimized (avoid too many iters) but focus on that later,
        // when it's all working fine, and we have good test coverage.
        // TODO2: realistically, the only eligible eras are the last two ones (current & previous). Code could be optimized for that.

        // Find the most relevant locked chunk for the specified era
        let index = if let Some(index) = self.0.iter().rposition(|&chunk| chunk.get_era() <= era) {
            index
        } else {
            // Covers scenario when there's only 1 chunk for the next era, and remove it if it's zero.
            self.0
                .iter_mut()
                .for_each(|chunk| chunk.saturating_reduce(amount));
            self.0.retain(|chunk| !chunk.get_amount().is_zero());
            return Ok(());
        };

        // Update existing or insert a new chunk
        let mut inner = self.0.clone().into_inner();
        let relevant_chunk_index = if inner[index].get_era() == era {
            inner[index].saturating_reduce(amount);
            index
        } else {
            // Take the most relevant chunk for the desired era,
            // and use it as 'base' for the new chunk.
            let mut chunk = inner[index];
            chunk.saturating_reduce(amount);
            chunk.set_era(era);

            // Insert the new chunk AFTER the previous 'most relevant chunk'.
            // The chunk we find is always either for the requested era, or some era before it.
            inner.insert(index + 1, chunk);
            index + 1
        };

        // Update all chunks after the relevant one, and remove eligible zero chunks
        inner[relevant_chunk_index + 1..]
            .iter_mut()
            .for_each(|chunk| chunk.saturating_reduce(amount));

        // Prune all consecutive zero chunks
        let mut new_inner = Vec::<P>::new();
        new_inner.push(inner[0]);
        for i in 1..inner.len() {
            if inner[i].get_amount().is_zero() && inner[i - 1].get_amount().is_zero() {
                continue;
            } else {
                new_inner.push(inner[i]);
            }
        }

        inner = new_inner;

        // Cleanup if only one zero chunk exists
        if inner.len() == 1 && inner[0].get_amount().is_zero() {
            inner.pop();
        }

        // Update `locked` to the new vector
        self.0 = BoundedVec::try_from(inner).map_err(|_| AccountLedgerError::NoCapacity)?;

        Ok(())
    }
}

/// Distinct period types in dApp staking protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum PeriodType {
    /// Period during which the focus is on voting.
    Voting,
    /// Period during which dApps and stakers earn rewards.
    BuildAndEarn,
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
    pub fn new(number: PeriodNumber, period_type: PeriodType, ending_era: EraNumber) -> Self {
        Self {
            number,
            period_type,
            ending_era,
        }
    }
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

/// dApp state in which some dApp is in.
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

/// How much was locked in a specific era
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct LockedChunk {
    #[codec(compact)]
    pub amount: Balance,
    #[codec(compact)]
    pub era: EraNumber,
}

impl Default for LockedChunk {
    fn default() -> Self {
        Self {
            amount: Balance::zero(),
            era: EraNumber::zero(),
        }
    }
}

impl AmountEraPair for LockedChunk {
    fn get_amount(&self) -> Balance {
        self.amount
    }
    fn get_era(&self) -> EraNumber {
        self.era
    }
    fn set_era(&mut self, era: EraNumber) {
        self.era = era;
    }
    fn saturating_accrue(&mut self, increase: Balance) {
        self.amount.saturating_accrue(increase);
    }
    fn saturating_reduce(&mut self, reduction: Balance) {
        self.amount.saturating_reduce(reduction);
    }
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

/// Information about how much was staked in a specific era.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct StakeChunk {
    #[codec(compact)]
    pub amount: Balance,
    #[codec(compact)]
    pub era: EraNumber,
}

impl Default for StakeChunk {
    fn default() -> Self {
        Self {
            amount: Balance::zero(),
            era: EraNumber::zero(),
        }
    }
}

impl AmountEraPair for StakeChunk {
    fn get_amount(&self) -> Balance {
        self.amount
    }
    fn get_era(&self) -> EraNumber {
        self.era
    }
    fn set_era(&mut self, era: EraNumber) {
        self.era = era;
    }
    fn saturating_accrue(&mut self, increase: Balance) {
        self.amount.saturating_accrue(increase);
    }
    fn saturating_reduce(&mut self, reduction: Balance) {
        self.amount.saturating_reduce(reduction);
    }
}

/// General info about user's stakes
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo)]
#[scale_info(skip_type_params(UnlockingLen, StakedLen))]
pub struct AccountLedger<
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    UnlockingLen: Get<u32>,
    StakedLen: Get<u32>,
> {
    /// How much active locked amount an account has.
    pub locked: Balance,
    /// How much started unlocking on a certain block
    pub unlocking: BoundedVec<UnlockingChunk<BlockNumber>, UnlockingLen>,
    /// How much user had staked in some period
    pub staked: SparseBoundedAmountEraVec<StakeChunk, StakedLen>,
    /// Last period in which account had staked.
    pub staked_period: Option<PeriodNumber>,
}

impl<BlockNumber, UnlockingLen, StakedLen> Default
    for AccountLedger<BlockNumber, UnlockingLen, StakedLen>
where
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    UnlockingLen: Get<u32>,
    StakedLen: Get<u32>,
{
    fn default() -> Self {
        Self {
            locked: Balance::zero(),
            unlocking: BoundedVec::<UnlockingChunk<BlockNumber>, UnlockingLen>::default(),
            staked: SparseBoundedAmountEraVec(BoundedVec::<StakeChunk, StakedLen>::default()),
            staked_period: None,
        }
    }
}

impl<BlockNumber, UnlockingLen, StakedLen> AccountLedger<BlockNumber, UnlockingLen, StakedLen>
where
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    UnlockingLen: Get<u32>,
    StakedLen: Get<u32>,
{
    /// Empty if no locked/unlocking/staked info exists.
    pub fn is_empty(&self) -> bool {
        self.locked.is_zero() && self.unlocking.is_empty() && self.staked.0.is_empty()
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

    /// Active staked balance.
    ///
    /// In case latest stored information is from the past period, active stake is considered to be zero.
    pub fn active_stake(&self, active_period: PeriodNumber) -> Balance {
        match self.staked_period {
            Some(last_staked_period) if last_staked_period == active_period => self
                .staked
                .0
                .last()
                .map_or(Balance::zero(), |chunk| chunk.amount),
            _ => Balance::zero(),
        }
    }

    /// Amount that is available for staking.
    ///
    /// This is equal to the total active locked amount, minus the staked amount already active.
    pub fn stakeable_amount(&self, active_period: PeriodNumber) -> Balance {
        self.active_locked_amount()
            .saturating_sub(self.active_stake(active_period))
    }

    /// Amount that is staked, in respect to currently active period.
    pub fn staked_amount(&self, active_period: PeriodNumber) -> Balance {
        match self.staked_period {
            Some(last_staked_period) if last_staked_period == active_period => self
                .staked
                .0
                .last()
                // We should never fallback to the default value since that would mean ledger is in invalid state.
                // TODO: perhaps this can be implemented in a better way to have some error handling? Returning 0 might not be the most secure way to handle it.
                .map_or(Balance::zero(), |chunk| chunk.amount),
            _ => Balance::zero(),
        }
    }

    /// Adds the specified amount to total staked amount, if possible.
    ///
    /// Staking is allowed only allowed if one of the two following conditions is met:
    /// 1. Staker is staking again in the period in which they already staked.
    /// 2. Staker is staking for the first time in this period, and there are no staking chunks from the previous eras.
    ///
    /// Additonally, the staked amount must not exceed what's available for staking.
    pub fn add_stake_amount(
        &mut self,
        amount: Balance,
        era: EraNumber,
        current_period: PeriodNumber,
    ) -> Result<(), AccountLedgerError> {
        if amount.is_zero() {
            return Ok(());
        }

        match self.staked_period {
            Some(last_staked_period) if last_staked_period != current_period => {
                return Err(AccountLedgerError::InvalidPeriod);
            }
            _ => (),
        }

        if self.stakeable_amount(current_period) < amount {
            return Err(AccountLedgerError::UnavailableStakeFunds);
        }

        self.staked.add_amount(amount, era)?;
        self.staked_period = Some(current_period);

        Ok(())
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
            .saturating_sub(self.active_stake(current_period))
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
}

/// Rewards pool for stakers & dApps
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct RewardInfo {
    /// Rewards pool for accounts which have locked funds in dApp staking
    #[codec(compact)]
    pub participants: Balance,
    /// Reward pool for dApps
    #[codec(compact)]
    pub dapps: Balance,
}

#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct StakeAmount {
    /// Amount of staked funds accounting for the voting period.
    #[codec(compact)]
    voting: Balance,
    /// Amount of staked funds accounting for the build&earn period.
    #[codec(compact)]
    build_and_earn: Balance,
}

impl StakeAmount {
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
    pub fn stake(&mut self, amount: Balance, period_type: PeriodType) {
        match period_type {
            PeriodType::Voting => self.voting.saturating_accrue(amount),
            PeriodType::BuildAndEarn => self.build_and_earn.saturating_accrue(amount),
        }
    }
}

/// Info about current era, including the rewards, how much is locked, unlocking, etc.
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct EraInfo {
    /// Info about era rewards
    pub rewards: RewardInfo,
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
        self.next_stake_amount.stake(amount, period_type);
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
}

/// Information about how much a particular staker staked on a particular smart contract.
///
/// Keeps track of amount staked in the 'voting period', as well as 'build&earn period'.
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct SingularStakingInfo {
    /// Total amount staked during the voting period.
    #[codec(compact)]
    vp_staked_amount: Balance,
    /// Total amount staked during the build&earn period.
    #[codec(compact)]
    bep_staked_amount: Balance,
    /// Period number for which this entry is relevant.
    #[codec(compact)]
    // TODO: rename to period_number?
    period: PeriodNumber,
    /// Indicates whether a staker is a loyal staker or not.
    loyal_staker: bool,
    /// Indicates whether staker claimed rewards
    // TODO: isn't this redundant?!
    reward_claimed: bool,
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
            vp_staked_amount: Balance::zero(),
            bep_staked_amount: Balance::zero(),
            period,
            // Loyalty staking is only possible if stake is first made during the voting period.
            loyal_staker: period_type == PeriodType::Voting,
            reward_claimed: false,
        }
    }

    /// Stake the specified amount on the contract, for the specified period type.
    pub fn stake(&mut self, amount: Balance, period_type: PeriodType) {
        match period_type {
            PeriodType::Voting => self.vp_staked_amount.saturating_accrue(amount),
            PeriodType::BuildAndEarn => self.bep_staked_amount.saturating_accrue(amount),
        }
    }

    /// Unstakes some of the specified amount from the contract.
    ///
    /// In case the `amount` being unstaked is larger than the amount staked in the `voting period`,
    /// and `voting period` has passed, this will remove the _loyalty_ flag from the staker.
    ///
    /// Returns the amount that was unstaked from the `voting period` stake, and from the `build&earn period` stake.
    pub fn unstake(&mut self, amount: Balance, period_type: PeriodType) -> (Balance, Balance) {
        // If B&E period stake can cover the unstaking amount, just reduce it.
        if self.bep_staked_amount >= amount {
            self.bep_staked_amount.saturating_reduce(amount);
            (Balance::zero(), amount)
        } else {
            // In case we have to dip into the voting period stake, make sure B&E period stake is reduced first.
            // Also make sure to remove loyalty flag from the staker.
            let bep_amount_snapshot = self.bep_staked_amount;
            let leftover_amount = amount.saturating_sub(self.bep_staked_amount);
            self.bep_staked_amount = Balance::zero();

            let vp_staked_amount_snapshot = self.vp_staked_amount;
            self.vp_staked_amount.saturating_reduce(leftover_amount);
            self.bep_staked_amount = Balance::zero();

            // It's ok if staker reduces their stake amount during voting period.
            // Once loyalty flag is removed, it cannot be returned.
            self.loyal_staker = self.loyal_staker && period_type == PeriodType::Voting;

            // Actual amount that was unstaked: (voting period unstake, B&E period unstake)
            (
                vp_staked_amount_snapshot.saturating_sub(self.vp_staked_amount),
                bep_amount_snapshot,
            )
        }
    }

    /// Total staked on the contract by the user. Both period type stakes are included.
    pub fn total_staked_amount(&self) -> Balance {
        self.vp_staked_amount.saturating_add(self.bep_staked_amount)
    }

    /// Returns amount staked in the specified period.
    pub fn staked_amount(&self, period_type: PeriodType) -> Balance {
        match period_type {
            PeriodType::Voting => self.vp_staked_amount,
            PeriodType::BuildAndEarn => self.bep_staked_amount,
        }
    }

    /// If `true` staker has staked during voting period and has never reduced their sta
    pub fn is_loyal(&self) -> bool {
        self.loyal_staker
    }

    /// Period for which this entry is relevant.
    pub fn period_number(&self) -> PeriodNumber {
        self.period
    }
}

/// Information about how much was staked on a contract during a specific era or period.
///
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo)]
pub struct ContractStakingInfo {
    #[codec(compact)]
    vp_staked_amount: Balance,
    #[codec(compact)]
    bep_staked_amount: Balance,
    #[codec(compact)]
    era: EraNumber,
    #[codec(compact)]
    period: PeriodNumber,
}

impl ContractStakingInfo {
    /// Create new instance of `ContractStakingInfo` with specified era & period.
    /// These parameters are immutable.
    ///
    /// Staked amounts are initialized to zero and can be increased or decreased.
    pub fn new(era: EraNumber, period: PeriodNumber) -> Self {
        Self {
            vp_staked_amount: Balance::zero(),
            bep_staked_amount: Balance::zero(),
            era,
            period,
        }
    }

    /// Total staked amount on the contract.
    pub fn total_staked_amount(&self) -> Balance {
        self.vp_staked_amount.saturating_add(self.bep_staked_amount)
    }

    /// Staked amount of the specified period type.
    ///
    /// Note:
    /// It is possible that voting period stake is reduced during the build&earn period.
    /// This is because stakers can unstake their funds during the build&earn period, which can
    /// chip away from the voting period stake.
    pub fn staked_amount(&self, period_type: PeriodType) -> Balance {
        match period_type {
            PeriodType::Voting => self.vp_staked_amount,
            PeriodType::BuildAndEarn => self.bep_staked_amount,
        }
    }

    /// Era for which this entry is relevant.
    pub fn era(&self) -> EraNumber {
        self.era
    }

    /// Period for which this entry is relevant.
    pub fn period(&self) -> PeriodNumber {
        self.period
    }

    /// Stake specified `amount` on the contract, for the specified `period_type`.
    pub fn stake(&mut self, amount: Balance, period_type: PeriodType) {
        match period_type {
            PeriodType::Voting => self.vp_staked_amount.saturating_accrue(amount),
            PeriodType::BuildAndEarn => self.bep_staked_amount.saturating_accrue(amount),
        }
    }

    /// Unstake specified `amount` from the contract, for the specified `period_type`.
    pub fn unstake(&mut self, amount: Balance, period_type: PeriodType) {
        match period_type {
            PeriodType::Voting => self.vp_staked_amount.saturating_reduce(amount),
            PeriodType::BuildAndEarn => self.bep_staked_amount.saturating_reduce(amount),
        }
    }

    /// `true` if no stake exists, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.vp_staked_amount.is_zero() && self.bep_staked_amount.is_zero()
    }
}

const STAKING_SERIES_HISTORY: u32 = 3;

/// Composite type that holds information about how much was staked on a contract during some past eras & periods, including the current era & period.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct ContractStakingInfoSeries(
    BoundedVec<ContractStakingInfo, ConstU32<STAKING_SERIES_HISTORY>>,
);
impl ContractStakingInfoSeries {
    /// Helper
    #[cfg(test)]
    pub fn new(inner: Vec<ContractStakingInfo>) -> Self {
        Self(BoundedVec::try_from(inner).expect("Test should ensure this is always valid"))
    }

    /// Length of the series.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// `true` if series is empty, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the `ContractStakingInfo` type for the specified era & period, if it exists.
    pub fn get(&self, era: EraNumber, period: PeriodNumber) -> Option<ContractStakingInfo> {
        let idx = self.0.binary_search_by(|info| info.era().cmp(&era));

        // There are couple of distinct scenarios:
        // 1. Era exists, so we just return it.
        // 2. Era doesn't exist, and ideal index is zero, meaning there's nothing in history that would cover this era.
        // 3. Era doesn't exist, and ideal index is greater than zero, meaning we can potentially use one of the previous entries to derive the information.
        // 3.1. In case periods are matching, we return that value.
        // 3.2. In case periods aren't matching, we return `None` since stakes don't carry over between periods.
        match idx {
            Ok(idx) => self.0.get(idx).map(|x| *x),
            Err(idx) if idx.is_zero() => None,
            Err(idx) if idx > 0 => {
                let mut info = self.0[idx - 1];
                if info.period() == period {
                    info.era = era;
                    Some(info)
                } else {
                    None
                }
            }
            Err(_) => {
                // TODO: this is unreachable, but compiler doesn't know that
                None
            }
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
        if let Some(last_element) = self.0.last() {
            if last_element.era() > era || last_element.period() > period_info.number {
                return Err(());
            }
        }

        // Get the most relevant `ContractStakingInfo` instance
        let (last_element_has_matching_era, last_element_has_matching_period) =
            if let Some(last_element) = self.0.last() {
                (
                    last_element.era() == era,
                    last_element.period() == period_info.number,
                )
            } else {
                (false, false)
            };

        // Prepare the new entry
        let mut staking_info = if last_element_has_matching_era {
            self.0.remove(self.0.len() - 1)
        } else if last_element_has_matching_period {
            // Periods match so we should 'copy' the last element to get correct staking amount
            let mut temp = self.0[self.0.len() - 1];
            temp.era = era;
            temp
        } else {
            // It's a new period, so we need a completely new instance
            ContractStakingInfo::new(era, period_info.number)
        };

        // Update the stake amount
        staking_info.stake(amount, period_info.period_type);

        // Prune the oldest entry if we have more than the limit
        if self.0.len() > STAKING_SERIES_HISTORY.saturating_sub(1) as usize {
            // TODO: this can be perhaps optimized so we prune entries which are very old.
            // However, this makes the code more complex & more error prone.
            // If kept like this, we always make sure we cover the history, and we never exceed it.
            self.0.remove(0);
        }

        // This should be infalible due to previous checks that ensure we don't end up overflowing the vector.
        self.0.try_push(staking_info).map_err(|_| ())
    }
}
