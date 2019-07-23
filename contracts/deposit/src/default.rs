use super::*;
use crate::no_std::ToString;
use commitment::traits::Commitment;
use ink_core::{memory::format, storage};
use primitives::{default::*, Verify};

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

impl Deposit {
    pub fn is_exist_exit(&self, exit_id: &Hash) -> bool {
        None != self.exit_redeemable_after.get(exit_id)
    }
    pub fn is_exist_checkpoints(&self, checkpoint_id: &Hash) -> bool {
        None != self.checkpoints.get(checkpoint_id)
    }
    pub fn is_exist_challenges(&self, challenge_id: &Hash) -> bool {
        None != self.challenges.get(challenge_id)
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

        self.total_deposited.set(Range { start: 0, end: 0 });
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
    fn start_checkpoint<T: Member + Codec, P: Member + Codec + commitment::traits::Verify>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<T>,
        inclusion_proof: P,
        deposited_range_id: RangeNumber,
    ) -> primitives::Result<CheckpointStarted<T>> {
        // verify the that checkpoint.stateUpdate was included with inclusionProof.
        if !self.commitment().verify_state_update_inclusion(
            env,
            &checkpoint.state_update,
            &inclusion_proof,
        ) {
            return Err(
                "error: verify the that checkpoint.stateUpdate was included with inclusionProof.",
            );
        }
        // verify that subRange is actually a sub-range of stateUpdate.range.
        if let Err(err) = checkpoint.verify() {
            return Err(
                "error: verify that subRange is actually a sub-range of stateUpdate.range.",
            );
        }
        // verify that the subRange is still exitable with the depositedRangeId .
        if let Some(exitable_range) = self.deposited_ranges.get(&deposited_range_id) {
            if !(exitable_range.start <= checkpoint.sub_range.start
                && checkpoint.sub_range.end <= exitable_range.end)
            {
                return Err(
                    "error: verify that the subRange is still exitable with the depositedRangeId.",
                );
            }
        } else {
            return Err(
				"error: verify that the subRange is still exitable with the depositedRangeId. Not found deposited_range_id.",
			);
        }

        // verify that an indentical checkpoint has not already been started.
        let checkpoint_id = checkpoint.id();
        if let Some(_) = self.checkpoints.get(&checkpoint_id) {
            return Err("error: verify that an indentical checkpoint has not already been started");
        }

        // add the new pending checkpoint to checkpoints with challengeableUntil equalling the current ethereum block.number + CHALLENGE_PERIOD.
        let challengeable_until = env.block_number() + self.CHALLENGE_PERIOD.get();
        self.checkpoints.insert(
            checkpoint_id,
            CheckpointStatus {
                challengeable_until: challengeable_until,
                outstanding_challenges: 0,
            },
        );

        // return that emitted a CheckpointStarted event.
        Ok(CheckpointStarted {
            checkpoint: checkpoint,
            challengeable_until: challengeable_until,
        })
    }

