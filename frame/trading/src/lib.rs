#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use operator::{OperatorFinder, TransferOperator};
use sp_runtime::traits::Bounded;
use sp_std::fmt::Debug;
use sp_std::prelude::*;
use support::{
    decl_event, decl_module, decl_storage,
    traits::{
        Currency, ExistenceRequirement, LockIdentifier, LockableCurrency, Time, WithdrawReasons,
    },
    StorageLinkedMap,
};
use system::ensure_signed;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum OfferState {
    WAITING,
    REJECT,
    ACCEPT,
}

impl Default for OfferState {
    fn default() -> OfferState {
        OfferState::WAITING
    }
}

#[derive(Clone, Eq, PartialEq, Default, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct Offer<AccountId, Balance, Moment> {
    pub buyer: AccountId,
    pub sender: AccountId,
    pub contracts: Vec<AccountId>,
    pub amount: Balance,
    pub expired: Moment,
    pub state: OfferState,
}

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
pub type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;
pub type OfferOf<T> = Offer<<T as system::Trait>::AccountId, BalanceOf<T>, MomentOf<T>>;

const TRADING_ID: LockIdentifier = *b"trading_";

/// The module's configuration trait.
pub trait Trait: system::Trait {
    // use amount of values.
    type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
    /// Time used calculating expired.
    type Time: Time;
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
        pub Offers: linked_map T::AccountId => Option<OfferOf<T>>;
    }
}

decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        /// Deploys a contact and insert relation of a contract and an operator to mapping.
        pub fn offer(origin, sender: T::AccountId, contracts: Vec<T::AccountId>, amount: BalanceOf<T>, expired: MomentOf<T>) {
            let buyer = ensure_signed(origin)?;
            let offer_account = buyer.clone();

            if T::Currency::free_balance(&buyer) <= amount {
                Err("buyer does not have enough balances.")?
            }
            // check that the operator has contracts.
            let has_contracts = T::OperatorFinder::contracts(&sender);
            if !contracts.iter().all(|v| {
                has_contracts.contains(&v)
            }) {
                Err("sender does not have these contracts.")?
            }

            let offer = OfferOf::<T> {
                buyer,
                sender,
                contracts,
                amount,
                expired: T::Time::now() + expired,
                state: OfferState::WAITING,
            };

            if let Some(current_offer) = <Offers<T>>::get(&offer_account) {
                if current_offer.state == OfferState::WAITING {
                    Err("this offer was already issued.")?
                }
            }

            // lock amount
            T::Currency::set_lock(
                TRADING_ID, &offer.buyer, offer.amount,
                <T as system::Trait>::BlockNumber::max_value(),
                WithdrawReasons::all(),
            );
            // insert new a offer.
            <Offers<T>>::insert(&offer_account, offer);
            Self::deposit_event(RawEvent::Offer(offer_account));
        }

        pub fn reject(origin, offer_id: T::AccountId) {
            let rejector = ensure_signed(origin)?;
            let mut offer = match <Offers<T>>::get(&offer_id) {
                Some(o) => o,
                None => Err("can not find the offer id.")?
            };
            if rejector != offer.sender && rejector != offer.buyer {
                Err("the rejector can not reject. only sender or buyer can reject.")?;
            }
            if offer.state == OfferState::ACCEPT {
                Err("the offer was already accepted.")?;
            }

            offer.state = OfferState::REJECT;

            // unlock amount
            T::Currency::remove_lock(TRADING_ID, &offer.buyer);
            // insert changing offer
            <Offers<T>>::insert(&offer_id, offer);
            Self::deposit_event(RawEvent::Reject(rejector, offer_id));
        }

        pub fn accept(origin, offer_id: T::AccountId) {
            let acceptor = ensure_signed(origin)?;
            let mut offer = match <Offers<T>>::get(&offer_id) {
                Some(o) => o,
                None => Err("can not find the offer id.")?
            };
            if acceptor != offer.sender {
                Err("the accept can not accept. only sender can accept.")?;
            }
            if T::Time::now() > offer.expired {
                Err("the offer was already expired.")?;
            }


            // check change operator's contracts
            T::TransferOperator::verify_transfer_operator(&offer.sender, &offer.contracts)?;

            // unlock amount
            T::Currency::remove_lock(TRADING_ID, &offer.buyer);
            // transfer amount
            T::Currency::transfer(&offer.buyer, &offer.sender, offer.amount, ExistenceRequirement::KeepAlive)?;

            // change operator's contracts
            T::TransferOperator::force_transfer_operator(offer.sender.clone(), offer.contracts.clone(), offer.buyer.clone());

            // insert changing offer
            offer.state = OfferState::ACCEPT;
            <Offers<T>>::insert(&offer_id, offer);
            Self::deposit_event(RawEvent::Accept(acceptor, offer_id));
        }

        pub fn remove(origin) {
            let remover = ensure_signed(origin)?;
            let offer = match <Offers<T>>::get(&remover) {
                Some(o) => o,
                None => Err("the remover does not have a offer.")?
            };
            if offer.state == OfferState::WAITING  && offer.expired >= T::Time::now() {
                Err("the offer is living.")?
            }
            // unlock amount
            T::Currency::remove_lock(TRADING_ID, &offer.buyer);
            // remove the offer
            <Offers<T>>::remove(&remover);
            Self::deposit_event(RawEvent::Remove(remover));
        }
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        /// When call offer,
        /// it is issued arguments:
        /// 1: New Operator(buyer)
        Offer(AccountId),

        /// When call reject,
        /// it is issued arguments:
        /// 1: Rejector account id(current operator and sender)
        /// 2: Offer account id
        Reject(AccountId, AccountId),

        /// When call reject,
        /// it is issued arguments:
        /// 1: Acceptor account id(current operator and sender)
        /// 2: Offer account id
        Accept(AccountId, AccountId),

        /// When call remove,
        /// it is issued arguments:
        /// 1: the remover
        Remove(AccountId),
    }
);
