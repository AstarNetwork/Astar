//! Define events using PGSpec.

use ink_core::env::{ContractEnv, DefaultSrmlTypes, EnvTypes};
use scale::Codec;

use super::default::*;
use super::traits;

type BlockNumber = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::BlockNumber;
type Hash = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Hash;

macro_rules! event {
    (
        $( #[$event_meta:meta] )*
        $event_name:ident $(<$generic:ident>)* {
            $(
                $( #[$field_meta:meta] )*
                $vis:vis $field_name:ident : $field_ty:ty ,
            )*
        }
    ) => {
        $( #[$event_meta] )*
        #[derive(Clone, scale::Encode, scale::Decode, PartialEq, Eq)]
        #[cfg_attr(not(no_std), derive(Debug))]
        pub struct $event_name $(<$generic: traits::Member + Codec>)* {
            $(
                $( #[$field_meta] )*
                $vis $field_name : $field_ty
            ),*
        }

        impl<T: traits::Member + Codec> From<$event_name $(<$generic>)* > for public::Event<T> {
            fn from(event: $event_name $(<$generic>)*) -> Self {
                public::Event::$event_name(event)
            }
        }
    }
}

event! {
    /// Event deposited when a submit merkle root to parent chain contract(this contract) from child chain.
    BlockSubmitted {
        /// Block number that was published.
        pub number: BlockNumber,
        /// Header for that block.
        pub header: Hash,
    }
}

event! {
    /// Emitted whenever a user attempts to checkpoint a state update.
    CheckpointStarted<T> {
        /// ID of the checkpoint that was started.
        pub checkpoint: Checkpoint<T>,
        /// Ethereum block in which the checkpoint was started.
        pub challengeable_until: BlockNumber,
    }
}

event! {
    /// Emitted whenever an invalid history challenge has been started on a checkpoint.
    CheckpointChallenged<T> {
        /// The details of the challenge.
        pub challenge: Challenge<T>,
    }
}

event! {
    /// Emitted whenever a checkpoint is finalized.
    CheckpointFinalized {
        /// ID of the checkpoint that was finalized.
        pub checkpoint: Hash,
    }
}

event! {
    /// Emitted whenever an exit is started.
    ExitStarted {
        /// ID of the exit that was started.
        pub exit: Hash,
        /// Ethereum block in which the exit will be redeemable.
        pub redeemable_after: BlockNumber,
    }
}

event! {
    /// Emitted whenever an exit is finalized.
    ExitFinalized<T> {
        /// The checkpoint that had its exit finalized.
        pub exit: Checkpoint<T>,
    }
}

pub mod public {
    use super::*;
    #[derive(scale::Encode, scale::Decode)]
    pub enum Event<T: traits::Member + Codec> {
        BlockSubmitted(BlockSubmitted),
        CheckpointStarted(CheckpointStarted<T>),
        CheckpointChallenged(CheckpointChallenged<T>),
        CheckpointFinalized(CheckpointFinalized),
        ExitStarted(ExitStarted),
        ExitFinalized(ExitFinalized<T>),
    }
}
