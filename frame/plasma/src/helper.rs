//! The helper of plasma modules.
//! - DisputeKind.sol
//! - DisputeHelper.sol
//! - CheckpointDispute.sol

use super::*;
use codec::Decode;

// Dispute Helper methods.
impl<T: Config> Module<T> {
    pub fn create_property(
        predicate_address: T::AccountId,
        su_bytes: &Vec<u8>,
        kind: &'static [u8],
    ) -> PropertyOf<T> {
        let mut inputs = vec![kind.to_vec(), su_bytes.clone()];
        PropertyOf::<T> {
            predicate_address,
            inputs,
        }
    }
}

// CheckpointDispute methods.
impl<T: Config> Module<T> {
    /// challenge checkpiont
    /// _inputs: [encode(stateUpdate)] challenged state update
    /// _challengeInputs: [encode(stateUpdate)] challenging state update
    /// _witness: [encode(inclusionProof)] inclusionProof of challenging state update
    pub fn validate_checkpoint_challenge(
        plapps_id: &T::AccountId,
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> DispatchResultT<(StateUpdateOf<T>, StateUpdateOf<T>, InclusionProofOf<T>)> {
        let state_update: StateUpdateOf<T> =
            Decode::decode(&mut &inputs[0][..]).map_err(|_| Error::<T>::MustBeDecodable)?;
        let challenge_state_update: StateUpdateOf<T> =
            Decode::decode(&mut &challenge_inputs[0][..])
                .map_err(|_| Error::<T>::MustBeDecodable)?;

        let inclusion_proof: InclusionProofOf<T> =
            Decode::decode(&mut &witness[0][..]).map_err(|_| Error::<T>::MustBeDecodable)?;

        ensure!(
            state_update.deposit_contract_address
                == challenge_state_update.deposit_contract_address,
            "DepositContractAddress is invalid",
        );
        ensure!(
            state_update.block_number > challenge_state_update.block_number,
            "BlockNumber must be smaller than challenged state",
        );
        ensure!(
            Self::is_sub_range(&challenge_state_update.range, &state_update.range),
            "Range must be subrange of stateUpdate",
        );

        // verify inclusion proof
        let root = Self::retrieve(plapps_id, &challenge_state_update.block_number);

        ensure!(
            Self::verify_inclusion_with_root(
                &T::Hashing::hash_of(&challenge_state_update.state_object),
                &challenge_state_update.deposit_contract_address,
                &challenge_state_update.range,
                &inclusion_proof,
                &root,
            )?,
            "Inclusion verification failed",
        );
        Ok((state_update, challenge_state_update, inclusion_proof))
    }

    pub fn is_sub_range(sub_range: &RangeOf<T>, surrounding_range: &RangeOf<T>) -> bool {
        sub_range.start >= surrounding_range.start && sub_range.end <= surrounding_range.end
    }

    pub fn has_intersection(range_a: &RangeOf<T>, range_b: &RangeOf<T>) -> bool {
        let a = range_a.start >= range_b.start && range_a.start < range_b.end;
        let b = range_b.start >= range_a.start && range_b.start < range_a.end;
        a || b
    }

    pub fn bytes_to_bytes32(source: Vec<u8>) -> DispatchResultT<T::Hash> {
        Ok(Decode::decode(&mut &source[..]).map_err(|_| Error::<T>::MustBeDecodable)?)
    }
}
