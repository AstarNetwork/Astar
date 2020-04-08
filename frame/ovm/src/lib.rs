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

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct Predicate(Vec<u8>);

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Property<AccountId> {
    predicate_address: AccountId,
    // Every input are bytes. Each Atomic Predicate decode inputs to the specific type.
    inputs: Vec<u8>,
}

#[derive(Encode, Decode, RuntimeDebug, PartialEq, Eq)]
pub enum Decision {
    Undecided,
    True,
    False,
}

#[derive(Encode, Decode, RuntimeDebug, PartialEq, Eq)]
pub struct ChallengeGame<AccountId, BlockNumber> {
    property: Property<AccountId>,
    challenges: Vec<u8>,
    decision: Decision,
    created_block: BlockNumber,
}

#[derive(Encode, Decode, RuntimeDebug, PartialEq, Eq)]
pub struct Range<Balance> {
    start: Balance,
    end: Balance,
}

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
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
        Predicates get(fn predicate): map hasher(blake2_128_concat)
         T::AccountId => Option<Predicate>;
        DisputePeriod get(fn dispute_period): MomentOf<T>;
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
        // (predicate_address: AccountId);
        DeployPredicate(AccountId),
        // (gameId: Hash, decision: bool)
        AtomicPropositionDecided(Hash, bool),
        // (game_id: Hash, property: Property, createdBlock: BlockNumber)
        NewPropertyClaimed(Hash, Property, BlockNumber),
        // (game_id: Hash, challengeGameId: Hash)
        ClaimChallenged(Hash, Hash),
        // (game_id: Hash, decision: bool)
        ClaimDecided(Hash, bool),
        // (game_id: Hash, challengeGameId: Hash)
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

        fn deploy(origin, predicate: Predicate) {
        }

        fn claim_property(origin, claim: PropertyOf<T>) {
        }

        fn decide_claim_to_true(origin, game_id: T::Hash) {
        }

        fn decide_claim_to_false(origin, game_id: T::Hash, challenging_game_id: T::Hash) {
        }

        fn remove_challenge(origin, game_id: T::Hash, challenging_game_id: T::Hash) {
        }

        fn set_predicate_decision(origin, game_id: T::Hash, decision: bool) {

        }

        /**
       * @dev challenge a game specified by gameId with a challengingGame specified by _challengingGameId
       * @param _gameId challenged game id
       * @param _challengeInputs array of input to verify child of game tree
       * @param _challengingGameId child of game tree
       */
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
    fn is_decided(property: PropertyOf<T>) -> bool {
        true
    }
    fn get_game(claim_id: T::Hash) -> Option<ChallengeGameOf<T>> {
        None
    }
    fn get_property_id(property: PropertyOf<T>) -> Option<T::Hash> {
        None
    }
}
