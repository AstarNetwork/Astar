//! The default implementation of the Deposit Contract which conforms to PGSpec.
//! The details of implementation, Please reference the description of trait.rs.

use super::*;
use commitment::traits::Commitment;
use ink_core::{
    memory::format,
    storage,
};
use primitives::{default::*, Verify};

ink_model::state! {
    pub struct Deposit {
    	/// Address of the commitment contract where block headers for the plasma chain are being published.
        COMMITMENT: commitment::default::Commitment,

        /// Address of the ERC-20 token which this deposit contract custodies.
        TOKEN_ADDRES: storage::Value<AccountId>,

        /// Number of Ethereum blocks for which a checkpoint may be challenged.
        CHALLENGE_PERIOD: storage::Value<BlockNumber>,

        /// Number of Ethereum blocks before an exit can be finalized.
        EXIT_PERIOD: storage::Value<BlockNumber>,

        /// Total amount deposited into this contract.
        total_deposited: storage::Value<RangeNumber>,

        /// Mapping from the ID of a checkpoint to the checkpoint’s status.
        checkpoints: storage::HashMap<Hash, CheckpointStatus>,

        /// Stores the list of ranges that have not been exited as a mapping from the start of a range to the full range.
        /// Prevents multiple exits from the same range of objects.
        deposited_ranges: storage::HashMap<RangeNumber, Range>,

        /// Mapping from the ID of an exit to the Ethereum block after which the exit can be finalized.
        exit_redeemable_after: storage::HashMap<Hash, BlockNumber>,

        /// Mapping from the ID of a challenge to whether or not the challenge is currently active.
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

    pub fn is_checkpoint_finalized(&self, checkpoint_id: &Hash, blk_num: &BlockNumber) -> bool {
        if let Some(chk_status) = self.checkpoints.get(checkpoint_id) {
            return chk_status.outstanding_challenges == 0
                && chk_status.challengeable_until < *blk_num;
        }
        false
    }

    pub fn extend_deposited_ranges(&mut self, amount: Balance) {
        let total_deposited = self.total_deposited.get().clone();
        let old_range = self
            .deposited_ranges
            .get(&total_deposited)
            .unwrap_or(&Range {
                start: total_deposited.clone(),
                end: total_deposited.clone(),
            })
            .clone();

        // Set the newStart for the last range
        let new_start: RangeNumber;
        if old_range.start == 0 && old_range.end == 0 {
            // Case 1: We are creating a new range (this is the case when the rightmost range has been removed)
            new_start = self.total_deposited.get().clone();
        } else {
            // Case 2: We are extending the old range (deleting the old range and making a new one with the total length)
            self.deposited_ranges.remove(&old_range.end);
            new_start = old_range.start;
        }

        // Set the newEnd to the totalDeposited plus how much was deposited
        let new_end: RangeNumber = total_deposited + amount as u128;
        // Finally create and store the range!
        self.deposited_ranges.insert(
            new_end.clone(),
            Range {
                start: new_start,
                end: new_end,
            },
        );
        // Increment total deposited now that we've extended our depositedRanges
        self.total_deposited.set(total_deposited + amount as u128);
    }

    /// This function is called when an exit is finalized to "burn" it--so that checkpoints and exits
    /// on the range cannot be made.  It is equivalent to the range having never been deposited.
    pub fn remove_deposited_range(&mut self, range: &Range, deposited_range_id: &RangeNumber) {
        let encompasing_range = self
            .deposited_ranges
            .get(&deposited_range_id)
            .unwrap()
            .clone();

        // Split the LEFT side
        // check if we we have a new deposited region to the left
        if range.start != encompasing_range.start {
            let left_split_range = Range {
                start: encompasing_range.start.clone(),
                end: range.start.clone(),
            };
            self.deposited_ranges
                .insert(left_split_range.end.clone(), left_split_range);
        }

        // Split the RIGHT side (there 3 possible splits)

        // 1) ##### -> $$$## -- check if we have leftovers to the right which are deposited
        if range.end != encompasing_range.end {
            // new deposited range from the newly exited end until the old unexited end
            let right_split_range = Range {
                start: range.end.clone(),
                end: encompasing_range.end.clone(),
            };
            // Store the new deposited range
            self.deposited_ranges
                .insert(right_split_range.end.clone(), right_split_range);
            return;
        }

        // 3) ##### -> $$$$$ -- without right-side leftovers & not the rightmost deposit, we can simply delete the value
        self.deposited_ranges.remove(&encompasing_range.end);
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
        self.commitment().deploy(env);

        //MUST be an address of ERC20 token
        self.TOKEN_ADDRES.set(token_address);
        self.CHALLENGE_PERIOD.set(chalenge_period);
        self.EXIT_PERIOD.set(exit_period);

        self.total_deposited.set(0);
        self.deposited_ranges.insert(0, Range { start: 0, end: 0 });
    }

    /// Allows a user to submit a deposit to the contract.
    /// Only allows users to submit deposits for the asset represented by this contract.
    ///
    /// Depositing is the mechanism which locks an asset into the plasma escrow agreement,
    /// allowing it to be transacted off-chain. The initialState defines its spending conditions,
    /// in the same way that a StateUpdate does once further transactions are made. Because deposits are verified on-chain transactions,
    /// they can be treated as checkpoints which are unchallengeable.
    fn deposit<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        depositer: AccountId,
        amount: Balance,
        initial_state: StateObject<T>,
    ) -> primitives::Result<CheckpointFinalized> {
        // Transfer the deposited amount from the depositer to the deposit contract’s address.
        // Transfer tokens to the deposit contract
        // erc20.transferFrom(msg.sender, address(this), _amount);
        // TODO

        let total_deposited = self.total_deposited.get().clone();
        let deposit_range = Range {
            start: total_deposited,
            end: total_deposited + amount as RangeNumber,
        };
        let state_update = StateUpdate {
            range: deposit_range.clone(),
            state_object: initial_state,
            plasma_block_number: self.commitment().current_block(env),
        };
        let checkpoint = Checkpoint {
            state_update: state_update,
            sub_range: deposit_range,
        };

        // Keep track of the total deposited assets, totalDeposited.
        // Create a state update with a state object equal to the provided initialState.
        // Compute the range of the created state update as totalDeposited to totalDeposited + amount.
        // Update the total amount deposited after the deposit is handled.
        self.extend_deposited_ranges(amount);

        // Insert the created state update into the checkpoints mapping with challengeableUntil being the current block number - 1.
        let checkpoint_id = checkpoint.id();
        let status = CheckpointStatus {
            challengeable_until: env.block_number() - 1,
            outstanding_challenges: 0,
        };
        self.checkpoints.insert(checkpoint_id.clone(), status);

        // Emit a CheckpointFinalized event for the inserted checkpoint.
        Ok(CheckpointFinalized {
            checkpoint: checkpoint_id,
        })
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
            return Err(
                "error: verify that an indentical checkpoint has not already been started.",
            );
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
				"error: ensure that the plasma blocknumber of the older_exit is less than that of newer_checkpoint.",
			);
        }

        // Check checkpoint finalized.
        let newer_checkpoint_id = newer_checkpoint.id();
        if let Some(checkpoint_status) = self.checkpoints.get(&newer_checkpoint_id) {
            // Ensure that the newerCheckpoint has no challenges.
            if checkpoint_status.outstanding_challenges != 0 {
                return Err("error: ensure that the newerCheckpoint has no challenges.");
            }
            // Ensure that the newerCheckpoint is no longer challengeable.
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
        let challenge_id = challenge.id();
        if let Some(_) = self.challenges.get(&challenge_id) {
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
        if !self.is_exist_challenges(&challenge_id) {
            return Err("error: check that the challenge was not already removed.");
        }

        // Check that the challenging exit has since been removed.
        if self.is_exist_exit(&challenging_id) {
            return Err("error: check that the challenging exit has since been removed.");
        }

        // Remove the challenge if above conditions are met.
        self.challenges.insert(challenge_id, false);

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

        // Ensure that the Contract address is the _checkpoint.stateUpdate.predicateAddress to authenticate the exit’s initiation.
        if checkpoint.state_update.state_object.predicate != env.address() {
            return Err("error: Ensure that the contract address is the checkpoint.state_update.predicate_address to authenticate the exit’s initiation.");
        }

        // Set the exit’s redeemableAfter status to the current Ethereum block.number + EXIT_PERIOD.
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
    fn deprecate_exit<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<T>,
    ) -> primitives::Result<()> {
        let checkpoint_id = checkpoint.id();
        // Ensure the contract address is the checkpoint.stateUpdate.predicateAddress to ensure the deprecation is authenticated.
        if checkpoint.state_update.state_object.predicate != env.address() {
            return Err("Ensure the contract address is the checkpoint.stateUpdate.predicateAddress to ensure the deprecation is authenticated.");
        }
        self.exit_redeemable_after.remove(&checkpoint_id);
        Ok(())
    }

    /// Finalizes an exit that has passed its exit period and has not been successfully challenged.
    fn finalize_exit<T: Member + Codec>(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        exit: Checkpoint<T>,
        deposited_range_id: RangeNumber,
    ) -> primitives::Result<ExitFinalized<T>> {
        let checkpoint_id = exit.id();
        let blk_number = env.block_number();
        // Ensure that the exit finalization is authenticated from the predicate by Contract address == _exit.stateUpdate.state.predicateAddress.
        if exit.state_update.state_object.predicate != env.address() {
            return Err("error: ensure that the exit finalization is authenticated from the predicate by Contract address == _exit.stateUpdate.state.predicateAddress.");
        }

        // Ensure that the checkpoint is finalized (current Ethereum block exceeds checkpoint.challengeableUntil).
        // Ensure that the checkpoint’s outstandingChallenges is 0.
        if !self.is_checkpoint_finalized(&checkpoint_id, &blk_number) {
            return Err("error: ensure that the checkpoint is finalized (current Ethereum block exceeds checkpoint.challengeableUntil and checkpoint’s outstandingChallenges is 0).");
        }

        // Ensure that the exit is finalized (current Ethereum block exceeds redeemablAfter ).
        if blk_number <= *self.exit_redeemable_after.get(&checkpoint_id).unwrap() {
            return Err("error: ensure that the exit is finalized (current Ethereum block exceeds redeemablAfter.");
        }

        // Ensure that the checkpoint is on a subrange of the currently exitable ranges via depositedRangeId.
        if let Some(deposited_range) = self.deposited_ranges.get(&deposited_range_id) {
            if !deposited_range.subrange(&exit.sub_range) {
                return Err("error: ensure that the checkpoint is on a subrange of the currently exitable ranges via depositedRangeId. Invalid SubRange.");
            }
        } else {
            return Err("error: ensure that the checkpoint is on a subrange of the currently exitable ranges via depositedRangeId. Not found depositedRangeId.");
        }

        // Remove the exited range by updating the depositedRanges mapping.
        self.remove_deposited_range(&exit.sub_range, &deposited_range_id);

        // MUST make an ERC20 transfer of the end - start amount to the predicate address.
        // Transfer tokens to the deposit contract
        //		uint256 amount = _exit.subrange.end - _exit.subrange.start;
        //		erc20.transfer(_exit.stateUpdate.stateObject.predicateAddress, amount);
        // TODO

        // Delete the exit.
        self.exit_redeemable_after.remove(&checkpoint_id);
        // Delete the checkpoint.
        self.checkpoints.remove(&checkpoint_id);

        // Emit an exitFinalized event.
        Ok(ExitFinalized { exit })
    }

    fn commitment(&mut self) -> &mut commitment::default::Commitment {
        &mut self.COMMITMENT
    }
    fn commitment_ref(&self) -> &commitment::default::Commitment {
        &self.COMMITMENT
    }
}

