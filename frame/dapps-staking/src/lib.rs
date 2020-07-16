//! # Plasm Staking Module
//!
//! The Plasm staking module manages era, total amounts of rewards and how to distribute.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{
        Currency, Get, Imbalance, LockIdentifier, LockableCurrency, OnUnbalanced, Time,
        WithdrawReasons,
    },
    weights::Weight,
    StorageMap, StorageValue,
};
use frame_system::{self as system, ensure_signed};
use pallet_contract_operator::ContractFinder;
use pallet_plasm_rewards::{
    traits::{ComputeEraWithParam, EraFinder, ForDappsEraRewardFinder, HistoryDepthFinder},
    EraIndex, Releases,
};
pub use pallet_staking::{Forcing, RewardDestination};
use sp_runtime::{
    traits::{CheckedSub, Saturating, StaticLookup, Zero},
    Perbill, RuntimeDebug,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*, result, vec::Vec};

#[cfg(test)]
mod mock;
pub mod parameters;
pub mod rewards;
#[cfg(test)]
mod tests;

pub use parameters::StakingParameters;
pub use rewards::ComputeRewardsForDapps;
pub use sp_staking::SessionIndex;

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
pub type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;

type PositiveImbalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::PositiveImbalance;
type NegativeImbalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::NegativeImbalance;

const MAX_NOMINATIONS: usize = 128;
const MAX_UNLOCKING_CHUNKS: usize = 32;
const STAKING_ID: LockIdentifier = *b"dapstake";

/// A record of the nominations made by a specific account.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Nominations<AccountId, Balance> {
    /// The targets of nomination and amounts of staking.
    pub targets: Vec<(AccountId, Balance)>,
    /// The era the nominations were submitted.
    ///
    /// Except for initial nominations which are considered submitted at era 0.
    pub submitted_in: EraIndex,
    /// Whether the nominations have been suppressed.
    pub suppressed: bool,
}

/// Reward points of an era. Used to split era total payout between dapps rewards.
///
/// This points will be used to reward contracts operators and their respective nominators.
#[derive(PartialEq, Encode, Decode, Default, RuntimeDebug)]
pub struct EraStakingPoints<AccountId: Ord, Balance: HasCompact> {
    /// Total number of staking. Equals the sum of staking points for each contracts.
    total: Balance,
    /// The balance of stakinng earned by a given contracts.
    individual: BTreeMap<AccountId, Balance>,
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
    /// The latest and highest era which the staker has claimed reward for.
    pub last_reward: Option<EraIndex>,
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
            last_reward: self.last_reward,
        }
    }
}

pub trait Trait: pallet_session::Trait {
    /// The staking balance.
    type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    // The check valid operated contracts.
    type ContractFinder: ContractFinder<Self::AccountId, parameters::StakingParameters>;

    /// Number of eras that staked funds must remain bonded for.
    type BondingDuration: Get<EraIndex>;

    /// Tokens have been minted and are unused for validator-reward. Maybe, dapps-staking uses ().
    type RewardRemainder: OnUnbalanced<NegativeImbalanceOf<Self>>;

    /// Handler for the unbalanced increment when rewarding a staker. Maybe, dapps-staking uses ().
    type Reward: OnUnbalanced<PositiveImbalanceOf<Self>>;

    //TODO Handler for the unbalanced reduction when slashing a staker.
    //type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;

    //TODO
    // Number of eras that slashes are deferred by, after computation. This
    // should be less than the bonding duration. Set to 0 if slashes should be
    // applied immediately, without opportunity for intervention.
    //type SlashDeferDuration: Get<EraIndex>;

    // TODO
    // The origin which can cancel a deferred slash. Root can always do this.
    //type SlashCancelOrigin: EnsureOrigin<Self::Origin>;

    /// Time used for computing era duration.
    type Time: Time;

    type ComputeRewardsForDapps: ComputeRewardsForDapps;

    /// The information of era.
    type EraFinder: EraFinder<EraIndex, SessionIndex, MomentOf<Self>>;

