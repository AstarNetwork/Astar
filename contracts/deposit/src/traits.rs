use super::*;
use ink_model::ContractState;
use primitives::*;

pub trait Deposit<I, C>: ContractState
where
    I: Member + SimpleArithmetic + Codec,
    C: commitment::traits::Commitment,
{
    /// Initilizes our state to `current_block is 0` upon deploying our smart contract.
    fn deploy(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        token_address: AccountId,
        chalenge_period: BlockNumber,
        exit_period: BlockNumber,
    );

    /// Allows a user to submit a deposit to the contract.
    /// 		//MUST keep track of the total deposited assets, totalDeposited.
    //		//MUST transfer the deposited amount from the depositer to the deposit contract’s address.
    ////            let params = vec!([Box::new(depositer),Box::new(env.address()),Box::new(amount)]);
    ////            let sent:bool = env.call(TOKEN_ADDRES, 0, 0, gen_input_data("transfer_from",params));
    //
    //		//MUST create a state update with a state object equal to the provided initialState.
    ////            let state_update = StateUpdate{
    ////                range: Range,
    ////                state_object: initial_state,
    ////                plasma_contract,
    ////                plasma_block_number,
    ////            };
    //		//MUST compute the range of the created state update as totalDeposited to totalDeposited + amount.
    //
    //
    //		//MUST update the total amount deposited after the deposit is handled.
    ////			*self.total_deposited = *self.total_deposited + amount;
    //
    //		//MUST insert the created state update into the checkpoints mapping with challengeableUntil being the current block number - 1.
    //
    //
    //
    //		//MUST emit a CheckpointFinalized event for the inserted checkpoint.
    ////            env.emit(
    ////                CheckpointFinalized{
    ////                    checkpoint,
    ////                }
    ////            );
    fn deposit<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        depositer: AccountId,
        amount: Balance,
        initial_state: StateObject<T>,
    ) -> Result<CheckpointFinalized>;

    /// Starts a checkpoint for a given state update.
    // MUST verify the that checkpoint.stateUpdate was included with inclusionProof.
    // MUST verify that subRange is actually a sub-range of stateUpdate.range.
    // MUST verify that the subRange is still exitable with the depositedRangeId .
    // MUST verify that an indentical checkpoint has not already been started.
    // MUST add the new pending checkpoint to checkpoints with challengeableUntil equalling the current ethereum block.number + CHALLENGE_PERIOD .
    // MUST emit a CheckpointStarted event.
    fn start_checkpoint<T: Member + Codec, P: Member + commitment::traits::Verify + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<T, I>,
        inclusion_proof: P,
        deposited_range_id: I,
    ) -> Result<CheckpointStarted<T>>;

    /// Deletes an exit by showing that there exists a newer finalized checkpoint. Immediately cancels the exit.
    // MUST ensure the checkpoint ranges intersect.
    // MUST ensure that the plasma blocknumber of the _olderExitt is less than that of _newerCheckpoint.
    // MUST ensure that the newerCheckpoint has no challenges.
    // MUST ensure that the newerCheckpoint is no longer challengeable.
    // MUST delete the entries in exitRedeemableAfter.
    fn delete_exit_outdated<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        older_exit: Checkpoint<T, I>,
        newer_checkpoint: Checkpoint<T, I>,
    ) -> Result<()>;

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
        challenge: Challenge<T, I>,
    ) -> Result<()>;

    /// Decrements the number of outstanding challenges on a checkpoint by showing that one of its challenges has been blocked.
    // MUST check that the challenge was not already removed.
    // MUST check that the challenging exit has since been removed.
    // MUST remove the challenge if above conditions are met.
    // MUST decrement the challenged checkpoint’s outstandingChallenges if the above conditions are met.
    fn remove_challenge<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        challenge: Challenge<T, I>,
    ) -> Result<()>;

    /// Allows the predicate contract to start an exit from a checkpoint. Checkpoint may be pending or finalized.
    // MUST ensure the checkpoint exists.
    // MUST ensure that the msg.sender is the _checkpoint.stateUpdate.predicateAddress to authenticate the exit’s initiation.
    // MUST ensure an exit on the checkpoint is not already underway.
    // MUST set the exit’s redeemableAfter status to the current Ethereum block.number + LOCKUP_PERIOD.
    // MUST emit an exitStarted event.
    fn start_exit<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<T, I>,
    ) -> Result<ExitStarted>;

    /// Allows the predicate address to cancel an exit which it determines is deprecated.
    // MUST ensure the msg.sender is the _checkpoint.stateUpdate.predicateAddress to ensure the deprecation is authenticated.
    // MUST delete the exit from exitRedeemableAfter at the checkpointId .
    fn deprecate_exit<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<T, I>,
    ) -> Result<()>;

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
        exit: Checkpoint<T, I>,
        deposited_range_id: I,
    ) -> Result<ExitFinalized<T>>;

    fn commitment(&mut self) -> &mut C;
}
