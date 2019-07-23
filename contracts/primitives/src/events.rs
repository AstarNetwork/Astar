use ink_core::env::{ContractEnv, DefaultSrmlTypes, EnvTypes};
use parity_codec::{Codec, Decode, Encode};

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
		#[derive(Clone, Encode, Decode, PartialEq, Eq)]
		#[cfg_attr(not(no_std), derive(Debug))]
		pub struct $event_name $(<$generic: traits::Member + Codec>)* {
			$(
				$( #[$field_meta] )*
				$vis $field_name : $field_ty
			),*
		}

		impl<T: traits::Member + Codec> From<$event_name $(<$generic>)* > for private::Event<T> {
			fn from(event: $event_name $(<$generic>)*) -> Self {
				private::Event::$event_name(event)
			}
		}
	}
}

event! {
    /// Event deposited when a submit merkle root to parent chain contract(this contract) from child chain.
    ///
    /// ```
    /// event BlockSubmitted(
    ///		uint256 _number,
    ///		bytes _header
    /// );
    /// ```
    BlockSubmitted {
        pub number: BlockNumber,
        pub header: Hash,
    }
}

event! {
    CheckpointStarted<T> {
        pub checkpoint: Checkpoint<T>,
        pub challengeable_until: BlockNumber,
    }
}

event! {
    CheckpointChallenged<T> {
        pub challenge: Challenge<T>,
    }
}

event! {
    CheckpointFinalized {
        pub checkpoint: Hash,
    }
}

event! {
    ExitStarted {
        pub exit: Hash,
        pub redeemable_after: BlockNumber,
    }
}

event! {
    ExitFinalized<T> {
        pub exit: Checkpoint<T>,
    }
}

mod private {
    use super::*;
    #[doc(hidden)]
    #[derive(Encode, Decode)]
    pub enum Event<T: traits::Member + Codec> {
        BlockSubmitted(BlockSubmitted),
        CheckpointStarted(CheckpointStarted<T>),
        CheckpointChallenged(CheckpointChallenged<T>),
        CheckpointFinalized(CheckpointFinalized),
        ExitStarted(ExitStarted),
        ExitFinalized(ExitFinalized<T>),
    }

    /// Used to seal the emit trait.
    pub trait Sealed {}
}

pub trait EmitEventExt: private::Sealed {
    /// Emits the given event.
    fn emit<E, T>(&self, event: E)
    where
        T: traits::Member + Codec,
        E: Into<private::Event<T>>,
    {
        use parity_codec::Encode as _;
        <ink_core::env::ContractEnv<DefaultSrmlTypes> as ink_core::env::Env>::deposit_raw_event(
            &[],
            event.into().encode().as_slice(),
        )
    }
}

impl EmitEventExt for ink_model::EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>> {}
impl private::Sealed for ink_model::EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>> {}
