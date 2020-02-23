//! # Plasm Staking Module
//!
//! The Plasm staking module manages era, total amounts of rewards and how to distribute.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use operator::ContractFinder;
use session::SessionManager;
use sp_runtime::{
    traits::{
        CheckedAdd, CheckedDiv, CheckedSub, One, SaturatedConversion, Saturating,
        StaticLookup, Zero,
    },
    Perbill, PerThing, RuntimeDebug,
};
use sp_std::{collections::btree_map::BTreeMap, convert::TryFrom, prelude::*, result, vec::Vec};
pub use staking::{Forcing, Nominations, RewardDestination};
use support::{
    decl_event, decl_module, decl_storage, ensure,
    traits::{
        Currency, Get, Imbalance, LockIdentifier, LockableCurrency, OnUnbalanced, Time,
        WithdrawReasons,
    },
    weights::SimpleDispatchInfo,
    StorageLinkedMap, StorageMap, StorageValue,
};
use system::{ensure_root, ensure_signed};

mod inflation;
mod migration;
#[cfg(test)]
mod mock;
pub mod parameters;
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

const MAX_NOMINATIONS: usize = 16;
const MAX_UNLOCKING_CHUNKS: usize = 32;
const STAKING_ID: LockIdentifier = *b"plmstake";

/// The amount of exposure (to slashing) than an individual nominator has.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug)]
pub struct IndividualExposure<AccountId, Balance: HasCompact> {
    /// The stash account of the nominator in question.
    who: AccountId,
    /// Amount of funds exposed.
    #[codec(compact)]
    value: Balance,
}

/// A snapshot of the stake backing a single validator in the system.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, Default, RuntimeDebug)]
pub struct Exposure<AccountId, Balance: HasCompact> {
    /// The total balance backing this validator.
    #[codec(compact)]
    pub total: Balance,
    /// The validator's own stash that is exposed.
    #[codec(compact)]
    pub own: Balance,
    /// The portions of nominators stashes that are exposed.
    pub others: Vec<IndividualExposure<AccountId, Balance>>,
}

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct UnlockChunk<Balance: HasCompact> {
    /// Amount of funds to be unlocked.
    #[codec(compact)]
    value: Balance,
    /// Era number at which point it'll be unlocked.
    #[codec(compact)]
    era: EraIndex,
}

/// The ledger of a (bonded) stash.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct StakingLedger<AccountId, Balance: HasCompact> {
    /// The stash account whose balance is actually locke,ed and at stake.
    pub stash: AccountId,
    /// The total amount of the stash's balance that we are currently accounting for.
    /// It's just `active` plus all the `unlocking` balances.
    #[codec(compact)]
    pub total: Balance,
    /// The total amount of the stash's balance that will be at stake in any forthcoming
    /// rounds.
    #[codec(compact)]
    pub active: Balance,
    /// Any balance that is becoming free, which may eventually be transferred out
    /// of the stash (assuming it doesn't get slashed first).
    pub unlocking: Vec<UnlockChunk<Balance>>,
}

impl<AccountId, Balance: HasCompact + Copy + Saturating> StakingLedger<AccountId, Balance> {
    /// Remove entries from `unlocking` that are sufficiently old and reduce the
    /// total by the sum of their balances.
    fn consolidate_unlocked(self, current_era: EraIndex) -> Self {
        let mut total = self.total;
        let unlocking = self
            .unlocking
            .into_iter()
            .filter(|chunk| {
                if chunk.era > current_era {
                    true
                } else {
                    total = total.saturating_sub(chunk.value);
                    false
                }
            })
            .collect();
        Self {
            total,
            active: self.active,
            stash: self.stash,
            unlocking,
        }
    }
}

