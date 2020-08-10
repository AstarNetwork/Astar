#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_event, decl_module, decl_storage,
    traits::{Currency, ExistenceRequirement, LockIdentifier, LockableCurrency, WithdrawReasons},
};
use frame_system::ensure_signed;
use pallet_contract_operator::{OperatorFinder, TransferOperator};
use sp_std::prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Clone, Eq, PartialEq, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum OfferState {
    Waiting,
    Reject,
    Accept,
}

impl Default for OfferState {
    fn default() -> OfferState {
        OfferState::Waiting
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
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
pub type OfferOf<T> = Offer<
    <T as frame_system::Trait>::AccountId,
    BalanceOf<T>,
    <T as frame_system::Trait>::BlockNumber,
>;

const TRADING_ID: LockIdentifier = *b"trading_";

/// The module's configuration trait.
pub trait Trait: frame_system::Trait {
    // use amount of values.
    type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
    /// The helper of checking the state of operators.
    type OperatorFinder: OperatorFinder<Self::AccountId>;
    /// The helper of transfering operator's authorities.
    type TransferOperator: TransferOperator<Self::AccountId>;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Operator {
        /// A mapping from the offering account id to Offer
        pub Offers: map hasher(blake2_128_concat) T::AccountId => Option<OfferOf<T>>;
    }
}

decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        /// Offer is an easy contract to trade.
        /// If the sender `accept` during the period, the operator trading will be completed.
        /// After the offer, the part of the amount of the buyer's balances will lock.
        ///
        /// Note: Only one offer can be issued at the same time each an account.
        /// TODO: weight
        #[weight = 500_000]
        pub fn offer(origin, sender: T::AccountId, contracts: Vec<T::AccountId>, amount: BalanceOf<T>, expired: T::BlockNumber) {
            let buyer = ensure_signed(origin)?;
            let offer_account = buyer.clone();
            let sender_account = sender.clone();

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
                expired,
                state: OfferState::Waiting,
            };

            if let Some(current_offer) = <Offers<T>>::get(&offer_account) {
                if current_offer.state == OfferState::Waiting {
                    Err("this offer was already issued.")?
                }
            }

            // lock amount
            T::Currency::set_lock(
                TRADING_ID, &offer.buyer, offer.amount,
                WithdrawReasons::all(),
            );
            // insert new a offer.
            <Offers<T>>::insert(&offer_account, offer);
            Self::deposit_event(RawEvent::Offer(offer_account, sender_account));
        }

        /// Reject the target offer.
        /// the offer's buyer or sender can reject the offer.
        /// After the reject, the buyer's balances will be unlock.
        /// TODO: weight
        #[weight = 100_000]
        pub fn reject(origin, offer_id: T::AccountId) {
            let rejector = ensure_signed(origin)?;
            let mut offer = match <Offers<T>>::get(&offer_id) {
                Some(o) => o,
                None => Err("can not find the offer id.")?
            };
            if rejector != offer.sender && rejector != offer.buyer {
                Err("the rejector can not reject. only sender or buyer can reject.")?;
            }
            if offer.state == OfferState::Accept {
                Err("the offer was already accepted.")?;
            }

            offer.state = OfferState::Reject;

            // unlock amount
            T::Currency::remove_lock(TRADING_ID, &offer.buyer);
            // insert changing offer
            <Offers<T>>::insert(&offer_id, offer);
            Self::deposit_event(RawEvent::Reject(rejector, offer_id));
        }

        /// Accept the target offer.
        /// Only the offer's sender can accept the offer.
        /// After the accept:
        ///  1. the buyer's balances will be unlock.
        ///  2. the buyer's balances tranfer to the sender.
        ///  3. the sender's target contracts transfer to the buyer.
        /// TODO: weight
        #[weight = 500_000]
        pub fn accept(origin, offer_id: T::AccountId) {
            let acceptor = ensure_signed(origin)?;
            let mut offer = match <Offers<T>>::get(&offer_id) {
                Some(o) => o,
                None => Err("can not find the offer id.")?
            };
            if acceptor != offer.sender {
                Err("the accept can not accept. only sender can accept.")?;
            }
            if <frame_system::Module<T>>::block_number() >= offer.expired {
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
            offer.state = OfferState::Accept;
            <Offers<T>>::insert(&offer_id, offer);
            Self::deposit_event(RawEvent::Accept(acceptor, offer_id));
        }

        /// Remove the offer.
        /// The offer's owner can remove the offer.
        /// But, if the offer is living(until expired), he can not remove the offer.
        /// TODO: weight
        #[weight = 100_000]
        pub fn remove(origin) {
            let remover = ensure_signed(origin)?;
            let offer = match <Offers<T>>::get(&remover) {
                Some(o) => o,
                None => Err("the remover does not have a offer.")?
            };
            if offer.state == OfferState::Waiting  && offer.expired > <frame_system::Module<T>>::block_number() {
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
        AccountId = <T as frame_system::Trait>::AccountId,
    {
        /// When call offer,
        /// it is issued arguments:
        /// 1: New Operator(buyer)
        /// 2: Current Operator(sender)
        Offer(AccountId, AccountId),

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
