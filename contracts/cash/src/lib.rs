//! Plasma-cash logic conforms to the PGSPec design.
//!
//! It is implemented in a Plasma cash contract.
//! PGSpec design are more abstract designs of PlasmaCash.
//! It is possible to describe PlasmaCash simply as PGSpec default implementation.
//! Also, this contract can be used as an example of implementation of Plasma contract that conforms to PGSpec.
//!
//! Standard PlasmaCash Contract implementation.

#![cfg_attr(not(any(test, feature = "std")), no_std)]

use commitment::{traits::Commitment, MerkleIntervalTreeInternalNode};
use core::option::Option;
use deposit::traits::Deposit;
use ink_core::{
    env::{ContractEnv, DefaultSrmlTypes, EnvTypes},
    memory::{format, vec::Vec},
    storage,
};
use ink_lang::contract;
use predicate::{
    ownership::{Signature, TransactionBody},
    traits::Predicate,
};
use primitives::{default::*, events::*, traits};
use scale::Decode;

contract! {
    #![env = ink_core::env::DefaultSrmlTypes]

    /// Cash Plasma Standard Contract.
	struct Cash {
        /// The current state of our flag.
        predicate: predicate::ownership::Predicate,
    }

    impl Deploy for Cash {
        /// Initializes our state to `false` upon deploying our smart contract.
        fn deploy(&mut self,
            token_address: AccountId,
            chalenge_period: BlockNumber,
            exit_period: BlockNumber,
        ) {
            self.predicate.deploy(env, token_address, chalenge_period, exit_period);
        }
    }

    impl Cash {
        /// Allows a user to submit a block with the given header(merkle root).
        /// Generally, allows a operator only execute this fucntion.
        pub(external) fn submit_block(&mut self, header: Hash) {
            match self.predicate.commitment().submit_block(env, header) {
                Ok(result) => env.emit(result),
                Err(err) => env.println(err),
            }
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
        pub(external) fn start_checkpoint(&mut self,
            checkpoint: Vec<u8>,
            inclusion_proof: Vec<u8>,
            deposited_range_id: u128) {
            let checkpoint = Checkpoint::<AccountId>::decode(&mut &checkpoint[..]).expect("expect Checkpoint checkpoint");
            let inclusion_proof = commitment::InclusionProof::<RangeNumber>::decode(&mut &inclusion_proof[..]).expect("expect inclusionProof inclusion_proof");
            match self.predicate.deposit().start_checkpoint(env, checkpoint, inclusion_proof, deposited_range_id) {
                Ok(result) => env.emit(result),
                Err(err) => env.println(err),
            }
        }

        /// Deletes an exit by showing that there exists a newer finalized checkpoint. Immediately cancels the exit.
        ///
        /// If a checkpoint game has finalized, the safety property should be that nothing is valid in that
        /// range’s previous blocks–”the history has been erased.” However,
        /// since there still might be some `StateUpdates` included in the blocks prior,
        /// invalid checkpoints can be initiated. This method allows the rightful owner to
        /// demonstrate that the initiated `older_checkpoint` is invalid and must be deleted.
        pub(external) fn delete_exit_outdated(&mut self,
            older_exit: Vec<u8>,
            newer_checkpoint: Vec<u8>) {
            let older_exit = Checkpoint::<AccountId>::decode(&mut &older_exit[..]).expect("expect Checkpoint<AccountId> older_exit.");
            let newer_checkpoint = Checkpoint::<AccountId>::decode(&mut &newer_checkpoint[..]).expect("expect <AccountId> newer_exit.");
            if let Err(err) = self.predicate.deposit().delete_exit_outdated(env, older_exit, newer_checkpoint) {
                env.println(err);
            }
        }

        /// Starts a challenge for a checkpoint by pointing to an exit that occurred in
        /// an earlier plasma block. Does not immediately cancel the checkpoint. Challenge can be blocked if the exit is cancelled.
        ///
        /// If the operator includes an invalid `StateUpdate`
        /// (i.e. there is not a deprecation for the last valid `StateUpdate` on an intersecting range),
        /// they may checkpoint it and attempt a malicious exit.
        /// To prevent this, the valid owner must checkpoint their unspent state,
        /// exit it, and create a challenge on the invalid checkpoint.
        pub(external) fn challenge_checkpoint(&mut self, challenge: Vec<u8>) {
        	let challenge = Challenge::<AccountId>::decode(&mut &challenge[..]).expect("expect Challenge<AccontId> challenge");
            if let Err(err) = self.predicate.deposit().challenge_checkpoint(env, challenge) {
                env.println(err);
            }
        }

        /// Decrements the number of outstanding challenges on a checkpoint by showing that one of its challenges has been blocked.
        ///
        /// Anyone can exit a prior state which was since spent and use it to challenge despite it being deprecated.
        /// To remove this invalid challenge, the challenged checkpointer may demonstrate the exit is deprecated,
        /// deleting it, and then call this method to remove the challenge.
        pub(external) fn remove_challenge(&mut self, challenge: Vec<u8>) {
        	let challenge = Challenge::<AccountId>::decode(&mut &challenge[..]).expect("challenge Challenge<AccountId> challenge");
            if let Err(err) = self.predicate.deposit().remove_challenge(env, challenge) {
                env.println(err);
            }
        }

        /// Allows the predicate contract to start an exit from a checkpoint. Checkpoint may be pending or finalized.
        ///
        /// For a user to redeem state from the plasma chain onto the main chain,
        /// they must checkpoint it and respond to all challenges on the checkpoint,
        /// and await a `EXIT_PERIOD` to demonstrate that the checkpointed subrange has not been deprecated. This is the method which starts the latter process on a given checkpoint.
        pub(external) fn start_exit(&mut self,	checkpoint: Vec<u8>){
        	let checkpoint = Checkpoint::<AccountId>::decode(&mut &checkpoint[..]).expect("expect Checkpoint<AccountId> checkpoint");
            match self.predicate.start_exit(env, checkpoint) {
                Ok(result) => env.emit(result),
                Err(err) => env.println(err),
            }
        }

        /// Allows the predicate address to cancel an exit which it determines is deprecated.
        pub(external) fn deprecate_exit(&mut self,
            deprecated_exit: Vec<u8>,
            transaction: Vec<u8>,
            witness: Vec<u8>,
            post_state: Vec<u8>) {
            let deprecated_exit = Checkpoint::<AccountId>::decode(&mut &deprecated_exit[..]).expect("expect Checkpoint<AccountId> deprecated_exit");
            let transaction = Transaction::<TransactionBody>::decode(&mut &transaction[..]).expect("expect Transaction<TransactionBody> transaction");
            let witness = Signature::decode(&mut &witness[..]).expect("expect Signature witness");
            let post_state = StateUpdate::<AccountId>::decode(&mut &post_state[..]).expect("expect StateUpdate<AccountId> post_state");
            if let Err(err) = self.predicate.deprecate_exit(env, deprecated_exit, transaction, witness, post_state) {
                env.println(err);
            }
        }

        /// Finalizes an exit that has passed its exit period and has not been successfully challenged.
        pub(external) fn finalize_exit(&mut self,
            checkpoint: Vec<u8>,
            deposited_range_id: u128) {
            let checkpoint = Checkpoint::<AccountId>::decode(&mut &checkpoint[..]).expect("expect Checkpoint<AccountId> checkpoint");
            match self.predicate.finalize_exit(env, checkpoint, deposited_range_id) {
                Ok(result) => env.emit(result),
                Err(err) => env.println(err),
            }
        }

        // ================================== Getter ===================================================
        ///	Gets Leatest Plasma block number.
        pub(external) fn current_block(&self) -> BlockNumber {
            self.predicate.commitment_ref().current_block(env)
        }


		/// Gets Plasma block hash by identified block number.
        pub(external) fn block_hash(&self, number: BlockNumber) -> Option<Hash> {
            self.predicate.commitment_ref().block_hash(env, number)
        }
    }
}

pub trait EmitEventExt {
    /// Emits the given event.
    fn emit<E>(&self, event: E)
    where
        E: Into<public::Event<<ContractEnv<DefaultSrmlTypes> as EnvTypes>::AccountId>>,
    {
        use scale::Encode as _;
        <ink_core::env::ContractEnv<DefaultSrmlTypes> as ink_core::env::Env>::deposit_raw_event(
            &[],
            event.into().encode().as_slice(),
        )
    }
}

impl EmitEventExt for ink_model::EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>> {}

#[cfg(all(test, feature = "test-env"))]
mod tests {
    use super::*;

    fn get_token_address() -> AccountId {
        AccountId::decode(&mut &[2u8; 32].to_vec()[..]).expect("account id decoded.")
    }
    fn get_sender_address() -> AccountId {
        AccountId::decode(&mut &[3u8; 32].to_vec()[..]).expect("account id decoded.")
    }
    fn get_receiver_address() -> AccountId {
        AccountId::decode(&mut &[4u8; 32].to_vec()[..]).expect("account id decoded.")
    }

    #[test]
    fn it_works() {
        let mut contract = Cash::deploy_mock(get_token_address(), 5, 5);
        assert_eq!(contract.current_block(), 0);
    }
}
