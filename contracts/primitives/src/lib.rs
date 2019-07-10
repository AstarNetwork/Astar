#![cfg_attr(not(any(test, feature = "std")), no_std)]

use ink_core::{
    env::{ContractEnv, DefaultSrmlTypes, EnvTypes},
    memory::{string::String, vec::Vec},
};
use parity_codec::{Decode, Encode};

type AccountId = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::AccountId;
type Balance = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Balance;

pub type RangeNumber = u128;

// TODO use ink_core::env::DefaultSrmlTypes::BlockNumber when its implemented
pub type BlockNumber = u128;
pub type ChallengeNumber = u128;

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Range {
    start: RangeNumber,
    end: RangeNumber,
}

#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct StateObject {
    id: String,
    predicate: AccountId,
    data: Vec<u8>,
}

#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct StateUpdate {
    range: Range,
    state_object: StateObject,
    plasma_contract: AccountId,
    plasma_block_number: BlockNumber,
}

#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Checkpoint {
    state_update: StateUpdate,
    sub_range: Range,
}

#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct CheckpointStatus {
    challengeable_until: BlockNumber,
    outstanding_challenges: ChallengeNumber,
}

#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Challenge {
    challenged_checkpoint: Checkpoint,
    challenging_checkpoint: Checkpoint,
}

#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Transaction {
	plasma_contract: AccountId,
	range: Range,
	method_id: Vec<u8>,
	parameters: Vec<u8>,
}
