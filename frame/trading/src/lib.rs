#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use operator::{OperatorFinder, TransferOperator};
use sp_runtime::traits::Hash;
use sp_std::fmt::Debug;
use sp_std::prelude::*;
use support::{decl_event, decl_module, decl_storage, Parameter, StorageLinkedMap};
use system::{ensure_signed, RawOrigin};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Clone, Eq, PartialEq, Default, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Offer<AccountId, Balance, Moment> {
    pub new_operator: AccountId,
    pub sender: AccountId,
    pub contracts: Vec<AccountId>,
    pub amount: Balance,
    pub expired: Moment,
}

pub type OfferOf<T> = Offer<
    <T as system::Trait>::AccountId,
    <T as balances::Trait>::Balance,
    <T as timestamp::Trait>::Moment,
>;

/// The module's configuration trait.
pub trait Trait: balances::Trait + timestamp::Trait {
    /// The helper of checking the state of operators.
    type OperatorFinder: OperatorFinder<Self::AccountId>;
    /// The helper of transfering operator's authorities.
    type TransferOperator: TransferOperator<Self::AccountId>;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Operator {
        /// A mapping from offerId to Offer
        pub Offers: linked_map T::Hash => Option<OfferOf<T>>;
    }
}

decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        /// Deploys a contact and insert relation of a contract and an operator to mapping.
        pub fn offer(origin, sender: T::AccountId, contracts: Vec<T::AccountId>, amount: T::Balance, expired: T::Moment) {
            let new_operator = ensure_signed(origin)?;
            let offer_account = new_operator.clone();

            // check that the operator has contracts.
            let has_contracts = T::OperatorFinder::contracts(&sender);
            if !contracts.iter().all(|v| {
                has_contracts.contains(&v)
            }) {
                Err("sender does not have these contracts")?
            }

            let offer = OfferOf::<T> {
                new_operator,
                sender,
                contracts,
                amount,
                expired,
            };
            let offer_id = T::Hashing::hash_of(&offer);

            // insert new a offer.
            <Offers<T>>::insert(&offer_id, offer);
            Self::deposit_event(RawEvent::Offer(offer_account, offer_id));
        }

        pub fn reject(origin, offer: T::Hash) {
        }

        pub fn accept(origin, offer: T::Hash) {
        }
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Hash = <T as system::Trait>::Hash,
    {
        /// When call offer,
        /// it is issued arguments:
        /// 1: New Operator(buyer)
        /// 2: Hash
        Offer(AccountId, Hash),

        /// When call reject,
        /// it is issued arguments:
        /// 1: Rejector account id(current operator and sender)
        /// 2: Hash
        Reject(AccountId, Hash),

        /// When call reject,
        /// it is issued arguments:
        /// 1: Acceptor account id(current operator and sender)
        /// 2: Hash
        Accept(AccountId, Hash),
    }
);
