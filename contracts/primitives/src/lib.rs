#![cfg_attr(not(any(test, feature = "std")), no_std)]

use ink_core::env::{ContractEnv, DefaultSrmlTypes, EnvTypes};
use parity_codec::{Codec, Decode, Encode};

type AccountId = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::AccountId;
type BlockNumber = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::BlockNumber;
type Hash = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Hash;

pub mod default;
pub mod events;
pub mod traits;

pub type Result<T> = core::result::Result<T, &'static str>;

pub trait Verify {
    fn verify(&self) -> Result<()>;
}

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct Range<I: traits::SimpleArithmetic + traits::Member + Codec> {
    pub start: I,
    pub end: I,
}

impl<I: traits::SimpleArithmetic + traits::Member + Codec> Range<I> {
    pub fn subrange(&self, sub_range: &Range<I>) -> bool {
        self.start <= sub_range.start && sub_range.end <= self.end
    }
}

pub fn is_intersects<I: traits::SimpleArithmetic + traits::Member + Codec>(
    a: &Range<I>,
    b: &Range<I>,
) -> bool {
    (a.start <= b.start && b.start <= a.end) || (a.start <= b.end && b.end <= a.end)
}

impl<I> Verify for Range<I>
where
    I: traits::SimpleArithmetic + traits::Member + Codec,
{
    fn verify(&self) -> Result<()> {
        if self.start > self.end {
            return Err("error: start > end.");
        }
        Ok(())
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct StateObject<T: traits::Member + Codec> {
    pub predicate: AccountId,
    pub data: T,
}

impl<T> Verify for StateObject<T>
where
    T: traits::Member + Codec,
{
    fn verify(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct StateUpdate<
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    pub range: Range<I>,
    pub state_object: StateObject<T>,
    pub plasma_block_number: BlockNumber,
}

impl<T, I> Verify for StateUpdate<T, I>
where
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
{
    fn verify(&self) -> Result<()> {
        self.range.verify()?;
        self.state_object.verify()?;
        Ok(())
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct Checkpoint<
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    pub state_update: StateUpdate<T, I>,
    pub sub_range: Range<I>,
}

impl<T, I> Verify for Checkpoint<T, I>
where
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
{
    fn verify(&self) -> Result<()> {
        self.state_update.verify()?;
        if self.state_update.range.start <= self.sub_range.start
            && self.sub_range.end <= self.state_update.range.end
        {
            return Err("error: sub_range is not sub range of state_update.range.");
        }
        Ok(())
    }
}

impl<T, I> Checkpoint<T, I>
where
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
{
    pub fn is_intersect(&self, checkpoint: &Checkpoint<T, I>) -> bool {
        (self.state_update.range.start <= checkpoint.state_update.range.start
            && checkpoint.state_update.range.start <= self.state_update.range.end)
            || (self.state_update.range.start <= checkpoint.state_update.range.end
                && checkpoint.state_update.range.end <= self.state_update.range.end)
    }

    pub fn id(&self) -> Hash {
        keccak256(&self)
    }
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct Transaction<
    U: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    pub predicate: AccountId,
    pub range: Range<I>,
    pub body: U,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(not(no_std), derive(Debug))]
pub struct Challenge<
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    pub challenged_checkpoint: Checkpoint<T, I>,
    pub challenging_checkpoint: Checkpoint<T, I>,
}

impl<T, I> Challenge<T, I>
where
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
{
    pub fn id(&self) -> Hash {
        keccak256(&self)
    }
}

pub fn keccak256<E: Encode>(data: &E) -> Hash {
    Hash::decode(&mut &ink_utils::hash::keccak256(&data.encode()[..])[..])
        .expect("Hash decoded error in keccak256.")
}
