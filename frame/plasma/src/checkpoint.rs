//! The exit dispute logic of plasma modules.
//! - CheckpointDispute.sol
//! - CheckpointChallengeValidator.sol
use super::*;

/// claim checkpoint
///  _propertyInputs: [encode(stateUpdate)]
///  _witness: [encode(inclusionProof)]
///  NOTE: might be possible to define concrete argument type but bytes[]
impl<T: Trait> Module<T> {
    fn bare_checkpoint_claim(plapps_id: T::AccountId, inputs: Vec<Vec<u8>>, witness: Vec<Vec<u8>>) {
        // validate inputs
        ensure!(
                    inputs.len() == 1,
                    Error::<T>::InputLengthDoesNotMatch,
                );
        ensure!(
                    witness.len() == 1,
                    Error::<T>::WitnessLengthDoesNotMatch,
                );

        let su_property: PropertyOf<T> = Decode::decode(&mut &inputs[0][..])
            .map_err(|_| Error::<T>::DecodeError)?;
        let state_update = Self::desrializable_state_update(su_property)?;
        let inclusion_proof: InclusionProofOf<T> = Decode::decode(&mut &witness[0][..])
            .map_err(|_| Error::<T>::DecodeError)?;

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
        let prooerty = Self::create_property(&plapps_id, &_inputs[0], CHECKPOINT_CLAIM);
        // types.Property memory property = createProperty(_inputs[0], CHECKPOINT_CLAIM);
        let plapps_origin_id = Origin::signed(plapps_id);
        pallet_ovm::Module<T>::claim(plapps_origin_id, property)?;
        Self::deposit_event(RawEvent::CheckpointClaimed(plapps_id, state_update, inclusion_proof));
    }
}