#[cfg(all(test, feature = "test-env"))]
mod tests {
    use super::*;
    use crate::traits::Deposit as _;
    use ink_core::storage::{
        alloc::{AllocateUsing, BumpAlloc, Initialize as _},
        Key,
    };
    use ink_model::EnvHandler;
	use scale::{Encode, Decode};

    #[cfg(all(test, feature = "test-env"))]
    extern crate commitment;

    const DEPOSIT_ADDRESS: [u8; 32] = [1u8; 32];

    impl Deposit {
        /// Deploys the testable contract by initializing it with the given values.
        pub fn deploy_mock(
            token_address: AccountId,
            challenge_period: BlockNumber,
            exit_period: BlockNumber,
        ) -> (
            Self,
            EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        ) {
            // initialize Environment.
            ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_address(
                AccountId::decode(&mut &DEPOSIT_ADDRESS[..]).unwrap(),
            );
            ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_block_number(1);

            let (mut deposit, mut env) = unsafe {
                let mut alloc = BumpAlloc::from_raw_parts(Key([0x0; 32]));
                (
                    Self::allocate_using(&mut alloc),
                    AllocateUsing::allocate_using(&mut alloc),
                )
            };
            deposit.initialize(());
            deposit.deploy(&mut env, token_address, challenge_period, exit_period);
            (deposit, env)
        }