pub trait Trait: session::Trait {
    /// The staking balance.
    type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    // The check valid operated contracts.
    type ContractFinder: operator::ContractFinder<Self::AccountId, parameters::StakingParameters>;

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

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as PlasmStaking {
        // ----- Staking uses.
        /// Map from all locked "stash" accounts to the controller account.
        pub Bonded get(fn bonded): map hasher(blake2_256) T::AccountId => Option<T::AccountId>;
        /// Map from all (unlocked) "controller" accounts to the info regarding the staking.
        pub Ledger get(fn ledger):
            map hasher(blake2_256) T::AccountId => Option<StakingLedger<T::AccountId, BalanceOf<T>>>;

        /// Where the reward payment should be made. Keyed by stash.
        pub Payee get(fn payee): map hasher(blake2_256) T::AccountId => RewardDestination;

        /// The map from nominator stash key to the set of stash keys of all contracts to nominate.
        ///
        /// NOTE: is private so that we can ensure upgraded before all typical accesses.
        /// Direct storage APIs can still bypass this protection.
        DappsNominations get(fn dapps_nominations): linked_map hasher(blake2_256) T::AccountId => Option<Nominations<T::AccountId>>;

        /// Nominators for a particular contract that is in action right now.
        ///
        /// This is keyed by the contract account id.
        pub StakedContracts get(fn staked_contracts): linked_map hasher(blake2_256) T::AccountId => Exposure<T::AccountId, BalanceOf<T>>;

        // ---- Era manages.
        /// The currently elected validator set keyed by stash account ID.
        pub CurrentElected get(fn current_elected): Vec<T::AccountId>;

        /// The current era index.
        pub CurrentEra get(fn current_era): EraIndex;

        /// The start of the current era.
        pub CurrentEraStart get(fn current_era_start): MomentOf<T>;

        /// The session index at which the current era started.
        pub CurrentEraStartSessionIndex get(fn current_era_start_session_index): SessionIndex;

        /// True if the next session change will be a new era regardless of index.
        pub ForceEra get(fn force_era) config(): Forcing;

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

        /// Take the origin account as a stash and lock up `value` of its balance. `controller` will
        /// be the account that controls it.
        ///
        /// `value` must be more than the `minimum_balance` specified by `T::Currency`.
        ///
        /// The dispatch origin for this call must be _Signed_ by the stash account.
        ///
        /// # <weight>
        /// - Independent of the arguments. Moderate complexity.
        /// - O(1).
        /// - Three extra DB entries.
        ///
        /// NOTE: Two of the storage writes (`Self::bonded`, `Self::payee`) are _never_ cleaned unless
        /// the `origin` falls below _existential deposit_ and gets removed as dust.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(500_000)]
        fn bond(origin,
            controller: <T::Lookup as StaticLookup>::Source,
            #[compact] value: BalanceOf<T>,
            payee: RewardDestination
        ) {
            let stash = ensure_signed(origin)?;

            if <Bonded<T>>::contains_key(&stash) {
                Err("stash already bonded")?
            }

            let controller = T::Lookup::lookup(controller)?;

            if <Ledger<T>>::contains_key(&controller) {
                Err("controller already paired")?
            }

            // reject a bond which is considered to be _dust_.
            if value < T::Currency::minimum_balance() {
                Err("can not bond with value less than minimum balance")?
            }

            // You're auto-bonded forever, here. We might improve this by only bonding when
            // you actually validate/nominate and remove once you unbond __everything__.
            <Bonded<T>>::insert(&stash, &controller);
            <Payee<T>>::insert(&stash, payee);

            let stash_balance = T::Currency::free_balance(&stash);
            let value = value.min(stash_balance);
            let item = StakingLedger { stash, total: value, active: value, unlocking: vec![] };
            Self::update_ledger(&controller, &item);
        }

        /// Add some extra amount that have appeared in the stash `free_balance` into the balance up
        /// for staking.
        ///
        /// Use this if there are additional funds in your stash account that you wish to bond.
        /// Unlike [`bond`] or [`unbond`] this function does not impose any limitation on the amount
        /// that can be added.
        ///
        /// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
        ///
        /// # <weight>
        /// - Independent of the arguments. Insignificant complexity.
        /// - O(1).
        /// - One DB entry.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(500_000)]
        fn bond_extra(origin, #[compact] max_additional: BalanceOf<T>) {
            let stash = ensure_signed(origin)?;

            let controller = Self::bonded(&stash).ok_or("not a stash")?;
            let mut ledger = Self::ledger(&controller).ok_or("not a controller")?;

            let stash_balance = T::Currency::free_balance(&stash);

            if let Some(extra) = stash_balance.checked_sub(&ledger.total) {
                let extra = extra.min(max_additional);
                ledger.total += extra;
                ledger.active += extra;
                Self::update_ledger(&controller, &ledger);
            }
        }

        /// Schedule a portion of the stash to be unlocked ready for transfer out after the bond
        /// period ends. If this leaves an amount actively bonded less than
        /// T::Currency::minimum_balance(), then it is increased to the full amount.
        ///
        /// Once the unlock period is done, you can call `withdraw_unbonded` to actually move
        /// the funds out of management ready for transfer.
        ///
        /// No more than a limited number of unlocking chunks (see `MAX_UNLOCKING_CHUNKS`)
        /// can co-exists at the same time. In that case, [`Call::withdraw_unbonded`] need
        /// to be called first to remove some of the chunks (if possible).
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// See also [`Call::withdraw_unbonded`].
        ///
        /// # <weight>
        /// - Independent of the arguments. Limited but potentially exploitable complexity.
        /// - Contains a limited number of reads.
        /// - Each call (requires the remainder of the bonded balance to be above `minimum_balance`)
        ///   will cause a new entry to be inserted into a vector (`Ledger.unlocking`) kept in storage.
        ///   The only way to clean the aforementioned storage item is also user-controlled via `withdraw_unbonded`.
        /// - One DB entry.
        /// </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(400_000)]
        fn unbond(origin, #[compact] value: BalanceOf<T>) {
            let controller = ensure_signed(origin)?;
            let mut ledger = Self::ledger(&controller).ok_or("not a controller")?;
            ensure!(
                ledger.unlocking.len() < MAX_UNLOCKING_CHUNKS,
                "can not schedule more unlock chunks"
            );

            let mut value = value.min(ledger.active);

            if !value.is_zero() {
                ledger.active -= value;

                // Avoid there being a dust balance left in the staking system.
                if ledger.active < T::Currency::minimum_balance() {
                    value += ledger.active;
                    ledger.active = Zero::zero();
                }

                let era = Self::current_era() + T::BondingDuration::get();
                ledger.unlocking.push(UnlockChunk { value, era });
                Self::update_ledger(&controller, &ledger);
            }
        }

        /// Remove any unlocked chunks from the `unlocking` queue from our management.
        ///
        /// This essentially frees up that balance to be used by the stash account to do
        /// whatever it wants.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// See also [`Call::unbond`].
        ///
        /// # <weight>
        /// - Could be dependent on the `origin` argument and how much `unlocking` chunks exist.
        ///  It implies `consolidate_unlocked` which loops over `Ledger.unlocking`, which is
        ///  indirectly user-controlled. See [`unbond`] for more detail.
        /// - Contains a limited number of reads, yet the size of which could be large based on `ledger`.
        /// - Writes are limited to the `origin` account key.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(400_000)]
        fn withdraw_unbonded(origin) {
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or("not a controller")?;
            let ledger = ledger.consolidate_unlocked(Self::current_era());

            if ledger.unlocking.is_empty() && ledger.active.is_zero() {
                // This account must have called `unbond()` with some value that caused the active
                // portion to fall below existential deposit + will have no more unlocking chunks
                // left. We can now safely remove this.
                let stash = ledger.stash;
                // remove the lock.
                T::Currency::remove_lock(STAKING_ID, &stash);
                // remove all staking-related information.
                Self::kill_stash(&stash);
            } else {
                // This was the consequence of a partial unbond. just update the ledger and move on.
                Self::update_ledger(&controller, &ledger);
            }
        }

        /// Declare the desire to nominate `targets` for the origin controller.
        ///
        /// Effects will be felt at the beginning of the next era.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// # <weight>
        /// - The transaction's complexity is proportional to the size of `targets`,
        /// which is capped at `MAX_NOMINATIONS`.
        /// - Both the reads and writes follow a similar pattern.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(750_000)]
        fn nominate_contracts(origin, targets: Vec<<T::Lookup as StaticLookup>::Source>) {
            Self::ensure_storage_upgraded();

            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or("not a controller")?;
            let stash = &ledger.stash;
            ensure!(!targets.is_empty(), "targets cannot be empty");
            let targets = targets.into_iter()
                .take(MAX_NOMINATIONS)
                .map(|t| T::Lookup::lookup(t))
                .collect::<result::Result<Vec<T::AccountId>, _>>()?;

            if !targets.iter().all(|t| T::ContractFinder::is_exists_contract(&t)) {
                Err("tragets must be operated contracts")?
            }

            let nominations = Nominations {
                targets,
                submitted_in: Self::current_era(),
                suppressed: false,
            };

            <DappsNominations<T>>::insert(stash, &nominations);
        }

        /// Declare no desire to either validate or nominate.
        ///
        /// Effects will be felt at the beginning of the next era.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// # <weight>
        /// - Independent of the arguments. Insignificant complexity.
        /// - Contains one read.
        /// - Writes are limited to the `origin` account key.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(500_000)]
        fn chill(origin) {
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or("not a controller")?;
            Self::chill_stash(&ledger.stash);
        }

        /// (Re-)set the payment target for a controller.
        ///
        /// Effects will be felt at the beginning of the next era.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// # <weight>
        /// - Independent of the arguments. Insignificant complexity.
        /// - Contains a limited number of reads.
        /// - Writes are limited to the `origin` account key.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(500_000)]
        fn set_payee(origin, payee: RewardDestination) {
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or("not a controller")?;
            let stash = &ledger.stash;
            <Payee<T>>::insert(stash, payee);
        }

        /// (Re-)set the controller of a stash.
        ///
        /// Effects will be felt at the beginning of the next era.
        ///
        /// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
        ///
        /// # <weight>
        /// - Independent of the arguments. Insignificant complexity.
        /// - Contains a limited number of reads.
        /// - Writes are limited to the `origin` account key.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedNormal(750_000)]
        fn set_controller(origin, controller: <T::Lookup as StaticLookup>::Source) {
            let stash = ensure_signed(origin)?;
            let old_controller = Self::bonded(&stash).ok_or("not a stash")?;
            let controller = T::Lookup::lookup(controller)?;
            if <Ledger<T>>::contains_key(&controller) {
                Err("controller already paired")?
            }
            if controller != old_controller {
                <Bonded<T>>::insert(&stash, &controller);
                if let Some(l) = <Ledger<T>>::take(&old_controller) {
                    <Ledger<T>>::insert(&controller, l);
                }
            }
        }

        // ----- Root calls.
        /// Force there to be no new eras indefinitely.
        ///
        /// # <weight>
        /// - No arguments.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedOperational(0)]
        fn force_no_eras(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceNone);
        }

