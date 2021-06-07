//! The exit dispute logic of plasma modules.
//! - ExitDispute.sol
//! - SpentChallengeValidator.sol
use super::helper::*;
use super::*;
use frame_support::dispatch::{DispatchError, DispatchResult};

// ExitDispute.sol
impl<T: Config> Module<T> {
    /// Claim Exit at StateUpdate
    /// There're two kind of exit claims. ExitStateUpdate and ExitCheckpoint.
    /// The former needs inclusion proof of stateUpdate. The latter don't need
    /// witness but check if checkpoint for the stateUpdate is finalized yet.
    /// inputs: [encode(stateUpdate), checkpoint]
    /// witness: [encode(inclusionProof)]
    pub fn bare_exit_claim(
        plapps_id: &T::AccountId,
        inputs: &Vec<Vec<u8>>,
        witness: &Vec<Vec<u8>>,
    ) -> DispatchResult {
        // validate inputs
        ensure!(
            inputs.len() >= 1,
            "inputs length does not match. at least 1"
        );
        let state_update: StateUpdateOf<T> =
            Decode::decode(&mut &inputs[0][..]).map_err(|_| Error::<T>::DecodeError)?;

        if witness.len() == 0 && inputs.len() == 2 {
            // ExitCheckpoint
            // check if checkpoint is stored in depositContract
            let checkpoint: StateUpdateOf<T> =
                Decode::decode(&mut &inputs[1][..]).map_err(|_| Error::<T>::DecodeError)?;
            ensure!(
                Self::checkpoint_exitable(&state_update, &checkpoint),
                "Checkpoint must be exitable for stateUpdate"
            );
        } else {
            // ExitStateUpdate
            let inclusion_proof: InclusionProofOf<T> =
                Decode::decode(&mut &witness[0][..]).map_err(|_| Error::<T>::DecodeError)?;
            let block_number_bytes = state_update.block_number.encode();
            let root = Self::bytes_to_bytes32(Self::retrieve(plapps_id, block_number_bytes));

            ensure!(
                Self::verify_inclusion_with_root(
                    T::Hashing::hash_of(&state_update.state_object),
                    state_update.deposit_contract_address.clone(),
                    state_update.range.clone(),
                    inclusion_proof,
                    root
                ),
                "Inclusion verification failed"
            );
        }
        // claim property to DisputeManager
        let property: PropertyOf<T> = Self::create_property(&inputs[0], EXIT_CLAIM);
        pallet_ovm::Call::<T>::claim(&property)?;

        Self::deposit_event(RawEvent::ExitClaimed(state_update));
    }

    /// challenge prove the exiting coin has been spent.
    /// First element of challengeInputs must be either of
    /// bytes("EXIT_SPENT_CHALLENGE") or bytes("EXIT_CHECKPOINT_CHALLENGE")
    /// SPENT_CHALLENGE
    /// input: [SU]
    /// challengeInput: [label, transaction]
    /// witness: [signature]
    /// CHECKPOINT
    /// input: [SU]
    /// challengeInput: [label, checkpointSU]
    /// witness: []
    pub fn bare_exit_challenge(
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
        witness: &Vec<Vec<u8>>,
    ) -> DispatchResult {
        ensure!(
            inputs.len() == 1,
            "inputs length does not match. expected 1"
        );
        ensure!(
            witness.len() == 1,
            "witness length does not match. expected 1"
        );
        ensure!(
            challenge_inputs.len() == 2,
            "challenge inputs length does not match. expected 2"
        );
        let state_update: StateUpdateOf<T> =
            Decode::decode(&mut &inputs[0][..]).map_err(|_| Error::<T>::DecodeError)?;
        let challenge_property = if T::Hashing::hash_of(&challenge_inputs[0])
            == T::Hashing::hash_of(EXIT_SPENT_CHALLENGE)
        {
            let spent_challenge_inputs = vec![challenge_inputs[1]];
            Self::validate_spent_challenge(inputs, &spent_challenge_inputs, witness)?;
            Self::deposit_event(RawEvent::ExitSpentChallenged(state_update));

            Ok(Self::create_property(
                challenge_inputs[0],
                EXIT_SPENT_CHALLENGE,
            ))
        } else if T::Hashing::hash_of(&challenge_inputs[0])
            == T::Hashing::hash_of(EXIT_CHECKPOINT_CHALLENGE)
        {
            let invalid_history_challenge_inputs = vec![challenge_inputs[1]];
            Self::valid_checkpoint_challenge(inputs, &invalid_history_challenge_inputs, witness)?;
            let challenge_state_update: StateUpdateOf<T> =
                Decode::decode(&mut &invalid_history_challenge_inputs[0][..])
                    .map_err(|_| Error::<T>::DecodeError)?;
            Self::deposit_event(RawEvent::ExitCheckpointChallenged(
                state_update,
                challenge_state_update,
            ));

            Ok(Self::create_property(
                &invalid_history_challenge_inputs[0],
                EXIT_CHECKPOINT_CHALLENGE,
            ))
        } else {
            return Err(DispatchError::Other("illegal challenge type"));
        }?;

        let claimed_property = Self::crate_property(&inputs[0], EXIT_CLAIM);
        ensure!(
            pallet_ovm::Call::<T>::started(Self::get_property_id(&claimed_property)),
            "Claim does not exist"
        );

        pallet_ovm::Module::<T>::challenge(
            Self::create_property(&inputs[0], EXIT_CLAIM),
            challenge_property,
        );
    }

