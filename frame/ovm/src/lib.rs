//! # Ovm Module
//!
//! The Ovm module provides functionality for handling layer2 dispute logics.
//! This refer to: https://github.com/cryptoeconomicslab/ovm-contracts/blob/master/contracts/UniversalAdjudicationContract.sol
//!
//! - [`ovm::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//! Ovm module is the substrate pallet to archive dispute game defined by predicate logic.
//!
//!
//!
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{Get, Time},
    weights::{DispatchClass, FunctionOf, Pays, WeighData, Weight},
    StorageMap,
};
use frame_system::{self as system, ensure_signed};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::Hash, RuntimeDebug};
use sp_std::{prelude::*, vec::Vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod predicate;
pub mod traits;

use predicate::{ExecResult, ExecutionContext, PredicateLoader, PredicateOvm};
pub use traits::PredicateAddressFor;

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
#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct Property<AccountId> {
    /// Indicates the address of Predicate.
    pub predicate_address: AccountId,
    /// Every input are bytes. Each Atomic Predicate decode inputs to the specific type.
    pub inputs: Vec<Vec<u8>>,
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

/// Definition of the cost schedule and other parameterizations for optimistic virtual machine.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub struct Schedule {
    /// Version of the schedule.
    pub version: u32,

    /// Cost of putting a byte of code into storage.
    pub put_code_per_byte_cost: Weight,

    /// Maximum allowed stack height.
    ///
    /// See https://wiki.parity.io/WebAssembly-StackHeight to find out
    /// how the stack frame cost is calculated.
    pub max_stack_height: u32,

    /// Maximum number of memory pages allowed for a contract.
    pub max_memory_pages: u32,

    /// Maximum allowed size of a declared table.
    pub max_table_size: u32,
    // TODO: add logical conecctive addresses.
}

// 500 (2 instructions per nano second on 2GHZ) * 1000x slowdown through wasmi
// This is a wild guess and should be viewed as a rough estimation.
// Proper benchmarks are needed before this value and its derivatives can be used in production.
const WASM_INSTRUCTION_COST: Weight = 500_000;

impl Default for Schedule {
    fn default() -> Schedule {
        Schedule {
            version: 0,
            put_code_per_byte_cost: WASM_INSTRUCTION_COST,
            max_stack_height: 64 * 1024,
            max_memory_pages: 16,
            max_table_size: 16 * 1024,
        }
    }
}

/// In-memory cache of configuration values.
///
/// We assume that these values can't be changed in the
/// course of transaction execution.
pub struct Config {
    pub schedule: Schedule,
    pub max_depth: u32, // about down 30.
}

impl Config {
    fn preload<T: Trait>() -> Config {
        Config {
            schedule: <Module<T>>::current_schedule(),
            max_depth: T::MaxDepth::get(),
        }
    }
}

type PredicateHash<T> = <T as system::Trait>::Hash;
type ChallengeGameOf<T> = ChallengeGame<
    <T as system::Trait>::AccountId,
    <T as system::Trait>::Hash,
    <T as system::Trait>::BlockNumber,
>;
pub type PropertyOf<T> = Property<<T as system::Trait>::AccountId>;
type AccountIdOf<T> = <T as frame_system::Trait>::AccountId;
type PredicateContractOf<T> = PredicateContract<<T as frame_system::Trait>::Hash>;

pub trait Trait: system::Trait {
    /// The maximum nesting level of a call/instantiate stack.
    type MaxDepth: Get<u32>;

    /// During the dispute period defined here, the user can challenge.
    /// If nothing is found, the state is determined after the dispute period.
    type DisputePeriod: Get<Self::BlockNumber>;

