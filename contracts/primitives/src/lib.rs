#![cfg_attr(not(any(test, feature = "std")), no_std)]

use ink_core::env::{ContractEnv, DefaultSrmlTypes, EnvTypes};
use parity_codec::{Codec, Decode, Encode};

type AccountId = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::AccountId;
type BlockNumber = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::BlockNumber;

pub mod default;
pub mod events;
pub mod traits;

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Range<I: traits::SimpleArithmetic + traits::Member + Codec> {
    pub start: I,
    pub end: I,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct StateObject<T: traits::Member + Codec> {
    pub predicate: AccountId,
    pub data: T,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct StateUpdate<
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    pub range: Range<I>,
    pub state_object: StateObject<T>,
    pub plasma_block_number: BlockNumber,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Checkpoint<
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    pub state_update: StateUpdate<T, I>,
    pub sub_range: Range<I>,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Transaction<
    U: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    pub deposit_contract: AccountId,
    pub range: Range<I>,
    pub body: U,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Challenge<
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    pub challenged_checkpoint: Checkpoint<T, I>,
    pub challenging_checkpoint: Checkpoint<T, I>,
}
