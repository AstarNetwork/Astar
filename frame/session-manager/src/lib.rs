#![cfg_attr(not(feature = "std"), no_std)]

use support::{decl_module, decl_event, decl_storage, dispatch::Result};
use system::ensure_root;
use rstd::prelude::*;

mod mock;
mod tests;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

        fn set_validators(origin, new_validators: Vec<T::AccountId>) -> Result {
            ensure_root(origin)?;
            <NextValidators<T>>::put(&new_validators);
            Self::deposit_event(RawEvent::NewValidators(new_validators));
            Ok(())
        }
    }
}

decl_event! {
    pub enum Event<T> where <T as system::Trait>::AccountId {
        NewValidators(Vec<AccountId>),
        RotateValidators(Vec<AccountId>),
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as SessionManager {
    	/// The current era validators.
        pub Validators get(fn validators) config(): Vec<T::AccountId>;

        /// The next era validators.
        pub NextValidators get(fn next_validators) : Vec<T::AccountId>;
    }
}

impl<T: Trait> session::OnSessionEnding<T::AccountId> for Module<T> {
    fn on_session_ending(
        _ending: u32,
        _start_session: u32,
    ) -> Option<Vec<T::AccountId>> {
		let new_validators = <NextValidators<T>>::get();
		if new_validators.is_empty() {
			return Some(<Validators<T>>::get())
		}
		<Validators<T>>::put(&new_validators);
		Self::deposit_event(RawEvent::RotateValidators(new_validators.clone()));
		Some(new_validators)
	}
}

impl<T: Trait> session::SelectInitialValidators<T::AccountId> for Module<T> {
    fn select_initial_validators() -> Option<Vec<T::AccountId>> {
        Some(<Validators<T>>::get())
    }
}