        /// Force there to be a new era at the end of the next session. After this, it will be
        /// reset to normal (non-forced) behaviour.
        ///
        /// # <weight>
        /// - No arguments.
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedOperational(0)]
        fn force_new_era(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceNew);
        }

        /// Force there to be a new era at the end of sessions indefinitely.
        ///
        /// # <weight>
        /// - One storage write
        /// # </weight>
        #[weight = SimpleDispatchInfo::FixedOperational(0)]
        fn force_new_era_always(origin) {
            ensure_root(origin)?;
            ForceEra::put(Forcing::ForceAlways);
        }

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
        /// The amount of minted rewards. (for validators with nominators, for dapps with nominators)
        Reward(Balance, Balance),
    }
);

impl<T: Trait> Module<T> {
    // MUTABLES (DANGEROUS)

    /// The total balance that can be slashed from a stash account as of right now.
    pub fn slashable_balance_of(stash: &T::AccountId) -> BalanceOf<T> {
        Self::bonded(stash)
            .and_then(Self::ledger)
            .map(|l| l.active)
            .unwrap_or_default()
    }

    /// Update the ledger for a controller. This will also update the stash lock. The lock will
    /// will lock the entire funds except paying for further transactions.
    fn update_ledger(
        controller: &T::AccountId,
        ledger: &StakingLedger<T::AccountId, BalanceOf<T>>,
    ) {
        T::Currency::set_lock(
            STAKING_ID,
            &ledger.stash,
            ledger.total,
            WithdrawReasons::all(),
        );
        <Ledger<T>>::insert(controller, ledger);
    }

