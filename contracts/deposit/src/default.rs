use super::*;
use ink_core::{memory::format, storage};
use primitives::default::*;

ink_model::state! {
    pub struct Deposit {
        COMMITMENT: commitment::default::Commitment,

        //MUST be an address of ERC20 token
        TOKEN_ADDRES: storage::Value<AccountId>,
        CHALLENGE_PERIOD: storage::Value<BlockNumber>,
        EXIT_PERIOD: storage::Value<BlockNumber>,

        //changable values
        total_deposited: storage::Value<Range>,
        checkpoints: storage::HashMap<Hash, CheckpointStatus>,
        deposited_ranges: storage::HashMap<RangeNumber, Range>,
        exit_redeemable_after: storage::HashMap<Hash, BlockNumber>,
        challenges: storage::HashMap<Hash, bool>,
    }
}

impl traits::Deposit<RangeNumber, commitment::default::Commitment> for Deposit {
    fn deploy(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        token_address: AccountId,
        chalenge_period: BlockNumber,
        exit_period: BlockNumber,
    ) {
        //MUST be an address of ERC20 token
        self.TOKEN_ADDRES.set(token_address);
        self.CHALLENGE_PERIOD.set(chalenge_period);
        self.EXIT_PERIOD.set(exit_period);
    }

    fn deposit<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        depositer: AccountId,
        amount: Balance,
        initial_state: StateObject<T>,
    ) {
    }

    /// Starts a checkpoint for a given state update.
    // MUST verify the that checkpoint.stateUpdate was included with inclusionProof.
    // MUST verify that subRange is actually a sub-range of stateUpdate.range.
    // MUST verify that the subRange is still exitable with the depositedRangeId .
    // MUST verify that an indentical checkpoint has not already been started.
    // MUST add the new pending checkpoint to checkpoints with challengeableUntil equalling the current ethereum block.number + CHALLENGE_PERIOD .
    // MUST emit a CheckpointStarted event.
    fn start_checkpoint<T: Member + Codec, P: Member + Codec + commitment::traits::Verify>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<T>,
        inclusion_proof: P,
        deposited_range_id: RangeNumber,
    ) {
    }

    /// Deletes an exit by showing that there exists a newer finalized checkpoint. Immediately cancels the exit.
    // MUST ensure the checkpoint ranges intersect.
    // MUST ensure that the plasma blocknumber of the _olderExitt is less than that of _newerCheckpoint.
    // MUST ensure that the newerCheckpoint has no challenges.
    // MUST ensure that the newerCheckpoint is no longer challengeable.
    // MUST delete the entries in exitRedeemableAfter.
    fn delete_exit_outdated<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        older_exit: Checkpoint<T>,
        newer_checkpoint: Checkpoint<T>,
    ) {
    }

    /// Starts a challenge for a checkpoint by pointing to an exit that occurred in an earlier plasma block.
    /// Does not immediately cancel the checkpoint. Challenge can be blocked if the exit is cancelled.
    /// MUST ensure that the checkpoint being used to challenge exists.
    ///
    // MUST ensure that the challenge ranges intersect.
    // MUST ensure that the checkpoint being used to challenge has an older plasmaBlockNumber.
    // MUST ensure that an identical challenge is not already underway.
    // MUST ensure that the current ethereum block is not greater than the challengeableUntil block for the checkpoint being challenged.
    // MUST increment the outstandingChallenges for the challenged checkpoint.
    // MUST set the challenges mapping for the challengeId to true.
    fn challenge_checkpoint<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        challenge: Challenge<T>,
    ) {
    }

    /// Decrements the number of outstanding challenges on a checkpoint by showing that one of its challenges has been blocked.
    // MUST check that the challenge was not already removed.
    // MUST check that the challenging exit has since been removed.
    // MUST remove the challenge if above conditions are met.
    // MUST decrement the challenged checkpoint’s outstandingChallenges if the above conditions are met.
    fn remove_challenge<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        challenge: Challenge<T>,
    ) {
    }

    /// Allows the predicate contract to start an exit from a checkpoint. Checkpoint may be pending or finalized.
    // MUST ensure the checkpoint exists.
    // MUST ensure that the msg.sender is the _checkpoint.stateUpdate.predicateAddress to authenticate the exit’s initiation.
    // MUST ensure an exit on the checkpoint is not already underway.
    // MUST set the exit’s redeemableAfter status to the current Ethereum block.number + LOCKUP_PERIOD.
    // MUST emit an exitStarted event.
    fn start_exit<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<T>,
    ) {
    }

    /// Allows the predicate address to cancel an exit which it determines is deprecated.
    // MUST ensure the msg.sender is the _checkpoint.stateUpdate.predicateAddress to ensure the deprecation is authenticated.
    // MUST delete the exit from exitRedeemableAfter at the checkpointId .
    fn deprecate_exit<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<T>,
    ) {
    }

    /// Finalizes an exit that has passed its exit period and has not been successfully challenged.
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
    fn finalize_exit<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        exit: Checkpoint<T>,
        deposited_range_id: RangeNumber,
    ) {
    }

    fn commitment(&self) -> &commitment::default::Commitment {
        &self.COMMITMENT
    }
}
