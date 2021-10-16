//! Dapps staking FRAME Pallet.

use super::*;
use frame_support::{
    dispatch::DispatchResult,
    ensure,
    pallet_prelude::*,
    traits::{
        Currency, ExistenceRequirement, Get, Imbalance, LockIdentifier, LockableCurrency,
        OnUnbalanced, ReservableCurrency, WithdrawReasons,
    },
    weights::Weight,
    PalletId,
};
use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
use sp_runtime::{
    print,
    traits::{AccountIdConversion, Saturating, Zero},
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
    // Negative imbalance type of this pallet.
    type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::NegativeImbalance;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for Pallet<T> {
        fn on_nonzero_unbalanced(block_reward: NegativeImbalanceOf<T>) {
            BlockRewardAccumulator::<T>::mutate(|accumulated_reward| {
                *accumulated_reward = accumulated_reward.saturating_add(block_reward.peek());
            });
            T::Currency::resolve_creating(&T::PalletId::get().into_account(), block_reward);
        }
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The staking balance.
        type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>
            + ReservableCurrency<Self::AccountId>;

        // type used for Accounts on EVM and on Substrate
        type SmartContract: IsContract + Parameter + Member;

        /// Number of blocks per era.
        #[pallet::constant]
        type BlockPerEra: Get<BlockNumberFor<Self>>;

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

        /// Number of eras that are valid when claiming rewards.
        ///
        /// All the rest will be either claimed by the treasury or discarded.
        #[pallet::constant]
        type HistoryDepth: Get<u32>;

        /// Dapps staking pallet Id
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Treasury pallet Id
        #[pallet::constant]
        type TreasuryPalletId: Get<PalletId>;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    /// Bonded amount for the staker
    #[pallet::storage]
    #[pallet::getter(fn ledger)]
    pub(crate) type Ledger<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    /// The current era index.
    #[pallet::storage]
    #[pallet::getter(fn current_era)]
    pub type CurrentEra<T> = StorageValue<_, EraIndex, ValueQuery>;

    /// Accumulator for block rewards during an era. It is reset at every new era
    #[pallet::storage]
    #[pallet::getter(fn block_reward_accumulator)]
    pub type BlockRewardAccumulator<T> = StorageValue<_, BalanceOf<T>, ValueQuery>;

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
        StorageMap<_, Blake2_128Concat, T::AccountId, T::SmartContract>;

    /// Registered dapp points to the developer who registered it
    #[pallet::storage]
    #[pallet::getter(fn registered_developer)]
    pub(crate) type RegisteredDapps<T: Config> =
        StorageMap<_, Blake2_128Concat, T::SmartContract, T::AccountId>;

    /// Total block rewards for the pallet per era and total staked funds
    #[pallet::storage]
    #[pallet::getter(fn era_reward_and_stake)]
    pub(crate) type EraRewardsAndStakes<T: Config> =
        StorageMap<_, Twox64Concat, EraIndex, EraRewardAndStake<BalanceOf<T>>>;

    /// Reward counter for individual stakers and the developer
    #[pallet::storage]
    #[pallet::getter(fn rewards_claimed)]
    pub(crate) type RewardsClaimed<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::SmartContract,
        Blake2_128Concat,
        T::AccountId,
        BalanceOf<T>,
        ValueQuery,
    >;

    /// Stores amount staked and stakers for a contract per era
    #[pallet::storage]
    #[pallet::getter(fn contract_era_stake)]
    pub(crate) type ContractEraStake<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::SmartContract,
        Twox64Concat,
        EraIndex,
        EraStakingPoints<T::AccountId, BalanceOf<T>>,
    >;

    /// Marks an Era when a contract is last claimed
    #[pallet::storage]
    #[pallet::getter(fn contract_last_claimed)]
    pub(crate) type ContractLastClaimed<T: Config> =
        StorageMap<_, Blake2_128Concat, T::SmartContract, EraIndex>;

    /// Marks an Era when a contract is last (un)staked
    #[pallet::storage]
    #[pallet::getter(fn contract_last_staked)]
    pub(crate) type ContractLastStaked<T: Config> =
        StorageMap<_, Blake2_128Concat, T::SmartContract, EraIndex>;

    #[pallet::type_value]
    pub(crate) fn PreApprovalOnEmpty() -> bool {
        false
    }
    /// Enable or disable pre-approval list for new contract registration
    #[pallet::storage]
    #[pallet::getter(fn pre_approval_is_enabled)]
    pub(crate) type PreApprovalIsEnabled<T> = StorageValue<_, bool, ValueQuery, PreApprovalOnEmpty>;

    /// List of pre-approved developers
    #[pallet::storage]
    #[pallet::getter(fn pre_approved_developers)]
    pub(crate) type PreApprovedDevelopers<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, (), ValueQuery>;

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
        /// Account has bonded and staked funds on a smart contract.
        BondAndStake(T::AccountId, T::SmartContract, BalanceOf<T>),
        /// Account has unbonded, unstaked and withdrawn funds.
        UnbondUnstakeAndWithdraw(T::AccountId, T::SmartContract, BalanceOf<T>),
        /// New contract added for staking.
        NewContract(T::AccountId, T::SmartContract),
        /// Contract removed from dapps staking.
        ContractRemoved(T::AccountId, T::SmartContract),
        /// New dapps staking era. Distribute era rewards to contracts.
        NewDappStakingEra(EraIndex),
        /// The contract's reward have been claimed, by an account, from era, until era.
        ContractClaimed(T::SmartContract, T::AccountId, EraIndex, EraIndex),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Can not stake with zero value.
        StakingWithNoValue,
        /// Can not stake with value less than minimum staking value
        InsufficientStakingValue,
        /// Number of stakers per contract exceeded.
        MaxNumberOfStakersExceeded,
        /// Targets must be operated contracts
        NotOperatedContract,
        /// Contract isn't staked.
        NotStakedContract,
        /// Unstaking a contract with zero value
        UnstakingWithNoValue,
        /// The contract is already registered by other account
        AlreadyRegisteredContract,
        /// User attempts to register with address which is not contract
        ContractIsNotValid,
        /// Contract not registered for dapps staking.
        ContractNotRegistered,
        /// This account was already used to register contract
        AlreadyUsedDeveloperAccount,
        /// Smart contract not owned by the account id.
        NotOwnedContract,
        /// Unexpected state error, used to abort transaction. Used for situations that 'should never happen'.
        UnexpectedState,
        /// Report issue on github if this is ever emitted
        UnknownStartStakingData,
        /// Report issue on github if this is ever emitted
        UnknownEraReward,
        /// There are no funds to reward the contract. Or already claimed in that era
        NothingToClaim,
        /// Contract already claimed in this era and reward is distributed
        AlreadyClaimedInThisEra,
        /// To register a contract, pre-approval is needed for this address
        RequiredContractPreApproval,
        /// Developer's account is already part of pre-approved list
        AlreadyPreApprovedDeveloper,
        /// Contract rewards haven't been claimed prior to unregistration
        ContractRewardsNotClaimed,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let force_new_era = Self::force_era().eq(&Forcing::ForceNew);
            let blocks_per_era = T::BlockPerEra::get();

            // Value is compared to 1 since genesis block is ignored
            if now % blocks_per_era == BlockNumberFor::<T>::from(1u32) || force_new_era {
                let previous_era = Self::current_era();
                Self::reward_balance_snapshoot(previous_era);
                let next_era = previous_era + 1;
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
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// register contract into staking targets.
        /// contract_id should be ink! or evm contract.
        ///
        /// Any user can call this function.
        /// However, caller have to have deposit amount.
        #[pallet::weight(T::WeightInfo::register())]
        pub fn register(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
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

            ensure!(contract_id.is_valid(), Error::<T>::ContractIsNotValid);
            if Self::pre_approval_is_enabled() {
                ensure!(
                    PreApprovedDevelopers::<T>::contains_key(&developer),
                    Error::<T>::RequiredContractPreApproval
                );
            }

            T::Currency::reserve(&developer, T::RegisterDeposit::get())?;

            RegisteredDapps::<T>::insert(contract_id.clone(), developer.clone());
            RegisteredDevelopers::<T>::insert(&developer, contract_id.clone());

            Self::deposit_event(Event::<T>::NewContract(developer, contract_id));

            Ok(().into())
        }

        /// Unregister existing contract from dapps staking
        ///
        /// This must be called by the developer who registered the contract.
        ///
        /// No unclaimed rewards must remain prior to this call (otherwise it will fail).
        /// Make sure to claim all the contract rewards prior to unregistering it.
        #[pallet::weight(T::WeightInfo::unregister(T::MaxNumberOfStakersPerContract::get()))]
        pub fn unregister(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            let developer = ensure_signed(origin)?;

            let registered_contract =
                RegisteredDevelopers::<T>::get(&developer).ok_or(Error::<T>::NotOwnedContract)?;

            // This is a sanity check for the unregistration since it requires the caller
            // to input the correct contract address.
            ensure!(
                registered_contract == contract_id,
                Error::<T>::NotOwnedContract
            );

            let current_era = Self::current_era();
            // in case contract wasn't staked yet
            let last_claim_era =
                Self::contract_last_claimed(&contract_id).unwrap_or(current_era.clone());

            // Ensure that all contract rewards have been claimed prior to unregistration.
            ensure!(
                last_claim_era == current_era,
                Error::<T>::ContractRewardsNotClaimed
            );

            // We need to unstake all funds that are currently staked
            let era_staking_points =
                ContractEraStake::<T>::take(&contract_id, &current_era).unwrap_or_default();
            let number_of_stakers = era_staking_points.stakers.len();
            let mut unstake_accumulator = BalanceOf::<T>::zero();
            for (staker_id, amount) in era_staking_points.stakers.iter() {
                let mut ledger = Self::ledger(staker_id);
                ledger = ledger.saturating_sub(*amount);
                Self::update_ledger(staker_id, &ledger);

                // increment total unstake value accumulator
                unstake_accumulator += *amount;
            }

            // Need to update total amount staked
            let mut reward_and_stake =
                Self::era_reward_and_stake(&current_era).ok_or(Error::<T>::UnexpectedState)?;
            reward_and_stake.staked = reward_and_stake.staked.saturating_sub(unstake_accumulator);
            EraRewardsAndStakes::<T>::insert(&current_era, reward_and_stake);

            // Continue with cleanup
            T::Currency::unreserve(&developer, T::RegisterDeposit::get());
            RegisteredDapps::<T>::remove(&contract_id);
            RegisteredDevelopers::<T>::remove(&developer);
            ContractLastStaked::<T>::remove(&contract_id);
            ContractLastClaimed::<T>::remove(&contract_id);
            RewardsClaimed::<T>::remove_prefix(&contract_id, None);

            Self::deposit_event(Event::<T>::ContractRemoved(developer, contract_id));

            Ok(Some(T::WeightInfo::unregister(number_of_stakers as u32)).into())
        }

        /// add contract address to the pre-approved list.
        /// contract_id should be ink! or evm contract.
        ///
        /// Sudo call is required
        #[pallet::weight(T::WeightInfo::developer_pre_approval())]
        pub fn developer_pre_approval(
            origin: OriginFor<T>,
            developer: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            ensure!(
                !PreApprovedDevelopers::<T>::contains_key(&developer),
                Error::<T>::AlreadyPreApprovedDeveloper
            );
            PreApprovedDevelopers::<T>::insert(developer, ());

            Ok(().into())
        }

        /// Enable or disable adding new contracts to the pre-approved list
        ///
        /// Sudo call is required
        #[pallet::weight(T::WeightInfo::enable_developer_pre_approval())]
        pub fn enable_developer_pre_approval(
            origin: OriginFor<T>,
            enabled: bool,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            PreApprovalIsEnabled::<T>::put(enabled);

            Ok(().into())
        }

        /// Lock up and stake balance of the origin account.
        ///
        /// `value` must be more than the `minimum_balance` specified by `T::Currency`
        /// unless account already has bonded value equal or more than 'minimum_balance'.
        ///
        /// The dispatch origin for this call must be _Signed_ by the staker's account.
        ///
        /// Effects of staking will be felt at the beginning of the next era.
        ///
        #[pallet::weight(T::WeightInfo::bond_and_stake(T::MaxNumberOfStakersPerContract::get()))]
        pub fn bond_and_stake(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let staker = ensure_signed(origin)?;
            ensure!(
                RegisteredDapps::<T>::contains_key(&contract_id),
                Error::<T>::NotOperatedContract
            );

            // Get the staking ledger or create an entry if it doesn't exist.
            let mut ledger = Self::ledger(&staker);

            // Ensure that staker has enough balance to bond & stake.
            let free_balance = T::Currency::free_balance(&staker);
            // Remove already locked funds from the free balance
            let available_balance = free_balance.saturating_sub(ledger);
            let value_to_stake = value.min(available_balance);
            ensure!(!value_to_stake.is_zero(), Error::<T>::StakingWithNoValue);

            // update the ledger value by adding the newly bonded funds
            ledger += value_to_stake;

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
                        former_staked_era: 0 as EraIndex,
                        claimed_rewards: BalanceOf::<T>::default(),
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
            latest_era_staking_points.total += value_to_stake;
            let entry = latest_era_staking_points
                .stakers
                .entry(staker.clone())
                .or_insert(Zero::zero());
            *entry += value_to_stake;

            ensure!(
                *entry >= T::MinimumStakingAmount::get(),
                Error::<T>::InsufficientStakingValue
            );

            let current_era = Self::current_era();

            // Update total staked value in era.
            let mut reward_and_stake_for_era =
                Self::era_reward_and_stake(current_era).ok_or(Error::<T>::UnexpectedState)?;
            reward_and_stake_for_era.staked += value_to_stake;
            EraRewardsAndStakes::<T>::insert(current_era, reward_and_stake_for_era);

            // Update ledger and payee
            Self::update_ledger(&staker, &ledger);

            // Update staked information for contract in current era
            if let Some(last_staked_era) = era_when_contract_last_staked.clone() {
                latest_era_staking_points.former_staked_era = last_staked_era;
            } else {
                latest_era_staking_points.former_staked_era = current_era;
            }
            let number_of_stakers = latest_era_staking_points.stakers.len();
            ContractEraStake::<T>::insert(
                contract_id.clone(),
                current_era,
                latest_era_staking_points,
            );

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

            Self::deposit_event(Event::<T>::BondAndStake(
                staker,
                contract_id,
                value_to_stake,
            ));

            Ok(Some(T::WeightInfo::bond_and_stake(number_of_stakers as u32)).into())
        }

        /// Unbond, unstake and withdraw balance from the contract.
        ///
        /// Value will be unlocked for the user.
        ///
        /// In case remaining staked balance on contract is below minimum staking amount,
        /// entire stake for that contract will be unstaked.
        ///
        #[pallet::weight(T::WeightInfo::unbond_unstake_and_withdraw(
            T::MaxNumberOfStakersPerContract::get()
        ))]
        pub fn unbond_unstake_and_withdraw(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let staker = ensure_signed(origin)?;
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
            let number_of_stakers = era_staking_points.stakers.len();

            // Calculate the value which will be unstaked.
            let mut value_to_unstake = value.min(staked_value);
            let remaining_staked_value = staked_value.saturating_sub(value_to_unstake);
            if remaining_staked_value < T::MinimumStakingAmount::get() {
                // if staked value would fall below threshold, unstake everything
                era_staking_points.stakers.remove(&staker);
                value_to_unstake = staked_value;
                // remove reward counter
                RewardsClaimed::<T>::remove(&contract_id, &staker);
            } else {
                era_staking_points
                    .stakers
                    .insert(staker.clone(), remaining_staked_value);
            }
            let value_to_unstake = value_to_unstake; // make it immutable
            era_staking_points.total = era_staking_points.total.saturating_sub(value_to_unstake);

            let current_era = Self::current_era();
            if current_era != era_when_contract_last_staked {
                era_staking_points.former_staked_era = era_when_contract_last_staked;
            }

            // Get the staking ledger and update it
            let mut ledger = Self::ledger(&staker);
            ledger = ledger.saturating_sub(value_to_unstake);
            Self::update_ledger(&staker, &ledger);

            // Update the era staking points
            ContractEraStake::<T>::insert(contract_id.clone(), current_era, era_staking_points);

            // Update total staked value in era.
            let mut era_reward_and_stake =
                Self::era_reward_and_stake(current_era).ok_or(Error::<T>::UnexpectedState)?;
            era_reward_and_stake.staked =
                era_reward_and_stake.staked.saturating_sub(value_to_unstake);
            EraRewardsAndStakes::<T>::insert(current_era, era_reward_and_stake);

            // Check if we need to update era in which contract was last changed. Can avoid one write.
            if era_when_contract_last_staked != current_era {
                ContractLastStaked::<T>::insert(&contract_id, current_era);
            }

            Self::deposit_event(Event::<T>::UnbondUnstakeAndWithdraw(
                staker,
                contract_id,
                value_to_unstake,
            ));

            Ok(Some(T::WeightInfo::unbond_unstake_and_withdraw(
                number_of_stakers as u32,
            ))
            .into())
        }

        /// claim the rewards earned by contract_id.
        /// All stakers and developer for this contract will be paid out with single call.
        /// claim is valid for all unclaimed eras but not longer than history_depth().
        /// Any reward older than history_depth() will go to Treasury.
        /// Any user can call this function.
        #[pallet::weight(T::WeightInfo::claim(
            T::MaxNumberOfStakersPerContract::get() * T::HistoryDepth::get() + 1,
            Pallet::<T>::get_unclaimed_reward_history_limit()
        ))]
        pub fn claim(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            let claimer = ensure_signed(origin)?;

            // check if this contract is registered
            let developer = Self::registered_developer(&contract_id)
                .ok_or(Error::<T>::ContractNotRegistered)?;

            // check if it was ever staked on this contract.
            let last_staked_era =
                Self::contract_last_staked(&contract_id).ok_or(Error::<T>::NothingToClaim)?;

            // check if the contract is already claimed in this era
            let current_era = Self::current_era();
            let last_claim_era =
                Self::contract_last_claimed(&contract_id).unwrap_or(current_era.clone());
            ensure!(
                current_era != last_claim_era,
                Error::<T>::AlreadyClaimedInThisEra
            );

            // oldest era to start with collecting rewards for devs and stakers
            let last_allowed_era = current_era.saturating_sub(T::HistoryDepth::get());

            // initialize rewards for stakers, developer and unclaimed rewards accumulator
            let mut rewards_for_stakers_map: BTreeMap<T::AccountId, BalanceOf<T>> =
                Default::default();
            let mut reward_for_developer: BalanceOf<T> = Zero::zero();
            let mut rewards_for_stakers: BalanceOf<T> = Zero::zero();
            let mut sum_of_all_rewards: BalanceOf<T> = Zero::zero();

            // Next we iterate of periods between staking points.
            // Since each era staking point struct has information about the former era when staking information
            // was changed, we start from top and move to bottom.
            // E.g.:
            // [last_staked_era, current_era>,
            // [last_staked_era.former_stake_era, last_staked_era>,
            //  ...

            // This will be the oldest era we will look at. Others will just be discarded.
            let lowest_allowed_era =
                current_era.saturating_sub(Self::get_unclaimed_reward_history_limit());

            let mut number_of_era_staking_points = 1u32;
            let mut lower_bound_era = last_staked_era;
            let mut upper_bound_era = current_era;
            let mut contract_staking_info =
                Self::contract_era_stake(&contract_id, &lower_bound_era)
                    .ok_or(Error::<T>::UnknownStartStakingData)?;
            loop {
                // accumulate rewards for this period
                for era in lower_bound_era.max(lowest_allowed_era)..upper_bound_era {
                    let reward_and_stake_for_era =
                        Self::era_reward_and_stake(era).ok_or(Error::<T>::UnknownEraReward)?;

                    // Calculate the contract reward for this era.
                    let reward_particle = Perbill::from_rational(
                        contract_staking_info.total,
                        reward_and_stake_for_era.staked,
                    );
                    let contract_reward_in_era = reward_particle * reward_and_stake_for_era.rewards;

                    // Used to accumulate all rewards for this era
                    sum_of_all_rewards += contract_reward_in_era;

                    // First arm refers to situations where both dev and staker are eligible for rewards
                    if era >= last_allowed_era {
                        // divide reward between stakers and the developer of the contract
                        let contract_developer_reward =
                            Perbill::from_rational(T::DeveloperRewardPercentage::get() as u64, 100)
                                * contract_reward_in_era;
                        let contract_staker_reward =
                            contract_reward_in_era - contract_developer_reward;

                        // accumulate rewards for the stakers
                        Self::stakers_era_reward(
                            &mut rewards_for_stakers_map,
                            &contract_staking_info,
                            contract_staker_reward,
                        );
                        // accumulate rewards for the developer
                        reward_for_developer += contract_developer_reward;
                        rewards_for_stakers += contract_staker_reward;
                    }
                } // end of one era interval iteration

                upper_bound_era = lower_bound_era;
                lower_bound_era = contract_staking_info.former_staked_era;

                // Check if this is the last unprocessed era staking point. If it is, stop.
                // Also check if upper bound is below lowest allowed era and stop if needed.
                if lower_bound_era == upper_bound_era || upper_bound_era < lowest_allowed_era {
                    // update struct so it reflects that it's the last known staking point value
                    contract_staking_info.former_staked_era = current_era;
                    break;
                }

                number_of_era_staking_points += 1;
                contract_staking_info = Self::contract_era_stake(&contract_id, &lower_bound_era)
                    .ok_or(Error::<T>::UnknownStartStakingData)?;
                // continue and process the next era interval
            }

            // These two values will be used for weight calculation
            let number_of_payees = rewards_for_stakers_map.len() as u32 + 1;
            let number_of_era_staking_points = number_of_era_staking_points;

            // This accumulates rewards and is used when paying them out.
            let dapps_staking_account_id = T::PalletId::get().into_account();

            // Withdraw reward funds from the dapps staking treasury
            let reward_pool = T::Currency::withdraw(
                &dapps_staking_account_id,
                sum_of_all_rewards,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::AllowDeath,
            )?;

            // Split imbalance between developer, stakers and treasuy
            let (reward_for_developer, reward_pool) = reward_pool.split(reward_for_developer);
            let (rewards_for_stakers, reward_for_treasury) = reward_pool.split(rewards_for_stakers);

            // updated counter for total rewards paid to the contract
            contract_staking_info.claimed_rewards +=
                rewards_for_stakers.peek() + reward_for_developer.peek();

            // Payout stakers
            Self::payout_stakers(&contract_id, &rewards_for_stakers_map, rewards_for_stakers);

            // send rewards to developer's account and update reward counter for developer's account
            RewardsClaimed::<T>::mutate(&contract_id, &developer, |balance| {
                *balance += reward_for_developer.peek()
            });
            T::Currency::resolve_creating(&developer, reward_for_developer);

            // In case we have some rewards that weren't claimed in time, we transfer them to treasury.
            if !reward_for_treasury.peek().is_zero() {
                T::Currency::resolve_creating(
                    &T::TreasuryPalletId::get().into_account(),
                    reward_for_treasury,
                );
            }

            // Remove all previous records of staking for this contract,
            // they have already been processed and won't be needed anymore.
            ContractEraStake::<T>::remove_prefix(&contract_id, None);
            // create contract era stake data in current era for further staking and claiming
            ContractEraStake::<T>::insert(&contract_id, current_era, contract_staking_info);

            // move contract pointers to current era
            ContractLastClaimed::<T>::insert(&contract_id, current_era);
            ContractLastStaked::<T>::insert(&contract_id, current_era);

            Self::deposit_event(Event::<T>::ContractClaimed(
                contract_id,
                claimer,
                last_claim_era.max(last_allowed_era),
                current_era,
            ));

            Ok(Some(T::WeightInfo::claim(
                number_of_payees,
                number_of_era_staking_points,
            ))
            .into())
        }

        /// Force there to be a new era at the end of the next block. After this, it will be
        /// reset to normal (non-forced) behaviour.
        ///
        /// The dispatch origin must be Root.
        ///
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
    }

    impl<T: Config> Pallet<T> {
        /// Returns the value of history depth but in regards to how deep do we go when
        /// processing unclaimed rewards.
        ///
        /// This value is bound together with history depth (it must be greater or same) so
        /// we fetch it via a dedicated helper method
        pub(crate) fn get_unclaimed_reward_history_limit() -> EraIndex {
            T::HistoryDepth::get() * 2
        }

        /// Update the ledger for a staker. This will also update the stash lock.
        /// This lock will lock the entire funds except paying for further transactions.
        fn update_ledger(staker: &T::AccountId, ledger: &BalanceOf<T>) {
            if ledger.is_zero() {
                Ledger::<T>::remove(&staker);
                T::Currency::remove_lock(STAKING_ID, &staker);
            } else {
                T::Currency::set_lock(STAKING_ID, &staker, ledger.clone(), WithdrawReasons::all());
                Ledger::<T>::insert(staker, ledger);
            }
        }

        /// Calculate rewards for all stakers for this era
        fn stakers_era_reward(
            staker_map: &mut BTreeMap<T::AccountId, BalanceOf<T>>,
            points: &EraStakingPoints<T::AccountId, BalanceOf<T>>,
            reward_for_contract: BalanceOf<T>,
        ) {
            for (staker, staked_balance) in &points.stakers {
                let staker_part = Perbill::from_rational(*staked_balance, (*points).total);
                let reward = staker_map
                    .entry(staker.clone())
                    .or_insert(Default::default());
                *reward += staker_part * reward_for_contract;
            }
        }

        /// Execute payout for stakers.
        /// Return total rewards claimed by stakers on this contract.
        fn payout_stakers(
            contract: &T::SmartContract,
            staker_map: &BTreeMap<T::AccountId, BalanceOf<T>>,
            staker_reward_pool: NegativeImbalanceOf<T>,
        ) {
            let mut remainder_reward = staker_reward_pool;
            for (payee, reward) in staker_map {
                let (reward_imbalance, remainder_temp) = remainder_reward.split(*reward);
                remainder_reward = remainder_temp;

                RewardsClaimed::<T>::mutate(contract, payee, |balance| *balance += *reward);
                T::Currency::resolve_creating(payee, reward_imbalance);
            }
        }

        /// The block rewards are accumulated on the pallets's account during an era.
        /// This function takes a snapshot of the pallet's balance accrued during current era
        /// and stores it for future distribution
        ///
        /// This is called just at the beginning of an era.
        fn reward_balance_snapshoot(ending_era: EraIndex) {
            let reward = Self::block_reward_accumulator();

            // Get the reward and stake information for previous era
            let mut reward_and_stake = Self::era_reward_and_stake(ending_era).unwrap_or_default();

            // Prepare info for the next era
            EraRewardsAndStakes::<T>::insert(
                ending_era + 1,
                EraRewardAndStake {
                    rewards: Zero::zero(),
                    staked: reward_and_stake.staked.clone(),
                },
            );

            // Set the reward for the previous era.
            reward_and_stake.rewards = reward;
            EraRewardsAndStakes::<T>::insert(ending_era, reward_and_stake);
        }
    }
}
