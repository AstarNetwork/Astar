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
    <T as Config>::MaxLockedChunks,
    <T as Config>::MaxUnlockingChunks,
    <T as Config>::MaxStakingChunks,
>;

/// Era number type
pub type EraNumber = u32;
/// Period number type
pub type PeriodNumber = u32;
/// Dapp Id type
pub type DAppId = u16;

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

/// Helper struct for easier manipulation of sparse <amount, era> pairs.
///
/// The struct guarantes the following:
/// -----------------------------------
/// 1. The vector is always sorted by era, in ascending order.
/// 2. There are no two consecutive zero chunks.
/// 3. There are no two chunks with the same era.
/// 4. The vector is always bounded by the specified maximum length.
///
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo)]
#[scale_info(skip_type_params(ML))]
// TODO: should I use `EncodeLike`?
pub struct SparseBoundedAmountEraVec<P: AmountEraPair, ML: Get<u32>>(pub BoundedVec<P, ML>);

impl<P, ML> SparseBoundedAmountEraVec<P, ML>
where
    P: AmountEraPair,
    ML: Get<u32>,
{
    // TODO: maybe add a custom error type?
    // Could be useful to know what exactly went wrong.

    // TODO2: write (or reuse) custom tests for this implementation.

    /// Places the specified <amount, era> pair into the vector, in an appropriate place.
    ///
    /// If entry for the specified era already exists, it's updated.
    ///
    /// If entry for the specified era doesn't exist, it's created and insertion is attempted.
    ///
    /// In case vector has no more capacity, error is returned, and whole operation is a noop.
    pub fn add_amount(&mut self, amount: Balance, era: EraNumber) -> Result<(), ()> {
        if amount.is_zero() {
            return Ok(());
        }

        let mut chunk = if let Some(&chunk) = self.0.last() {
            ensure!(chunk.get_era() <= era, ());
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
            self.0.try_push(chunk).map_err(|_| ())?;
        }

        Ok(())
    }

    /// Subtracts the specified amount of the total locked amount, if possible.
    ///
    /// If entry for the specified era already exists, it's updated.
    ///
    /// If entry for the specified era doesn't exist, it's created and insertion is attempted.
    /// In case vector has no more capacity, error is returned, and whole operation is a noop.
    pub fn subtract_amount(&mut self, amount: Balance, era: EraNumber) -> Result<(), ()> {
        if amount.is_zero() || self.0.is_empty() {
            return Ok(());
        }
        // TODO: this method can surely be optimized (avoid too many iters) but focus on that later,
        // when it's all working fine, and we have good test coverage.

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
            let mut chunk = inner[index];
            chunk.saturating_reduce(amount);
            chunk.set_era(era);

            inner.insert(index + 1, chunk);
            index + 1
        };

        // Update all chunks after the relevant one, and remove eligible zero chunks
        inner[relevant_chunk_index + 1..]
            .iter_mut()
            .for_each(|chunk| chunk.saturating_reduce(amount));

        // Merge all consecutive zero chunks
        let mut i = relevant_chunk_index;
        while i < inner.len() - 1 {
            if inner[i].get_amount().is_zero() && inner[i + 1].get_amount().is_zero() {
                inner.remove(i + 1);
            } else {
                i += 1;
            }
        }

        // Cleanup if only one zero chunk exists
        if inner.len() == 1 && inner[0].get_amount().is_zero() {
            inner.pop();
        }

        // Update `locked` to the new vector
        self.0 = BoundedVec::try_from(inner).map_err(|_| ())?;

        Ok(())
    }
}