        // Test Getters
        pub fn total_deposited(&self) -> RangeNumber {
            self.total_deposited.get().clone()
        }
        pub fn checkpoints(&self, key: &Hash) -> Option<&CheckpointStatus> {
            self.checkpoints.get(key)
        }
        pub fn deposited_ranges(&self, key: &RangeNumber) -> Option<&Range> {
            self.deposited_ranges.get(key)
        }
        pub fn exit_redeemable_after(&self, key: &Hash) -> Option<&BlockNumber> {
            self.exit_redeemable_after.get(key)
        }
        pub fn challenges(&self, key: &Hash) -> Option<&bool> {
            self.challenges.get(key)
        }
    }

    fn get_token_address() -> AccountId {
        AccountId::decode(&mut &[2u8; 32].to_vec()[..]).expect("account id decoded.")
    }

    #[test]
    fn deposit_normal() {
        let erc20_address = get_token_address();
        let (mut contract, mut env) = Deposit::deploy_mock(erc20_address, 5, 5);
        let this = env.address();

        let amount = 10000 as Balance;
        let initial_state = StateObject {
            predicate: erc20_address,
            data: erc20_address,
        };

        let exp_checkpoint = Checkpoint {
            state_update: StateUpdate {
                range: Range {
                    start: 0,
                    end: amount.clone() as RangeNumber,
                },
                state_object: initial_state.clone(),
                plasma_block_number: 0,
            },
            sub_range: Range {
                start: 0,
                end: amount.clone() as RangeNumber,
            },
        };

        assert_eq!(
            Ok(CheckpointFinalized {
                checkpoint: exp_checkpoint.id(),
            }),
            contract.deposit(&mut env, this, amount, initial_state,)
        );
        assert_eq!(10000, contract.total_deposited());
        assert_eq!(
            Some(&Range {
                start: 0,
                end: 10000
            }),
            contract.deposited_ranges(&10000)
        );
        assert_eq!(
            Some(&CheckpointStatus {
                challengeable_until: env.block_number() - 1,
                outstanding_challenges: 0,
            }),
            contract.checkpoints(&exp_checkpoint.id())
        );
    }