    /// Remove all associated data of a stash account from the staking system.
    ///
    /// Assumes storage is upgraded before calling.
    ///
    /// This is called :
    /// - Immediately when an account's balance falls below existential deposit.
    /// - after a `withdraw_unbond()` call that frees all of a stash's bonded balance.
    fn kill_stash(stash: &T::AccountId) {
        if let Some(controller) = <Bonded<T>>::take(stash) {
            <Ledger<T>>::remove(&controller);
        }
        <Payee<T>>::remove(stash);
        <DappsNominations<T>>::remove(stash);
    }

    /// Chill a stash account.
    fn chill_stash(stash: &T::AccountId) {
        <DappsNominations<T>>::remove(stash);
    }

    /// Session has just ended. Provide the validator set for the next session if it's an era-end, along
    /// with the exposure of the prior validator set.
    pub fn new_session(
        new_index: SessionIndex,
    ) -> Option<Vec<T::AccountId>> {
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
    pub fn new_era(
        new_index: SessionIndex,
    ) -> Option<Vec<T::AccountId>> {
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
        let staked_contracts = <StakedContracts<T>>::enumerate()
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
        let nominations = <DappsNominations<T>>::enumerate()
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
                                total: value.clone(),
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
    fn end_session(_end_index: SessionIndex) {}
}
