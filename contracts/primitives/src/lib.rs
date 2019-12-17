//! Primitives provide the primitive types of Plasma PGSpec.
//! Some of primitive types contain generics, the concrete implementations for Predicate.

#![cfg_attr(not(any(test, feature = "std")), no_std)]

use ink_core::env::{ContractEnv, DefaultSrmlTypes, EnvTypes};
use scale::{Codec, Decode, Encode};

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

/// Represents a range of state objects.
#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "ink-generate-abi", derive(type_metadata::Metadata))]
pub struct Range<I: traits::SimpleArithmetic + traits::Member + Codec> {
    /// Start of the range of objects.
    pub start: I,
    /// End of the range of objects.
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

impl<I: traits::SimpleArithmetic + traits::Member + Codec> ink_core::storage::Flush for Range<I> {
    fn flush(&mut self) {}
}

/// Represents a state object.
/// Contains the address of the predicate contract and input data to that
/// contract which control the conditions under which the object may be mutated.
#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "ink-generate-abi", derive(type_metadata::Metadata))]
pub struct StateObject<T: traits::Member + Codec> {
    /// Address of the predicate contract that dictates how the object can be mutated.
    pub predicate: AccountId,
    /// Arbitrary state data for the object.
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

/// Represents a state update, which contains the contextual information for how a particular range of state objects was mutated.
#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "ink-generate-abi", derive(type_metadata::Metadata))]
pub struct StateUpdate<
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    /// Range of state objects that were mutated.
    pub range: Range<I>,
    /// Resulting state object created by the mutation of the input objects.
    pub state_object: StateObject<T>,
    /// Plasma block number in which the update occurred.
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

/// Represents a checkpoint of a particular state update on which a “checkpoint game” is being or has been played out.
/// Checkpoints which have successfully passed the checkpoint game are considered “finalized”,
/// meaning the plasma contract should ignore all state updates on that range with an older plasma block number.
#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "ink-generate-abi", derive(type_metadata::Metadata))]
pub struct Checkpoint<
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    /// State update being checkpointed.
    pub state_update: StateUpdate<T, I>,
    /// Sub-range of the state update being checkpointed. We include this field because the update may be partially spent.
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
            return Ok(());
        }
        Err("error: sub_range is not sub range of state_update.range.")
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

/// From the perspective of each predicate, a transaction just consists of an arbitrary string of bytes.
/// Each predicate could parse these bytes in a unique way and therefore define its own transaction format.
/// However, clients should be able to correctly generate a transaction for any given predicate.
/// As a result, we’ve developed a standard transaction format that simplifies the transaction generation process.
#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "ink-generate-abi", derive(type_metadata::Metadata))]
pub struct Transaction<
    U: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    /// The address of the specific plasma deposit contract which identifies the asset being transferred.
    /// This is somewhat equivalent to Ethereum’s chain ID transaction parameter.
    pub predicate: AccountId,
    /// the range being transacted.
    pub range: Range<I>,
    /// Input parameters to be sent to the predicate along with method to compute the state transiton.
    /// Must be ABI encoded according to the Predicate API. This is similar to the transaction input value encoding in Ethereum.
    pub body: U,
}

/// Describes a challenge against a checkpoint.
/// A challenge is a claim that the challengingCheckpoint has no valid transactions,
/// meaning that the state update in the challengedCheckpoint could never have been reached and thus is invalid.
#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "ink-generate-abi", derive(type_metadata::Metadata))]
pub struct Challenge<
    T: traits::Member + Codec,
    I: traits::SimpleArithmetic + traits::Member + Codec,
> {
    /// Checkpoint being challenged.
    pub challenged_checkpoint: Checkpoint<T, I>,
    /// Checkpoint being used to challenge.
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