    use commitment::{default::default_hash, merkle};
    pub type TreeGenerator = merkle::MerkleIntervalTreeGenerator<AccountId, fn(&[u8]) -> Hash>;

    fn make_test_leafs(
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        plasm_block_number: BlockNumber,
    ) -> Vec<StateUpdate<AccountId>> {
        (0..8)
            .map(|i| StateUpdate::<AccountId> {
                range: Range {
                    start: i * 100,
                    end: i * 100 + 100,
                },
                state_object: StateObject {
                    predicate: env.address(),
                    data: AccountId::decode(&mut &[i as u8; 32][..]).unwrap(),
                },
                plasma_block_number: plasm_block_number,
            })
            .collect()
    }

    use std::convert::TryInto;
    fn make_test_merkle_tree_and_state(
        contract: &mut Deposit,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        leafs: &mut Vec<StateUpdate<AccountId>>,
        plasm_block_number: BlockNumber,
    ) -> merkle::Tree {
        let nodes = TreeGenerator::generate_leafs(leafs, default_hash);
        let tree = TreeGenerator::generate_internal_nodes(&nodes, default_hash);

        let header: Hash = default_hash(&tree[0][0].encode()[..]);
        assert_eq!(
            Ok(BlockSubmitted {
                number: plasm_block_number,
                header: header.clone(),
            }),
            contract.commitment().submit_block(env, header.clone())
        );
        tree
    }

