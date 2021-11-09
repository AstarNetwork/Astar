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
    traits::{AccountIdConversion, Saturating, Zero},
    Perbill,
};
use sp_std::convert::From;

const STAKING_ID: LockIdentifier = *b"dapstake";

pub(crate) const REWARD_SCALING: u32 = 2;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    /// The balance type of this pallet.
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    // Negative imbalance type of this pallet.
    type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::NegativeImbalance;

    impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for Pallet<T> {
        fn on_nonzero_unbalanced(block_reward: NegativeImbalanceOf<T>) {
            BlockRewardAccumulator::<T>::mutate(|accumulated_reward| {
                *accumulated_reward = accumulated_reward.saturating_add(block_reward.peek());
            });
            T::Currency::resolve_creating(&Self::account_id(), block_reward);
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
        type DeveloperRewardPercentage: Get<Perbill>;

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

        /// Number of eras of doubled claim rewards.
        #[pallet::constant]
        type BonusEraDuration: Get<u32>;

        /// Dapps staking pallet Id
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Minimum amount that should be left on staker account after staking.
        #[pallet::constant]
        type MinimumRemainingAmount: Get<BalanceOf<Self>>;

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

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
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
        /// Reward paid to staker or developer.
        Reward(T::AccountId, T::SmartContract, EraIndex, BalanceOf<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Can not stake with zero value.
        StakingWithNoValue,
        /// Can not stake with value less than minimum staking value
        InsufficientValue,
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
        /// This account was already used to register contract
        AlreadyUsedDeveloperAccount,
        /// Smart contract not owned by the account id.
        NotOwnedContract,
        /// Report issue on github if this is ever emitted
        UnknownEraReward,
        /// Contract hasn't been staked on in this era.
        NotStaked,
        /// Contract already claimed in this era and reward is distributed
        AlreadyClaimedInThisEra,
        /// Era parameter is out of bounds
        EraOutOfBounds,
        /// To register a contract, pre-approval is needed for this address
        RequiredContractPreApproval,
        /// Developer's account is already part of pre-approved list
        AlreadyPreApprovedDeveloper,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let force_new_era = Self::force_era().eq(&Forcing::ForceNew);
            let blocks_per_era = T::BlockPerEra::get();
            let previous_era = Self::current_era();

            // Value is compared to 1 since genesis block is ignored
            if now % blocks_per_era == BlockNumberFor::<T>::from(1u32)
                || force_new_era
                || previous_era.is_zero()
            {
                let next_era = previous_era + 1;
                CurrentEra::<T>::put(next_era);

                let reward = BlockRewardAccumulator::<T>::take();
                Self::reward_balance_snapshoot(previous_era, reward);

                if force_new_era {
                    ForceEra::<T>::put(Forcing::ForceNone);
                }

                Self::deposit_event(Event::<T>::NewDappStakingEra(next_era));
            }

            T::DbWeight::get().writes(5)
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
                Error::<T>::AlreadyUsedDeveloperAccount,
            );
            ensure!(
                !RegisteredDapps::<T>::contains_key(&contract_id),
                Error::<T>::AlreadyRegisteredContract,
            );
            ensure!(contract_id.is_valid(), Error::<T>::ContractIsNotValid);

            if Self::pre_approval_is_enabled() {
                ensure!(
                    PreApprovedDevelopers::<T>::contains_key(&developer),
                    Error::<T>::RequiredContractPreApproval,
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
        /// Warning: After this action contract can not be assigned again.
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
                Error::<T>::NotOwnedContract,
            );

            // We need to unstake all funds that are currently staked
            let current_era = Self::current_era();
            let staking_info = Self::staking_info(&contract_id, current_era);
            for (staker, amount) in staking_info.stakers.iter() {
                let ledger = Self::ledger(staker);
                Self::update_ledger(staker, ledger.saturating_sub(*amount));
            }

            // Need to update total amount staked
            let staking_total = staking_info.total;
            EraRewardsAndStakes::<T>::mutate(
                &current_era,
                // XXX: RewardsAndStakes should be set by `on_initialize` for each era
                |value| {
                    if let Some(x) = value {
                        x.staked = x.staked.saturating_sub(staking_total)
                    }
                },
            );

            // Nett to update staking data for next era
            let empty_staking_info = EraStakingPoints::<T::AccountId, BalanceOf<T>>::default();
            ContractEraStake::<T>::insert(contract_id.clone(), current_era, empty_staking_info);

            // Developer account released but contract can not be released more.
            T::Currency::unreserve(&developer, T::RegisterDeposit::get());
            RegisteredDevelopers::<T>::remove(&developer);

            Self::deposit_event(Event::<T>::ContractRemoved(developer, contract_id));

            let number_of_stakers = staking_info.stakers.len();
            Ok(Some(T::WeightInfo::unregister(number_of_stakers as u32)).into())
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
        #[pallet::weight(T::WeightInfo::bond_and_stake())]
        pub fn bond_and_stake(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let staker = ensure_signed(origin)?;

            // Check that contract is ready for staking.
            ensure!(
                Self::is_active(&contract_id),
                Error::<T>::NotOperatedContract
            );

            // Get the staking ledger or create an entry if it doesn't exist.
            let mut ledger = Self::ledger(&staker);

            // Ensure that staker has enough balance to bond & stake.
            let free_balance =
                T::Currency::free_balance(&staker).saturating_sub(T::MinimumRemainingAmount::get());

            // Remove already locked funds from the free balance
            let available_balance = free_balance.saturating_sub(ledger);
            let value_to_stake = value.min(available_balance);
            ensure!(!value_to_stake.is_zero(), Error::<T>::StakingWithNoValue);

            // update the ledger value by adding the newly bonded funds
            ledger += value_to_stake;

            // Get the latest era staking point info or create it if contract hasn't been staked yet so far.
            let current_era = Self::current_era();
            let mut staking_info = Self::staking_info(&contract_id, current_era);

            // Ensure that we can add additional staker for the contract.
            if !staking_info.stakers.contains_key(&staker) {
                ensure!(
                    staking_info.stakers.len() < T::MaxNumberOfStakersPerContract::get() as usize,
                    Error::<T>::MaxNumberOfStakersExceeded,
                );
            }

            // Increment total staked amount.
            staking_info.total += value_to_stake;

            // Increment personal staking amount.
            let entry = staking_info.stakers.entry(staker.clone()).or_default();
            *entry += value_to_stake;

            ensure!(
                *entry >= T::MinimumStakingAmount::get(),
                Error::<T>::InsufficientValue,
            );

            // Update total staked value in era.
            EraRewardsAndStakes::<T>::mutate(&current_era, |value| {
                if let Some(x) = value {
                    x.staked = x.staked.saturating_add(value_to_stake)
                }
            });

            // Update ledger and payee
            Self::update_ledger(&staker, ledger);

            // Update staked information for contract in current era
            ContractEraStake::<T>::insert(contract_id.clone(), current_era, staking_info);

            Self::deposit_event(Event::<T>::BondAndStake(
                staker,
                contract_id,
                value_to_stake,
            ));
            Ok(Some(T::WeightInfo::bond_and_stake()).into())
        }

        /// Unbond, unstake and withdraw balance from the contract.
        ///
        /// Value will be unlocked for the user.
        ///
        /// In case remaining staked balance on contract is below minimum staking amount,
        /// entire stake for that contract will be unstaked.
        ///
        #[pallet::weight(T::WeightInfo::unbond_unstake_and_withdraw())]
        pub fn unbond_unstake_and_withdraw(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let staker = ensure_signed(origin)?;

            ensure!(value > Zero::zero(), Error::<T>::UnstakingWithNoValue);
            ensure!(
                Self::is_active(&contract_id),
                Error::<T>::NotOperatedContract,
            );

            // Get the latest era staking points for the contract.
            let current_era = Self::current_era();
            let mut staking_info = Self::staking_info(&contract_id, current_era);

            ensure!(
                staking_info.stakers.contains_key(&staker),
                Error::<T>::NotStakedContract,
            );
            let staked_value = staking_info.stakers[&staker];

            ensure!(value <= staked_value, Error::<T>::InsufficientValue);

            // Calculate the value which will be unstaked.
            let remaining = staked_value.saturating_sub(value);
            let value_to_unstake = if remaining < T::MinimumStakingAmount::get() {
                staking_info.stakers.remove(&staker);
                staked_value
            } else {
                staking_info.stakers.insert(staker.clone(), remaining);
                value
            };

            // Get the staking ledger and update it
            let ledger = Self::ledger(&staker);
            Self::update_ledger(&staker, ledger.saturating_sub(value_to_unstake));

            // Update total staked value in era.
            EraRewardsAndStakes::<T>::mutate(&current_era, |value| {
                if let Some(x) = value {
                    x.staked = x.staked.saturating_sub(value_to_unstake)
                }
            });

            // Update the era staking points
            staking_info.total = staking_info.total.saturating_sub(value_to_unstake);
            ContractEraStake::<T>::insert(contract_id.clone(), current_era, staking_info);

            Self::deposit_event(Event::<T>::UnbondUnstakeAndWithdraw(
                staker,
                contract_id,
                value_to_unstake,
            ));

            Ok(Some(T::WeightInfo::unbond_unstake_and_withdraw()).into())
        }

        /// claim the rewards earned by contract_id.
        /// All stakers and developer for this contract will be paid out with single call.
        /// claim is valid for all unclaimed eras but not longer than history_depth().
        /// Any reward older than history_depth() will go to Treasury.
        /// Any user can call this function.
        #[pallet::weight(T::WeightInfo::claim(T::MaxNumberOfStakersPerContract::get() + 1))]
        pub fn claim(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            era: EraIndex,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;

            let developer =
                RegisteredDapps::<T>::get(&contract_id).ok_or(Error::<T>::NotOperatedContract)?;

            let current_era = Self::current_era();
            let era_low_bound = current_era.saturating_sub(T::HistoryDepth::get());

            ensure!(
                era < current_era && era >= era_low_bound,
                Error::<T>::EraOutOfBounds,
            );

            let mut staking_info = Self::staking_info(&contract_id, era);

            ensure!(
                staking_info.claimed_rewards.is_zero(),
                Error::<T>::AlreadyClaimedInThisEra,
            );

            ensure!(!staking_info.stakers.is_empty(), Error::<T>::NotStaked,);

            let reward_and_stake =
                Self::era_reward_and_stake(era).ok_or(Error::<T>::UnknownEraReward)?;

            // Calculate the contract reward for this era.
            let reward_ratio = Perbill::from_rational(staking_info.total, reward_and_stake.staked);
            let contract_reward = if era < T::BonusEraDuration::get() {
                // Double reward as a bonus.
                reward_ratio * reward_and_stake.rewards * REWARD_SCALING.into()
            } else {
                reward_ratio * reward_and_stake.rewards
            };

            // Withdraw reward funds from the dapps staking
            let reward_pool = T::Currency::withdraw(
                &Self::account_id(),
                contract_reward,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::AllowDeath,
            )?;

            // Divide reward between stakers and the developer of the contract
            let (developer_reward, mut stakers_reward) =
                reward_pool.split(T::DeveloperRewardPercentage::get() * contract_reward);

            Self::deposit_event(Event::<T>::Reward(
                developer.clone(),
                contract_id.clone(),
                era,
                developer_reward.peek(),
            ));
            T::Currency::resolve_creating(&developer, developer_reward);

            // Calculate & pay rewards for all stakers
            let stakers_total_reward = stakers_reward.peek();
            for (staker, staked_balance) in &staking_info.stakers {
                let ratio = Perbill::from_rational(*staked_balance, staking_info.total);
                let (reward, new_stakers_reward) =
                    stakers_reward.split(ratio * stakers_total_reward);
                stakers_reward = new_stakers_reward;

                Self::deposit_event(Event::<T>::Reward(
                    staker.clone(),
                    contract_id.clone(),
                    era,
                    reward.peek(),
                ));
                T::Currency::resolve_creating(staker, reward);
            }

            let number_of_payees = staking_info.stakers.len() + 1;

            // updated counter for total rewards paid to the contract
            staking_info.claimed_rewards = contract_reward;
            <ContractEraStake<T>>::insert(&contract_id, era, staking_info);

            Ok(Some(T::WeightInfo::claim(number_of_payees as u32)).into())
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
    }

    impl<T: Config> Pallet<T> {
        /// Get AccountId assigned to the pallet.
        fn account_id() -> T::AccountId {
            T::PalletId::get().into_account()
        }

        /// Update the ledger for a staker. This will also update the stash lock.
        /// This lock will lock the entire funds except paying for further transactions.
        fn update_ledger(staker: &T::AccountId, ledger: BalanceOf<T>) {
            if ledger.is_zero() {
                Ledger::<T>::remove(&staker);
                T::Currency::remove_lock(STAKING_ID, &staker);
            } else {
                T::Currency::set_lock(STAKING_ID, &staker, ledger, WithdrawReasons::all());
                Ledger::<T>::insert(staker, ledger);
            }
        }

        /// The block rewards are accumulated on the pallets's account during an era.
        /// This function takes a snapshot of the pallet's balance accrued during current era
        /// and stores it for future distribution
        ///
        /// This is called just at the beginning of an era.
        fn reward_balance_snapshoot(era: EraIndex, reward: BalanceOf<T>) {
            // Get the reward and stake information for previous era
            let mut reward_and_stake = Self::era_reward_and_stake(era).unwrap_or_default();

            // Prepare info for the next era
            EraRewardsAndStakes::<T>::insert(
                era + 1,
                EraRewardAndStake {
                    rewards: Zero::zero(),
                    staked: reward_and_stake.staked.clone(),
                },
            );

            // Set the reward for the previous era.
            reward_and_stake.rewards = reward;
            EraRewardsAndStakes::<T>::insert(era, reward_and_stake);
        }

        /// This helper returns `EraStakingPoints` for given era if possible or latest stored data
        /// or finally default value if storage have no data for it.
        pub(crate) fn staking_info(
            contract_id: &T::SmartContract,
            era: EraIndex,
        ) -> EraStakingPoints<T::AccountId, BalanceOf<T>> {
            if let Some(staking_info) = ContractEraStake::<T>::get(contract_id, era) {
                staking_info
            } else {
                let avail_era = ContractEraStake::<T>::iter_key_prefix(&contract_id)
                    .filter(|x| *x <= era)
                    .max()
                    .unwrap_or(Zero::zero());

                let mut staking_points =
                    ContractEraStake::<T>::get(contract_id, avail_era).unwrap_or_default();
                // Needs to be reset since otherwise it might seem as if rewards were already claimed for this era.
                staking_points.claimed_rewards = Zero::zero();
                staking_points
            }
        }

        /// Check that contract have active developer linkage.
        fn is_active(contract_id: &T::SmartContract) -> bool {
            if let Some(developer) = RegisteredDapps::<T>::get(contract_id) {
                if let Some(r_contract_id) = RegisteredDevelopers::<T>::get(&developer) {
                    return r_contract_id == *contract_id;
                }
            }
            false
        }
    }
}
