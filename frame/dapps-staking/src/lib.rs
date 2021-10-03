//! # dApps Staking Module
//!
//! The dApps staking module manages era, total amounts of rewards and how to distribute.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use frame_support::traits::Currency;
use frame_system::{self as system};
use sp_runtime::RuntimeDebug;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

pub mod pallet;
pub mod traits;
pub mod weights;
pub use traits::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod testing_utils;
#[cfg(test)]
mod tests;

pub use pallet::{pallet::*, *};
pub use sp_staking::SessionIndex;
pub use weights::WeightInfo;

pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

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

/// A record for total rewards and total amount staked for an era
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
pub struct EraRewardAndStake<Balance> {
    /// Total amount of rewards for an era
    rewards: Balance,
    /// Total staked amount for an era
    staked: Balance,
}

/// Used to split total EraPayout among contracts.
/// Each tuple (contract, era) has this structure.
/// This will be used to reward contracts developer and his stakers.
#[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug)]
pub struct EraStakingPoints<AccountId: Ord, Balance: HasCompact> {
    /// Total staked amount.
    total: Balance,
    /// The map of stakers and the amount they staked.
    stakers: BTreeMap<AccountId, Balance>,
    /// Era when this contract was staked last time before this one.
    /// In case only a single staking era exists, it will be set to that one. This indicates the final element in the chain.
    former_staked_era: EraIndex,
    /// Accrued and claimed rewards on this contract both for stakers and the developer
    claimed_rewards: Balance,
}
