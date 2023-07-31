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

use frame_support::{pallet_prelude::*, traits::Currency, BoundedVec};
use frame_system::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::traits::{AtLeast32BitUnsigned, Zero};

use crate::pallet::Config;

// TODO: instead of using `pub` visiblity for fields, either use `pub(crate)` or add dedicated methods for accessing them.

/// The balance type used by the currency system.
pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Convenience type for `AccountLedger` usage.
pub type AccountLedgerFor<T> = AccountLedger<
    BalanceOf<T>,
    BlockNumberFor<T>,
    <T as Config>::MaxLockedChunks,
    <T as Config>::MaxUnlockingChunks,
>;

/// Era number type
pub type EraNumber = u32;
/// Period number type
pub type PeriodNumber = u32;
/// Dapp Id type
pub type DAppId = u16;

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
pub struct LockedChunk<Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy> {
    #[codec(compact)]
    pub amount: Balance,
    #[codec(compact)]
    pub era: EraNumber,
}

impl<Balance> Default for LockedChunk<Balance>
where
    Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
{
    fn default() -> Self {
        Self {
            amount: Balance::zero(),
            era: EraNumber::zero(),
        }
    }
}

// TODO: would users get better UX if we kept using eras? Using blocks is more precise though.
/// How much was unlocked in some block.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct UnlockingChunk<
    Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen,
> {
    #[codec(compact)]
    pub amount: Balance,
    #[codec(compact)]
    pub unlock_block: BlockNumber,
}

impl<Balance, BlockNumber> Default for UnlockingChunk<Balance, BlockNumber>
where
    Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen,
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
#[scale_info(skip_type_params(LockedLen, UnlockingLen))]
pub struct AccountLedger<
    Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen,
    LockedLen: Get<u32>,
    UnlockingLen: Get<u32>,
> {
    /// How much was staked in each era
    pub locked: BoundedVec<LockedChunk<Balance>, LockedLen>,
    /// How much started unlocking on a certain block
    pub unlocking: BoundedVec<UnlockingChunk<Balance, BlockNumber>, UnlockingLen>,
    //TODO, make this a compact struct!!!
    /// How much user had staked in some period
    // #[codec(compact)]
    pub staked: (Balance, PeriodNumber),
}

impl<Balance, BlockNumber, LockedLen, UnlockingLen> Default
    for AccountLedger<Balance, BlockNumber, LockedLen, UnlockingLen>
where
    Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen,
    LockedLen: Get<u32>,
    UnlockingLen: Get<u32>,
{
    fn default() -> Self {
        Self {
            locked: BoundedVec::<LockedChunk<Balance>, LockedLen>::default(),
            unlocking: BoundedVec::<UnlockingChunk<Balance, BlockNumber>, UnlockingLen>::default(),
            staked: (Balance::zero(), 0),
        }
    }
}

impl<Balance, BlockNumber, LockedLen, UnlockingLen>
    AccountLedger<Balance, BlockNumber, LockedLen, UnlockingLen>
where
    Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen,
    LockedLen: Get<u32>,
    UnlockingLen: Get<u32>,
{
    /// Empty if no locked/unlocking/staked info exists.
    pub fn is_empty(&self) -> bool {
        self.locked.is_empty() && self.unlocking.is_empty() && self.staked.0.is_zero()
    }

    /// Returns latest locked chunk if it exists, `None` otherwise
    pub fn latest_locked_chunk(&self) -> Option<&LockedChunk<Balance>> {
        self.locked.last()
    }

    /// Returns locked amount.
    /// If `zero`, means that associated account hasn't locked any funds.
    pub fn locked_amount(&self) -> Balance {
        self.latest_locked_chunk()
            .map_or(Balance::zero(), |locked| locked.amount)
    }

    /// Returns latest era in which locked amount was updated or zero in case no lock amount exists
    pub fn era(&self) -> EraNumber {
        self.latest_locked_chunk()
            .map_or(EraNumber::zero(), |locked| locked.era)
    }

    /// Adds the specified amount to the total locked amount, if possible.
    ///
    /// If entry for the specified era already exists, it's updated.
    ///
    /// If entry for the specified era doesn't exist, it's created and insertion is attempted.
    /// In case vector has no more capacity, error is returned, and whole operation is a noop.
    pub fn add_lock_amount(&mut self, amount: Balance, era: EraNumber) -> Result<(), ()> {
        if amount.is_zero() {
            return Ok(());
        }

        let mut locked_chunk = if let Some(&locked_chunk) = self.locked.last() {
            locked_chunk
        } else {
            LockedChunk::default()
        };

        locked_chunk.amount.saturating_accrue(amount);

        if locked_chunk.era == era && !self.locked.is_empty() {
            if let Some(last) = self.locked.last_mut() {
                *last = locked_chunk;
            }
        } else {
            locked_chunk.era = era;
            self.locked.try_push(locked_chunk).map_err(|_| ())?;
        }

        Ok(())
    }
}

/// Rewards pool for lock participants & dApps
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct RewardInfo<Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy> {
    /// Rewards pool for accounts which have locked funds in dApp staking
    #[codec(compact)]
    pub participants: Balance,
    /// Reward pool for dApps
    #[codec(compact)]
    pub dapps: Balance,
}

/// Info about current era, including the rewards, how much is locked, unlocking, etc.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct EraInfo<Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy> {
    /// Info about era rewards
    pub rewards: RewardInfo<Balance>,
    /// How much balance is considered to be locked in the current era.
    /// This value influences the reward distribution.
    #[codec(compact)]
    pub active_era_locked: Balance,
    /// How much balance is locked in dApp staking, in total.
    /// For rewards, this amount isn't relevant for the current era, but only from the next one.
    #[codec(compact)]
    pub total_locked: Balance,
    /// How much balance is undergoing unlocking process (still counts into locked amount)
    #[codec(compact)]
    pub unlocking: Balance,
}
