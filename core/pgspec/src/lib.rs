#![cfg_attr(not(feature = "std"), no_std)]

use parity_codec::{Codec, Decode, Encode};
use sr_primitives::traits::{MaybeDisplay, MaybeSerializeDebug, Member, SimpleArithmetic, Verify};
use support::{decl_event, decl_module, decl_storage, dispatch::Result, Parameter, StorageValue};
use system::ensure_signed;

pub mod traits;
pub mod types;

/// tests for this module
#[cfg(test)]
mod tests;

/// The module's configuration trait.
pub trait Trait: contract::Trait {
    type RangeNumber: Member + Parameter + SimpleArithmetic + Default + Copy;
    type Data: Member + Parameter + Default;
    type TxBody: Member + Parameter + Default;
    type Signature: Parameter + Default + Verify<Signer = Self::AccountId>;

    // The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

pub type Range<T> = types::Range<<T as Trait>::RangeNumber>;
pub type StateObject<T> = types::StateObject<<T as system::Trait>::AccountId, <T as Trait>::Data>;
pub type StateUpdate<T> = types::StateUpdate<
    <T as system::Trait>::AccountId,
    <T as Trait>::Data,
    <T as Trait>::RangeNumber,
    <T as system::Trait>::BlockNumber,
>;
pub type Checkpoint<T> = types::Checkpoint<
    <T as system::Trait>::AccountId,
    <T as Trait>::Data,
    <T as Trait>::RangeNumber,
    <T as system::Trait>::BlockNumber,
>;
pub type Transaction<T> = types::Transaction<
    <T as system::Trait>::AccountId,
    <T as Trait>::TxBody,
    <T as Trait>::RangeNumber,
>;
pub type Challenge<T> = types::Challenge<
    <T as system::Trait>::AccountId,
    <T as Trait>::Data,
    <T as Trait>::RangeNumber,
    <T as system::Trait>::BlockNumber,
>;
pub type InclusionProof<T> =
    types::InclusionProof<<T as Trait>::RangeNumber, <T as system::Trait>::Hash>;

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as PGSpec {
        PlasmaCode get(plasma_code): Option<Vec<u8>>;
    }
}

// The module's dispatchable functions.
decl_module! {
    // The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event<T>() = default;

        /// Allows a user to submit a block with the given header(merkle root).
        /// Generally, allows a operator only execute this fucntion.
        pub fn submit_block(origin, header: T::Hash) -> Result {
            Ok(())
        }

        /// Starts a checkpoint for a given state update.
        ///
        /// Checkpoints are assertions that a certain state update occured/was included,
        /// and that it has no intersecting unspent state updates in its history.
        /// Because the operator may publish an invalid block,
        /// it must undergo a challenge period in which the parties who care about the unspent state update in the history exit it,
        /// and use it to challenge the checkpoint.
        ///
        /// deposited_range_id is end of deposited_ranges.
        pub fn start_checkpoint(origin,
            checkpoint: Checkpoint<T>,
            inclusion_proof: InclusionProof<T>,
            deposited_range_id: T::RangeNumber) -> Result {
            Ok(())
        }

        /// Deletes an exit by showing that there exists a newer finalized checkpoint. Immediately cancels the exit.
        ///
        /// If a checkpoint game has finalized, the safety property should be that nothing is valid in that
        /// range’s previous blocks–”the history has been erased.” However,
        /// since there still might be some `StateUpdates` included in the blocks prior,
        /// invalid checkpoints can be initiated. This method allows the rightful owner to
        /// demonstrate that the initiated `older_checkpoint` is invalid and must be deleted.
        pub fn delete_exit_outdated(origin,
            older_exit: Checkpoint<T>,
            newer_checkpoint: Checkpoint<T>) -> Result {
            Ok(())
        }

        /// Starts a challenge for a checkpoint by pointing to an exit that occurred in
        /// an earlier plasma block. Does not immediately cancel the checkpoint. Challenge can be blocked if the exit is cancelled.
        ///
        /// If the operator includes an invalid `StateUpdate`
        /// (i.e. there is not a deprecation for the last valid `StateUpdate` on an intersecting range),
        /// they may checkpoint it and attempt a malicious exit.
        /// To prevent this, the valid owner must checkpoint their unspent state,
        /// exit it, and create a challenge on the invalid checkpoint.
        pub fn challenge_checkpoint(origin, challenge: Challenge<T>) -> Result {
            Ok(())
        }

        /// Decrements the number of outstanding challenges on a checkpoint by showing that one of its challenges has been blocked.
        ///
        /// Anyone can exit a prior state which was since spent and use it to challenge despite it being deprecated.
        /// To remove this invalid challenge, the challenged checkpointer may demonstrate the exit is deprecated,
        /// deleting it, and then call this method to remove the challenge.
        pub fn remove_challenge(origin, challenge: Challenge<T>) -> Result {
            Ok(())
        }

        /// Allows the predicate contract to start an exit from a checkpoint. Checkpoint may be pending or finalized.
        ///
        /// For a user to redeem state from the plasma chain onto the main chain,
        /// they must checkpoint it and respond to all challenges on the checkpoint,
        /// and await a `EXIT_PERIOD` to demonstrate that the checkpointed subrange has not been deprecated. This is the method which starts the latter process on a given checkpoint.
        pub fn start_exit(origin, checkpoint: Checkpoint<T>) -> Result {
            Ok(())
        }

        /// Allows the predicate address to cancel an exit which it determines is deprecated.
        pub fn deprecate_exit(origin,
            deprecated_exit: Checkpoint<T>,
            transaction: Transaction<T>,
            witness: T::Signature,
            post_state: StateUpdate<T>) -> Result {
            Ok(())
        }

        /// Finalizes an exit that has passed its exit period and has not been successfully challenged.
        pub fn finalize_exit(origin,
            checkpoint: Checkpoint<T>,
            deposited_range_id: T::RangeNumber) -> Result {
            Ok(())
        }

        ///	Gets Leatest Plasma block number.
        pub fn current_block(origin) -> Result {
            Ok(())
        }

        /// Gets Plasma block hash by identified block number.
        pub fn block_hash(origin, number: T::BlockNumber) -> Result {
            Ok(())
        }

        pub fn deploy(origin,
            #[compact] endowment: contract::BalanceOf<T>,
            #[compact] gas_limit: contract::Gas,
            code_hash: contract::CodeHash<T>,
            token_address: T::AccountId,
            challenge_period: T::BlockNumber,
            exit_period: T::BlockNumber) -> Result {
            <contract::Module<T>>::create(origin,
                endowment,
                gas_limit,
                code_hash,
                (token_address, challenge_period, exit_period).encode())
        }
    }
}

decl_event!(
    /// An event in this module.
    pub enum Event<T>
    where
        BlockNumber = <T as system::Trait>::BlockNumber,
    {
        /// Transaction was executed successfully
        CurrentBlock(BlockNumber),
    }
);