    fn all_deposit_leafs(
        contract: &mut Deposit,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        leafs: &mut Vec<StateUpdate<AccountId>>,
    ) {
        for leaf in leafs.iter() {
            let exp_checkpoint = Checkpoint {
                state_update: leaf.clone(),
                sub_range: Range {
                    start: leaf.range.start,
                    end: leaf.range.end,
                },
            };
            assert_eq!(
                Ok(CheckpointFinalized {
                    checkpoint: exp_checkpoint.id()
                }),
                contract.deposit(
                    env,
                    leaf.state_object.data.clone(),
                    ((leaf.range.end - leaf.range.start) as u64)
                        .try_into()
                        .unwrap(),
                    leaf.state_object.clone(),
                )
            );
        }
    }

    #[test]
    fn start_checkpoint_normal() {
        let erc20_address = get_token_address();
        let (mut contract, mut env) = Deposit::deploy_mock(erc20_address, 5, 5);
        let this = env.address();

        let mut leafs = make_test_leafs(&mut env, 1);
        let tree = make_test_merkle_tree_and_state(&mut contract, &mut env, &mut leafs, 1);
        all_deposit_leafs(&mut contract, &mut env, &mut leafs);

        let mut new_state = leafs[3].clone();
        new_state.plasma_block_number = 2;
        new_state.state_object.data = AccountId::decode(&mut &[0 as u8; 32][..]).unwrap();
        let mut new_leafs = leafs.clone();
        new_leafs[3] = new_state;
        let new_tree = make_test_merkle_tree_and_state(&mut contract, &mut env, &mut new_leafs, 2);

        let inclusion_proof =
            TreeGenerator::generate_proof(&new_tree, &new_leafs[3], default_hash).unwrap();
        let deposited_range_id = new_leafs.last().unwrap().range.end;

        let checkpoint = Checkpoint {
            state_update: new_leafs[3].clone(),
            sub_range: Range {
                start: new_leafs[3].range.start,
                end: new_leafs[3].range.end,
            },
        };

        let checkpoint_id = checkpoint.id();
        // Check the emit value.
        assert_eq!(
            Ok(CheckpointStarted {
                checkpoint: checkpoint.clone(),
                challengeable_until: env.block_number() + 5,
            }),
            contract.start_checkpoint(&mut env, checkpoint, inclusion_proof, deposited_range_id)
        );
        // Check change the value.
        assert_eq!(
            Some(&CheckpointStatus {
                challengeable_until: env.block_number() + 5,
                outstanding_challenges: 0,
            }),
            contract.checkpoints(&checkpoint_id),
        );
    }

    #[test]
    fn start_exit_normal() {
        let erc20_address = get_token_address();
        let (mut contract, mut env) = Deposit::deploy_mock(erc20_address, 5, 5);
        let this = env.address();

        let mut leafs = make_test_leafs(&mut env, 1);
        let tree = make_test_merkle_tree_and_state(&mut contract, &mut env, &mut leafs, 1);
        all_deposit_leafs(&mut contract, &mut env, &mut leafs);

        let checkpoint = Checkpoint {
            state_update: leafs[3].clone(),
            sub_range: Range {
                start: leafs[3].range.start,
                end: leafs[3].range.end,
            },
        };

        let checkpoint_id = checkpoint.id();

        // Check the emit value.
        assert_eq!(
            Ok(ExitStarted {
                exit: checkpoint_id,
                redeemable_after: env.block_number() + 5,
            }),
            contract.start_exit(&mut env, checkpoint)
        );
        // Check change the value.
        assert_eq!(
            Some(&(env.block_number() + 5)),
            contract.exit_redeemable_after(&checkpoint_id),
        );
    }

