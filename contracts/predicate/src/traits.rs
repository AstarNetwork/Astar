use super::*;
use ink_model::ContractState;
use primitives::*;

pub trait Predicate<T, B, W, I, C, D>: ContractState
where
    T: Member + Codec,
    B: Member + Codec,
    W: Codec,
    I: Member + SimpleArithmetic + Codec,
    C: commitment::traits::Commitment,
    D: deposit::traits::Deposit<I, C>,
{
    /// deplpy predicate contract.
    fn deploy(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        token_address: AccountId,
        chalenge_period: BlockNumber,
        exit_period: BlockNumber,
    );

    /// Predicates MUST define a custom _witness struct for their particular type of state.
    /// Predicates MUST disallow state transitions which pass verification without some interested party’s consent, e.g. the owner’s signature
    fn verify_transaction(
        &self,
        pre_state: StateUpdate<T, I>,
        transaction: Transaction<B, I>,
        witness: W,
        post_state: StateUpdate<T, I>,
    ) -> bool;

    /// Allows the predicate contract to start an exit from a checkpoint. Checkpoint may be pending or finalized.
    fn start_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<T, I>,
    );

    /// Allows the predicate address to cancel an exit which it determines is deprecated.
    fn deprecate_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        deprecated_exit: Checkpoint<T, I>,
        transaction: Transaction<B, I>,
        witness: W,
        post_state: StateUpdate<T, I>,
    );

    /// Finalizes an exit that has passed its exit period and has not been successfully challenged.
    fn finalize_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        exit: Checkpoint<T, I>,
        deposited_range_id: I,
    );

    fn commitment(&self) -> &C;
    fn deposit(&self) -> &D;
}
