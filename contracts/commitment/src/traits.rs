use super::*;
use ink_core::env::{ContractEnv, DefaultSrmlTypes};
use ink_model::{ContractState, EnvHandler};
use primitives::{
    events::*,
    traits::{Member, SimpleArithmetic},
    *,
};

/// Means of signature verification.
pub trait Verify {
    /// Verify a state_update. Return `true` if state_update is valid for the value.
    /// must be using leeaf node(state_update), idx as a left-index and merkle root(root).
    fn verify<T, I>(&self, state_update: &StateUpdate<T, I>, root: Hash) -> bool
    where
        T: Member + Codec,
        I: Member + SimpleArithmetic + Codec;
}

/// Each plasma chain MUST have at least one commitment contract.
/// Commitment contracts hold the block headers for the plasma chain.
/// Whenever the operator creates a new plasma block, they MUST publish this block to the commitment contract.
pub trait Commitment: ContractState {
    /// Initilizes our state to `current_block is 0` upon deploying our smart contract.
    fn deploy(&mut self, env: &mut EnvHandler<ContractEnv<DefaultSrmlTypes>>);

    /// Returns the current block number.
    fn current_block(&self, env: &mut EnvHandler<ContractEnv<DefaultSrmlTypes>>) -> BlockNumber;

    /// Returns the balance of the given AccountId.
    fn block_hash(
        &self,
        env: &mut EnvHandler<ContractEnv<DefaultSrmlTypes>>,
        number: BlockNumber,
    ) -> Option<Hash>;

    /// Allows a user to submit a block with the given header.
    /// `function submitBlock(bytes _header) public`
    fn submit_block(
        &mut self,
        env: &mut EnvHandler<ContractEnv<DefaultSrmlTypes>>,
        header: Hash,
    ) -> Result<BlockSubmitted>;

    /// Inclusion Proof.
    /// This function verifies state_update in PlasmaChain with inclusion_proof.
    fn verify_state_update_inclusion<T, P, I>(
        &self,
        env: &mut EnvHandler<ContractEnv<DefaultSrmlTypes>>,
        state_update: &StateUpdate<T, I>,
        inclusion_proof: &P,
    ) -> bool
    where
        T: Member + Codec,
        P: Member + Verify + Codec,
        I: Member + SimpleArithmetic + Codec;

    /// Inclusion Proof upper layer.
    /// verifyAssetStateRootInclusion
    fn verify_asset_state_root_inclusion<T, P, I>(
        &self,
        env: &mut EnvHandler<ContractEnv<DefaultSrmlTypes>>,
        asset_state: &StateUpdate<T, I>,
        inclusion_proof: &P,
    ) -> bool
    where
        T: Member + Codec,
        P: Member + Verify + Codec,
        I: Member + SimpleArithmetic + Codec;
}
