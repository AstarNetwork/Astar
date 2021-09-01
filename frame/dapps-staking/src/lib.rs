//! # dApps Staking Module
//!
//! The dApps staking module manages era, total amounts of rewards and how to distribute.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use frame_support::traits::{Currency, Get};
use frame_system::{self as system};
use sp_core::crypto::UncheckedFrom;
use sp_runtime::{
    curve::PiecewiseLinear,
    traits::{AtLeast32BitUnsigned, Saturating, StaticLookup, Zero},
    RuntimeDebug,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*, vec::Vec};

pub mod pallet;
pub mod weights;

// #[cfg(test)]
// mod mock;
// #[cfg(test)]
// mod tests;

pub use pallet::{pallet::*, *};
pub use sp_staking::SessionIndex;
pub use weights::WeightInfo;

pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance;

type PositiveImbalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::PositiveImbalance;
type NegativeImbalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::NegativeImbalance;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

const MAX_NOMINATIONS: usize = 128;
const MAX_UNLOCKING_CHUNKS: usize = 32;
const MAX_VOTES: usize = 128;
const VOTES_REQUIREMENT: u32 = 12;

/// A destination account for payment.
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum RewardDestination<AccountId> {
    /// Pay into the stash account, increasing the amount at stake accordingly.
    Staked,
    /// Pay into the stash account, not increasing the amount at stake.
    Stash,
    /// Pay into the controller account.
    Controller,
    /// Pay into a specified account.
    Account(AccountId),
    /// Receive no reward.
    None,
}

impl<AccountId> Default for RewardDestination<AccountId> {
    fn default() -> Self {
        RewardDestination::Staked
    }
}

/// Information regarding the active era (era in used in session).
#[derive(Encode, Decode, RuntimeDebug)]
pub struct ActiveEraInfo {
    /// Index of era.
    pub index: EraIndex,
    /// Moment of start expressed as millisecond from `$UNIX_EPOCH`.
    ///
    /// Start can be none if start hasn't been set for the era yet,
    /// Start is set on the first on_finalize of the era to guarantee usage of `Time`.
    start: Option<u64>,
}

/// Handler for determining how much of a balance should be paid out on the current era.
pub trait EraPayout<Balance> {
    /// Determine the payout for this era.
    ///
    /// Returns the amount to be paid to stakers in this era, as well as whatever else should be
    /// paid out ("the rest").
    fn era_payout(
        total_staked: Balance,
        total_issuance: Balance,
        era_duration_millis: u64,
    ) -> (Balance, Balance);
}

impl<Balance: Default> EraPayout<Balance> for () {
    fn era_payout(
        _total_staked: Balance,
        _total_issuance: Balance,
        _era_duration_millis: u64,
    ) -> (Balance, Balance) {
        (Default::default(), Default::default())
    }
}

/// Mode of era-forcing.
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum Forcing {
    /// Not forcing anything - just let whatever happen.
    NotForcing,
    /// Force a new era, then reset to `NotForcing` as soon as it is done.
    /// Note that this will force to trigger an election until a new era is triggered, if the
    /// election failed, the next session end will trigger a new election again, until success.
    ForceNew,
    /// Avoid a new era indefinitely.
    ForceNone,
    /// Force a new era at the end of all sessions indefinitely.
    ForceAlways,
}

impl Default for Forcing {
    fn default() -> Self {
        Forcing::NotForcing
    }
}

/// A record of the nominations made by a specific account.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Nominations<AccountId, Balance> {
    /// The targets of nomination and amounts of staking.
    pub targets: Vec<(AccountId, Balance)>,
    /// The era the nominations were submitted.
    ///
    /// Except for initial nominations which are considered submitted at era 0.
    pub submitted_in: EraIndex,
    /// Whether the nominations have been suppressed.
    pub suppressed: bool,
}