    /// A function type to get the contract address given the instantiator.
    type DeterminePredicateAddress: PredicateAddressFor<PredicateHash<Self>, Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Ovm {
        /// Current cost schedule for contracts.
        pub CurrentSchedule get(fn current_schedule) config(): Schedule = Schedule::default();

        /// A mapping from an original code hash to the original code, untouched by instrumentation.
        pub PredicateCodes get(fn predicate_codes): map hasher(identity) PredicateHash<T> => Option<Vec<u8>>;

        /// A mapping between an original code hash and instrumented ovm(predicate) code, ready for execution.
        pub PredicateCache get(fn predicate_cache): map hasher(identity) PredicateHash<T> => Option<predicate::PrefabOvmModule>;

        /// Mapping the predicate address to Predicate.
        /// Predicate is handled similar to contracts.
        pub Predicates get(fn predicates): map hasher(blake2_128_concat)
         T::AccountId => Option<PredicateContractOf<T>>;

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
        /// Does not exist game
        DoesNotExistGame,
        /// Does not exist predicate
        DoesNotExistPredicate,
        /// claim should be decidable
        ClaimShouldBeDecidable,
        /// challenge isn't valid
        ChallengeIsNotValid,
        /// challenging game haven't been decided true
        ChallengingGameNotTrue,
        /// Decision must be undecided
        DecisionMustBeUndecided,
        /// There must be no challenge
        ThereMustBeNoChallenge,
        /// property must be true with given witness
        PropertyMustBeTrue,
        /// challenging game haven't been decided false
        ChallengingGameNotFalse,
        /// setPredicateDecision must be called from predicate
        MustBeCalledFromPredicate,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// During the dispute period defined here, the user can challenge.
        /// If nothing is found, the state is determined after the dispute period.
        const DisputePeriod: <T as system::Trait>::BlockNumber = T::DisputePeriod::get();

        type Error = Error<T>;

        fn deposit_event() = default;

        fn on_runtime_upgrade() -> Weight {
            migrate::<T>();
            // TODO: weight
            T::MaximumBlockWeight::get()
        }

        /// Stores the given binary Wasm code into the chain's storage and returns its `codehash`.
        /// You can instantiate contracts only with stored code.
        #[weight = FunctionOf(
            |args: (&Vec<u8>,)| Module::<T>::calc_code_put_costs(args.0),
            DispatchClass::Normal,
            Pays::Yes
        )]
        pub fn put_code(
            origin,
            predicate: Vec<u8>
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let schedule = Self::current_schedule();
            match predicate::save_code::<T>(predicate, &schedule) {
                Ok(predicate_hash) => {
                    Self::deposit_event(RawEvent::PutPredicate(predicate_hash));
                },
                Err(err) => return Err(err.into()),
            }
            Ok(())
        }


        /// Deploy predicate and made predicate address as AccountId.
        /// TODO: weight
        #[weight = 100_000]
        pub fn instantiate(origin, predicate_hash: PredicateHash<T>, inputs: Vec<u8>) {
            let origin = ensure_signed(origin)?;

            // Calc predicate address.
            let predicate_address = T::DeterminePredicateAddress::predicate_address_for(
                &predicate_hash,
                &inputs,
                &origin);
            let predicate_contract = PredicateContract {
                predicate_hash,
                inputs,
            };

            <Predicates<T>>::insert(&predicate_address, predicate_contract);

            Self::deposit_event(RawEvent::InstantiatePredicate(predicate_address));
        }

        /// Claims property and create new game. Id of game is hash of claimed property
        /// TODO: weight
        #[weight = 100_000]
        fn claim_property(origin, claim: PropertyOf<T>) {
            // get the id of this property
            let game_id = Self::get_property_id(&claim);
            let block_number = Self::block_number();

            // make sure a claim on this property has not already been made
            ensure!(
                None == Self::instantiated_games(&game_id),
                Error::<T>::CiamIsNotEmpty,
            );

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
        /// TODO: weight
        #[weight = 100_000]
        fn decide_claim_to_true(origin, game_id: T::Hash) {
            ensure!(
                Self::is_decidable(&game_id),
                Error::<T>::ClaimShouldBeDecidable,
            );

            // Note: if is_deciable(&game_id) is true, must exists instantiated_games(&game_id).
            let mut game = Self::instantiated_games(&game_id).unwrap();

            // game should be decided true
            game.decision = Decision::True;
            <InstantiatedGames<T>>::insert(&game_id, game);

            Self::deposit_event(RawEvent::ClaimDecided(game_id, true));
        }

        /// Sets the game decision false when its challenge has been evaluated to true.
        /// TODO: weight
        #[weight = 100_000]
        fn decide_claim_to_false(origin, game_id: T::Hash, challenging_game_id: T::Hash) {
            let mut game = match Self::instantiated_games(&game_id) {
                Some(game) => game,
                None => Err(Error::<T>::DoesNotExistGame)?,
            };

            let challenging_game = match Self::instantiated_games(&challenging_game_id) {
                Some(game) => game,
                None => Err(Error::<T>::DoesNotExistGame)?,
            };

            // check _challenge is in game.challenges
            let challenging_game_id = Self::get_property_id(&challenging_game.property);
            ensure!(
                game.challenges
                    .iter()
                    .any(|challenge| challenge == &challenging_game_id),
                Error::<T>::ChallengeIsNotValid,
            );

            // game.createdBlock > block.number - dispute
            // check _challenge have been decided true
            ensure!(
                challenging_game.decision == Decision::True,
                Error::<T>::ChallengingGameNotTrue,
            );

            // game should be decided false
            game.decision = Decision::False;
            <InstantiatedGames<T>>::insert(&game_id, game);

            Self::deposit_event(RawEvent::ClaimDecided(game_id, false));
        }

        /// Decide the game decision with given witness.
        /// TODO: weight
        #[weight = 100_000]
        fn decide_claim_with_witness(origin, game_id: T::Hash, witness: Vec<u8>) {
            let mut game = match Self::instantiated_games(&game_id) {
                Some(game) => game,
                None => Err(Error::<T>::DoesNotExistGame)?,
            };

            // Decision must be undecided
            ensure!(
                game.decision == Decision::Undecided,
                Error::<T>::DecisionMustBeUndecided,
            );
            // There must be no challenge
            ensure!(
                game.challenges.is_empty(),
                Error::<T>::ThereMustBeNoChallenge,
            );

            let property = match Self::predicates(&game.property.predicate_address) {
                Some(predicate) => predicate,
                None => Err(Error::<T>::DoesNotExistPredicate)?,
            };

            // TODO: property must be true with given witness
            // ensure!(property.decide_with_witness(game.property.inputs, witness),
            //     Error::<T>::PropertyMustBeTrue);

            game.decision = Decision::True;
            <InstantiatedGames<T>>::insert(&game_id, game);
            Self::deposit_event(RawEvent::ClaimDecided(game_id, true));
        }

        /// Removes a challenge when its decision has been evaluated to false.
        ///
        /// TODO: weight
        #[weight = 100_000]
        fn remove_challenge(origin, game_id: T::Hash, challenging_game_id: T::Hash) {
            let mut game = match Self::instantiated_games(&game_id) {
                Some(game) => game,
                None => Err(Error::<T>::DoesNotExistGame)?,
            };

            let challenging_game = match Self::instantiated_games(&challenging_game_id) {
                Some(game) => game,
                None => Err(Error::<T>::DoesNotExistGame)?,
            };

            // check _challenge is in _game.challenges
            let challenging_game_id = Self::get_property_id(&challenging_game.property);

            // challenge isn't valid
            ensure!(
                game.challenges
                    .iter()
                    .any(|challenge| challenge == &challenging_game_id),
                Error::<T>::ChallengeIsNotValid,
            );

            // _game.createdBlock > block.number - dispute
            // check _challenge have been decided true
            ensure!(
                challenging_game.decision == Decision::False,
                Error::<T>::ChallengingGameNotFalse,
            );

            // remove challenge
            game.challenges = game
                .challenges
                .into_iter()
                .filter(|challenge| challenge != &challenging_game_id)
                .collect();
            <InstantiatedGames<T>>::insert(&game_id, game);

            Self::deposit_event(RawEvent::ChallengeRemoved(game_id, challenging_game_id));
        }

        /// Set a predicate decision by called from Predicate itself.
        ///
        /// TODO: weight
        #[weight = 100_000]
        fn set_predicate_decision(origin, game_id: T::Hash, decision: bool) {
            let origin = ensure_signed(origin)?;
            let mut game = match Self::instantiated_games(&game_id) {
                Some(game) => game,
                None => Err(Error::<T>::DoesNotExistGame)?,
            };

            // only the prodicate can decide a claim
            ensure!(
                game.property.predicate_address == origin,
                Error::<T>::MustBeCalledFromPredicate,
            );

            if decision {
                game.decision = Decision::True;
            } else {
                game.decision = Decision::False;
            }
            Self::deposit_event(RawEvent::AtomicPropositionDecided(game_id, decision));
        }

        /// Challenge a game specified by gameId with a challengingGame specified by _challengingGameId.
        ///
        /// @param game_id challenged game id.
        /// @param challenge_inputs array of input to verify child of game tree.
        /// @param challenging_game_id child of game tree.
        ///
        /// TODO: weight
        #[weight = 100_000]
        fn challenge(origin, game_id: T::Hash, challenge_inputs: Vec<u8>, challenging_game_id: T::Hash) {
            let mut game = match Self::instantiated_games(&game_id) {
                Some(game) => game,
                None => Err(Error::<T>::DoesNotExistGame)?,
            };

            let challenging_game = match Self::instantiated_games(&challenging_game_id) {
                Some(game) => game,
                None => Err(Error::<T>::DoesNotExistGame)?,
            };

            // TODO: challenge isn't valid
            // ensure!(
            //     LogicalConnective(game.property.predicate_address).isValidChallenge(
            //         game.property.inputs,
            //         _challengeInputs,
            //         challengingGame.property
            //     ),
            //     Error::<T>::ChallengeIsNotValid,
            // );
            game.challenges.push(challenging_game_id.clone());
            Self::deposit_event(RawEvent::ClaimChallenged(game_id, challenging_game_id));
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
}

impl<T: Trait> Module<T> {
    fn calc_code_put_costs(code: &Vec<u8>) -> Weight {
        <Module<T>>::current_schedule()
            .put_code_per_byte_cost
            .saturating_mul(code.len() as Weight)
    }

    // ======= main ==========
    /// Perform a call to a specified contract.
    ///
    /// This function is similar to `Self::call`, but doesn't perform any address lookups and better
    /// suitable for calling directly from Rust.
    pub fn bare_call(origin: T::AccountId, dest: T::AccountId, input_data: Vec<u8>) -> ExecResult {
        Self::execute_ovm(origin, |ctx| ctx.call(dest, input_data))
    }

    fn execute_ovm(
        origin: T::AccountId,
        func: impl FnOnce(&mut ExecutionContext<T, PredicateOvm, PredicateLoader>) -> ExecResult,
    ) -> ExecResult {
        let cfg = Config::preload::<T>();
        let vm = PredicateOvm::new(&cfg.schedule);
        let loader = PredicateLoader::new(&cfg.schedule);
        let mut ctx = ExecutionContext::top_level(origin.clone(), &cfg, &vm, &loader);

        func(&mut ctx)
    }

    // ======= callable ======
    /// Get of true/false the decision of property.
    pub fn is_decided(property: &PropertyOf<T>) -> Decision {
        let game = match Self::instantiated_games(Self::get_property_id(property)) {
            Some(game) => game,
            None => return Decision::Undecided,
        };
        game.decision
    }

    /// Get of the instatiated challenge game from claim_id.
    pub fn get_game(claim_id: &T::Hash) -> Option<ChallengeGameOf<T>> {
        Self::instantiated_games(claim_id)
    }

    /// Get of the property id from the propaty itself.
    pub fn get_property_id(property: &PropertyOf<T>) -> T::Hash {
        T::Hashing::hash_of(property)
    }

    // ======= helper =======
    pub fn block_number() -> <T as system::Trait>::BlockNumber {
        <system::Module<T>>::block_number()
    }

    pub fn is_decidable(property_id: &T::Hash) -> bool {
        let game = match Self::instantiated_games(property_id) {
            Some(game) => game,
            None => return false,
        };

        if game.created_block > Self::block_number() - T::DisputePeriod::get() {
            return false;
        }

        // check all game.challenges should be false
        game.challenges.iter().all(|challenge| {
            if let Some(challenging_game) = Self::instantiated_games(challenge) {
                return challenging_game.decision == Decision::False;
            }
            false
        })
    }
}
