//! The exit dispute logic of plasma modules.
//! - ExitDispute.sol
//! - SpentChallengeValidator.sol

use super::*;
use frame_support::dispatch::{DispatchError, DispatchResult};

// Dispute Kinds.
pub const EXIT_CLAIM: &'static [u8] = b"EXIT_CLAIM";
pub const EXIT_SPENT_CHALLENGE: &'static [u8] = b"EXIT_SPENT_CHALLENGE";
pub const EXIT_CHECKPOINT_CHALLENGE: &'static [u8] = b"EXIT_CHECKPOINT_CHALLENGE";

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
        state_update: &StateUpdateOf<T>,
        checkpoint: &Option<StateUpdateOf<T>>,
        witness: &Option<InclusionProofOf<T>>,
    ) -> DispatchResult {
        if let Some(checkpoint) = checkpoint {
            // ExitCheckpoint
            // check if checkpoint is stored in depositContract
            ensure!(
                Self::checkpoint_exitable(plapps_id, state_update, checkpoint)?,
                "Checkpoint must be exitable for stateUpdate"
            );
        } else if let Some(inclusion_proof) = witness {
            // ExitStateUpdate
            let root = Self::retrieve(plapps_id, &state_update.block_number);

            ensure!(
                Self::verify_inclusion_with_root(
                    &T::Hashing::hash_of(&state_update.state_object),
                    &state_update.deposit_contract_address,
                    &state_update.range,
                    inclusion_proof,
                    &root
                )?,
                "Inclusion verification failed"
            );
        }
        // claim property to DisputeManager
        let exit_predicate = Self::exit_predicate(plapps_id);

        let property: PropertyOf<T> =
            Self::create_property(exit_predicate.clone(), &state_update.encode(), EXIT_CLAIM);
        // origin == property.predicate_address
        pallet_ovm::Call::<T>::claim(property);
        Ok(())
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
        plapps_id: &T::AccountId,
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
            Decode::decode(&mut &inputs[0][..]).map_err(|_| Error::<T>::MustBeDecodable)?;
        let challenge_property = if T::Hashing::hash_of(&challenge_inputs[0])
            == T::Hashing::hash(EXIT_SPENT_CHALLENGE)
        {
            let spent_challenge_inputs = vec![challenge_inputs[1].clone()];
            Self::validate_spent_challenge(plapps_id, inputs, &spent_challenge_inputs, witness)?;
            Self::deposit_event(RawEvent::ExitSpentChallenged(state_update));

            let exit_predicate = Self::exit_predicate(plapps_id);
            Ok(Self::create_property(
                exit_predicate,
                &challenge_inputs[0],
                EXIT_SPENT_CHALLENGE,
            )) as DispatchResultT<PropertyOf<T>>
        } else if T::Hashing::hash_of(&challenge_inputs[0])
            == T::Hashing::hash(EXIT_CHECKPOINT_CHALLENGE)
        {
            let invalid_history_challenge_inputs = vec![challenge_inputs[1].clone()];
            Self::validate_checkpoint_challenge(
                plapps_id,
                inputs.clone(),
                invalid_history_challenge_inputs.clone(),
                witness.clone(),
            )?;
            let challenge_state_update: StateUpdateOf<T> =
                Decode::decode(&mut &invalid_history_challenge_inputs[0][..])
                    .map_err(|_| Error::<T>::MustBeDecodable)?;
            Self::deposit_event(RawEvent::ExitCheckpointChallenged(
                state_update,
                challenge_state_update,
            ));

            let exit_predicate = Self::exit_predicate(plapps_id);
            Ok(Self::create_property(
                exit_predicate,
                &invalid_history_challenge_inputs[0],
                EXIT_CHECKPOINT_CHALLENGE,
            )) as DispatchResultT<PropertyOf<T>>
        } else {
            return Err(DispatchError::Other("illegal challenge type"));
        }?;

        let exit_predicate = Self::exit_predicate(plapps_id);
        let claimed_property =
            Self::create_property(exit_predicate.clone(), &inputs[0], EXIT_CLAIM);
        ensure!(
            pallet_ovm::Module::<T>::started(&pallet_ovm::Module::<T>::get_property_id(
                &claimed_property
            )),
            "Claim does not exist"
        );

        // TODO: bare_challenge
        pallet_ovm::Module::<T>::bare_challenge(
            exit_predicate.clone(),
            Self::create_property(exit_predicate, &inputs[0], EXIT_CLAIM),
            challenge_property,
        )
    }

    pub fn bare_exit_remove_challenge(
        _plapps_id: &T::AccountId,
        _inputs: Vec<Vec<u8>>,
        _challenge_inputs: Vec<Vec<u8>>,
        _witness: Vec<Vec<u8>>,
    ) -> DispatchResult {
        Ok(())
    }

    /// prove exit is coin which hasn't been spent.
    /// check checkpoint
    pub fn bare_exit_settle(plapps_id: &T::AccountId, inputs: Vec<Vec<u8>>) -> DispatchResult {
        ensure!(
            inputs.len() == 1,
            "inputs length does not match. expected 1"
        );

        let exit_predicate = Self::exit_predicate(plapps_id);
        let property = Self::create_property(exit_predicate.clone(), &inputs[0], EXIT_CLAIM);
        pallet_ovm::Module::<T>::bare_settle_game(exit_predicate, property)?;

        let state_update: StateUpdateOf<T> =
            Decode::decode(&mut &inputs[0][..]).map_err(|_| Error::<T>::MustBeDecodable)?;

        Self::deposit_event(RawEvent::ExitSettled(state_update, true));
        Ok(())
    }

    fn get_id(su: &StateUpdateOf<T>) -> T::Hash {
        T::Hashing::hash_of(su)
    }

    fn get_claim_decision(
        predicate_address: T::AccountId,
        su: &StateUpdateOf<T>,
    ) -> DispatchResultT<Decision> {
        let su_bytes = su.encode();
        let exit_property =
            Self::create_property(predicate_address, &su_bytes.encode(), EXIT_CLAIM);
        let id = pallet_ovm::Module::<T>::get_property_id(&exit_property);
        let game = pallet_ovm::Module::<T>::get_game(&id).ok_or(Error::<T>::NotFoundGame)?;
        Ok(game.decision)
    }

    /// If the exit can be withdrawable, isCompletable returns true.
    fn is_completable(plapps_id: &T::AccountId, su: &StateUpdateOf<T>) -> bool {
        let su_bytes = su.encode();
        let exit_predicate = Self::exit_predicate(plapps_id);
        let exit_property = Self::create_property(exit_predicate, &su_bytes, EXIT_CLAIM);
        let id = pallet_ovm::Module::<T>::get_property_id(&exit_property);
        pallet_ovm::Module::<T>::is_decidable(&id)
    }

    fn checkpoint_exitable(
        plapps_id: &T::AccountId,
        state_update: &StateUpdateOf<T>,
        checkpoint: &StateUpdateOf<T>,
    ) -> DispatchResultT<bool> {
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
        Ok(true)
    }
}

// SpentChallengeValidator.sol
impl<T: Config> Module<T> {
    fn validate_spent_challenge(
        plapps_id: &T::AccountId,
        inputs: &Vec<Vec<u8>>,
        challenge_inputs: &Vec<Vec<u8>>,
        witness: &Vec<Vec<u8>>,
    ) -> DispatchResult {
        let state_update: StateUpdateOf<T> =
            Decode::decode(&mut &inputs[0][..]).map_err(|_| Error::<T>::MustBeDecodable)?;
        let transaction: TransactionOf<T> = Decode::decode(&mut &challenge_inputs[0][..])
            .map_err(|_| Error::<T>::MustBeDecodable)?;
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

        let result_bytes = pallet_ovm::Module::<T>::bare_call(
            plapps_id.clone(),
            state_update.state_object.predicate_address,
            predicate_decide_inputs,
        )
        .map_err(|_| Error::<T>::PredicateExecError)?;
        let result: bool =
            Decode::decode(&mut &result_bytes[..]).map_err(|_| Error::<T>::MustBeDecodable)?;
        ensure!(result, "State object decided to false");
        Ok(())
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