/// Reward points of an era. Used to split era total payout between dapps rewards.
///
/// This points will be used to reward contracts operators and their respective nominators.
#[derive(PartialEq, Encode, Decode, Default, RuntimeDebug)]
pub struct EraStakingPoints<AccountId: Ord, Balance: HasCompact> {
    /// Total number of staking. Equals the sum of staking points for each contracts.
    total: Balance,
    /// The balance of stakinng earned by a given contracts.
    individual: BTreeMap<AccountId, Balance>,
}

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct UnlockChunk<Balance: HasCompact> {
    /// Amount of funds to be unlocked.
    #[codec(compact)]
    value: Balance,
    /// Era number at which point it'll be unlocked.
    #[codec(compact)]
    era: EraIndex,
}

/// The ledger of a (bonded) stash.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct StakingLedger<AccountId, Balance: HasCompact> {
    /// The stash account whose balance is actually locke,ed and at stake.
    pub stash: AccountId,
    /// The total amount of the stash's balance that we are currently accounting for.
    /// It's just `active` plus all the `unlocking` balances.
    #[codec(compact)]
    pub total: Balance,
    /// The total amount of the stash's balance that will be at stake in any forthcoming
    /// rounds.
    #[codec(compact)]
    pub active: Balance,
    /// Any balance that is becoming free, which may eventually be transferred out
    /// of the stash (assuming it doesn't get slashed first).
    pub unlocking: Vec<UnlockChunk<Balance>>,
    /// The latest and highest era which the staker has claimed reward for.
    pub last_reward: EraIndex,
}

impl<AccountId, Balance: HasCompact + Copy + Saturating + Ord + Zero>
    StakingLedger<AccountId, Balance>
{
    /// Remove entries from `unlocking` that are sufficiently old and reduce the
    /// total by the sum of their balances.
    fn consolidate_unlocked(self, current_era: EraIndex, amount_locked: Balance) -> Self {
        let mut total = self.total;
        let mut unlocking: Vec<UnlockChunk<Balance>> = self
            .unlocking
            .into_iter()
            .filter(|chunk| {
                if chunk.era > current_era {
                    true
                } else {
                    total = total.saturating_sub(chunk.value);
                    false
                }
            })
            .collect();
        if amount_locked > Zero::zero() {
            total = total.saturating_add(amount_locked);
            unlocking.push(UnlockChunk {
                value: amount_locked,
                era: current_era,
            });
        }
        Self {
            total,
            active: self.active,
            stash: self.stash,
            unlocking,
            last_reward: self.last_reward,
        }
    }
}

impl<AccountId, Balance> StakingLedger<AccountId, Balance>
where
    Balance: AtLeast32BitUnsigned + Saturating + Copy,
{
    /// Slash the account for a given amount of balance.
    ///
    /// Slashes from `active` funds first, and then `unlocking`, starting with the
    /// chunks that are closest to unlocking.
    fn slash(&mut self, mut value: Balance) -> Balance {
        let pre_total = self.total;
        let total = &mut self.total;
        let active = &mut self.active;

        let slash_out_of =
            |total_remaining: &mut Balance, target: &mut Balance, value: &mut Balance| {
                let slash_from_target = (*value).min(*target);

                if !slash_from_target.is_zero() {
                    *target -= slash_from_target;
                    *total_remaining = total_remaining.saturating_sub(slash_from_target);
                    *value -= slash_from_target;
                }
            };

        slash_out_of(total, active, &mut value);

        let i = self
            .unlocking
            .iter_mut()
            .map(|chunk| {
                slash_out_of(total, &mut chunk.value, &mut value);
                chunk.value
            })
            .take_while(|value| value.is_zero()) // take all fully-consumed chunks out.
            .count();

        // kill all drained chunks.
        let _ = self.unlocking.drain(..i);

        pre_total.saturating_sub(*total)
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum Vote {
    Bad,
    Good,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default, RuntimeDebug)]
pub struct VoteCounts {
    bad: u32,
    good: u32,
}

pub trait ContractFinder<AccountId> {
    fn is_exists_contract(contract_id: &AccountId) -> bool;
}

impl<T: Config> ContractFinder<T::AccountId> for Pallet<T>
where
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
    fn is_exists_contract(contract_id: &T::AccountId) -> bool {
        // <ContractHasOperator<T>>::contains_key(contract_id)
        true
    }
}
