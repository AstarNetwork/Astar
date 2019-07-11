#![cfg_attr(not(any(test, feature = "std")), no_std)]

use ink_core::{
    memory::{format, string::String, vec::Vec},
    storage,
};

use ink_lang::contract;
use ink_model::gen_input_data::gen_input_data;
use primitives::*;

contract! {
    #![env = ink_core::env::DefaultSrmlTypes]

    event CheckpointStarted{
        checkpoint: Checkpoint,
        challengeable_until: BlockNumber,
    }

    event CheckpointChallenged{
        challenge: Challenge,
    }

    event CheckpointFinalized{
        checkpoint: Hash,
    }

    event ExitStarted{
        exit: Hash,
        redeemable_after: BlockNumber,
    }

    event ExitFinalized{
        exit: Checkpoint,
    }

    struct Deposit {
        //constant values
        COMMITMENT_ADDRESS: storage::Value<AccountId>,
        //MUST be an adress of ERC20 token
        TOKEN_ADDRES: storage::Value<AccountId>,
        CHALLENGE_PERIOD: storage::Value<BlockNumber>,
        EXIT_PERIOD: storage::Value<BlockNumber>,

        //changable values
        total_deposited: storage::Value<Range>,
        checkpoints: storage::HashMap<Hash,CheckpointStatus>,
        deposited_ranges: storage::HashMap<RangeNumber, Range>,
        exit_redeemable_after: storage::HashMap<Hash,BlockNumber>,
        challenges: storage::HashMap<Hash,bool>,
    }

    impl Deploy for Deposit {
        fn deploy(&mut self , init_ac: AccountId) {
            self.TOKEN_ADDRES.set(init_ac);
        }
    }

    impl Deposit {
        /// Allows a user to submit a deposit to the contract.
        pub(external) fn deposit(&mut self, depositer: AccountId, amount: Balance, initial_state: StateObject){
            //MUST keep track of the total deposited assets, totalDeposited.
            //MUST transfer the deposited amount from the depositer to the deposit contract’s address.
//            let params = vec!([Box::new(depositer),Box::new(env.address()),Box::new(amount)]);
//            let sent:bool = env.call(TOKEN_ADDRES, 0, 0, gen_input_data("transfer_from",params));

            //MUST create a state update with a state object equal to the provided initialState.
//            let state_update = StateUpdate{
//                range: Range,
//                state_object: initial_state,
//                plasma_contract,
//                plasma_block_number,
//            };
            //MUST compute the range of the created state update as totalDeposited to totalDeposited + amount.


            //MUST update the total amount deposited after the deposit is handled.
//			*self.total_deposited = *self.total_deposited + amount;

            //MUST insert the created state update into the checkpoints mapping with challengeableUntil being the current block number - 1.



            //MUST emit a CheckpointFinalized event for the inserted checkpoint.
//            env.emit(
//                CheckpointFinalized{
//                    checkpoint,
//                }
//            );
        }

        /// Starts a checkpoint for a given state update.
        pub(external) fn start_checkpoint(&mut self,
            checkpoint: Checkpoint,
            inclusion_proof: Vec<u8>,
            deposited_range_id: RangeNumber,
        ) {
           // MUST verify the that checkpoint.stateUpdate was included with inclusionProof.
           // MUST verify that subRange is actually a sub-range of stateUpdate.range.
           // MUST verify that the subRange is still exitable with the depositedRangeId .
           // MUST verify that an indentical checkpoint has not already been started.
           // MUST add the new pending checkpoint to checkpoints with challengeableUntil equalling the current ethereum block.number + CHALLENGE_PERIOD .
           // MUST emit a CheckpointStarted event.
        }

        /// Deletes an exit by showing that there exists a newer finalized checkpoint. Immediately cancels the exit.
        pub(external) fn delete_exit_outdated(&mut self,
            older_exit: Checkpoint,
            newer_checkpoint: Checkpoint,
        ) {
            // MUST ensure the checkpoint ranges intersect.
            // MUST ensure that the plasma blocknumber of the _olderExitt is less than that of _newerCheckpoint.
            // MUST ensure that the newerCheckpoint has no challenges.
            // MUST ensure that the newerCheckpoint is no longer challengeable.
            // MUST delete the entries in exitRedeemableAfter.
        }

        /// Starts a challenge for a checkpoint by pointing to an exit that occurred in an earlier plasma block.
        /// Does not immediately cancel the checkpoint. Challenge can be blocked if the exit is cancelled.
        pub(external) fn challenge_checkpoint(&mut self,
            challenge: Challenge,
        ) {
            // MUST ensure that the checkpoint being used to challenge exists.
            // MUST ensure that the challenge ranges intersect.
            // MUST ensure that the checkpoint being used to challenge has an older plasmaBlockNumber.
            // MUST ensure that an identical challenge is not already underway.
            // MUST ensure that the current ethereum block is not greater than the challengeableUntil block for the checkpoint being challenged.
            // MUST increment the outstandingChallenges for the challenged checkpoint.
            // MUST set the challenges mapping for the challengeId to true.
        }

        /// Decrements the number of outstanding challenges on a checkpoint by showing that one of its challenges has been blocked.
        pub(external) fn remove_challenge(&mut self,
            challenge: Challenge,
        ) {
            // MUST check that the challenge was not already removed.
            // MUST check that the challenging exit has since been removed.
            // MUST remove the challenge if above conditions are met.
            // MUST decrement the challenged checkpoint’s outstandingChallenges if the above conditions are met.
        }


        /// Allows the predicate contract to start an exit from a checkpoint. Checkpoint may be pending or finalized.
        pub(external) fn start_exit(&mut self, checkpoint: Checkpoint) {
            // MUST ensure the checkpoint exists.
            // MUST ensure that the msg.sender is the _checkpoint.stateUpdate.predicateAddress to authenticate the exit’s initiation.
            // MUST ensure an exit on the checkpoint is not already underway.
            // MUST set the exit’s redeemableAfter status to the current Ethereum block.number + LOCKUP_PERIOD.
            // MUST emit an exitStarted event.
        }

        /// Allows the predicate address to cancel an exit which it determines is deprecated.
        pub(external) fn deprecate_exit(&mut self,
            checkpoint: Checkpoint,
        ){
            // MUST ensure the msg.sender is the _checkpoint.stateUpdate.predicateAddress to ensure the deprecation is authenticated.
            // MUST delete the exit from exitRedeemableAfter at the checkpointId .
        }

        /// Finalizes an exit that has passed its exit period and has not been successfully challenged.
        pub(external) fn finalize_exit(&mut self,
            exit: Checkpoint,
            deposited_range_id: RangeNumber,
        ) {
            // MUST ensure that the exit finalization is authenticated from the predicate by msg.sender == _exit.stateUpdate.state.predicateAddress.
            // MUST ensure that the checkpoint is finalized (current Ethereum block exceeds checkpoint.challengeableUntil).
            // MUST ensure that the checkpoint’s outstandingChallenges is 0.
            // MUST ensure that the exit is finalized (current Ethereum block exceeds redeemablAfter ).
            // MUST ensure that the checkpoint is on a subrange of the currently exitable ranges via depositedRangeId.
            // MUST make an ERC20 transfer of the end - start amount to the predicate address.
            // MUST delete the exit.
            // MUST remove the exited range by updating the depositedRanges mapping.
            // MUST delete the checkpoint.
            // MUST emit an exitFinalized event.
        }

    }
}

#[cfg(all(test, feature = "test-env"))]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut contract = Deposit::deploy_mock();
    }
}
