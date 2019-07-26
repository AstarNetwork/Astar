use parity_codec::{Codec, Decode, Encode};
use sr_primitives::traits::{
    MaybeDisplay, MaybeSerializeDebug, Member, SimpleArithmetic, SimpleBitOps,
};
use support::Parameter;

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Range<I: SimpleArithmetic + Member + Codec> {
    pub start: I,
    pub end: I,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct StateObject<
    AccountId: Parameter + Member + MaybeSerializeDebug + MaybeDisplay + Ord + Default,
    T: Member + Codec,
> {
    pub predicate: AccountId,
    pub data: T,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct StateUpdate<
    AccountId: Parameter + Member + MaybeSerializeDebug + MaybeDisplay + Ord + Default,
    T: Member + Codec,
    I: SimpleArithmetic + Member + Codec,
    BlockNumber: SimpleArithmetic + Member + Codec,
> {
    pub range: Range<I>,
    pub state_object: StateObject<AccountId, T>,
    pub plasma_block_number: BlockNumber,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Checkpoint<
    AccountId: Parameter + Member + MaybeSerializeDebug + MaybeDisplay + Ord + Default,
    T: Member + Codec,
    I: SimpleArithmetic + Member + Codec,
    BlockNumber: SimpleArithmetic + Member + Codec,
> {
    pub state_update: StateUpdate<AccountId, T, I, BlockNumber>,
    pub sub_range: Range<I>,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Transaction<
    AccountId: Parameter + Member + MaybeSerializeDebug + MaybeDisplay + Ord + Default,
    U: Member + Codec,
    I: SimpleArithmetic + Member + Codec,
> {
    pub predicate: AccountId,
    pub range: Range<I>,
    pub body: U,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Challenge<
    AccountId: Parameter + Member + MaybeSerializeDebug + MaybeDisplay + Ord + Default,
    T: Member + Codec,
    I: SimpleArithmetic + Member + Codec,
    BlockNumber: SimpleArithmetic + Member + Codec,
> {
    pub challenged_checkpoint: Checkpoint<AccountId, T, I, BlockNumber>,
    pub challenging_checkpoint: Checkpoint<AccountId, T, I, BlockNumber>,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct MerkleIndexTreeInternalNode<
    I: Member + SimpleArithmetic + Codec,
    Hash: Member
        + MaybeSerializeDebug
        + ::rstd::hash::Hash
        + Copy
        + MaybeDisplay
        + Default
        + SimpleBitOps
        + Codec
        + AsRef<[u8]>
        + AsMut<[u8]>,
> {
    pub index: I,
    pub hash: Hash,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct InclusionProof<
    I: Member + SimpleArithmetic + Codec,
    Hash: Member
        + MaybeSerializeDebug
        + ::rstd::hash::Hash
        + Copy
        + MaybeDisplay
        + Default
        + SimpleBitOps
        + Codec
        + AsRef<[u8]>
        + AsMut<[u8]>,
> {
    pub proofs: Vec<MerkleIndexTreeInternalNode<I, Hash>>,
    pub idx: I,
}
