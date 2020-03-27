//! # Plasm Staking Module
//!
//! The Plasm staking module manages era, total amounts of rewards and how to distribute.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use operator::ContractFinder;
use rewards::traits::EraFinder;
use session::SessionManager;
use sp_runtime::{
    traits::{
        CheckedAdd, CheckedDiv, CheckedSub, One, SaturatedConversion, Saturating, StaticLookup,
        Zero,
    },
    PerThing, Perbill, RuntimeDebug,
};
use sp_std::{collections::btree_map::BTreeMap, convert::TryFrom, prelude::*, result, vec::Vec};
pub use staking::{Forcing, Nominations, RewardDestination};
use support::{
    decl_event, decl_module, decl_storage, ensure,
    storage::IterableStorageMap,
    traits::{
        Currency, Get, Imbalance, LockIdentifier, LockableCurrency, OnUnbalanced, Time,
        WithdrawReasons,
    },
    weights::SimpleDispatchInfo,
    StorageMap, StorageValue,
};
use system::{ensure_root, ensure_signed};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use sp_staking::SessionIndex;

pub type EraIndex = u32;
pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
pub type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;

type PositiveImbalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::PositiveImbalance;
type NegativeImbalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::NegativeImbalance;

pub trait Trait: session::Trait {
    /// The staking balance.
    type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    /// Number of eras that staked funds must remain bonded for.
    type BondingDuration: Get<EraIndex>;

    /// Tokens have been minted and are unused for validator-reward. Maybe, plasm-staking uses ().
    type RewardRemainder: OnUnbalanced<NegativeImbalanceOf<Self>>;

    /// Handler for the unbalanced increment when rewarding a staker. Maybe, plasm-staking uses ().
    type Reward: OnUnbalanced<PositiveImbalanceOf<Self>>;

    /// Time used for computing era duration.
    type Time: Time;

    /// Number of sessions per era.
    type SessionsPerEra: Get<SessionIndex>;

    /// The information of era.
    type EraFinder: EraFinder;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as PlasmStaking {
        // ---- Era manages.
        /// The currently elected validator set keyed by stash account ID.
        pub CurrentElected get(fn current_elected): Vec<T::AccountId>;

        /// The version of storage for upgrade.
        pub StorageVersion get(fn storage_version) config(): u32;

        /// Set of next era accounts that can validate blocks.
        pub Validators get(fn validators) config(): Vec<T::AccountId>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        fn on_initialize() {
            Self::ensure_storage_upgraded();
        }

        fn on_finalize() {
            // TODOT::GettingEra
        }

        // ----- Root calls.
        /// Manually set new validators.
        ///
        /// # <weight>
        /// - One storage write
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedOperational(0)]
        fn set_validators(origin, new_validators: Vec<T::AccountId>) {
            ensure_root(origin)?;
            <Validators<T>>::put(&new_validators);
            Self::deposit_event(RawEvent::NewValidators(new_validators));
        }


    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = BalanceOf<T>,
    {
        /// Validator set changed.
        NewValidators(Vec<AccountId>),
        /// The amount of minted rewards for validators.
        ValidatorReward(AccountId, Balance),
    }
);

impl<T: Trait> Module<T> {
    pub fn reward_to_validators(
        total_payout: BalanceOf<T>,
        max_payout: BalanceOf<T>,
    ) -> BalanceOf<T> {
        let validators = Self::current_elected();
        let validator_len: u64 = validators.len() as u64;
        let mut total_imbalance = <PositiveImbalanceOf<T>>::zero();
        for v in validators.iter() {
            let reward = Perbill::from_rational_approximation(1, validator_len) * total_payout;
            total_imbalance.subsume(Self::reward_validator(v, reward));
        }
        let total_payout = total_imbalance.peek();

        let rest = max_payout.saturating_sub(total_payout.clone());

        T::Reward::on_unbalanced(total_imbalance);
        T::RewardRemainder::on_unbalanced(T::Currency::issue(rest));
        total_payout
    }

    fn reward_validator(stash: &T::AccountId, reward: BalanceOf<T>) -> PositiveImbalanceOf<T> {
        T::Currency::deposit_into_existing(&stash, reward)
            .unwrap_or(PositiveImbalanceOf::<T>::zero())
    }
}

/// Returns the next validator candidate for calling by plasm-rewards when new era.
impl<T: Trait> MaybeValidators<EraIndex, T::AccountId> for Module<T> {
    fn maybe_validators(_current_era: EraIndex) -> Option<Vec<T::AccountId>> {
        // Apply new validator set
        <CurrentElected<T>>::put(<Validators<T>>::get());
        Some(Self::validators())
    }
}

/// Get the amount of staking per Era in a module in the Plasm Network
/// for callinng by plasm-rewards when end era.
impl<T: Trait> GetEraStakingAmount<EraIndex, T::Balance> for Module<T> {
    fn get_era_staking_amount(era: EraIndex) -> T::Balance {
        0
    }
}
