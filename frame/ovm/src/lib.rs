//! # OVM Module
//!
//! The OVM module provides functionality for handling layer2 dispute logics.
//!
//! - [`plasm_rewards::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//!
//!
//!
//!
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    traits::{Currency, Get, Time},
    weights::SimpleDispatchInfo,
    StorageMap, StorageValue,
};
use frame_system::{self as system, ensure_root};
use sp_runtime::{
    traits::{SaturatedConversion, Zero},
    Perbill, RuntimeDebug,
};
use sp_std::{prelude::*, vec::Vec};

// #[cfg(test)]
// mod mock;
// #[cfg(test)]
// mod tests;

/// Predicates write properties and it can prove to true or false under dispute logic.
///
/// Required functions of each Predicate:
/// - isValidChallenge
/// - decide
/// isValidChallenge validates valid child node of game tree.
#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct Predicate(Vec<u8>);

/// Property stands for dispute logic and we can claim every Properties to Adjudicator Contract.
/// Property has its predicate address and array of input.
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Property<AccountId> {
    /// Indicates the address of Predicate.
    predicate_address: AccountId,
    /// Every input are bytes. Each Atomic Predicate decode inputs to the specific type.
    inputs: Vec<u8>,
}

/// The game decision by predicates.
#[derive(Encode, Decode, RuntimeDebug, PartialEq, Eq)]
pub enum Decision {
    Undecided,
    True,
    False,
}

/// ChallengeGame is a part of L2 dispute. It's instantiated by claiming property.
/// The client can get a game instance from this module.
#[derive(Encode, Decode, RuntimeDebug, PartialEq, Eq)]
pub struct ChallengeGame<AccountId, BlockNumber> {
    /// Property of challenging targets.
    property: Property<AccountId>,
    /// challenges inputs
    challenges: Vec<u8>,
    /// the result of this challenge.
    decision: Decision,
    /// the block number when this was issued.
    created_block: BlockNumber,
}

pub type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;
pub type ChallengeGameOf<T> =
    ChallengeGame<<T as frame_system::Trait>::AccountId, <T as frame_system::Trait>::BlockNumber>;
pub type PropertyOf<T> = Property<<T as frame_system::Trait>::AccountId>;

pub trait Trait: frame_system::Trait {
    /// The balance.
    type Currency: Currency<Self::AccountId>;

    /// Time used for computing era duration.
    type Time: Time;

    /// During the dispute period defined here, the user can challenge.
    /// If nothing is found, the state is determined after the dispute period.
    type DisputePeriod: Get<MomentOf<Self>>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as OVM {
        /// Mapping the predicate address to Predicate.
        /// Predicate is handled similar to contracts.
        Predicates get(fn predicate): map hasher(blake2_128_concat)
         T::AccountId => Option<Predicate>;

        /// Mapping the game id to Challenge Game.
        InstantiatedGames get(fn instantiated_games):
         map hasher(blake2_128_concat) T::Hash => Option<ChallengeGameOf<T>>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        Property = PropertyOf<T>,
        Hash = <T as frame_system::Trait>::Hash,
        BlockNumber = <T as frame_system::Trait>::BlockNumber,
    {
        /// (predicate_address: AccountId);
        DeployPredicate(AccountId),
        /// (gameId: Hash, decision: bool)
        AtomicPropositionDecided(Hash, bool),
        /// (game_id: Hash, property: Property, createdBlock: BlockNumber)
        NewPropertyClaimed(Hash, Property, BlockNumber),
        /// (game_id: Hash, challengeGameId: Hash)
        ClaimChallenged(Hash, Hash),
        /// (game_id: Hash, decision: bool)
        ClaimDecided(Hash, bool),
        /// (game_id: Hash, challengeGameId: Hash)
        ChallengeRemoved(Hash, Hash),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// Duplicate index.
        DuplicateIndex,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// During the dispute period defined here, the user can challenge.
        /// If nothing is found, the state is determined after the dispute period.
        const DisputePeriod: MomentOf<T> = T::DisputePeriod::get();

        type Error = Error<T>;

        fn deposit_event() = default;

        fn on_runtime_upgrade() {
            migrate::<T>();
        }

        /// Deploy predicate and made predicate address as AccountId.
        fn deploy(origin, predicate: Predicate) {
        }

        /// Claims property and create new game. Id of game is hash of claimed property
        fn claim_property(origin, claim: PropertyOf<T>) {
        }

        /// Sets the game decision true when its dispute period has already passed.
        fn decide_claim_to_true(origin, game_id: T::Hash) {
        }

        /// Sets the game decision false when its challenge has been evaluated to true.
        fn decide_claim_to_false(origin, game_id: T::Hash, challenging_game_id: T::Hash) {
        }

        /// Decide the game decision with given witness.
        fn decide_claim_with_witness(origin, gameId: T::Hash, witness: Vec<u8>) {

        }

        /// Removes a challenge when its decision has been evaluated to false.
        fn remove_challenge(origin, game_id: T::Hash, challenging_game_id: T::Hash) {
        }

        /// Set a predicate decision by called from Predicate itself.
        fn set_predicate_decision(origin, game_id: T::Hash, decision: bool) {

        }

        /// Challenge a game specified by gameId with a challengingGame specified by _challengingGameId.
        ///
        /// @param game_id challenged game id.
        /// @param challenge_inputs array of input to verify child of game tree.
        /// @param challenging_game_id child of game tree.
        fn challenge(origin, game_id: T::Hash, challenge_inputs: Vec<u8>, challenging_game_id: T::Hash) {

        }
    }
}

fn migrate<T: Trait>() {
    // TODO: When runtime upgrade, migrate stroage.
    // if let Some(current_era) = CurrentEra::get() {
    //     let history_depth = HistoryDepth::get();
    //     for era in current_era.saturating_sub(history_depth)..=current_era {
    //         ErasStartSessionIndex::migrate_key_from_blake(era);
    //     }
    // }
}

impl<T: Trait> Module<T> {
    // ======= callable ======
    /// Get of true/false the result of decided property
    fn is_decided(property: PropertyOf<T>) -> Decision {
        Decision::Undecided
    }
    fn get_game(claim_id: T::Hash) -> Option<ChallengeGameOf<T>> {
        None
    }
    fn get_property_id(property: PropertyOf<T>) -> Option<T::Hash> {
        None
    }
}
