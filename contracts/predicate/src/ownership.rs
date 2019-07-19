use super::*;
use commitment::traits::Verify;
use core::marker::PhantomData;
use ink_core::{
    memory::{format, vec::Vec},
    storage,
};
use primitives::default::*;
use deposit::traits::Deposit;

ink_model::state! {
    pub struct Predicate {
        // deposit contract
        DEPOSIT: deposit::default::Deposit,
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct TransactionBody {
    new_state_object: StateObject<AccountId>,
    origin_block: BlockNumber,
    max_block: BlockNumber,
}

#[derive(Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Signature(pub [u8; 64]);

impl
    traits::Predicate<
        AccountId,
        TransactionBody,
        Signature,
        RangeNumber,
        commitment::default::Commitment,
        deposit::default::Deposit,
    > for Predicate
{
    /// deplpy predicate contract.
    fn deploy(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        token_address: AccountId,
        chalenge_period: BlockNumber,
        exit_period: BlockNumber,
    ) {
    }

    /// Predicates MUST define a custom _witness struct for their particular type of state.
    /// Predicates MUST disallow state transitions which pass verification without some interested party’s consent, e.g. the owner’s signature
    fn verify_transaction(
        &self,
        pre_state: StateUpdate<AccountId>,
        transaction: Transaction<TransactionBody>,
        witness: Signature,
        post_state: StateUpdate<AccountId>,
    ) -> bool {
        true
    }

    /// Allows the predicate contract to start an exit from a checkpoint. Checkpoint may be pending or finalized.
    fn start_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<AccountId>,
    ) {
    }

    /// Allows the predicate address to cancel an exit which it determines is deprecated.
    fn deprecate_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        deprecated_exit: Checkpoint<AccountId>,
        transaction: Transaction<TransactionBody>,
        witness: Signature,
        post_state: StateUpdate<AccountId>,
    ) {
    }

    /// Finalizes an exit that has passed its exit period and has not been successfully challenged.
    fn finalize_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        exit: Checkpoint<AccountId>,
        deposited_range_id: RangeNumber,
    ) {
    }

    fn commitment(&self) -> &commitment::default::Commitment {
        self.DEPOSIT.commitment()
    }
    fn deposit(&self) -> &deposit::default::Deposit {
        &self.DEPOSIT
    }
}
