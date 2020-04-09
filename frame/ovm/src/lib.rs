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
    decl_error, decl_event, decl_module, decl_storage, ensure,
    traits::{Currency, Get, Time},
    weights::SimpleDispatchInfo,
    StorageMap, StorageValue,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_runtime::{
    traits::{SaturatedConversion, Zero, Hash},
    Perbill, RuntimeDebug,
};
use sp_std::{prelude::*, vec::Vec};

// #[cfg(test)]
// mod mock;
// #[cfg(test)]
// mod tests;

pub mod traits;
use traits::{PredicateAddressFor};

/// Predicates write properties and it can prove to true or false under dispute logic.
#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct Predicate(Vec<u8>);

/// PredicateContract wrapped Predicate and initial arguments.
///
/// Required functions of each PredicateContract:
/// - isValidChallenge
/// - decide
///
/// isValidChallenge validates valid child node of game tree.
#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct PredicateContract<CodeHash> {
    pub predicate_hash: CodeHash,
    pub inputs: Vec<u8>,
}

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
pub struct ChallengeGame<AccountId, Hash, BlockNumber> {
    /// Property of challenging targets.
    property: Property<AccountId>,
    /// challenges inputs
    challenges: Vec<Hash>,
    /// the result of this challenge.
    decision: Decision,
    /// the block number when this was issued.
    created_block: BlockNumber,
}

pub type PredicateHash<T> = <T as system::Trait>::Hash;
pub type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;
pub type ChallengeGameOf<T> =
    ChallengeGame<<T as system::Trait>::AccountId, <T as system::Trait>::Hash, <T as system::Trait>::BlockNumber>;
pub type PropertyOf<T> = Property<<T as system::Trait>::AccountId>;

pub trait Trait: system::Trait {
    /// The balance.
    type Currency: Currency<Self::AccountId>;

    /// Time used for computing era duration.
    type Time: Time;

    /// During the dispute period defined here, the user can challenge.
    /// If nothing is found, the state is determined after the dispute period.
    type DisputePeriod: Get<MomentOf<Self>>;

    /// A function type to get the contract address given the instantiator.
    type DeterminePredicateAddress: PredicateAddressFor<PredicateHash<Self>, Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as OVM {
        /// A mapping from an original code hash to the original code, untouched by instrumentation.
		pub PredicateCodes get(fn predicate_codes): map hasher(identity) PredicateHash<T> => Option<Predicate>;

        /// Mapping the predicate address to Predicate.
        /// Predicate is handled similar to contracts.
        pub Predicates get(fn predicates): map hasher(blake2_128_concat)
         T::AccountId => Option<PredicateContract<PredicateHash<T>>>;

        /// Mapping the game id to Challenge Game.
        pub InstantiatedGames get(fn instantiated_games):
         map hasher(blake2_128_concat) T::Hash => Option<ChallengeGameOf<T>>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Property = PropertyOf<T>,
        Hash = <T as system::Trait>::Hash,
        BlockNumber = <T as system::Trait>::BlockNumber,
    {
        /// (predicate_address: AccountId);
        PutPredicate(Hash),
        /// (predicate_address: AccountId);
        InstantiatePredicate(AccountId),
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
        /// claim isn't empty
        CiamIsNotEmpty,
        /// No property id
        NoPropertyId,
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

        /// Stores the given binary Wasm code into the chain's storage and returns its `codehash`.
		/// You can instantiate contracts only with stored code.
		pub fn put_code(
			origin,
			predicate: Predicate
		) {
            let _ = ensure_signed(origin)?;
            let predicate_hash = <T as system::Trait>::Hashing::hash_of(&predicate);
            <PredicateCodes<T>>::insert(&predicate_hash, predicate);
            Self::deposit_event(RawEvent::PutPredicate(predicate_hash));
		}


        /// Deploy predicate and made predicate address as AccountId.
        fn instantiate(origin, predicate_hash: PredicateHash<T>, inputs: Vec<u8>) {
            let origin = ensure_signed(origin)?;
            let predicate_address = T::DeterminePredicateAddress::predicate_address_for(
                &predicate_hash,
                &inputs,
                &origin);
            let predicate = Self::predicate_codes(&predicate_hash);
            let predicate_contract = PredicateContract {
                predicate_hash,
                inputs,
            };
            <Predicates<T>>::insert(&predicate_address, predicate_contract);
            Self::deposit_event(RawEvent::InstantiatePredicate(predicate_address));
        }

        /// Claims property and create new game. Id of game is hash of claimed property
        fn claim_property(origin, claim: PropertyOf<T>) {
            let _ = ensure_signed(origin)?;
            // get the id of this property
            let game_id = match Self::get_property_id(&claim) {
                Some(id) => id,
                None => Err(Error::<T>::NoPropertyId)?,
            };
            let block_number = Self::block_number();

            // make sure a claim on this property has not already been made
            ensure!(None == Self::instantiated_games(&game_id), Error::<T>::CiamIsNotEmpty);

            // create the claim status. Always begins with no proven contradictions
            let new_game = ChallengeGameOf::<T> {
                property: claim.clone(),
                challenges: vec!{},
                decision: Decision::Undecided,
                created_block: block_number.clone(),
            };

            // store the claim
           <InstantiatedGames<T>>::insert(&game_id, new_game);
           Self::deposit_event(RawEvent::NewPropertyClaimed(game_id, claim, block_number));
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
    /// Get of true/false the decision of property.
    fn is_decided(property: &PropertyOf<T>) -> Decision {
        Decision::Undecided
    }
    /// Get of the instatiated challenge game from claim_id.
    fn get_game(claim_id: &T::Hash) -> Option<ChallengeGameOf<T>> {
        None
    }
    /// Get of the property id from the propaty itself.
    fn get_property_id(property: &PropertyOf<T>) -> Option<T::Hash> {
        None
    }

    // ======= heler =======
    fn block_number() -> <T as system::Trait>::BlockNumber {
        <system::Module::<T>>::block_number()
    }
}