    fn start_exit_unwrap(
        contract: &mut Deposit,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        leaf: &StateUpdate<AccountId>,
    ) -> Checkpoint<AccountId> {
        let checkpoint = Checkpoint {
            state_update: leaf.clone(),
            sub_range: Range {
                start: leaf.range.start,
                end: leaf.range.end,
            },
        };
        let checkpoint_id = checkpoint.id();

        // Check the emit value.
        assert_eq!(
            Ok(ExitStarted {
                exit: checkpoint_id,
                redeemable_after: env.block_number() + 5,
            }),
            contract.start_exit(env, checkpoint.clone())
        );
        checkpoint
    }

    #[test]
    fn deprecate_exit_normal() {
        let erc20_address = get_token_address();
        let (mut contract, mut env) = Deposit::deploy_mock(erc20_address, 5, 5);
        let this = env.address();

        let mut leafs = make_test_leafs(&mut env, 1);
        let tree = make_test_merkle_tree_and_state(&mut contract, &mut env, &mut leafs, 1);
        all_deposit_leafs(&mut contract, &mut env, &mut leafs);

        let checkpoint = start_exit_unwrap(&mut contract, &mut env, &leafs[6]);
        let checkpoint_id = checkpoint.id();

        // check previous state.
        assert_eq!(true, contract.is_exist_exit(&checkpoint_id));
        // check return value.
        assert_eq!(Ok(()), contract.deprecate_exit(&mut env, checkpoint));
        // check after state.
        assert_eq!(false, contract.is_exist_exit(&checkpoint_id));
    }

    fn start_checkpoint_unwrap(
        contract: &mut Deposit,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        leaf: &StateUpdate<AccountId>,
        inclusion_proof: commitment::InclusionProof<RangeNumber>,
        deposited_range_id: RangeNumber,
    ) -> Checkpoint<AccountId> {
        let checkpoint = Checkpoint {
            state_update: leaf.clone(),
            sub_range: Range {
                start: leaf.range.start,
                end: leaf.range.end,
            },
        };
        let checkpoint_id = checkpoint.id();
        // Check the emit value.
        assert_eq!(
            Ok(CheckpointStarted {
                checkpoint: checkpoint.clone(),
                challengeable_until: env.block_number() + 5,
            }),
            contract.start_checkpoint(env, checkpoint.clone(), inclusion_proof, deposited_range_id)
        );
        checkpoint
    }

    #[test]
    fn delete_exit_outdated_normal() {
        let erc20_address = get_token_address();
        let (mut contract, mut env) = Deposit::deploy_mock(erc20_address, 5, 5);
        let this = env.address();

        // initail state...
        let mut leafs = make_test_leafs(&mut env, 1);
        let tree = make_test_merkle_tree_and_state(&mut contract, &mut env, &mut leafs, 1);
        all_deposit_leafs(&mut contract, &mut env, &mut leafs);

        // [5] exit.
        let older_exit = start_exit_unwrap(&mut contract, &mut env, &leafs[5]);

        ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_block_number(2);

        // second state...
        let mut new_state = leafs[5].clone();
        new_state.plasma_block_number = 2;
        new_state.state_object.data = AccountId::decode(&mut &[0 as u8; 32][..]).unwrap();
        let mut new_leafs = leafs.clone();
        new_leafs[5] = new_state;
        let new_tree = make_test_merkle_tree_and_state(&mut contract, &mut env, &mut new_leafs, 2);
        let inclusion_proof =
            TreeGenerator::generate_proof(&new_tree, &new_leafs[5], default_hash).unwrap();
        let deposited_range_id = new_leafs.last().unwrap().range.end;

        let newer_checkpoint = start_checkpoint_unwrap(
            &mut contract,
            &mut env,
            &new_leafs[5],
            inclusion_proof,
            deposited_range_id,
        );

        // check the ng case :(error: ensure that the newerCheckpoint is no longer challengeable)
        assert_eq!(
            Err("error: ensure that the newerCheckpoint is no longer challengeable."),
            contract.delete_exit_outdated(&mut env, older_exit.clone(), newer_checkpoint.clone())
        );

        // passed
        ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_block_number(8);
        // check ther previous storage.
        let older_exit_id = older_exit.id();
        assert_eq!(Some(&6), contract.exit_redeemable_after(&older_exit_id));
        // check the return value.a
        assert_eq!(
            Ok(()),
            contract.delete_exit_outdated(&mut env, older_exit, newer_checkpoint)
        );
        // check change the storage.
        assert_eq!(None, contract.exit_redeemable_after(&older_exit_id))
    }

