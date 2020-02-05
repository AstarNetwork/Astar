#![cfg_attr(not(feature = "std"), no_std)]

use contracts::{BalanceOf, CodeHash, ContractAddressFor, Gas};
use sp_runtime::traits::{MaybeDisplay, MaybeSerialize, Member};
use sp_std::prelude::*;
use support::{decl_event, decl_module, decl_storage, Parameter};
use system::{ensure_signed, RawOrigin};

#[cfg(test)]
mod tests;

use crate::parameters::Verifiable;

pub trait OperatorFinder<AccountId> {
    fn contracts(operator_id: &AccountId) -> Vec<AccountId>;
}

#[derive(Clone, Eq, PartialEq, Default, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Offer<AccountId, Balance, Moment> {
    pub new_operator: AccountId,
    pub sender: AccountId,
    pub contracts: Vec<AccountId>,
    pub amount: Balance,
    pub expired: Moment,
}

pub type OfferOf<T> = Offer<<T as system::Trait>::AccountId, <T as balances::Trait>::Balance, <T as timestamp::Trait>::Moment>;

/// The module's configuration trait.
pub trait Trait: balances::Trait + timestamp::Trait {
    /// The identified offers.
    type OfferId: Parameter + Member + MaybeSerializeDeserialize + Debug + MaybeDisplay + Ord + Default;
    /// The helper of checking the state of operators.
    type OperatorFinder: OperatorFinder<Self::AccountId>;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Operator {
        /// A mapping from offerId
        pub Offers: linked_map T::OfferId => Option<T::AccountId>;

        /// A mapping from operators to operated contracts by them.
        pub OperatorHasContracts: map T::AccountId => Vec<T::AccountId>;
        /// A mapping from operated contract by operator to it.
        pub ContractHasOperator: linked_map T::AccountId => Option<T::AccountId>;
    }
}

decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        /// Deploys a contact and insert relation of a contract and an operator to mapping.
        pub fn offer(origin, sender: T::AccountId, contracts: Vec<T::AccountId>, amount: T::Balance, expired: Expired) {
            let new_operator = ensure_signed(origin)?;
            let offer_account = new_operator.clone()

            // check that the operator has contracts.
            let has_contracts = operator::<OperatorHasContracts<T>>::get(&sender);
            if !contracts.all(|v| {
                has_contracts.contains(&v)
            }) {
                return Err("sender does not have these contracts");
            }

            let offer = OfferOf<T> {
                new_operator,
                sender,
                contracts,
                amount,
                expired,
            }
            let offer_id = T::Hashing::hash_of(&offer);

            // insert new a offer.
            <Offers<T>>::insert(&offer_id, offer);
            Self::deposit_event(RawEvent::Offer(offer_account, offer_id));
        }

        pub fn reject(origin, offer: OfferId) {
        }

        pub fn accept(origin, offer: OfferId) {
        }
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = <T as balances::Trait>::Balance,
        Moment = <T as timestamp::Trait>::Moment,
        OfferId = <T as Trait>::OfferId,
    {
        /// When call offer,
        /// it is issued arguments:
        /// 1: New Operator(buyer)
        /// 2: OfferId
        Offer(AccountId, OfferId),

        /// When call reject,
        /// it is issued arguments:
        /// 1: Rejector account id(current operator and sender)
        /// 2: OfferId
        Reject(AccountId, OfferId),

        /// When call reject,
        /// it is issued arguments:
        /// 1: Acceptor account id(current operator and sender)
        /// 2: OfferId
        Accept(AccountId, OfferId),

    }
);
