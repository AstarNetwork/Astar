//! The helper of plasma modules.
//! - DisputeKind.sol
//! - DisputeHelper.sol
//! - CheckpointDispute.sol

use super::*;

// Dispute Kinds.
pub const CHECKPOINT_CLAIM: &'static [u8] = b"CHECKPOINT_CLAIM";
pub const CHECKPOINT_CHALLENGE: &'static [u8] = b"CHECKPOINT_CHALLENGE";
pub const EXIT_CLAIM: &'static [u8] = b"EXIT_CLAIM";
pub const EXIT_SPENT_CHALLENGE: &'static [u8] = b"EXIT_SPENT_CHALLENGE";
pub const EXIT_CHECKPOINT_CHALLENGE: &'static [u8] = b"EXIT_CHECKPOINT_CHALLENGE";

// Dispute Helper methods.
impl<T: Trait> Module<T> {
    pub fn create_property(su_bytes: &Vec<u8>, kind: &'static [u8]) -> PropertyOf<T> {
        let mut inputs = vec![kind.to_vec(), su_bytes.clone()];
        PropertyOf::<T> {
            predicate_address: su_bytes.clone(),
            inputs,
        }
    }
}

// CheckpointDispute methods.
impl<T: Trait> Module<T> {
    /// challenge checkpiont
    /// _inputs: [encode(stateUpdate)] challenged state update
    /// _challengeInputs: [encode(stateUpdate)] challenging state update
    /// _witness: [encode(inclusionProof)] inclusionProof of challenging state update
    pub fn validate_checkpoint_challenge(
        plapps_id: &T::AccountId,
        inputs: Vec<Vec<u8>>,
        challenge_inputs: Vec<Vec<u8>>,
        witness: Vec<Vec<u8>>,
    ) -> (StateUpdateOf<T>, StateUpdateOf<T>, InclusionProofOf<T>) {
        let state_update: StateUpdateOf<T> = Decode::decode(&mut &inputs[0][..])?;
        let challenge_state_update: StateUpdateOf<T> =
            Decode::decode(&mut &challenge_inputs[0][..])?;

        let inclusion_proof: InclusionProofOf<T> = Decode::decode(&mut &witness[0][..])?;

        ensure!(
            state_update.deposit_contract_address
                == challenge_state_update.deposit_contract_address,
            "DepositContractAddress is invalid",
        );
        ensure!(
            state_update.blockNumber > challenge_state_update.blockNumber,
            "BlockNumber must be smaller than challenged state",
        );
        ensure!(
            Self::is_sub_range(challenge_state_update.range, state_update.range),
            "Range must be subrange of stateUpdate",
        );

        // verify inclusion proof
        let block_number_bytes = Encode::encode(&challenge_state_update.block_number);
        let root = Self::retrieve(plapps_id, block_number_bytes);

        ensure!(
            Self::verify_inclusion_with_root(
                T::Hashing::hash_of(&challenge_state_update.state_object),
                challenge_state_update.deposit_contract_address,
                challenge_state_update.range,
                inclusion_proof,
                root,
            ),
            "Inclusion verification failed",
        );
        return (state_update, challenge_state_update, inclusion_proof);
    }

    fn is_sub_range(sub_range: RangeOf<T>, surrounding_range: RangeOf<T>) -> bool {
        sub_range.start >= surrounding_range.start && sub_range.end <= surrounding_range.end
    }

    pub fn bytes_to_bytes32(source: Vec<u8>) -> T::Hash {
        Decode::decode(&mut &source[..])?;
    }
}
