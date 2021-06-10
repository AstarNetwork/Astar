//! The exit dispute logic of plasma modules.
//! - CheckpointDispute.sol
//! - CheckpointChallengeValidator.sol
use super::helper::*;
use super::*;
use frame_support::dispatch::DispatchResult;
use frame_system::Origin;

/// claim checkpoint
///  _propertyInputs: [encode(stateUpdate)]
///  _witness: [encode(inclusionProof)]
///  NOTE: might be possible to define concrete argument type but bytes[]
impl<T: Config> Module<T> {
    fn bare_checkpoint_claim(
        plapps_id: &T::AccountId,
        inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> DispatchResult {
        // validate inputs
        ensure!(inputs.len() == 1, Error::<T>::InputLengthDoesNotMatch,);
        ensure!(witness.len() == 1, Error::<T>::WitnessLengthDoesNotMatch,);

        let su_property: PropertyOf<T> =
            Decode::decode(&mut &inputs[0][..]).map_err(|_| Error::<T>::DecodeError)?;
        let state_update = Self::desrializable_state_update(su_property)?;
        let inclusion_proof: InclusionProofOf<T> =
            Decode::decode(&mut &witness[0][..]).map_err(|_| Error::<T>::DecodeError)?;

        // verify inclusion proof
        let root = Self::retrive(plapps_id, state_update.block_number);
        ensure!(
            Self::verifyInclusion_with_root(
                T::Hashing::hash_of(&state_update.state_object),
                state_update.deposit_contract_address,
                state_update.range,
                inclusion_proof,
                root
            ),
            Error::<T>::InclusionVerificationFailed,
        );

        // claim property to DisputeManager
        // TODO: WIP implmenting.
        let property = Self::create_property(&plapps_id, &inputs[0], CHECKPOINT_CLAIM);
        let plapps_origin_id = Origin::signed(plapps_id);
        pallet_ovm::Call::<T>::claim(plapps_origin_id, property)?;
        Self::deposit_event(RawEvent::CheckpointClaimed(
            plapps_id,
            state_update,
            inclusion_proof,
        ));
    }

    fn bare_checkpoint_challenge(
        plapps_id: &T::AccountId,
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> DispatchResult {
        ensure!(
            inputs.len() == 1,
            "inputs length does not match. expected 1"
        );
        ensure!(
            challenge_inputs.len() == 1,
            "challenge inputs length does not match. expected 1"
        );
        ensure!(
            witness.len() == 1,
            "witness length does not match. expected 1"
        );

        let (state_update, challenge_state_update, inclusion_proof) =
            Self::validate_checkpoint_challenge(plapps_id, inputs, challenge_inputs, witness);

        let claim_property = Self::create_property(inputs[0].clone(), helper::CHECKPOINT_CLAIM);
        let challenge_property =
            Self::create_property(challenge_inputs[0].clone(), helper::CHECKPOINT_CHALLENGE);

        ensure!(
            pallet_ovm::Module::<T>::started(pallet_ovm::Call::<T>::get_property_id(
                claim_property
            )),
            "Claim does not exist",
        );
        let plapps_origin_id = Origin::signed(plapps_id);
        pallet_ovm::Call::<T>::challenge(plapps_origin_id, claim_property, challenge_property)?;

        Self::deposit_event(RawEvent::CheckpointChallenged(
            state_update,
            challenge_state_update,
            inclusion_proof,
        ));
    }

    fn bare_checkpoint_remove_challenge(
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> DispatchResult {
        ensure!(
            inputs.len() == 1,
            "inputs length does not match. expected 1"
        );
        ensure!(
            challenge_inputs.len() == 1,
            "challenge inputs length does not match. expected 1"
        );
        ensure!(witness.len() >= 1, "witness must be at least 1");

        let (challenge_property, property, state_update, challenge_state_update) =
            Self::validate_challenge_removal(&inputs, &challenge_inputs, &witness);

        pallet_ovm::Call::<T>::set_game_result(challenge_property.clone(), false)?;
        pallet_ovm::Call::<T>::remove_challenge(property)?;

        Self::deposit_event(RawEvent::ChallengeRemoved(
            state_update,
            challenge_state_update,
        ));
    }

    fn bare_checkpoint_settle(plapps_id: &T::AccountId, inputs: &Vec<Vec<u8>>) -> DispatchResult {
        ensure!(
            inputs.len() == 1,
            "inputs length does not match. expected 1"
        );
        let property = Self::create_property(&inputs[0], helper::CHECKPOINT_CLAIM);
        let plapps_origin_id = Origin::signed(plapps_id.clone());
        let result = pallet_ovm::Call::<T>::settle_game(plapps_origin_id.clone(), property);

        let state_update: StateUpdateOf<T> = Decode::decode(&mut &inputs[0])?;

        Self::deposit_event(RawEvent::CheckpointSettled(state_update.clone()));
        if result {
            return Self::bare_finalize_checkpoint(plapps_origin_id, state_update)?;
        }
        Ok(())
    }
}
