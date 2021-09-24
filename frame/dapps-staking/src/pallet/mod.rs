//! Dapps staking FRAME Pallet.

use super::*;
use frame_support::{
    dispatch::DispatchResult,
    ensure,
    pallet_prelude::*,
    traits::{
        Currency, Get, LockIdentifier, LockableCurrency, OnUnbalanced, UnixTime, WithdrawReasons,
    },
    weights::Weight,
};
use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
use sp_runtime::{
    print,
    traits::{SaturatedConversion, Saturating, Zero},
    Perbill,
};
use sp_std::convert::From;

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

        /// Reword amount per block. Will be divided by DAppsRewardPercentage
        type RewardAmount: Get<BalanceOf<Self>>;

        /// The percentage of the network block reward that goes to this pallet
        type DAppsRewardPercentage: Get<u32>;

        /// Number of blocks per era.
        #[pallet::constant]
        type BlockPerEra: Get<BlockNumberFor<Self>>;

        /// Number of eras that staked funds must remain bonded for after calling unbond.
        #[pallet::constant]
        type UnbondingDuration: Get<EraIndex>;

        /// Minimum bonded deposit for new contract registration.
        #[pallet::constant]
        type RegisterDeposit: Get<BalanceOf<Self>>;

        /// Percentage of reward paid to developer.
        #[pallet::constant]
        type DeveloperRewardPercentage: Get<u32>;

        /// Maximum number of unique stakers per contract.
        #[pallet::constant]
        type MaxNumberOfStakersPerContract: Get<u32>;

        /// Minimum amount user must stake on contract.
        /// User can stake less if they already have the minimum staking amount staked on that particular contract.
        #[pallet::constant]
        type MinimumStakingAmount: Get<BalanceOf<Self>>;

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

    /// Accumulator for block rewards during an era. It is reset at every new era
    #[pallet::storage]
    #[pallet::getter(fn block_reward_accumulator)]
    pub type BlockRewardAccumulator<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

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
    #[pallet::getter(fn era_reward_and_stake)]
    pub(crate) type EraRewardsAndStakes<T: Config> =
        StorageMap<_, Twox64Concat, EraIndex, EraRewardAndStake<BalanceOf<T>>>;

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
        /// Account has bonded and staked funds on a smart contract.
        BondAndStake(T::AccountId, SmartContract<T::AccountId>, BalanceOf<T>),
        /// Account has unbonded, unstaked and withdrawn funds.
        UnbondUnstakeAndWithdraw(T::AccountId, SmartContract<T::AccountId>, BalanceOf<T>),
        /// New contract added for staking, with deposit value
        NewContract(T::AccountId, SmartContract<T::AccountId>),
        /// New dapps staking era. Distribute era rewards to contracts
        NewDappStakingEra(EraIndex),
        /// The contract's reward have been claimed, by an account, from era, until era
        ContractClaimed(
            SmartContract<T::AccountId>,
            T::AccountId,
            EraIndex,
            EraIndex,
        ),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Not a controller account.
        NotController,
        /// Not a stash account.
        NotStash,
        /// Account is not an active staker
        NotStaker,
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
        InsufficientBondValue,
        /// Can not stake with zero value.
        StakingWithNoValue,
        /// Can not stake with value less than minimum staking value
        InsufficientStakingValue,
        /// Can not schedule more unlock chunks.
        NoMoreChunks,
        /// Can not rebond without unlocking chunks.
        NoUnlockChunk,
        /// Attempting to target a stash that still has funds.
        FundedTarget,
        /// Invalid era to reward.
        InvalidEraToReward,
        /// Number of stakers per contract exceeded.
        MaxNumberOfStakersExceeded,
        /// Items are not sorted and unique.
        NotSortedAndUnique,
        /// Targets must be latest 1.
        EmptyNominateTargets,
        /// Targets must be operated contracts
        NotOperatedContract,
        /// Contract isn't staked.
        NotStakedContract,
        /// Unstaking a contract with zero value
        UnstakingWithNoValue,
        /// The nominations amount more than active staking amount.
        NotEnoughStaking,
        /// The contract is already registered by other account
        AlreadyRegisteredContract,
        /// User attempts to register with address which is not contract
        ContractIsNotValid,
        /// Missing deposit for the contract registration
        InsufficientDeposit,
        /// This account was already used to register contract
        AlreadyUsedDeveloperAccount,
        /// Unexpected state error, used to abort transaction
        UnexpectedState,
        /// Report issue on github if this is ever emitted
        UnknownStartStakingData,
        /// Report issue on github if this is ever emitted
        UnknownEraReward,
        /// There are no funds to reward the contract. Or already claimed in that era
        NothingToClaim,
        /// Claiming contract with no developer account
        ContractNotRegistered,
        /// Contract already claimed in this era and reward is distributed
        AlreadyClaimedInThisEra,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            // Handle dapps staking era
            let block_rewards = Self::block_reward_accumulator();
            BlockRewardAccumulator::<T>::put(block_rewards + T::RewardAmount::get());
            let force_new_era = Self::force_era().eq(&Forcing::ForceNew);
            let blocks_pre_era = T::BlockPerEra::get();
            if (now % blocks_pre_era).is_zero() || force_new_era {
                let current_era = Self::get_current_era();
                Self::reward_balance_snapshoot(current_era);
                let next_era = current_era + 1;
                CurrentEra::<T>::put(next_era);
                let zero_balance: BalanceOf<T> = Default::default();
                BlockRewardAccumulator::<T>::put(zero_balance);
                if force_new_era {
                    ForceEra::<T>::put(Forcing::ForceNone);
                }
                Self::deposit_event(Event::<T>::NewDappStakingEra(next_era));
            }

            // just return the weight of the on_finalize.
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
        /// Lock up and stake balance of the origin account.
        ///
        /// `value` must be more than the `minimum_balance` specified by `T::Currency`
        /// unless account already has bonded value equal or more than 'minimum_balance'.
        ///
        /// The dispatch origin for this call must be _Signed_ by the staker's account.
        ///
        /// Effects of staking will be felt at the beginning of the next era.
        ///
        /// TODO: Weight!
        #[pallet::weight(10)] // TODO: fix this later. Probably a new calculation will be required since logic was changed significantly.
        pub fn bond_and_stake(
            origin: OriginFor<T>,
            contract_id: SmartContract<T::AccountId>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let staker = ensure_signed(origin)?;
            ensure!(
                Self::is_contract_valid(&contract_id),
                Error::<T>::ContractIsNotValid
            );
            ensure!(
                RegisteredDapps::<T>::contains_key(&contract_id),
                Error::<T>::NotOperatedContract
            );

            // Get the staking ledger or create an entry if it doesn't exist.
            let mut ledger = if let Some(ledger) = Self::ledger(&staker) {
                ledger
            } else {
                StakingLedger {
                    stash: staker.clone(),
                    total: Zero::zero(),
                    active: Zero::zero(),
                    unlocking: vec![],
                    last_reward: Zero::zero(),
                }
            };

            // Ensure that staker has enough balance to bond & stake.
            let free_balance = T::Currency::free_balance(&staker);
            // Remove already locked funds from the free balance
            let available_balance = free_balance.saturating_sub(ledger.total);
            let bonded_value = value.min(available_balance);
            ensure!(!bonded_value.is_zero(), Error::<T>::StakingWithNoValue);

            // update the ledger value by adding the newly bonded funds
            ledger.total += bonded_value;
            ledger.active += bonded_value;

            // Get the latest era staking point info or create it if contract hasn't been staked yet so far.
            let era_when_contract_last_staked = Self::contract_last_staked(&contract_id);
            let mut latest_era_staking_points =
                if let Some(last_stake_era) = era_when_contract_last_staked.clone() {
                    // No era staking points struct available even though we have information that contract was staked before. This is a bug!
                    Self::contract_era_stake(&contract_id, &last_stake_era)
                        .ok_or(Error::<T>::UnexpectedState)?
                } else {
                    EraStakingPoints {
                        total: Zero::zero(),
                        stakers: BTreeMap::<T::AccountId, BalanceOf<T>>::new(),
                    }
                };

            // Ensure that we can add additional staker for the contract.
            if !latest_era_staking_points.stakers.contains_key(&staker) {
                ensure!(
                    latest_era_staking_points.stakers.len()
                        < T::MaxNumberOfStakersPerContract::get() as usize,
                    Error::<T>::MaxNumberOfStakersExceeded
                );
            }

            // Increment the staked amount.
            latest_era_staking_points.total += bonded_value;
            let entry = latest_era_staking_points
                .stakers
                .entry(staker.clone())
                .or_insert(Zero::zero());
            *entry += bonded_value;

            ensure!(
                *entry >= T::MinimumStakingAmount::get(),
                Error::<T>::InsufficientStakingValue
            );

            // Update ledger and payee
            Self::update_ledger(&staker, &ledger);

            let current_era = Self::get_current_era();

            // Update staked information for contract in current era
            ContractEraStake::<T>::insert(
                contract_id.clone(),
                current_era,
                latest_era_staking_points,
            );

            // Update total staked value in era. There are 3 possible scenarios here.
            let mut reward_and_stake_for_era =
                if let Some(reward_and_stake) = Self::era_reward_and_stake(current_era) {
                    reward_and_stake
                } else if era_when_contract_last_staked.is_some() {
                    Self::era_reward_and_stake(era_when_contract_last_staked.unwrap())
                        .ok_or(Error::<T>::UnexpectedState)?
                } else {
                    Default::default()
                };
            reward_and_stake_for_era.staked += bonded_value;
            EraRewardsAndStakes::<T>::insert(current_era, reward_and_stake_for_era);

            // If contract wasn't claimed nor staked yet, insert current era as last claimed era.
            // When calculating reward, this will provide correct information to the algorithm since nothing exists
            // for this contract prior to the current era.
            if !era_when_contract_last_staked.is_some() {
                ContractLastClaimed::<T>::insert(contract_id.clone(), current_era);
            }

            // Check if we need to update era in which contract was last changed. Can avoid one write.
            let contract_last_staked_change_needed =
                if let Some(previous_era) = era_when_contract_last_staked {
                    // if values aren't different, no reason to do another write
                    previous_era != current_era
                } else {
                    true
                };
            if contract_last_staked_change_needed {
                ContractLastStaked::<T>::insert(&contract_id, current_era);
            }

            Self::deposit_event(Event::<T>::BondAndStake(staker, contract_id, bonded_value));

            Ok(().into())
        }

        /// Unbond, unstake and withdraw balance from the contract.
        ///
        /// Value will be unlocked for the user.
        ///
        /// In case remaining staked balance on contract is below minimum staking amount,
        /// entire stake for that contract will be unstaked.
        ///
        /// # <weight>
        /// TODO!
        /// </weight>
        #[pallet::weight(10)]
        pub fn unbond_unstake_and_withdraw(
            origin: OriginFor<T>,
            contract_id: SmartContract<T::AccountId>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let staker = ensure_signed(origin)?;
            ensure!(
                Self::is_contract_valid(&contract_id),
                Error::<T>::ContractIsNotValid
            );
            ensure!(
                RegisteredDapps::<T>::contains_key(&contract_id),
                Error::<T>::NotOperatedContract
            );
            ensure!(value > Zero::zero(), Error::<T>::UnstakingWithNoValue);

            // Get the latest era staking points for the contract.
            let era_when_contract_last_staked =
                Self::contract_last_staked(&contract_id).ok_or(Error::<T>::NotStakedContract)?;
            let mut era_staking_points = Self::contract_era_stake(&contract_id, &era_when_contract_last_staked).ok_or_else(|| {
                print("No era staking points for contract even though information exists that it was staked. This is a bug!");
                Error::<T>::UnexpectedState
            })?;

            // Ensure that the staker actually has this contract staked.
            let staked_value = *era_staking_points
                .stakers
                .get(&staker)
                .ok_or(Error::<T>::NotStakedContract)?;

            // Calculate the value which will be unstaked.
            let mut value_to_unstake = value.min(staked_value);
            let remaining_staked_value = staked_value.saturating_sub(value_to_unstake);
            if remaining_staked_value < T::MinimumStakingAmount::get() {
                // if staked value would fall below threshold, unstake everything
                era_staking_points.stakers.remove(&staker);
                value_to_unstake = staked_value;
            } else {
                era_staking_points
                    .stakers
                    .insert(staker.clone(), remaining_staked_value);
            }
            let value_to_unstake = value_to_unstake; // make it immutable
            era_staking_points.total = era_staking_points.total.saturating_sub(value_to_unstake);

            // Get the staking ledger and update it
            let mut ledger = Self::ledger(&staker).ok_or(Error::<T>::UnexpectedState)?;
            ledger.total = ledger.total.saturating_sub(value_to_unstake);
            ledger.active = ledger.active.saturating_sub(value_to_unstake);
            Self::update_ledger(&staker, &ledger);

            let current_era = Self::get_current_era();

            // Update the era staking points
            ContractEraStake::<T>::insert(contract_id.clone(), current_era, era_staking_points);

            // Update total staked value in era.
            let mut era_reward = if let Some(era_reward) = Self::era_reward_and_stake(current_era) {
                era_reward
            } else {
                // If there was no stake/unstake operation in current era, fetch if in the last era there was.
                Self::era_reward_and_stake(era_when_contract_last_staked)
                    .ok_or(Error::<T>::UnexpectedState)?
            };
            era_reward.staked = era_reward.staked.saturating_sub(value_to_unstake);
            EraRewardsAndStakes::<T>::insert(current_era, era_reward);

            // Check if we need to update era in which contract was last changed. Can avoid one write.
            if era_when_contract_last_staked != current_era {
                ContractLastStaked::<T>::insert(&contract_id, current_era);
            }

            Self::deposit_event(Event::<T>::UnbondUnstakeAndWithdraw(
                staker,
                contract_id,
                value_to_unstake,
            ));

            Ok(().into())
        }

        /// register contract into staking targets.
        /// contract_id should be ink! or evm contract.
        ///
        /// Any user can call this function.
        /// However, caller have to have deposit amount.
        /// TODO: weight, and add registrationFee
        #[pallet::weight(1_000_000_000)]
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
                Self::is_contract_valid(&contract_id),
                Error::<T>::ContractIsNotValid
            );

            RegisteredDapps::<T>::insert(contract_id.clone(), developer.clone());
            RegisteredDevelopers::<T>::insert(&developer, contract_id.clone());

            Self::deposit_event(Event::<T>::NewContract(developer, contract_id));

            Ok(().into())
        }

        /// claim the rewards earned by contract_id.
        /// All stakers and developer for this contract will be paid out with single call.
        /// claim is valid for all unclaimed eras but not longer than history_depth().
        /// Any reward older than history_depth() will go to Treasury.
        /// Any user can call this function.
        #[pallet::weight(1_000_000)]
        pub fn claim(
            origin: OriginFor<T>,
            contract_id: SmartContract<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let claimer = ensure_signed(origin)?;

            // check if this contract is registered
            let developer = Self::registered_developer(&contract_id)
                .ok_or(Error::<T>::ContractNotRegistered)?;

            // check if it was ever staked on this contract.
            Self::contract_last_staked(&contract_id).ok_or(Error::<T>::NothingToClaim)?;

            // check if the contract is already claimed in this era
            let current_era = Self::current_era().unwrap_or(Zero::zero());
            let last_claim_era =
                Self::contract_last_claimed(&contract_id).unwrap_or(current_era.clone());
            ensure!(
                current_era != last_claim_era,
                Error::<T>::AlreadyClaimedInThisEra
            );

            // oldest era to start with collecting rewards
            let last_allowed_era = current_era.saturating_sub(Self::history_depth());
            let start_from_era = last_claim_era.max(last_allowed_era);
            if start_from_era > last_claim_era {
                // TODO collect all unclaimed rewards and send to Treasury pallet
            }

            // for the first claimable era "start_from_era", this storage item must be in place!
            let mut contract_staking_info_prev =
                Self::contract_era_stake(&contract_id, &start_from_era)
                    .ok_or(Error::<T>::UnknownStartStakingData)?;

            // initialize rewards for stakers and the developer
            let mut rewards_for_stakers_map: BTreeMap<T::AccountId, BalanceOf<T>> =
                Default::default();
            let mut reward_for_developer: BalanceOf<T> = Default::default();

            // for any era after start_from_era, the ContractEraStake is present only if there
            // was a change in staking amount. If it is not present we process last recorded ContractEraStake
            for era in start_from_era..current_era {
                let reward_and_stake_for_era =
                    Self::era_reward_and_stake(era).ok_or(Error::<T>::UnknownEraReward)?;
                let contract_staking_info = Self::contract_era_stake(&contract_id, era)
                    .unwrap_or(contract_staking_info_prev);

                // smallest unit of the reward in this era to use in calculation
                let reward_particle = Perbill::from_rational(
                    reward_and_stake_for_era.rewards,
                    reward_and_stake_for_era.staked,
                );

                // this contract's total reward in this era
                let contract_reward_in_era = reward_particle * contract_staking_info.total;

                // divide reward between stakers and the developer of the contract
                let contract_staker_reward =
                    Perbill::from_rational((100 - T::DeveloperRewardPercentage::get()) as u64, 100)
                        * contract_reward_in_era;
                let contract_developer_reward =
                    Perbill::from_rational(T::DeveloperRewardPercentage::get() as u64, 100)
                        * contract_reward_in_era;

                // accumulate rewards for the stakers
                Self::stakers_era_reward(
                    &mut rewards_for_stakers_map,
                    &contract_staking_info,
                    contract_staker_reward,
                );
                // accumulate rewards for the developer
                reward_for_developer += contract_developer_reward;

                // store current record in case next era has no record of changed stake amount
                contract_staking_info_prev = contract_staking_info;
            }
            // send rewards to stakers
            Self::payout_stakers2(&rewards_for_stakers_map);
            // send rewards to developer
            T::Currency::deposit_into_existing(&developer, reward_for_developer).ok();

            // Remove all previous records of staking for this contract,
            // they have already been processed and won't be needed anymore.
            ContractEraStake::<T>::remove_prefix(&contract_id, None);
            // create contract era stake data in current era for further staking and claiming
            ContractEraStake::<T>::insert(&contract_id, current_era, contract_staking_info_prev);

            // move contract pointers to current era
            ContractLastClaimed::<T>::insert(&contract_id, current_era);
            ContractLastStaked::<T>::insert(&contract_id, current_era);

            Self::deposit_event(Event::<T>::ContractClaimed(
                contract_id,
                claimer,
                start_from_era,
                current_era,
            ));

            Ok(().into())
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
        /// Update the ledger for a staker. This will also update the stash lock.
        /// This lock will lock the entire funds except paying for further transactions.
        fn update_ledger(
            staker: &T::AccountId,
            ledger: &StakingLedger<T::AccountId, BalanceOf<T>>,
        ) {
            if ledger.unlocking.is_empty() && ledger.active.is_zero() {
                Ledger::<T>::remove(&staker);
                T::Currency::remove_lock(STAKING_ID, &staker);
            } else {
                T::Currency::set_lock(STAKING_ID, &staker, ledger.total, WithdrawReasons::all());
                Ledger::<T>::insert(staker, ledger);
            }
        }

        /// Checks if there is a valid smart contract for the provided address
        fn is_contract_valid(address: &SmartContract<T::AccountId>) -> bool {
            match address {
                SmartContract::Wasm(_account) => {
                    //     <pallet_contracts::ContractInfoOf<T>>::get(&account).is_some()
                    false
                }
                SmartContract::Evm(_account) => {
                    // pallet_evm::Module::<T>::account_codes(&account).len() > 0 TODO remove comment after EVM mege
                    true
                }
            }
        }

        /// Calculate rewards for all stakers for this era
        fn stakers_era_reward(
            staker_map: &mut BTreeMap<T::AccountId, BalanceOf<T>>,
            points: &EraStakingPoints<T::AccountId, BalanceOf<T>>,
            reward_for_contract: BalanceOf<T>,
        ) {
            let staker_part = Perbill::from_rational(reward_for_contract, (*points).total);

            for (s, b) in &points.stakers {
                let reward = staker_map.entry(s.clone()).or_insert(Default::default());
                *reward += staker_part * *b;
            }
        }

        /// Execute payout for stakers
        fn payout_stakers2(staker_map: &BTreeMap<T::AccountId, BalanceOf<T>>) {
            for (s, b) in staker_map {
                T::Currency::deposit_into_existing(&s, *b).ok();
            }
        }

        /// Getter for the current era which also takes care of returning zero if no era was set yet.
        fn get_current_era() -> EraIndex {
            Self::current_era().unwrap_or(Zero::zero())
        }

        /// The block rewards are accumulated on the pallets's account during an era.
        /// This function takes a snapshot of the pallet's balance accured during current era
        /// and stores it for future distribution
        ///
        /// This is called at the end of each Era
        fn reward_balance_snapshoot(current_era: EraIndex) {
            let reward = Perbill::from_percent(T::DAppsRewardPercentage::get())
                * Self::block_reward_accumulator();
            // copy amount staked from previous era 'reward_and_stake.staked'
            let mut reward_and_stake =
                Self::era_reward_and_stake(current_era).unwrap_or(Default::default());
            // add reward amount to the current (which is just ending) era
            reward_and_stake.rewards = reward;

            EraRewardsAndStakes::<T>::insert(current_era, reward_and_stake);
        }
    }
}