    #[test]
    fn challenge_checkpoint_normal() {
        let erc20_address = get_token_address();
        let (mut contract, mut env) = Deposit::deploy_mock(erc20_address, 5, 5);
        let this = env.address();

        // initail state...
        let mut leafs = make_test_leafs(&mut env, 1);
        let tree = make_test_merkle_tree_and_state(&mut contract, &mut env, &mut leafs, 1);
        all_deposit_leafs(&mut contract, &mut env, &mut leafs);

        // [5] exit.
        let challenging_exit = start_exit_unwrap(&mut contract, &mut env, &leafs[5]);

        ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_block_number(2);

        // second state...
        let mut new_state = leafs[5].clone();
        new_state.plasma_block_number = 2;
        new_state.state_object.data = AccountId::decode(&mut &[0 as u8; 32][..]).unwrap();
        let mut new_leafs = leafs.clone();
        new_leafs[5] = new_state;
        let new_tree = make_test_merkle_tree_and_state(&mut contract, &mut env, &mut new_leafs, 2);
        let inclusion_proof =
            TreeGenerator::generate_proof(&new_tree, &new_leafs[5], default_hash).unwrap();
        let deposited_range_id = new_leafs.last().unwrap().range.end;

        let challenged_checkpoint = start_checkpoint_unwrap(
            &mut contract,
            &mut env,
            &new_leafs[5],
            inclusion_proof,
            deposited_range_id,
        );

        // passed
        let challenge = Challenge {
            challenged_checkpoint: challenged_checkpoint.clone(),
            challenging_checkpoint: challenging_exit.clone(),
        };
        let challenged_id = challenged_checkpoint.id();
        let challenge_id = challenge.id();

        // check ther previous storage.
        assert_eq!(
            Some(&CheckpointStatus {
                challengeable_until: 7,
                outstanding_challenges: 0,
            }),
            contract.checkpoints(&challenged_id)
        );

        // check the return value.
        assert_eq!(Ok(()), contract.challenge_checkpoint(&mut env, challenge));

        assert_eq!(
            Some(&CheckpointStatus {
                challengeable_until: 7,
                outstanding_challenges: 1,
            }),
            contract.checkpoints(&challenged_id)
        );
        assert_eq!(Some(&true), contract.challenges(&challenge_id));
    }

    fn challenge_checkpoint_unwrap(
        contract: &mut Deposit,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        challenged: &Checkpoint<AccountId>,
        challenging: &Checkpoint<AccountId>,
    ) -> Challenge<AccountId> {
        let challenge = Challenge {
            challenged_checkpoint: challenged.clone(),
            challenging_checkpoint: challenging.clone(),
        };
        // check the return value.
        assert_eq!(
            Ok(()),
            contract.challenge_checkpoint(env, challenge.clone())
        );
        challenge
    }

