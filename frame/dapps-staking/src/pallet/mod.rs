//! Dapps staking FRAME Pallet.

use super::*;
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
    pallet_prelude::*,
    traits::{
        Currency, CurrencyToVote, EnsureOrigin, EstimateNextNewSession, Get, LockIdentifier,
        LockableCurrency, OnUnbalanced, UnixTime, WithdrawReasons,
    },
    weights::Weight,
};
use frame_system::{ensure_root, ensure_signed, offchain::SendTransactionTypes, pallet_prelude::*};
use sp_runtime::{
    traits::{CheckedSub, SaturatedConversion, StaticLookup, Zero},
    Perbill, Percent,
};
use sp_std::{convert::From, prelude::*, result};

const STAKING_ID: LockIdentifier = *b"dapstake";

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    /// The balance type of this pallet.
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The staking balance.
        type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

        /// Time used for computing era duration.
        type UnixTime: UnixTime;

        /// Tokens have been minted and are unused for validator-reward. Maybe, dapps-staking uses ().
        type RewardRemainder: OnUnbalanced<NegativeImbalanceOf<Self>>;

        /// Handler for the unbalanced increment when rewarding a staker. Maybe, dapps-staking uses ().
        type Reward: OnUnbalanced<PositiveImbalanceOf<Self>>;

        /// Number of blocks per era.
        #[pallet::constant]
        type BlockPerEra: Get<BlockNumberFor<Self>>;

        /// Number of eras that staked funds must remain bonded for after calling unbond.
        #[pallet::constant]
        type UnbondingDuration: Get<EraIndex>;

        /// Minimum bonded deposit for new contract registration.
        #[pallet::constant]
        type RegisterDeposit: Get<BalanceOf<Self>>;

        /// The payout for validators and the system for the current era.
        /// See [Era payout](./index.html#era-payout).
        type EraPayout: EraPayout<BalanceOf<Self>>;

        /// The maximum number of stakers rewarded for each contracts.
        #[pallet::constant]
        type MaxStakings: Get<u32>;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::type_value]
    pub(crate) fn HistoryDepthOnEmpty() -> u32 {
        84u32
    }

    /// Map from all locked "stash" accounts to the controller account.
    #[pallet::storage]
    #[pallet::getter(fn bonded)]
    pub(crate) type Bonded<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

    /// Map from all (unlocked) "controller" accounts to the info regarding the staking.
    #[pallet::storage]
    #[pallet::getter(fn ledger)]
    pub(crate) type Ledger<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, StakingLedger<T::AccountId, BalanceOf<T>>>;

    /// Where the reward payment should be made. Keyed by stash.
    #[pallet::storage]
    #[pallet::getter(fn payee)]
    pub(crate) type Payee<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, RewardDestination<T::AccountId>>;

    /// Number of eras to keep in history.
    ///
    /// Information is kept for eras in `[current_era - history_depth; current_era]`.
    ///
    /// Must be more than the number of eras delayed by session otherwise. I.e. active era must
    /// always be in history. I.e. `active_era > current_era - history_depth` must be
    /// guaranteed.
    #[pallet::storage]
    #[pallet::getter(fn history_depth)]
    pub(crate) type HistoryDepth<T> = StorageValue<_, u32, ValueQuery, HistoryDepthOnEmpty>;

    /// The already untreated era is EraIndex.
    #[pallet::storage]
    #[pallet::getter(fn untreated_era)]
    pub type UntreatedEra<T> = StorageValue<_, EraIndex, ValueQuery>;

    /// The current era index.
    ///
    /// This is the latest planned era, depending on how the Session pallet queues the validator
    /// set, it might be active or not.
    #[pallet::storage]
    #[pallet::getter(fn current_era)]
    pub type CurrentEra<T> = StorageValue<_, EraIndex>;

    /// The active era information, it holds index and start.
    ///
    /// The active era is the era being currently rewarded. Validator set of this era must be
    /// equal to [`SessionInterface::validators`].
    #[pallet::storage]
    #[pallet::getter(fn active_era)]
    pub type ActiveEra<T> = StorageValue<_, ActiveEraInfo>;

    #[pallet::type_value]
    pub fn ForceEraOnEmpty() -> Forcing {
        Forcing::ForceNone
    }
    /// Mode of era forcing.
    #[pallet::storage]
    #[pallet::getter(fn force_era)]
    pub type ForceEra<T> = StorageValue<_, Forcing, ValueQuery, ForceEraOnEmpty>;

    /// Registered developer accounts points to coresponding contract
    #[pallet::storage]
    #[pallet::getter(fn registered_contract)]
    pub(crate) type RegisteredDevelopers<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, SmartContract<T::AccountId>>;

    /// Registered dapp points to the developer who registered it
    #[pallet::storage]
    #[pallet::getter(fn registered_developer)]
    pub(crate) type RegisteredDapps<T: Config> =
        StorageMap<_, Twox64Concat, SmartContract<T::AccountId>, T::AccountId>;

    /// Total block rewards for the pallet per era
    #[pallet::storage]
    #[pallet::getter(fn get_era_total)]
    pub(crate) type PalletEraRewards<T: Config> =
        StorageMap<_, Twox64Concat, EraIndex, BalanceOf<T>>;

    /// Stores amount staked and stakers for a contract per era
    #[pallet::storage]
    #[pallet::getter(fn contract_era_stake)]
    pub(crate) type ContractEraStake<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        SmartContract<T::AccountId>,
        Twox64Concat,
        EraIndex,
        EraStakingPoints<T::AccountId, BalanceOf<T>>,
    >;

    /// Marks an Era when a contract is last claimed
    #[pallet::storage]
    #[pallet::getter(fn contract_last_claimed)]
    pub(crate) type ContractLastClaimed<T: Config> =
        StorageMap<_, Twox64Concat, SmartContract<T::AccountId>, EraIndex>;

    /// Marks an Era when a contract is last (un)staked
    #[pallet::storage]
    #[pallet::getter(fn contract_last_staked)]
    pub(crate) type ContractLastStaked<T: Config> =
        StorageMap<_, Twox64Concat, SmartContract<T::AccountId>, EraIndex>;

    // Declare the genesis config (optional).
    //
    // The macro accepts either a struct or an enum; it checks that generics are consistent.
    //
    // Type must implement the `Default` trait.
    #[pallet::genesis_config]
    #[derive(Default)]
    pub struct GenesisConfig {
        _myfield: u32,
    }

    // Declare genesis builder. (This is need only if GenesisConfig is declared)
    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {}
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
    pub enum Event<T: Config> {
        /// The amount of minted rewards. (for dapps with nominators)
        Reward(BalanceOf<T>, BalanceOf<T>),
        /// An account has bonded this amount.
        ///
        /// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
        /// it will not be emitted for staking rewards when they are added to stake.
        Bonded(T::AccountId, BalanceOf<T>),
        /// A stash account has changed paired controller account
        /// (stash Id, controller Id)
        ControllerChanged(T::AccountId, T::AccountId),
        /// An account has unbonded this amount.
        Unbonded(T::AccountId, BalanceOf<T>),
        /// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
        /// from the unlocking queue.
        Withdrawn(T::AccountId, BalanceOf<T>),
        /// The total amount of minted rewards for dapps.
        TotalDappsRewards(EraIndex, BalanceOf<T>),
        /// Stake of stash address.
        Stake(T::AccountId),
        /// New contract added for staking, with deposit value
        NewContract(T::AccountId, SmartContract<T::AccountId>),
        /// New dapps staking era. Distribute era rewards to contracts
        NewDappStakingEra(EraIndex),
    }

    #[pallet::error]
    pub enum Error<T> {
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
        /// The contract is already registered by other account
        AlreadyRegisteredContract,
        /// User attempts to register with address which is not contract
        AddressIsNotContract,
        /// Missing deposit for the contract registration
        InsufficientDeposit,
        /// This account was already used to register contract
        AlreadyUsedDeveloperAccount,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            // just return the weight of the on_finalize.
            let force_new_era = Self::force_era().eq(&Forcing::ForceNew);
            if (now % T::BlockPerEra::get()).is_zero() || force_new_era {
                Self::reward_balance_snapshoot();
                let next_era = Self::current_era().unwrap_or(Zero::zero()) + 1;
                CurrentEra::<T>::put(next_era);
                if force_new_era {
                    ForceEra::<T>::put(Forcing::ForceNone);
                }
                Self::deposit_event(Event::<T>::NewDappStakingEra(next_era));
            }

            T::DbWeight::get().reads(1)
        }

        fn on_finalize(_n: BlockNumberFor<T>) {
            // TODO: era calculation
            // Set the start of the first era.
            if let Some(mut active_era) = Self::active_era() {
                if active_era.start.is_none() {
                    let now_as_millis_u64 = T::UnixTime::now().as_millis().saturated_into::<u64>();
                    active_era.start = Some(now_as_millis_u64);
                    // This write only ever happens once, we don't include it in the weight in general
                    ActiveEra::<T>::put(active_era);
                }
            }
            // `on_finalize` weight is tracked in `on_initialize`
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
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
        #[pallet::weight(T::WeightInfo::bond())]
        pub fn bond(
            origin: OriginFor<T>,
            controller: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] value: BalanceOf<T>,
            payee: RewardDestination<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let stash = ensure_signed(origin)?;
            ensure!(
                !Bonded::<T>::contains_key(&stash),
                Error::<T>::AlreadyBonded
            );

            let controller = T::Lookup::lookup(controller)?;
            ensure!(
                !Ledger::<T>::contains_key(&controller),
                Error::<T>::AlreadyPaired
            );

            // reject a bond which is considered to be dust
            ensure!(
                value >= T::Currency::minimum_balance(),
                Error::<T>::InsufficientValue
            );

            let free_stash = T::Currency::free_balance(&stash);
            ensure!(!free_stash.is_zero(), Error::<T>::InsufficientValue);

            Bonded::<T>::insert(&stash, &controller);
            Payee::<T>::insert(&stash, payee);

            let value = value.min(free_stash);
            let ledger = StakingLedger {
                stash: stash.clone(),
                total: value,
                active: value,
                unlocking: vec![],
                last_reward: Self::current_era().unwrap_or(Zero::zero()),
            };
            Self::update_ledger(&controller, &ledger);
            Self::deposit_event(Event::<T>::Bonded(stash, value));

            Ok(().into())
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
        #[pallet::weight(T::WeightInfo::bond_extra())]
        pub fn bond_extra(
            origin: OriginFor<T>,
            #[pallet::compact] max_additional: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let stash = ensure_signed(origin)?;

            let controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

            let free_stash = T::Currency::free_balance(&stash);

            if let Some(extra) = free_stash.checked_sub(&ledger.total) {
                let extra = extra.min(max_additional);
                ledger.total += extra;
                ledger.active += extra;
                Self::update_ledger(&controller, &ledger);
                Self::deposit_event(Event::<T>::Bonded(stash, extra));
            }

            Ok(().into())
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
        #[pallet::weight(T::WeightInfo::unbond())]
        pub fn unbond(
            origin: OriginFor<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
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
                    // add dust to the unbonding value
                    value += ledger.active;
                    ledger.active = Zero::zero();
                }
                let era = Self::current_era().unwrap_or(Zero::zero()) + T::UnbondingDuration::get();
                ledger.unlocking.push(UnlockChunk { value, era });
                Self::update_ledger(&controller, &ledger);
                Self::deposit_event(Event::<T>::Unbonded(ledger.stash.clone(), value));
            } else {
                // just deposit event with zero
                Self::deposit_event(Event::<T>::Unbonded(ledger.stash.clone(), Zero::zero()));
            }

            Ok(().into())
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
        #[pallet::weight(T::WeightInfo::withdraw_unbonded())]
        pub fn withdraw_unbonded(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let controller = ensure_signed(origin)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            let (stash, old_total) = (ledger.stash.clone(), ledger.total);

            let current_era = Self::current_era().unwrap_or(Zero::zero());
            ledger = ledger.consolidate_unlocked(current_era);

            let withdrawn_value = old_total.saturating_sub(ledger.total);

            if ledger.unlocking.is_empty() && ledger.active.is_zero() {
                // This account must have called `unbond()` with some value that caused the active
                // portion to fall below existential deposit + will have no more unlocking chunks
                // left. We can now safely remove all staking-related information.
                Self::kill_stash(&stash)?;
                // remove the lock.
                T::Currency::remove_lock(STAKING_ID, &stash);
            } else if !withdrawn_value.is_zero() {
                // Partial unbond, update ledger and move on.
                Self::update_ledger(&controller, &ledger);
            }

            if !withdrawn_value.is_zero() {
                Self::deposit_event(Event::<T>::Withdrawn(stash, withdrawn_value));
            }

            Ok(().into())
        }

        /// Declare the desire to stake(nominate) `targets` for the origin contracts.
        ///
        /// Effects will be felt at the beginning of the next era.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// It will automatically be diversified into `targets` based on the amount bound.
        /// For example, if you stake 4 contracts while bonding 100 SDNs, he stakes 25 SDNs for each contract.
        ///
        /// # <weight>
        /// - The transaction's complexity is proportional to the size of `targets`,
        /// which is capped at `MAX_STAKINGS`.
        /// - Both the reads and writes follow a similar pattern.
        #[pallet::weight(T::WeightInfo::stake_contracts(targets.len() as u32))]
        pub fn stake_contracts(
            origin: OriginFor<T>,
            targets: Vec<<T::Lookup as StaticLookup>::Source>,
        ) -> DispatchResult {
            // TODO: impls
            Ok(())
        }

        /// vote some contracts with Bad/Good.
        /// If you have already voted for a contract on your account, your vote for that contract will be overridden.
        /// If you didn't bond, you can not vote.
        /// The voting power equal to amount of bonded.
        ///
        /// Effects will be felt at the beginning of the next era.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// # <weight>
        #[pallet::weight(T::WeightInfo::vote_contracts(targets.len() as u32))]
        pub fn vote_contracts(
            origin: OriginFor<T>,
            targets: Vec<(<T::Lookup as StaticLookup>::Source, Vote)>,
        ) -> DispatchResult {
            // TODO: impls
            Ok(())
        }

        /// Declare no desire to either staking.
        ///
        /// Effects will be felt at the beginning of the next era.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// # <weight>
        /// - Independent of the arguments. Insignificant complexity.
        /// - Contains one read
        /// - Contains one read.
        /// - Writes are limited to the `origin` account key.
        /// # </weight>
        #[pallet::weight(T::WeightInfo::chill())]
        pub fn chill(origin: OriginFor<T>) -> DispatchResult {
            // TOOD: impls
            Ok(())
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
        #[pallet::weight(T::WeightInfo::set_payee())]
        pub fn set_payee(
            origin: OriginFor<T>,
            payee: RewardDestination<T::AccountId>,
        ) -> DispatchResult {
            // TOOD: impls
            Ok(())
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
        #[pallet::weight(T::WeightInfo::set_controller())]
        pub fn set_controller(
            origin: OriginFor<T>,
            controller: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResultWithPostInfo {
            let stash = ensure_signed(origin)?;
            let old_controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
            let controller = T::Lookup::lookup(controller)?;
            ensure!(
                !Ledger::<T>::contains_key(&controller),
                Error::<T>::AlreadyPaired
            );
            // change controller for given stash
            Bonded::<T>::insert(&stash, &controller);

            //create new Ledger from existing. Use new controler as the key
            if let Some(ledger) = <Ledger<T>>::take(&old_controller) {
                Ledger::<T>::insert(&controller, ledger);
            }
            Self::deposit_event(Event::<T>::ControllerChanged(stash, controller));

            Ok(().into())
        }

        /// rewards are claimed by the staker on contract_id.
        ///
        /// era must be in the range `[current_era - history_depth; active_era)`.
        ///
        /// Any user can call this function.
        #[pallet::weight(T::WeightInfo::payout_stakers_alive_staked(T::MaxStakings::get()))]
        pub fn payout_stakers(
            _origin: OriginFor<T>,
            contract_id: T::AccountId,
            era: EraIndex,
        ) -> DispatchResult {
            // TODO: impls
            Ok(())
        }

        /// rewards are claimed by the contract.
        ///
        /// era must be in the range [current_era - history_depth; active_era).
        ///
        /// Any user can call this function.
        /// TODO: weight
        #[pallet::weight(T::WeightInfo::payout_stakers_alive_staked(T::MaxStakings::get()))]
        pub fn payout_contract(
            origin: OriginFor<T>,
            contract_id: T::AccountId,
            era: EraIndex,
        ) -> DispatchResult {
            // TODO: impls
            Ok(())
        }

        /// register contract into staking targets.
        /// contract_id should be ink! or evm contract.
        ///
        /// Any user can call this function.
        /// However, caller have to have deposit amount.
        /// TODO: weight, and add registrationFee
        #[pallet::weight(T::WeightInfo::payout_stakers_alive_staked(T::MaxStakings::get()))]
        pub fn register(
            origin: OriginFor<T>,
            contract_id: SmartContract<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let developer = ensure_signed(origin)?;

            ensure!(
                !RegisteredDevelopers::<T>::contains_key(&developer),
                Error::<T>::AlreadyUsedDeveloperAccount
            );
            ensure!(
                !RegisteredDapps::<T>::contains_key(&contract_id),
                Error::<T>::AlreadyRegisteredContract
            );
            ensure!(
                Self::is_contract(&contract_id),
                Error::<T>::AddressIsNotContract
            );

            RegisteredDapps::<T>::insert(contract_id.clone(), developer.clone());
            RegisteredDevelopers::<T>::insert(&developer, contract_id.clone());

            // create new ContractEraStake item
            let era_staking_points = EraStakingPoints {
                total: <BalanceOf<T>>::default(),
                stakers: BTreeMap::new(),
            };
            let current = Self::current_era().unwrap_or(Zero::zero());
            ContractEraStake::<T>::insert(&contract_id, &current, era_staking_points);

            Self::deposit_event(Event::<T>::NewContract(developer, contract_id));

            Ok(().into())
        }

        /// set deposit amount for registering contract.
        ///
        /// The dispatch origin for this call must be _Signed_ by the root.
        ///
        /// TODO: weight
        #[pallet::weight(T::WeightInfo::payout_stakers_alive_staked(T::MaxStakings::get()))]
        pub fn set_register_deposit(
            origin: OriginFor<T>,
            #[pallet::compact] deposit_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            // if let Some(_deposit) = RegisterDeposit::<T>::take() {
            //     RegisterDeposit::<T>::set(Some(deposit_amount));
            // }
            Ok(().into())
        }

        /// Set `HistorcargoyDepth` value. This function will delete any history information
        /// when `HistoryDepth` is reduced.
        ///
        /// Parameters:
        /// - `new_history_depth`: The new history depth you would like to set.
        /// - `era_items_deleted`: The number of items that will be deleted by this dispatch.
        ///    This should report all the storage items that will be deleted by clearing old
        ///    era history. Needed to report an accurate weight for the dispatch. Trusted by
        ///    `Root` to report an accurate number.
        ///
        /// Origin must be root.
        ///
        /// # <weight>
        /// - E: Number of history depths removed, i.e. 10 -> 7 = 3
        /// - Weight: O(E)
        /// - DB Weight:
        ///     - Reads: Current Era, History Depth
        ///     - Writes: History Depth
        ///     - Clear Prefix Each: Era Stakers, EraStakersClipped, ErasValidatorPrefs
        ///     - Writes Each: ErasValidatorReward, ErasRewardPoints, ErasTotalStake, ErasStartSessionIndex
        /// # </weight>
        #[pallet::weight(T::WeightInfo::set_history_depth(*_era_items_deleted))]
        pub fn set_history_depth(
            origin: OriginFor<T>,
            #[pallet::compact] new_history_depth: EraIndex,
            #[pallet::compact] _era_items_deleted: u32,
        ) -> DispatchResult {
            ensure_root(origin)?;
            if let Some(current_era) = Self::current_era() {
                HistoryDepth::<T>::mutate(|history_depth| {
                    let last_kept = current_era.checked_sub(*history_depth).unwrap_or(0);
                    let new_last_kept = current_era.checked_sub(new_history_depth).unwrap_or(0);
                    for era_index in last_kept..new_last_kept {
                        // Self::clear_era_information(era_index);
                    }
                    *history_depth = new_history_depth
                })
            }
            Ok(())
        }

        // =============== Era ==================

        /// Force there to be no new eras indefinitely.
        ///
        /// The dispatch origin must be Root.
        ///
        /// # Warning
        ///
        /// The election process starts multiple blocks before the end of the era.
        /// Thus the election process may be ongoing when this is called. In this case the
        /// election will continue until the next era is triggered.
        ///
        /// # <weight>
        /// - No arguments.
        /// - Weight: O(1)
        /// - Write: ForceEra
        /// # </weight>
        #[pallet::weight(T::WeightInfo::force_no_eras())]
        pub fn force_no_eras(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;
            ForceEra::<T>::put(Forcing::ForceNone);
            Ok(())
        }

        /// Force there to be a new era at the end of the next session. After this, it will be
        /// reset to normal (non-forced) behaviour.
        ///
        /// The dispatch origin must be Root.
        ///
        /// # Warning
        ///
        /// The election process starts multiple blocks before the end of the era.
        /// If this is called just before a new era is triggered, the election process may not
        /// have enough blocks to get a result.
        ///
        /// # <weight>
        /// - No arguments.
        /// - Weight: O(1)
        /// - Write ForceEra
        /// # </weight>
        #[pallet::weight(T::WeightInfo::force_new_era())]
        pub fn force_new_era(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;
            ForceEra::<T>::put(Forcing::ForceNew);
            Ok(())
        }

        /// Force there to be a new era at the end of blocks indefinitely.
        ///
        /// The dispatch origin must be Root.
        ///
        /// # Warning
        ///
        /// The election process starts multiple blocks before the end of the era.
        /// If this is called just before a new era is triggered, the election process may not
        /// have enough blocks to get a result.
        ///
        /// # <weight>
        /// - Weight: O(1)
        /// - Write: ForceEra
        /// # </weight>
        #[pallet::weight(T::WeightInfo::force_new_era_always())]
        pub fn force_new_era_always(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;
            ForceEra::<T>::put(Forcing::ForceAlways);
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Update the ledger for a controller. This will also update the stash lock.
        /// This lock will lock the entire funds except paying for further transactions.
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
            // if let Some(nominations) = Self::dapps_nominations(stash) {
            //     Self::remove_nominations(stash, nominations);
            // }

            //system::Module::<T>::dec_consumers(stash);
            Ok(())
        }

        /// Checks if there is a valid smart contract for the provided address
        fn is_contract(address: &SmartContract<T::AccountId>) -> bool {
            match address {
                SmartContract::Wasm(account) => {
                    //     <pallet_contracts::ContractInfoOf<T>>::get(&account).is_some()
                    false
                }
                SmartContract::Evm(account) => {
                    // pallet_evm::Module::<T>::account_codes(&account).len() > 0 TODO remove comment after EVM mege
                    true
                }
            }
        }

        /// The block rewards are accumulated on the pallets's account during an era.
        /// This function takes a snapshot of the pallet's balance and stores it for future distribution
        ///
        /// This is called at the end of each Era
        fn reward_balance_snapshoot() {}
    }
}
