use parity_codec::{Codec, Decode, Encode};
use ink_core::env::{ContractEnv, DefaultSrmlTypes, EnvTypes};

use super::default::*;
use super::traits;

type BlockNumber = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Balance;
type Hash = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Hash;

macro_rules! event {
	(
		$( #[$event_meta:meta] )*
		$event_name:ident $( <$generic:ident> )* {
			$(
				$( #[$field_meta:meta] )*
				$field_name:ident : $field_ty:ty ,
			)*
		}
	) => {
		$( #[$event_meta] )*
		#[derive(Encode, Decode)]
		#[cfg_attr(feature = "std", derive(Debug))]
		pub struct $event_name $(<$generic>)* {
			$(
			$( #[$field_meta] )*
			$field_name : $field_ty
			),*
		}

		impl<T: traits::Member + Codec> From<$event_name $(<$generic>)* > for private::Event<T> {
			fn from(event: $event_name) -> Self {
				private::Event::$event_name(event)
			}
		}
	}
}

event! {
	BlockSubmitted {
		number: BlockNumber,
		header: Hash,
	}
}

#[derive(Encode, Decode)]
pub struct CheckpointStarted<T: traits::Member + Codec> {
    checkpoint: Checkpoint<T>,
    challengeable_until: BlockNumber,
}

#[derive(Encode, Decode)]
pub struct CheckpointChallenged<T: traits::Member + Codec> {
    challenge: Challenge<T>,
}

#[derive(Encode, Decode)]
pub struct CheckpointFinalized {
    checkpoint: Hash,
}

#[derive(Encode, Decode)]
pub struct ExitStarted {
    exit: Hash,
    redeemable_after: BlockNumber,
}

#[derive(Encode, Decode)]
pub struct ExitFinalized<T: traits::Member + Codec> {
    exit: Checkpoint<T>,
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
