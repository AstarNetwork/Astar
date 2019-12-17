//! # Dapps Rewards Module
//!
//! The Dapps rewards module manages era and total amounts of rewards and how to distribute.

use support::{decl_module, decl_storage, decl_event, StorageValue, weights::SimpleDispatchInfo,
              traits::{LockableCurrency, Time, Get, Currency}};
use system::ensure_root;
use session::OnSessionEnding;
use sp_staking::SessionIndex;
use sp_runtime::RuntimeDebug;
#[cfg(feature = "std")]
use sp_runtime::{Serialize, Deserialize};

use validator_manager::{EraIndex, OnEraEnding};
use codec::{Encode, Decode};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod traits;

pub type BalanceOf<T> =
<<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
pub type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;

pub trait Trait: session::Trait {
    /// The staking balance.
    type Currency: LockableCurrency<Self::AccountId, Moment=Self::BlockNumber>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as DappsRewards {

    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        fn on_initialize() {
        }

        fn on_finalize() {
            // Set the start of the first era.
            if !<CurrentEraStart<T>>::exists() {
                <CurrentEraStart<T>>::put(T::Time::now());
            }
        }

        fn staking_to_contract(origin, targets: Vec<<T::Lookup as StaticLookup>::Source) {

        }


    }
}

decl_event!(
    pub enum Event<T> where Balance = BalanceOf<T> {
        /// All validators have been rewarded;
        Reward(Balance),
    }
);


impl<T: Trait> Module<T> {}

impl<T: Trait> OnDistributeRewards<T::AccountId> for Module<T> {
    fn on_session_ending(ending: SessionIndex, will_apply_at: SessionIndex) -> Option<Vec<T::AccountId>> {
        Self::ensure_storage_upgraded();
        Self::new_session(ending, will_apply_at)
    }
}