    /// The rewards for dapps operator.
    type ForDappsEraReward: ForDappsEraRewardFinder<BalanceOf<Self>>;

    /// the history depth
    type HistoryDepthFinder: HistoryDepthFinder;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as DappsStaking {
        /// The already untreated era is EraIndex.
        pub UntreatedEra get(fn untreated_era): EraIndex;

        // The untreated era for each nominator
        pub NominatorsUntreatedEra get(fn nominators_untreated_era):
            map hasher(twox_64_concat) T::AccountId => EraIndex;

        // The untreated era for each operator
        pub OperatorsUntreatedEra get(fn operators_untreated_era):
                map hasher(twox_64_concat) T::AccountId => EraIndex;

        // ----- Staking uses.
        /// Map from all locked "stash" accounts to the controller account.
        pub Bonded get(fn bonded): map hasher(twox_64_concat) T::AccountId => Option<T::AccountId>;

        /// Map from all (unlocked) "controller" accounts to the info regarding the staking.
        pub Ledger get(fn ledger):
            map hasher(blake2_128_concat) T::AccountId
            => Option<StakingLedger<T::AccountId, BalanceOf<T>>>;

        /// Where the reward payment should be made. Keyed by stash.
        pub Payee get(fn payee): map hasher(twox_64_concat) T::AccountId => RewardDestination;

        /// The map from nominator stash key to the set of stash keys of all contracts to nominate.
        ///
        /// NOTE: is private so that we can ensure upgraded before all typical accesses.
        /// Direct storage APIs can still bypass this protection.
        DappsNominations get(fn dapps_nominations): map hasher(twox_64_concat)
                                                    T::AccountId => Option<Nominations<T::AccountId, BalanceOf<T>>>;

        /// Similarly to `ErasStakers` this holds the parameters of contracts.
        ///
        /// This is keyed first by the era index to allow bulk deletion and then the contracts account.
        ///
        /// Is it removed after `HISTORY_DEPTH` eras.
        pub ErasContractsParameters get(fn eras_contracts_parameters):
            double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
            => Option<StakingParameters>;

        /// Rewards of stakers for contracts(called by "Dapps Nominator") at era.
        ///
        /// This is keyed first by the era index, 2nd keyed contract account to allow the stash account.
        /// Rewards for the last `HISTORY_DEPTH` eras.
        ///
        /// If reward hasn't been set or has been removed then 0 reward is returned.
        pub ErasStakingPoints get(fn eras_staking_points):
            double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
            => EraStakingPoints<T::AccountId, BalanceOf<T>>;

        /// The total amount staked for the last `HISTORY_DEPTH` eras.
        /// If total hasn't been set or has been removed then 0 stake is returned.
        pub ErasTotalStake get(fn eras_total_stake):
            map hasher(twox_64_concat) EraIndex => BalanceOf<T>;

        /// The total amounts of staking for each nominators
        ErasNominateTotals get(fn eras_nominate_totals):
            double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
            => BalanceOf<T>;

        /// The total amounts of staking for each operators
        ErasStakedOperators get(fn eras_staked_operators):
            double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
            => BalanceOf<T>;

        /// Storage version of the pallet.
        ///
        /// This is set to v1.0.0 for new networks.
        StorageVersion build(|_: &GenesisConfig| Releases::V1_0_0): Releases;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = BalanceOf<T>,
    {
        /// The amount of minted rewards. (for dapps with nominators)
        Reward(Balance, Balance),
        /// An account has bonded this amount.
        ///
        /// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
        /// it will not be emitted for staking rewards when they are added to stake.
        Bonded(AccountId, Balance),
        /// An account has unbonded this amount.
        Unbonded(AccountId, Balance),
        /// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
        /// from the unlocking queue.
        Withdrawn(AccountId, Balance),
        /// The total amount of minted rewards for dapps.
        TotalDappsRewards(EraIndex, Balance),
        /// Nominate of stash address.
        Nominate(AccountId),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// Not a controller account.
        NotController,
        /// Not a stash account.
        NotStash,
        /// Stash is already bonded.
        AlreadyBonded,
        /// Controller is already paired.
        AlreadyPaired,
        /// Targets cannot be empty.
        EmptyTargets,
        /// Duplicate index.
        DuplicateIndex,
        /// Slash record index out of bounds.
        InvalidSlashIndex,
        /// Can not bond with value less than minimum balance.
        InsufficientValue,
        /// Can not schedule more unlock chunks.
        NoMoreChunks,
        /// Can not rebond without unlocking chunks.
        NoUnlockChunk,
        /// Attempting to target a stash that still has funds.
        FundedTarget,
        /// Invalid era to reward.
        InvalidEraToReward,
        /// Invalid number of nominations.
        InvalidNumberOfNominations,
        /// Items are not sorted and unique.
        NotSortedAndUnique,
        /// Targets must be latest 1.
        EmptyNominateTargets,
        /// Targets must be operated contracts
        NotOperatedContracts,
        /// The nominations amount more than active staking amount.
        NotEnoughStaking,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        fn on_runtime_upgrade() -> Weight {
            migrate::<T>();
            // TODO: weight
            50_000
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
        /// TODO: weight
        #[weight = 500_000]
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

            // increments account reference counter for not removing accounts.
            system::Module::<T>::inc_ref(&stash);

            let stash_balance = T::Currency::free_balance(&stash);
            let value = value.min(stash_balance);
            Self::deposit_event(RawEvent::Bonded(stash.clone(), value.clone()));
            let item = StakingLedger {
                stash,
                total: value,
                active: value,
                unlocking: vec![],
                last_reward: T::EraFinder::current()
            };
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
        /// TODO: weight
        #[weight = 500_000]
        fn bond_extra(origin, #[compact] max_additional: BalanceOf<T>) {
            let stash = ensure_signed(origin)?;

            let controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

            let stash_balance = T::Currency::free_balance(&stash);

            if let Some(extra) = stash_balance.checked_sub(&ledger.total) {
                let extra = extra.min(max_additional);
                ledger.total += extra;
                ledger.active += extra;
                Self::deposit_event(RawEvent::Bonded(stash, extra));
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
        ///   The only way to clean the aforementioned storage item is also user-controlled via
        ///   `withdraw_unbonded`.
        /// - One DB entry.
        /// </weight>
        /// TODO: weight
        #[weight = 400_000]
        fn unbond(origin, #[compact] value: BalanceOf<T>) {
            let controller = ensure_signed(origin)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            ensure!(
                ledger.unlocking.len() < MAX_UNLOCKING_CHUNKS,
                Error::<T>::NoMoreChunks
            );

            let mut value = value.min(ledger.active);

            if !value.is_zero() {
                ledger.active -= value;

                // Avoid there being a dust balance left in the staking system.
                if ledger.active < T::Currency::minimum_balance() {
                    value += ledger.active;
                    ledger.active = Zero::zero();
                }

                Self::deposit_event(RawEvent::Unbonded(ledger.stash.clone(), value));
                let era = T::EraFinder::current().unwrap_or(Zero::zero()) + T::BondingDuration::get();
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
        /// Emits `Withdrawn`.
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
        /// TODO: weight
        #[weight = 400_000]
        fn withdraw_unbonded(origin) {
            let controller = ensure_signed(origin)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            let (stash, old_total) = (ledger.stash.clone(), ledger.total);
            if let Some(current_era) = T::EraFinder::current() {
                ledger = ledger.consolidate_unlocked(current_era)
            }

            if ledger.unlocking.is_empty() && ledger.active.is_zero() {
                // This account must have called `unbond()` with some value that caused the active
                // portion to fall below existential deposit + will have no more unlocking chunks
                // left. We can now safely remove all staking-related information.
                Self::kill_stash(&stash)?;
                // remove the lock.
                T::Currency::remove_lock(STAKING_ID, &stash);
            } else {
                // This was the consequence of a partial unbond. just update the ledger and move on.
                Self::update_ledger(&controller, &ledger);
            }

            // `old_total` should never be less than the new total because
            // `consolidate_unlocked` strictly subtracts balance.
            if ledger.total < old_total {
                // Already checked that this won't overflow by entry condition.
                let value = old_total - ledger.total;
                Self::deposit_event(RawEvent::Withdrawn(stash, value));
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
        /// TODO: weight
        #[weight = 750_000]
        fn nominate_contracts(origin, targets: Vec<(<T::Lookup as StaticLookup>::Source, BalanceOf<T>)>) {
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            let stash = &ledger.stash;
            ensure!(!targets.is_empty(), Error::<T>::EmptyNominateTargets);
            let targets = targets.into_iter()
                .take(MAX_NOMINATIONS)
                .map(|t| match T::Lookup::lookup(t.0) {
                    Ok(a) => Ok((a, t.1)),
                    Err(err) => Err(err),
                })
                .collect::<result::Result<Vec<(T::AccountId, BalanceOf<T>)>, _>>()?;

            // check the is targets operated contracts?
            if !targets.iter().all(|t| T::ContractFinder::is_exists_contract(&(t.0))) {
                Err(Error::<T>::NotOperatedContracts)?
            }

            if targets
                .iter()
                .fold(BalanceOf::<T>::zero(),
                 |sum, t| sum.saturating_add(t.1)) > ledger.active {
                Err(Error::<T>::NotEnoughStaking)?
            }

            let nominations = Nominations {
                targets,
                submitted_in: T::EraFinder::current().unwrap_or(Zero::zero()),
                suppressed: false,
            };

            Self::take_in_nominations(stash, nominations);
            Self::deposit_event(RawEvent::Nominate(stash.clone()));
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
        /// TODO: weight
        #[weight = 500_000]
        fn chill(origin) {
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
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
        /// TODO: weight
        #[weight = 500_000]
        fn set_payee(origin, payee: RewardDestination) {
            let controller = ensure_signed(origin)?;
            let ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
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
        /// TODO: weight
        #[weight = 750_000]
        fn set_controller(origin, controller: <T::Lookup as StaticLookup>::Source) {
            let stash = ensure_signed(origin)?;
            let old_controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
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

        /// rewards are claimed by the nominator.
        ///
        /// era must be in the range `[current_era - history_depth; active_era)`.
        ///
        /// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
        /// TODO: weight
        #[weight = 1_000]
        fn claim_for_nominator(origin, era: EraIndex) {
            let nominator = ensure_signed(origin)?;

            // check if era is valid
            if let Some(active_era) = T::EraFinder::active() {
                if era >= active_era.index {
                    Err("cannot claim yet")?
                }
            }
            if let Some(current_era) = T::EraFinder::current() {
                if era < current_era.saturating_sub(T::HistoryDepthFinder::get()) {
                    Err("the era is expired")?
                }
            }

            let mut untreated_era = Self::nominators_untreated_era(&nominator);
            if era == untreated_era {
                Err("the nominator already rewarded")?
            }
            while era > untreated_era {
                Self::propagate_nominate_totals(&nominator, &untreated_era, &(untreated_era + 1));
                untreated_era += 1;
            }
            <NominatorsUntreatedEra<T>>::insert(&nominator, untreated_era);

            let rewards = match T::ForDappsEraReward::get(&era) {
                Some(rewards) => rewards,
                None => {
                    frame_support::print("Error: start_session_index must be set for current_era");
                    BalanceOf::<T>::zero()
                }
            };

            let actual_rewarded = Self::reward_nominator(&era, rewards, &nominator);
            // deposit event to total validator rewards
            Self::deposit_event(RawEvent::TotalDappsRewards(era, actual_rewarded));
        }

        /// rewards are claimed by the operator.
        ///
        /// era must be in the range [current_era - history_depth; active_era).
        ///
        /// The dispatch origin for this call must be _Signed_ by the stash, not the controller
        /// TODO: weight
        #[weight = 1_000]
        fn claim_for_operator(origin, era: EraIndex) {
            let operator = ensure_signed(origin)?;

            // check if era is valid
            if let Some(active_era) = T::EraFinder::active() {
                if era >= active_era.index {
                    Err("cannot claim yet")?
                }
            }
            if let Some(current_era) = T::EraFinder::current() {
                if era < current_era.saturating_sub(T::HistoryDepthFinder::get()) {
                    Err("the era is expired")?
                }
            }

            let mut untreated_era = Self::operators_untreated_era(&operator);
            if era == untreated_era {
                Err("the operator already rewarded")?
            }
            while era > untreated_era {
                Self::propagate_eras_staked_operators(&operator, &untreated_era, &(untreated_era + 1));
                untreated_era += 1;
            }
            <OperatorsUntreatedEra<T>>::insert(&operator, untreated_era);

            let rewards = match T::ForDappsEraReward::get(&era) {
                Some(rewards) => rewards,
                None => {
                    frame_support::print("Error: start_session_index must be set for current_era");
                    BalanceOf::<T>::zero()
                }
            };

            let actual_rewarded = Self::reward_operator(&era, rewards, &operator);
            // deposit event to total validator rewards
            Self::deposit_event(RawEvent::TotalDappsRewards(era, actual_rewarded));
        }
    }
}

fn migrate<T: Trait>() {}

impl<T: Trait> Module<T> {
    // MUTABLES (DANGEROUS)

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
    fn kill_stash(stash: &T::AccountId) -> DispatchResult {
        let controller = Bonded::<T>::take(stash).ok_or(Error::<T>::NotStash)?;
        <Ledger<T>>::remove(&controller);

        <Payee<T>>::remove(stash);
        if let Some(nominations) = Self::dapps_nominations(stash) {
            Self::remove_nominations(stash, nominations);
        }

        system::Module::<T>::dec_ref(stash);
        Ok(())
    }

    /// Chill a stash account.
    fn chill_stash(stash: &T::AccountId) {
        if let Some(nominations) = Self::dapps_nominations(stash) {
            Self::remove_nominations(stash, nominations);
        }
    }

    fn propagate_nominate_totals(nominator: &T::AccountId, src_era: &EraIndex, dst_era: &EraIndex) {
        if <ErasNominateTotals<T>>::contains_key(src_era, nominator) {
            let untreated_nootate_total = <ErasNominateTotals<T>>::get(src_era, nominator);

            <ErasNominateTotals<T>>::mutate(dst_era, nominator, |total| {
                *total += untreated_nootate_total;
            })
        }
    }

    fn reward_nominator(
        era: &EraIndex,
        max_payout: BalanceOf<T>,
        nominator: &T::AccountId,
    ) -> BalanceOf<T> {
        let mut total_imbalance = <PositiveImbalanceOf<T>>::zero();
        let (_, nominators_reward) =
            T::ComputeRewardsForDapps::compute_rewards_for_dapps(max_payout);

        let total_staked = Self::eras_total_stake(era);

        let nominate_total = Self::eras_nominate_totals(era, nominator);
        let reward =
            Perbill::from_rational_approximation(nominate_total, total_staked) * nominators_reward;
        total_imbalance.subsume(
            Self::make_payout(nominator, reward).unwrap_or(PositiveImbalanceOf::<T>::zero()),
        );

        let total_payout = total_imbalance.peek();

        let rest = max_payout.saturating_sub(total_payout.clone());

        T::Reward::on_unbalanced(total_imbalance);
        T::RewardRemainder::on_unbalanced(T::Currency::issue(rest));
        total_payout
    }

    fn propagate_eras_staked_operators(
        operator: &T::AccountId,
        src_era: &EraIndex,
        dst_era: &EraIndex,
    ) {
        if <ErasStakedOperators<T>>::contains_key(src_era, operator) {
            let untreated_staked_operator = <ErasStakedOperators<T>>::get(src_era, operator);

            <ErasStakedOperators<T>>::mutate(dst_era, operator, |total| {
                *total += untreated_staked_operator;
            });
        }
    }

    fn reward_operator(
        era: &EraIndex,
        max_payout: BalanceOf<T>,
        operator: &T::AccountId,
    ) -> BalanceOf<T> {
        let mut total_imbalance = <PositiveImbalanceOf<T>>::zero();
        let (operators_reward, _) =
            T::ComputeRewardsForDapps::compute_rewards_for_dapps(max_payout);

        let total_staked = Self::eras_total_stake(era);

        let staked_operator = Self::eras_staked_operators(era, operator);
        let reward =
            Perbill::from_rational_approximation(staked_operator, total_staked) * operators_reward;
        total_imbalance.subsume(
            T::Currency::deposit_into_existing(operator, reward)
                .unwrap_or(PositiveImbalanceOf::<T>::zero()),
        );

        let total_payout = total_imbalance.peek();

        let rest = max_payout.saturating_sub(total_payout.clone());

        T::Reward::on_unbalanced(total_imbalance);
        T::RewardRemainder::on_unbalanced(T::Currency::issue(rest));
        total_payout
    }

    fn compute_total_stake(era: &EraIndex) -> BalanceOf<T> {
        let mut untreated_era = Self::untreated_era();
        while *era > untreated_era {
            let total = Self::eras_total_stake(&untreated_era);
            <ErasTotalStake<T>>::mutate(&untreated_era + 1, |next_total| *next_total += total);
            untreated_era += 1;
        }
        UntreatedEra::put(untreated_era);
        let total_staked = Self::eras_total_stake(era);
        total_staked
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

    fn take_in_nominations(
        stash: &T::AccountId,
        nominations: Nominations<T::AccountId, BalanceOf<T>>,
    ) {
        if let Some(current_era) = T::EraFinder::current() {
            let next_era = current_era + 1;

            for (contract, value) in nominations.targets.iter() {
                if <ErasStakingPoints<T>>::contains_key(&next_era, &contract) {
                    <ErasStakingPoints<T>>::mutate(&next_era, &contract, |points| {
                        (*points).total += value.clone();
                        (*points).individual.insert(stash.clone(), value.clone());
                    });
                } else {
                    let points = EraStakingPoints {
                        total: value.clone(),
                        individual: vec![(stash.clone(), value.clone())]
                            .into_iter()
                            .collect::<BTreeMap<T::AccountId, BalanceOf<T>>>(),
                    };
                    <ErasStakingPoints<T>>::insert(&next_era, &contract, points);
                }

                <ErasNominateTotals<T>>::mutate(&next_era, stash, |total| {
                    *total += value.clone();
                });

                if let Some(operator) = T::ContractFinder::operator(&contract) {
                    <ErasStakedOperators<T>>::mutate(&next_era, operator, |total| {
                        *total += value.clone();
                    });
                }

                <ErasTotalStake<T>>::mutate(&next_era, |total| {
                    *total += value.clone();
                });
            }
        }
        <DappsNominations<T>>::insert(stash, nominations);
    }

    fn remove_nominations(
        stash: &T::AccountId,
        nominations: Nominations<T::AccountId, BalanceOf<T>>,
    ) {
        let era = nominations.submitted_in + 1;
        for (contract, value) in nominations.targets.iter() {
            <ErasStakingPoints<T>>::mutate(&era, &contract, |points| {
                (*points).total = points.total.saturating_sub(value.clone());
                (*points).individual.remove(stash);
            });

            <ErasNominateTotals<T>>::mutate(&era, stash, |total| {
                *total = total.saturating_sub(value.clone());
            });

            if let Some(operator) = T::ContractFinder::operator(&contract) {
                <ErasStakedOperators<T>>::mutate(&era, &operator, |total| {
                    *total = total.saturating_sub(value.clone());
                });
            }

            <ErasTotalStake<T>>::mutate(&era, |total| {
                *total = total.saturating_sub(value.clone());
            });
        }
        <DappsNominations<T>>::remove(stash);
    }
}

/// Get the amount of staking per Era in a module in the Plasm Network.
impl<T: Trait> ComputeEraWithParam<EraIndex> for Module<T> {
    type Param = BalanceOf<T>;
    fn compute(era: &EraIndex) -> BalanceOf<T> {
        Self::compute_total_stake(era)
    }
}