    pub fn bare_exit_remove_challenge(
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> DispatchResult {
        Ok(())
    }

    /// prove exit is coin which hasn't been spent.
    /// check checkpoint
    pub fn bare_exit_settle(inputs: Vec<Vec<u8>>) -> DispatchResult {
        ensure!(
            inputs.len() == 1,
            "inputs length does not match. expected 1"
        );

        let property = Self::create_property(&inputs[0], EXIT_CLAIM);
        let decision = Self::<Module<T>>::settle_game(property)?;

        let state_update: StateUpdateOf<T> =
            Decode::decode(&mut &inputs[0][..]).map_err(|_| Error::<T>::DecodeError)?;

        Self::deposit_event(RawEvent::ExitSettled(state_update, true));
    }

    fn get_id(su: &StateUpdateOf<T>) -> T::Hash {
        T::Hashing::hash_of(su)
    }

    fn get_claim_decision(su: &StateUpdateOf<T>) -> Decision {
        let su_bytes = su.encode();
        let exit_property = Self::create_property(su_bytes, EXIT_CLAIM);
        let id = Self::get_property_id(&exit_property)?;
        let game = pallet_ovm::Module::<T>::get_game(id)?;
        game.decision
    }

    /// If the exit can be withdrawable, isCompletable returns true.
    fn is_completable(su: &StateUpdateOf<T>) -> bool {
        let su_bytes = su.encode();
        let exit_property = Self::create_property(&su_bytes, EXIT_CLAIM);
        let id = Self::get_property_id(&exit_property);
        pallet_ovm::Module::<T>::is_decidable(id)?
    }

    fn checkpoint_exitable(
        plapps_id: &T::AccountId,
        state_update: &StateUpdateOf<T>,
        checkpoint: &StateUpdateOf<T>,
    ) -> bool {
        ensure!(
            Self::is_subrange(&state_update.range, &checkpoint.range),
            "StateUpdate range must be subrange of checkpoint"
        );
        ensure!(
            state_update.block_number == checkpoint.block_number,
            "BlockNumber must be same"
        );

        let id = Self::get_id(checkpoint);
        ensure!(
            Self::checkpoints(plapps_id, &id),
            "Checkpoint needs to be finalized or inclusionProof have to be provided"
        );
        true
    }

    fn is_subrange(subrange: &RangeOf<T>, surrounding_range: &RangeOf<T>) -> bool {
        subrange.start >= surrounding_range.start && subrange.end <= surrounding_range.end
    }
}

// SpentChallengeValidator.sol
impl<T: Config> Module<T> {
    fn validate_spent_challeng(
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
        witness: &Vec<Vec<u8>>,
    ) -> DispatchResult {
        let state_update: StateUpdateOf<T> =
            Decode::decode(&mut &inputs[0][..]).map_err(|_| Error::<T>::DecodeError)?;
        let transaction: TransactionOf<T> =
            Decode::decode(&mut &challenge_inputs[0][..]).map_err(|_| Error::<T>::DecodeError)?;
        ensure!(
            transaction.deposit_contract_address == state_update.deposit_contract_address,
            "token must be same"
        );
        // To support spending multiple state updates
        ensure!(
            Self::has_intersection(&transaction.range, &state_update.range),
            "range must contain subrange"
        );
        ensure!(
            transaction.max_block_number >= state_update.block_number,
            "blockNumber must be valid"
        );

        // inputs for stateObject property
        let new_inputs = vec![
            state_update.state_object.inputs[0].clone(),
            challenge_inputs[0].clone(),
        ];

        let predicate_decide_inputs =
            Self::make_compiled_predicate_decide_inputs(new_inputs, witness.clone());

        ensure!(
            pallet_ovm::Module::<T>::bare_call_predicate(
                state_update.state_object.predicate_address,
                predicate_decide_inputs
            ),
            "State object decided to false"
        );
    }

    fn make_compiled_predicate_decide_inputs(
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> Vec<u8> {
        ovmi::predicates::PredicateCallInputs::CompiledPredicate::<T::AccountId>(
            ovmi::predicates::CompiledPredicateCallInputs::DecideTrue { inputs, witness },
        )
        .encode()
    }
}