/// Distinct period types in dApp staking protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum PeriodType {
    /// Period during which the focus is on voting.
    /// Inner value is the era in which the voting period ends.
    Voting(#[codec(compact)] EraNumber),
    /// Period during which dApps and stakers earn rewards.
    /// Inner value is the era in which the Build&Eearn period ends.
    BuildAndEarn(#[codec(compact)] EraNumber),
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
    /// Ongoing period number.
    #[codec(compact)]
    pub period: PeriodNumber,
    /// Ongoing period type and when is it expected to end.
    pub period_type: PeriodType,
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
            period: 0,
            period_type: PeriodType::Voting(0),
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
#[scale_info(skip_type_params(LockedLen, UnlockingLen, StakedLen))]
pub struct AccountLedger<
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    LockedLen: Get<u32>,
    UnlockingLen: Get<u32>,
    StakedLen: Get<u32>,
> {
    /// How much was staked in each era
    pub locked: SparseBoundedAmountEraVec<LockedChunk, LockedLen>,
    /// How much started unlocking on a certain block
    pub unlocking: BoundedVec<UnlockingChunk<BlockNumber>, UnlockingLen>,
    /// How much user had staked in some period
    pub staked: SparseBoundedAmountEraVec<StakeChunk, StakedLen>,
    /// Last period in which account had staked.
    pub staked_period: Option<PeriodNumber>,
}

impl<BlockNumber, LockedLen, UnlockingLen, StakedLen> Default
    for AccountLedger<BlockNumber, LockedLen, UnlockingLen, StakedLen>
where
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    LockedLen: Get<u32>,
    UnlockingLen: Get<u32>,
    StakedLen: Get<u32>,
{
    fn default() -> Self {
        Self {
            locked: SparseBoundedAmountEraVec(BoundedVec::<LockedChunk, LockedLen>::default()),
            unlocking: BoundedVec::<UnlockingChunk<BlockNumber>, UnlockingLen>::default(),
            staked: SparseBoundedAmountEraVec(BoundedVec::<StakeChunk, StakedLen>::default()),
            staked_period: None,
        }
    }
}

impl<BlockNumber, LockedLen, UnlockingLen, StakedLen>
    AccountLedger<BlockNumber, LockedLen, UnlockingLen, StakedLen>
where
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    LockedLen: Get<u32>,
    UnlockingLen: Get<u32>,
    StakedLen: Get<u32>,
{
    /// Empty if no locked/unlocking/staked info exists.
    pub fn is_empty(&self) -> bool {
        self.locked.0.is_empty() && self.unlocking.is_empty() && self.staked.0.is_empty()
    }

    /// Returns latest locked chunk if it exists, `None` otherwise
    pub fn latest_locked_chunk(&self) -> Option<&LockedChunk> {
        self.locked.0.last()
    }

    /// Returns active locked amount.
    /// If `zero`, means that associated account hasn't got any active locked funds.
    pub fn active_locked_amount(&self) -> Balance {
        self.latest_locked_chunk()
            .map_or(Balance::zero(), |locked| locked.amount)
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
        self.active_locked_amount() + self.unlocking_amount()
    }

    /// Returns latest era in which locked amount was updated or zero in case no lock amount exists
    pub fn lock_era(&self) -> EraNumber {
        self.latest_locked_chunk()
            .map_or(EraNumber::zero(), |locked| locked.era)
    }

    // TODO: can active_period be provided somehow different instead of using a parameter? It should be 'free' to read the data from storage since it will be whitelisted.

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

    /// Adds the specified amount to the total locked amount, if possible.
    /// Caller must ensure that the era matches the next one, not the current one.
    ///
    /// If entry for the specified era already exists, it's updated.
    ///
    /// If entry for the specified era doesn't exist, it's created and insertion is attempted.
    /// In case vector has no more capacity, error is returned, and whole operation is a noop.
    pub fn add_lock_amount(&mut self, amount: Balance, era: EraNumber) -> Result<(), ()> {
        self.locked.add_amount(amount, era)
    }

    /// Subtracts the specified amount of the total locked amount, if possible.
    ///
    /// If entry for the specified era already exists, it's updated.
    ///
    /// If entry for the specified era doesn't exist, it's created and insertion is attempted.
    /// In case vector has no more capacity, error is returned, and whole operation is a noop.
    pub fn subtract_lock_amount(&mut self, amount: Balance, era: EraNumber) -> Result<(), ()> {
        self.locked.subtract_amount(amount, era)
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
    ) -> Result<(), ()> {
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
                    .map_err(|_| ())?;
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
        let mut total_unlocked = Balance::zero();

        // Probably faster approach than partitioning, summing & collecting.
        let mut i = 0;
        while i < self.unlocking.len() {
            let chunk = self.unlocking[i];
            if chunk.unlock_block <= current_block_number {
                total_unlocked.saturating_accrue(chunk.amount);
                self.unlocking.remove(i);
            } else {
                i += 1;
            }
        }

        total_unlocked
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
}
