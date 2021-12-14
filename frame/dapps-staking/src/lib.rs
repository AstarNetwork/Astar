//! # dApps Staking Module
//!
//! The dApps staking module manages era, total amounts of rewards and how to distribute.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use frame_support::traits::Currency;
use frame_system::{self as system};
use scale_info::TypeInfo;
use sp_arithmetic::traits::AtLeast32BitUnsigned;
use sp_runtime::RuntimeDebug;
use sp_std::{collections::btree_map::BTreeMap, ops::Add, prelude::*};

pub mod pallet;
pub mod traits;
pub mod weights;
pub use traits::*;

#[cfg(any(feature = "runtime-benchmarks"))]
pub mod benchmarking;
pub mod migrations;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod testing_utils;
#[cfg(test)]
mod tests;

pub use pallet::pallet::*;
pub use sp_staking::SessionIndex;
pub use weights::WeightInfo;

pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

/// DApp State descriptor
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum DAppState {
    /// Contract is registered and active.
    Registered,
    /// Contract has been unregistered and is inactive.
    /// Claim for past eras and unbonding is still possible but no additional staking can be done.
    Unregistered,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct DeveloperInfo<AccountId> {
    /// Developer (owner) account
    developer: AccountId,
    /// Current DApp State
    state: DAppState,
}

impl<AccountId> DeveloperInfo<AccountId> {
    /// Create new `DeveloperInfo` struct instance with the given developer and state `Registered`
    fn new(developer: AccountId) -> Self {
        Self {
            developer: developer,
            state: DAppState::Registered,
        }
    }
}

/// Mode of era-forcing.
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
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

/// A record for total rewards and total amount staked for an era
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct EraRewardAndStake<Balance: HasCompact> {
    /// Total amount of rewards for an era
    #[codec(compact)]
    rewards: Balance,
    /// Total staked amount for an era
    #[codec(compact)]
    staked: Balance,
}

/// Used to store information about total staked amount and the number of stakers
#[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct EraStakingPoints<Balance: HasCompact> {
    /// Total staked amount.
    #[codec(compact)]
    total: Balance,
    /// Total number of active stakers on this contract for an era
    #[codec(compact)]
    number_of_stakers: u32,
    // TODO: might be useful to keep this as map in order for easier tracking of who staked when/what. Depends on the frotend actually.
}

#[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct StakerInfo<Balance: HasCompact> {
    /// Total staked amount.
    #[codec(compact)]
    staked: Balance,
    /// Claimed rewards in this era
    #[codec(compact)]
    claimed_rewards: Balance,
}

/// Storage value representing the current Dapps staking pallet storage version.
/// Used by `on_runtime_upgrade` to determine whether a storage migration is needed or not.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum Version {
    V1_0_0,
    V2_0_0,
    V3_0_0,
}

impl Default for Version {
    fn default() -> Self {
        Version::V1_0_0
    }
}

/// Represents an balance amount undergoing the unbonding process.
/// Since unbonding takes time, it's important to keep track of when and how much was unbonded.
#[derive(Clone, Copy, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct UnlockingChunk<Balance> {
    /// Amount being unlocked
    #[codec(compact)]
    amount: Balance,
    /// Era in which the amount will become unlocked and can be withdrawn.
    #[codec(compact)]
    unlock_era: EraIndex,
}

impl<Balance> UnlockingChunk<Balance>
where
    Balance: Add<Output = Balance> + Copy,
{
    // Adds the specified amount to this chunk
    fn add_amount(&mut self, amount: Balance) {
        self.amount = self.amount + amount
    }
}

/// Contains unlocking chunks.
/// This is a convenience struct that provides various utility methods to help with unbonding handling.
#[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct UnbondingInfo<Balance> {
    // Vector of unlocking chunks. Sorted in ascending order in respect to unlock_era.
    unlocking_chunks: Vec<UnlockingChunk<Balance>>,
}

impl<Balance> UnbondingInfo<Balance>
where
    Balance: AtLeast32BitUnsigned + Default + Copy,
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

/// Contains information about account's locked & unbonding balances.
#[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct AccountLedger<SmartContract: Ord + Clone, Balance: HasCompact> {
    /// Total balance locked.
    #[codec(compact)]
    locked: Balance,
    /// Information about unbonding chunks.
    unbonding_info: UnbondingInfo<Balance>,
    /// Keys represent all contracts that are (or were) staked.
    /// If value is `None`, contract is still staked.
    /// If value is `Some(era)`, contract was fully unstaked in `era`.
    /// Unstaked contracts are removed from the map after some time.
    staked_contracts: BTreeMap<SmartContract, Option<EraIndex>>,
    // TODO: introduce reward destination for Compounding Rewards prep?
}

impl<SmartContract: Ord + Clone, Balance: AtLeast32BitUnsigned + Default + Copy>
    AccountLedger<SmartContract, Balance>
{
    /// `true` if ledger is empty (no locked funds, no unbonding chunks), `false` otherwise.
    pub(crate) fn is_empty(&self) -> bool {
        self.locked.is_zero() && self.unbonding_info.is_empty()
    }

    /// Should be called when contract is staked by a staker.
    pub(crate) fn contract_staked(&mut self, contract_id: &SmartContract) {
        if !self.staked_contracts.contains_key(contract_id) {
            self.staked_contracts.insert(contract_id.clone(), None);
        }
    }

    /// Should be called when contract is fully unstaked by a staker.
    pub(crate) fn contract_unstaked(
        &mut self,
        contract_id: &SmartContract,
        era: EraIndex,
        history_depth: EraIndex,
    ) {
        self.staked_contracts.insert(contract_id.clone(), Some(era));

        self.staked_contracts.retain(|_, v| match v {
            None => true,
            Some(unstake_era) => *unstake_era > era.saturating_sub(history_depth),
        });
    }
}
