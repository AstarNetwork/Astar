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
        /// Number of sessions per era.
        const SessionsPerEra: SessionIndex = T::SessionsPerEra::get();

        fn deposit_event() = default;

        fn on_initialize() {
            Self::ensure_storage_upgraded();
        }

        fn on_finalize() {
            // Set the start of the first era.
            if !<CurrentEraStart<T>>::exists() {
                <CurrentEraStart<T>>::put(T::Time::now());
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
        ValidatorReward(AccountId, Balance),
    }
);

impl<T: Trait> Module<T> {
    // MUTABLES (DANGEROUS)

    /// Session has just ended. Provide the validator set for the next session if it's an era-end, along
    /// with the exposure of the prior validator set.
    pub fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        let era_length = new_index
            .checked_sub(Self::current_era_start_session_index())
            .unwrap_or(0);
        match ForceEra::get() {
            Forcing::ForceNew => ForceEra::kill(),
            Forcing::ForceAlways => (),
            Forcing::NotForcing if era_length > T::SessionsPerEra::get() => (),
            _ => return None,
        }
        Self::new_era(new_index)
    }

    /// The era has changed - enact new staking set.
    ///
    /// NOTE: This always happens immediately before a session change to ensure that new validators
    /// get a chance to set their session keys.
    pub fn new_era(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        let now = T::Time::now();
        let previous_era_start = <CurrentEraStart<T>>::mutate(|v| sp_std::mem::replace(v, now));
        let era_duration = now - previous_era_start;
        if !era_duration.is_zero() {
            // When PoA, used by compute_total_payout_test.
            let (total_payout, _) = inflation::compute_total_payout_test(
                T::Currency::total_issuance(),
                era_duration.saturated_into::<u64>(),
            );
            let total_payout_v = total_payout
                .checked_div(&BalanceOf::<T>::from(2))
                .unwrap_or(BalanceOf::<T>::zero());
            let total_payout_o = total_payout
                .checked_sub(&total_payout_v)
                .unwrap_or(BalanceOf::<T>::zero());
            let reward_v = Self::reward_to_validators(total_payout_v.clone(), total_payout_v);
            let reward_o = Self::reward_to_operators(total_payout_o.clone(), total_payout_o);
            Self::deposit_event(RawEvent::Reward(reward_v, reward_o));
        }

        CurrentEra::mutate(|era| *era += 1);
        CurrentEraStartSessionIndex::put(new_index - 1);

        Self::elected_operators();

        // Apply new validator set
        <CurrentElected<T>>::put(<Validators<T>>::get());
        Some(<Validators<T>>::get())
    }

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

    pub fn reward_to_operators(
        total_payout: BalanceOf<T>,
        max_payout: BalanceOf<T>,
    ) -> BalanceOf<T> {
        let mut total_imbalance = <PositiveImbalanceOf<T>>::zero();
        let operators_reward =
            Perbill::from_rational_approximation(BalanceOf::<T>::from(4), BalanceOf::<T>::from(5))
                * total_payout;
        let nominators_reward = total_payout
            .checked_sub(&operators_reward)
            .unwrap_or(BalanceOf::<T>::zero());
        let staked_contracts = <StakedContracts<T>>::iter()
            .collect::<Vec<(T::AccountId, Exposure<T::AccountId, BalanceOf<T>>)>>();
        let total_staked = staked_contracts
            .iter()
            .fold(BalanceOf::<T>::zero(), |sum, (_, exposure)| {
                sum.checked_add(&exposure.total).unwrap_or(sum)
            });

        for (contract, exposure) in staked_contracts.iter() {
            let reward = Perbill::from_rational_approximation(exposure.total, total_staked)
                * operators_reward;
            total_imbalance.subsume(Self::reward_contract(&contract, reward));
        }

        let nominate_totals = staked_contracts.iter().fold(
            BTreeMap::<T::AccountId, BalanceOf<T>>::new(),
            |bmap, (_, exposure)| {
                exposure.others.iter().fold(bmap, |mut bmap, ind| {
                    if bmap.contains_key(&ind.who) {
                        if let Some(indv) = bmap.get_mut(&ind.who) {
                            *indv += ind.value;
                        }
                    } else {
                        bmap.insert(ind.who.clone(), ind.value);
                    }
                    return bmap;
                })
            },
        );

        for (nominator, staked) in nominate_totals.iter() {
            let reward =
                Perbill::from_rational_approximation(*staked, total_staked) * nominators_reward;
            total_imbalance.subsume(
                Self::make_payout(nominator, reward).unwrap_or(PositiveImbalanceOf::<T>::zero()),
            );
        }
        let total_payout = total_imbalance.peek();

        let rest = max_payout.saturating_sub(total_payout.clone());

        T::Reward::on_unbalanced(total_imbalance);
        T::RewardRemainder::on_unbalanced(T::Currency::issue(rest));
        total_payout
    }

    fn elected_operators() {
        let nominations = <DappsNominations<T>>::iter()
            .filter(|(_, nomination)| !nomination.suppressed)
            .collect::<Vec<(T::AccountId, Nominations<T::AccountId>)>>();
        let nominators = nominations
            .iter()
            .cloned()
            .map(|(stash, _)| stash)
            .collect::<Vec<T::AccountId>>();
        let nominators_to_staking = nominators
            .into_iter()
            .map(|nominator| {
                if let Some(ctrl) = Self::bonded(&nominator) {
                    if let Some(ledger) = Self::ledger(&ctrl) {
                        return (nominator, ledger.active);
                    }
                }
                (nominator, BalanceOf::<T>::zero())
            })
            .collect::<BTreeMap<T::AccountId, BalanceOf<T>>>();

        let staked_contracts = nominations.iter().fold(
            BTreeMap::<T::AccountId, Exposure<T::AccountId, BalanceOf<T>>>::new(),
            |mut bmap, (stash, nomination)| {
                let value = Perbill::from_rational_approximation(
                    BalanceOf::<T>::from(1),
                    BalanceOf::<T>::try_from(nomination.targets.len())
                        .unwrap_or(BalanceOf::<T>::one()),
                ) * *(nominators_to_staking
                    .get(stash)
                    .unwrap_or(&BalanceOf::<T>::zero()));
                let indv = IndividualExposure {
                    who: stash.clone(),
                    value: value,
                };
                for contract in nomination.targets.iter() {
                    if bmap.contains_key(&contract) {
                        if let Some(exposure) = bmap.get_mut(&contract) {
                            (*exposure).total += value;
                            (*exposure).others.push(indv.clone())
                        }
                    } else {
                        bmap.insert(
                            contract.clone(),
                            Exposure {
                                own: BalanceOf::<T>::zero(),
                                total: value.into(),
                                others: vec![indv.clone()],
                            },
                        );
                    }
                }
                return bmap;
            },
        );

        // Updating staked contracts info
        for (contract, exposure) in staked_contracts.iter() {
            <StakedContracts<T>>::mutate(&contract, |ex| *ex = exposure.clone());
        }
    }

    fn reward_validator(stash: &T::AccountId, reward: BalanceOf<T>) -> PositiveImbalanceOf<T> {
        T::Currency::deposit_into_existing(&stash, reward)
            .unwrap_or(PositiveImbalanceOf::<T>::zero())
    }

    fn reward_contract(contract: &T::AccountId, reward: BalanceOf<T>) -> PositiveImbalanceOf<T> {
        if let Some(operator) = T::ContractFinder::operator(contract) {
            return T::Currency::deposit_into_existing(&operator, reward)
                .unwrap_or(PositiveImbalanceOf::<T>::zero());
        }
        PositiveImbalanceOf::<T>::zero()
    }

    fn make_payout(stash: &T::AccountId, amount: BalanceOf<T>) -> Option<PositiveImbalanceOf<T>> {
        let dest = Self::payee(stash);
        match dest {
            RewardDestination::Controller => Self::bonded(stash).and_then(|controller| {
                T::Currency::deposit_into_existing(&controller, amount).ok()
            }),
            RewardDestination::Stash => T::Currency::deposit_into_existing(stash, amount).ok(),
            RewardDestination::Staked => Self::bonded(stash)
                .and_then(|c| Self::ledger(&c).map(|l| (c, l)))
                .and_then(|(controller, mut l)| {
                    l.active += amount;
                    l.total += amount;
                    let r = T::Currency::deposit_into_existing(stash, amount).ok();
                    Self::update_ledger(&controller, &l);
                    r
                }),
        }
    }

    /// Ensures storage is upgraded to most recent necessary state.
    fn ensure_storage_upgraded() {
        migration::perform_migrations::<T>();
    }
}

impl<T: Trait> SessionManager<T::AccountId> for Module<T> {
    fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        Self::ensure_storage_upgraded();
        Self::new_session(new_index)
    }
    fn start_session(_: u32) {
        todo!()
    }
    fn end_session(_end_index: SessionIndex) {}
}
