//! # Plasm Staking Module
//!
//! The Plasm staking module manages era, total amounts of rewards and how to distribute.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_event, decl_module, decl_storage,
    traits::{Currency, Imbalance, LockableCurrency, OnUnbalanced, Time},
    weights::SimpleDispatchInfo,
    StorageMap, StorageValue,
};
use frame_system::{self as system, ensure_root};
use pallet_plasm_rewards::{
    traits::{ComputeEraWithParam, EraFinder, ForSecurityEraRewardFinder, MaybeValidators},
    EraIndex,
};
use sp_runtime::{
    traits::{Saturating, Zero},
    Perbill,
};
pub use sp_staking::SessionIndex;
use sp_std::{prelude::*, vec::Vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod compute_era;
pub use compute_era::*;

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
pub type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;

type PositiveImbalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::PositiveImbalance;
type NegativeImbalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::NegativeImbalance;

pub trait Trait: system::Trait {
    /// The staking balance.
    type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    /// Time used for computing era duration.
    type Time: Time;

    /// Tokens have been minted and are unused for validator-reward. Maybe, dapps-staking uses ().
    type RewardRemainder: OnUnbalanced<NegativeImbalanceOf<Self>>;

    /// Handler for the unbalanced increment when rewarding a staker. Maybe, dapps-staking uses ().
    type Reward: OnUnbalanced<PositiveImbalanceOf<Self>>;

    /// The information of era.
    type EraFinder: EraFinder<EraIndex, SessionIndex, MomentOf<Self>>;

    /// The rewards for validators.
    type ForSecurityEraReward: ForSecurityEraRewardFinder<BalanceOf<Self>>;

    /// The return type of ComputeEraWithParam.
    type ComputeEraParam;

    /// Acutually computing of ComputeEraWithParam.
    type ComputeEra: ComputeEraOnModule<Self::ComputeEraParam>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as DappsStaking {
        /// The already untreated era is EraIndex.
        pub UntreatedEra get(fn untreated_era): EraIndex;

        /// The currently elected validator set keyed by stash account ID.
        pub ElectedValidators get(fn elected_validators):
            map hasher(twox_64_concat) EraIndex => Option<Vec<T::AccountId>>;

        /// Set of next era accounts that can validate blocks.
        pub Validators get(fn validators) config(): Vec<T::AccountId>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        fn on_finalize() {
            if let Some(active_era) = T::EraFinder::active() {
                let mut untreated_era = Self::untreated_era();

                while active_era.index > untreated_era {
                    let rewards = match T::ForSecurityEraReward::get(&untreated_era) {
                        Some(rewards) => rewards,
                        None => {
                            frame_support::print("Error: start_session_index must be set for current_era");
                            return;
                        }
                    };
                    let actual_rewarded = Self::reward_to_validators(&untreated_era, &rewards);

                    // deposit event to total validator rewards
                    Self::deposit_event(RawEvent::TotalValidatorRewards(untreated_era, actual_rewarded));

                    untreated_era+=1;
                }
                UntreatedEra::put(untreated_era);
            }
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
        ValidatorReward(EraIndex, AccountId, Balance),
        /// The total amount of minted rewards for validators.
        TotalValidatorRewards(EraIndex, Balance),
    }
);

impl<T: Trait> Module<T> {
    pub fn reward_to_validators(era: &EraIndex, max_payout: &BalanceOf<T>) -> BalanceOf<T> {
        if let Some(validators) = Self::elected_validators(era) {
            let validator_len: u64 = validators.len() as u64;
            let mut total_imbalance = <PositiveImbalanceOf<T>>::zero();
            for v in validators.iter() {
                let reward =
                    Perbill::from_rational_approximation(1, validator_len) * max_payout.clone();
                total_imbalance.subsume(Self::reward_validator(v, reward));
            }
            let total_payout = total_imbalance.peek();

            let rest = max_payout.saturating_sub(total_payout.clone());

            T::Reward::on_unbalanced(total_imbalance);
            T::RewardRemainder::on_unbalanced(T::Currency::issue(rest));
            total_payout
        } else {
            BalanceOf::<T>::zero()
        }
    }

    fn reward_validator(stash: &T::AccountId, reward: BalanceOf<T>) -> PositiveImbalanceOf<T> {
        T::Currency::deposit_into_existing(&stash, reward)
            .unwrap_or(PositiveImbalanceOf::<T>::zero())
    }
}

/// Returns the next validator candidate for calling by plasm-rewards when new era.
impl<T: Trait> MaybeValidators<EraIndex, T::AccountId> for Module<T> {
    fn compute(current_era: EraIndex) -> Option<Vec<T::AccountId>> {
        // Apply new validator set
        <ElectedValidators<T>>::insert(&current_era, <Validators<T>>::get());
        Some(Self::validators())
    }
}

/// Get the amount of staking per Era in a module in the Plasm Network
/// for callinng by plasm-rewards when end era.
impl<T: Trait> ComputeEraWithParam<EraIndex> for Module<T> {
    type Param = T::ComputeEraParam;
    fn compute(era: &EraIndex) -> T::ComputeEraParam {
        T::ComputeEra::compute(era)
    }
}
