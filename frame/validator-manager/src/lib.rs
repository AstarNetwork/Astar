#![cfg_attr(not(feature = "std"), no_std)]

use support::{decl_module, decl_event, decl_storage, dispatch::Result};
use system::ensure_root;
use sp_std::prelude::*;

mod mock;
mod tests;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

pub trait OnEraEnding<ValidatorId, EraIndex> {
	fn on_era_ending(
		_ending: EraIndex,
		_start_era: EraIndex,
	) -> Option<Vec<ValidatorId>>;
}


pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        pub fn set_validators(origin, new_validators: Vec<T::AccountId>) -> Result {
            ensure_root(origin)?;
            <Validators<T>>::put(&new_validators);
            Self::deposit_event(RawEvent::NewValidators(new_validators));
            Ok(())
        }
    }
}

decl_event! {
    pub enum Event<T> where <T as system::Trait>::AccountId {
        NewValidators(Vec<AccountId>),
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as ValidatorManager {
        pub Validators get(fn validators) config(): Vec<T::AccountId>;
    }
}

impl<T: Trait> OnEraEnding<T::AccountId, EraIndex> for Module<T> {
    fn on_era_ending(
        _ending: EraIndex,
        _start_era: EraIndex,
    ) -> Option<Vec<T::AccountId>> {
        Some(<Validators<T>>::get())
    }
}

impl<T: Trait> session::OnSessionEnding<T::AccountId> for Module<T> {
	fn on_session_ending(
		_ending: u32,
		_start_session: u32,
	) -> Option<Vec<T::AccountId>> {
		Some(<Validators<T>>::get())
	}
}

impl<T: Trait> session::SelectInitialValidators<T::AccountId> for Module<T> {
    fn select_initial_validators() -> Option<Vec<T::AccountId>> {
        Some(<Validators<T>>::get())
    }
}