    /// Deletes an exit by showing that there exists a newer finalized checkpoint. Immediately cancels the exit.
    fn delete_exit_outdated<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        older_exit: Checkpoint<T>,
        newer_checkpoint: Checkpoint<T>,
    ) -> primitives::Result<()> {
        // Ensure the checkpoint ranges intersect.
        if !older_exit.is_intersect(&newer_checkpoint) {
            return Err("error: ensure the checkpoint ranges intersect.");
        }

        // Ensure that the plasma blocknumber of the _olderExitt is less than that of _newerCheckpoint.
        if older_exit.state_update.plasma_block_number
            >= newer_checkpoint.state_update.plasma_block_number
        {
            return Err(
				"error: ensure that the plasma blocknumber of the older_exitt is less than that of newer_checkpoint.",
			);
        }

        // Ensure that the newerCheckpoint has no challenges.
        let newer_checkpoint_id = newer_checkpoint.id();
        if let Some(true) = self.challenges.get(&newer_checkpoint_id) {
            return Err("error: ensure that the newerCheckpoint has no challenges.");
        }

        // Ensure that the newerCheckpoint is no longer challengeable.
        if let Some(checkpoint_status) = self.checkpoints.get(&newer_checkpoint_id) {
            if checkpoint_status.challengeable_until > env.block_number() {
                return Err("error: ensure that the newerCheckpoint is no longer challengeable.");
            }
        } else {
            return Err("error: ensure that the newerCheckpoint is no longer challengeable. Not found checkpoint_status.");
        }

        // Delete the entries in exitRedeemableAfter.
        let older_checkpoint_id = older_exit.id();
        self.exit_redeemable_after.remove(&older_checkpoint_id);

        Ok(())
    }

    /// Starts a challenge for a checkpoint by pointing to an exit that occurred in an earlier plasma block.
    /// Does not immediately cancel the checkpoint. Challenge can be blocked if the exit is cancelled.
    fn challenge_checkpoint<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        challenge: Challenge<T>,
    ) -> primitives::Result<()> {
        let challenged_id = challenge.challenged_checkpoint.id();
        let challenging_id = challenge.challenging_checkpoint.id();
        // Ensure that the checkpoint being used to challenge exists.
        if !self.is_exist_checkpoints(&challenged_id) {
            return Err("error: ensure that the checkpoint being used to challenge exists. Not found challenged checkpoints.");
        }
        if !self.is_exist_exit(&challenging_id) {
            return Err("error: ensure that the checkpoint being used to challenge exists. Not found challenging exit.");
        }

        // Ensure that the challenge ranges intersect.
        if !challenge
            .challenged_checkpoint
            .is_intersect(&challenge.challenging_checkpoint)
        {
            return Err("error: ensure that the challenge ranges intersect.");
        }

        // Ensure that the checkpoint being used to challenge has an older plasmaBlockNumber.
        if challenge
            .challenging_checkpoint
            .state_update
            .plasma_block_number
            >= challenge
                .challenged_checkpoint
                .state_update
                .plasma_block_number
        {
            return Err("error: ensure that the checkpoint being used to challenge has an older plasmaBlockNumber.");
        }

        // Ensure that an identical challenge is not already underway.
        let challenge_id = challenge.challenged_checkpoint.id();
        if None == self.challenges.get(&challenge_id) {
            return Err("error: ensure that an identical challenge is not already underway.");
        }

        let mut challenged_status = self.checkpoints.get(&challenged_id).unwrap().clone();
        // Ensure that the current ethereum block is not greater than the challengeableUntil block for the checkpoint being challenged.
        if challenged_status.challengeable_until <= env.block_number() {
            return Err("error: ensure that the current ethereum block is not greater than the challengeableUntil block for the checkpoint being challenged.");
        }

        // increment the outstandingChallenges for the challenged checkpoint.
        challenged_status.outstanding_challenges += 1;
        self.checkpoints.insert(challenged_id, challenged_status);

        // MUST set the challenges mapping for the challengeId to true.
        self.challenges.insert(challenge_id, true);

        Ok(())
    }

    /// Decrements the number of outstanding challenges on a checkpoint by showing that one of its challenges has been blocked.
    fn remove_challenge<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        challenge: Challenge<T>,
    ) -> primitives::Result<()> {
        // Check that the challenge was not already removed.
        let challenge_id = challenge.id();
        let challenging_id = challenge.challenging_checkpoint.id();
        let challenged_id = challenge.challenged_checkpoint.id();
        if self.is_exist_challenges(&challenge_id) {
            return Err("error: check that the challenge was not already removed.");
        }

        // Check that the challenging exit has since been removed.
        if self.is_exist_exit(&challenging_id) {
            return Err("error: check that the challenging exit has since been removed.");
        }

        // Remove the challenge if above conditions are met.
        self.challenges.insert(challenge_id, true);

        // Decrement the challenged checkpoint’s outstandingChallenges if the above conditions are met.
        let mut challenged_status = self.checkpoints.get(&challenged_id).unwrap().clone();
        challenged_status.outstanding_challenges -= 1;
        self.checkpoints.insert(challenged_id, challenged_status);
        Ok(())
    }

    /// Allows the predicate contract to start an exit from a checkpoint. Checkpoint may be pending or finalized.
    fn start_exit<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<T>,
    ) -> primitives::Result<ExitStarted> {
        let checkpoint_id = checkpoint.id();
        // Ensure the checkpoint exists.
        if !self.is_exist_checkpoints(&checkpoint_id) {
            return Err("error: Ensure the checkpoint exists.");
        }

        // Ensure an exit on the checkpoint is not already underway.
        if self.is_exist_exit(&checkpoint_id) {
            return Err("error: Ensure an exit on the checkpoint is not already underway.");
        }

        // Ensure that the msg.sender is the _checkpoint.stateUpdate.predicateAddress to authenticate the exit’s initiation.
        if checkpoint.state_update.state_object.predicate != env.address() {
            return Err("error: Ensure that the contract address is the checkpoint.state_update.predicate_address to authenticate the exit’s initiation.");
        }

        // Set the exit’s redeemableAfter status to the current Ethereum block.number + LOCKUP_PERIOD.
        let redeemable_after = env.block_number() + *self.EXIT_PERIOD;
        self.exit_redeemable_after
            .insert(checkpoint_id.clone(), redeemable_after);

        // Emit an exitStarted event.
        Ok(ExitStarted {
            exit: checkpoint_id,
            redeemable_after: redeemable_after,
        })
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