    #[test]
    fn remove_challenge_normal() {
        let erc20_address = get_token_address();
        let (mut contract, mut env) = Deposit::deploy_mock(erc20_address, 5, 5);
        let this = env.address();

        // initail state...
        let mut leafs = make_test_leafs(&mut env, 1);
        let tree = make_test_merkle_tree_and_state(&mut contract, &mut env, &mut leafs, 1);
        all_deposit_leafs(&mut contract, &mut env, &mut leafs);

        // [5] exit.
        let challenging_exit = start_exit_unwrap(&mut contract, &mut env, &leafs[5]);

        ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_block_number(2);

        // second state...
        let mut new_state = leafs[5].clone();
        new_state.plasma_block_number = 2;
        new_state.state_object.data = AccountId::decode(&mut &[0 as u8; 32][..]).unwrap();
        let mut new_leafs = leafs.clone();
        new_leafs[5] = new_state;
        let new_tree = make_test_merkle_tree_and_state(&mut contract, &mut env, &mut new_leafs, 2);
        let inclusion_proof =
            TreeGenerator::generate_proof(&new_tree, &new_leafs[5], default_hash).unwrap();
        let deposited_range_id = new_leafs.last().unwrap().range.end;

        let challenged_checkpoint = start_checkpoint_unwrap(
            &mut contract,
            &mut env,
            &new_leafs[5],
            inclusion_proof,
            deposited_range_id,
        );

        let challenge = challenge_checkpoint_unwrap(
            &mut contract,
            &mut env,
            &challenged_checkpoint,
            &challenging_exit,
        );
        let challenge_id = challenge.id();

        // deprecate exit.
        assert_eq!(
            Ok(()),
            contract.deprecate_exit(&mut env, challenging_exit.clone())
        );

        // check privious state.
        assert_eq!(Some(&true), contract.challenges(&challenge_id));
        assert_eq!(
            Some(&CheckpointStatus {
                challengeable_until: 7,
                outstanding_challenges: 1,
            }),
            contract.checkpoints.get(&challenged_checkpoint.id())
        );

        // check return value.
        assert_eq!(Ok(()), contract.remove_challenge(&mut env, challenge));

        // check after state.
        assert_eq!(Some(&false), contract.challenges(&challenge_id));
        assert_eq!(
            Some(&CheckpointStatus {
                challengeable_until: 7,
                outstanding_challenges: 0,
            }),
            contract.checkpoints.get(&challenged_checkpoint.id())
        );
    }

    #[test]
    fn finalize_exit_normal() {
        let erc20_address = get_token_address();
        let (mut contract, mut env) = Deposit::deploy_mock(erc20_address, 5, 5);
        let this = env.address();

        // initail state...
        let mut leafs = make_test_leafs(&mut env, 1);
        let tree = make_test_merkle_tree_and_state(&mut contract, &mut env, &mut leafs, 1);
        all_deposit_leafs(&mut contract, &mut env, &mut leafs);

        // [5] exit.
        let exit = start_exit_unwrap(&mut contract, &mut env, &leafs[5]);
        let deposited_range_id = leafs.last().unwrap().range.end;

        // previous exit finalize.
        ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_block_number(2);

        assert_eq!(Err("error: ensure that the exit is finalized (current Ethereum block exceeds redeemablAfter."),
				   contract.finalize_exit(&mut env, exit.clone(), deposited_range_id));

        // passed block number
        ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_block_number(10);

        // previous state.
        assert_eq!(800, contract.total_deposited());
        assert_eq!(
            Some(&Range { start: 0, end: 800 }),
            contract.deposited_ranges(&800)
        );
        assert_eq!(None, contract.deposited_ranges(&500));

        // check return value.
        assert_eq!(
            Ok(ExitFinalized { exit: exit.clone() }),
            contract.finalize_exit(&mut env, exit.clone(), deposited_range_id)
        );

        // check after value.
        assert_eq!(800, contract.total_deposited());
        assert_eq!(
            Some(&Range {
                start: 600,
                end: 800
            }),
            contract.deposited_ranges(&800)
        );
        assert_eq!(
            Some(&Range { start: 0, end: 500 }),
            contract.deposited_ranges(&500)
        );
        assert_eq!(None, contract.checkpoints(&exit.id()));
        assert_eq!(None, contract.exit_redeemable_after(&exit.id()));
    }
}
