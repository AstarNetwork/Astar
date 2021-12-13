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
    traits::{AccountIdConversion, CheckedAdd, Saturating, Zero},
    ArithmeticError, Perbill,
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
        type SmartContract: IsContract + Parameter + Member + Ord;

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

        /// Max number of unlocking chunks per account Id <-> contract Id pairing.
        /// If value is zero, unlocking becomes impossible.
        #[pallet::constant]
        type MaxUnlockingChunks: Get<u32>;

        /// Number of eras that need to pass until unstaked value can be withdrawn.
        /// If this value is zero, it's equivalent to having no unbonding period.
        #[pallet::constant]
        type UnbondingPeriod: Get<u32>;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    /// Bonded amount for the staker
    #[pallet::storage]
    #[pallet::getter(fn ledger)]
    pub(crate) type Ledger<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        AccountLedger<T::SmartContract, BalanceOf<T>>,
        ValueQuery,
    >;

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
        StorageMap<_, Blake2_128Concat, T::SmartContract, DeveloperInfo<T::AccountId>>;

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
        EraStakingPoints<BalanceOf<T>>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn staker_contract_era_stake)]
    pub(crate) type StakerContractEraInfo<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        (T::AccountId, T::SmartContract),
        Twox64Concat,
        EraIndex,
        StakerInfo<BalanceOf<T>>,
    >;

    /// Stores the current pallet storage version.
    #[pallet::storage]
    #[pallet::getter(fn storage_version)]
    pub(crate) type StorageVersion<T> = StorageValue<_, Version, ValueQuery>;

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
        /// Account has unbonded & unstaked some funds. Unbonding process begins.
        UnbondAndUnstake(T::AccountId, T::SmartContract, BalanceOf<T>),
        /// Account has withdrawn unbonded funds.
        Withdrawn(T::AccountId, BalanceOf<T>),
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
        /// Contract isn't unregistered.
        NotUnregisteredContract,
        /// Unstaking a contract with zero value
        UnstakingWithNoValue,
        /// There are no previously unbonded funds that can be unstaked and withdrawn.
        NothingToWithdraw,
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
        /// Contract has too many unlocking chunks. Withdraw the existing chunks if possible
        /// or wait for current chunks to complete unlocking process to withdraw them.
        TooManyUnlockingChunks,
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

            RegisteredDapps::<T>::insert(
                contract_id.clone(),
                DeveloperInfo::new(developer.clone()),
            );
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

            let mut dapp_info =
                RegisteredDapps::<T>::get(&contract_id).ok_or(Error::<T>::NotOperatedContract)?;
            // TODO: new error for unregistering unregistered contract? Seems like an overkill.
            ensure!(
                dapp_info.state == DAppState::Registered,
                Error::<T>::NotOperatedContract
            );
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
            let mut staking_info = Self::contract_staking_info(&contract_id, current_era);

            // Need to update total amount staked
            let staking_total = staking_info.total;
            EraRewardsAndStakes::<T>::mutate(&current_era, |value| {
                if let Some(x) = value {
                    x.staked = x.staked.saturating_sub(staking_total)
                }
            });

            // This makes contract ilegible for rewards from this era onwards.
            staking_info.total = Zero::zero();
            ContractEraStake::<T>::insert(contract_id.clone(), current_era, staking_info);

            T::Currency::unreserve(&developer, T::RegisterDeposit::get());

            // TODO: remove staker and reduce number of stakers to zero?

            dapp_info.state = DAppState::Unregistered;
            RegisteredDapps::<T>::insert(&contract_id, dapp_info);

            Self::deposit_event(Event::<T>::ContractRemoved(developer, contract_id));

            // let number_of_stakers = staking_info.stakers.len();
            Ok(Some(T::WeightInfo::unregister(1 as u32)).into())
        }

        // TODO: weight and doc
        #[pallet::weight(T::WeightInfo::unregister(T::MaxNumberOfStakersPerContract::get()))]
        pub fn unbond_from_unregistered_contract(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            let staker = ensure_signed(origin)?;

            // dApp must exist and it has to be unregistered
            let dapp_info =
                RegisteredDapps::<T>::get(&contract_id).ok_or(Error::<T>::NotOperatedContract)?;
            ensure!(
                dapp_info.state == DAppState::Unregistered,
                Error::<T>::NotUnregisteredContract
            );

            let current_era = Self::current_era();

            // There should be some leftover staked amount
            let staking_info = Self::staker_staking_info(&staker, &contract_id, current_era);
            ensure!(
                staking_info.staked > Zero::zero(),
                Error::<T>::NotStakedContract
            );

            // Unlock the staked amount immediately. No unbonding period for this scenario.
            let mut ledger = Self::ledger(&staker);
            ledger.locked = ledger.locked.saturating_sub(staking_info.staked);
            Self::update_ledger(&staker, ledger);

            // Write default empty staker info struct to state that no remaining staked amount remains.
            StakerContractEraInfo::<T>::insert(
                (&staker, &contract_id),
                current_era,
                StakerInfo::default(),
            );

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
            let available_balance = free_balance.saturating_sub(ledger.locked);
            let value_to_stake = value.min(available_balance);
            ensure!(
                value_to_stake > Zero::zero(),
                Error::<T>::StakingWithNoValue
            );

            // Get the latest era staking point info or create it if contract hasn't been staked yet so far.
            let current_era = Self::current_era();
            let mut contract_info = Self::contract_staking_info(&contract_id, current_era);
            let mut staker_info = Self::staker_staking_info(&staker, &contract_id, current_era);

            // Ensure that we can add additional staker for the contract.
            if staker_info.staked.is_zero() {
                ensure!(
                    contract_info.number_of_stakers < T::MaxNumberOfStakersPerContract::get(),
                    Error::<T>::MaxNumberOfStakersExceeded,
                );
                contract_info.number_of_stakers += 1;
            }

            // Increment personal staking amount.
            staker_info.staked = staker_info
                .staked
                .checked_add(&value_to_stake)
                .ok_or(ArithmeticError::Overflow)?;

            ensure!(
                staker_info.staked >= T::MinimumStakingAmount::get(),
                Error::<T>::InsufficientValue,
            );

            // Increment total staked amount.
            contract_info.total = contract_info
                .total
                .checked_add(&value_to_stake)
                .ok_or(ArithmeticError::Overflow)?;

            ledger.locked = ledger
                .locked
                .checked_add(&value_to_stake)
                .ok_or(ArithmeticError::Overflow)?;
            ledger.contract_staked(&contract_id);

            // Update total staked value in era.
            EraRewardsAndStakes::<T>::mutate(&current_era, |value| {
                if let Some(x) = value {
                    x.staked = x.staked.saturating_add(value_to_stake)
                }
            });

            // Update staked amount for (staker, contract) pairing for current era
            StakerContractEraInfo::<T>::insert((&staker, &contract_id), current_era, staker_info);

            // Update ledger and payee
            Self::update_ledger(&staker, ledger);

            // Update staked information for contract in current era
            ContractEraStake::<T>::insert(&contract_id, current_era, contract_info);

            Self::deposit_event(Event::<T>::BondAndStake(
                staker,
                contract_id,
                value_to_stake,
            ));
            Ok(().into())
        }

        /// Start unbonding process and unstake balance from the contract.
        ///
        /// The unstaked amount will no longer be eligible for rewards but still won't be unlocked.
        /// User needs to wait for the unbonding period to finish before being able to withdraw
        /// the funds via `withdraw_unbonded` call.
        ///
        /// In case remaining staked balance on contract is below minimum staking amount,
        /// entire stake for that contract will be unstaked.
        ///
        #[pallet::weight(T::WeightInfo::unbond_and_unstake())]
        pub fn unbond_and_unstake(
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

            let mut staker_info = Self::staker_staking_info(&staker, &contract_id, current_era);
            ensure!(
                staker_info.staked > Zero::zero(),
                Error::<T>::NotStakedContract,
            );

            // Calculate the value which will be unstaked.
            let remaining = staker_info.staked.saturating_sub(value);
            let value_to_unstake = if remaining < T::MinimumStakingAmount::get() {
                staker_info.staked
            } else {
                value
            };

            // Sanity check
            ensure!(
                value_to_unstake > Zero::zero(),
                Error::<T>::UnstakingWithNoValue
            );

            let mut ledger = Self::ledger(&staker);

            // Update the chunks and write them to storage
            const SKIP_CURRENT_ERA: u32 = 1;
            ledger.unbonding_info.add(UnlockingChunk {
                amount: value_to_unstake,
                unlock_era: current_era + SKIP_CURRENT_ERA + T::UnbondingPeriod::get(),
            });
            // This should be done AFTER insertion since it's possible for chunks to merge
            ensure!(
                ledger.unbonding_info.len() <= T::MaxUnlockingChunks::get(),
                Error::<T>::TooManyUnlockingChunks
            );

            if value_to_unstake == staker_info.staked {
                ledger.contract_unstaked(&contract_id, current_era, T::HistoryDepth::get());
            }
            Self::update_ledger(&staker, ledger);

            let mut contract_info = Self::contract_staking_info(&contract_id, current_era);
            contract_info.total = contract_info.total.saturating_sub(value_to_unstake);
            if value_to_unstake == staker_info.staked {
                contract_info.number_of_stakers -= 1;
            }

            // Update total staked value in era.
            EraRewardsAndStakes::<T>::mutate(&current_era, |value| {
                if let Some(x) = value {
                    x.staked = x.staked.saturating_sub(value_to_unstake)
                }
            });

            // Update the era staking points
            ContractEraStake::<T>::insert(contract_id.clone(), current_era, contract_info);

            // Update the info for staker. Note that this has to be written even if `remaining` is zero.
            staker_info.staked = staker_info.staked.saturating_sub(value_to_unstake);
            StakerContractEraInfo::<T>::insert((&staker, &contract_id), current_era, staker_info);

            Self::deposit_event(Event::<T>::UnbondAndUnstake(
                staker,
                contract_id,
                value_to_unstake,
            ));

            Ok(().into())
        }

        /// Withdraw all funds that have completed the unbonding process.
        ///
        /// If there are unbonding chunks which will be fully unbonded in future eras,
        /// they will remain and can be withdrawn later.
        ///
        #[pallet::weight(T::WeightInfo::withdraw_unbonded())]
        pub fn withdraw_unbonded(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let staker = ensure_signed(origin)?;

            let mut ledger = Self::ledger(&staker);
            let current_era = Self::current_era();

            let (valid_chunks, future_chunks) = ledger.unbonding_info.partition(current_era);
            let withdraw_amount = valid_chunks.sum();

            ensure!(!withdraw_amount.is_zero(), Error::<T>::NothingToWithdraw);

            // Get the staking ledger and update it
            ledger.locked = ledger.locked.saturating_sub(withdraw_amount);
            ledger.unbonding_info = future_chunks;

            Self::update_ledger(&staker, ledger);

            Self::deposit_event(Event::<T>::Withdrawn(staker, withdraw_amount));

            Ok(().into())
        }

        /// Claim the rewards earned by contract_id.
        /// All stakers and developer for this contract will be paid out with single call.
        /// claim is valid for all unclaimed eras but not longer than history_depth().
        /// Any reward older than history_depth() will go to Treasury.
        /// Any user can call this function.
        #[pallet::weight(T::WeightInfo::claim(1))]
        pub fn claim(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            #[pallet::compact] era: EraIndex,
        ) -> DispatchResultWithPostInfo {
            let claimer = ensure_signed(origin)?;

            let dapp_info =
                RegisteredDapps::<T>::get(&contract_id).ok_or(Error::<T>::NotOperatedContract)?;

            let contract_info = Self::contract_staking_info(&contract_id, era);
            if dapp_info.state == DAppState::Unregistered {
                // TODO: Add a new error here? Claim beyond unregitered era?
                ensure!(
                    contract_info.total > Zero::zero(),
                    Error::<T>::NotOperatedContract
                );
            }

            let current_era = Self::current_era();
            let era_low_bound = current_era.saturating_sub(T::HistoryDepth::get());
            ensure!(
                era < current_era && era >= era_low_bound,
                Error::<T>::EraOutOfBounds
            );

            let mut staker_info = Self::staker_staking_info(&claimer, &contract_id, era);
            ensure!(
                staker_info.claimed_rewards.is_zero(),
                Error::<T>::AlreadyClaimedInThisEra,
            );
            // Dev doesn't need to have anything staked
            if claimer != dapp_info.developer {
                ensure!(staker_info.staked > Zero::zero(), Error::<T>::NotStaked,);
            }

            let reward_and_stake =
                Self::era_reward_and_stake(era).ok_or(Error::<T>::UnknownEraReward)?;

            // Calculate the contract reward for this era.
            let reward_ratio = Perbill::from_rational(contract_info.total, reward_and_stake.staked);
            let contract_reward = reward_ratio * reward_and_stake.rewards;

            // Calculate the developer part of the reward. Only dev is eligible for this part.
            let developer_part_reward = if claimer == dapp_info.developer {
                T::DeveloperRewardPercentage::get() * contract_reward
            } else {
                Zero::zero()
            };

            // Calculate the staker part of the reward. This is required since atm dev can be a staker too.
            let staker_part_reward = if staker_info.staked > Zero::zero() {
                let stakers_reward =
                    contract_reward - T::DeveloperRewardPercentage::get() * contract_reward;

                let claimer_ratio = Perbill::from_rational(staker_info.staked, contract_info.total);

                claimer_ratio * stakers_reward
            } else {
                Zero::zero()
            };

            let claimer_reward = developer_part_reward + staker_part_reward;

            // Withdraw reward funds from the dapps staking and transfer them to claimer
            let reward_imbalance = T::Currency::withdraw(
                &Self::account_id(),
                claimer_reward,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::AllowDeath,
            )?;
            T::Currency::resolve_creating(&claimer, reward_imbalance);

            Self::deposit_event(Event::<T>::Reward(
                claimer.clone(),
                contract_id.clone(),
                era,
                claimer_reward,
            ));
            // TODO: maybe deposit two events? One for staker part and one for developer part?

            staker_info.claimed_rewards = claimer_reward;

            ContractEraStake::<T>::insert(&contract_id, era, contract_info);
            StakerContractEraInfo::<T>::insert((&claimer, &contract_id), era, staker_info);

            Ok(Some(T::WeightInfo::claim(1 as u32)).into())
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
        fn update_ledger(
            staker: &T::AccountId,
            ledger: AccountLedger<T::SmartContract, BalanceOf<T>>,
        ) {
            if ledger.locked.is_zero() && ledger.unbonding_info.is_empty() {
                Ledger::<T>::remove(&staker);
                T::Currency::remove_lock(STAKING_ID, &staker);
            } else {
                T::Currency::set_lock(STAKING_ID, &staker, ledger.locked, WithdrawReasons::all());
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
        pub(crate) fn contract_staking_info(
            contract_id: &T::SmartContract,
            era: EraIndex,
        ) -> EraStakingPoints<BalanceOf<T>> {
            if let Some(staking_info) = ContractEraStake::<T>::get(contract_id, era) {
                staking_info
            } else {
                let avail_era = ContractEraStake::<T>::iter_key_prefix(&contract_id)
                    .filter(|x| *x <= era)
                    .max()
                    .unwrap_or(Zero::zero());

                ContractEraStake::<T>::get(contract_id, avail_era).unwrap_or_default()
            }
        }

        /// This helper returns staked amount for given contract in the given era if possible or latest stored data
        /// or finally default value if storage have no data for it.
        pub(crate) fn staker_staking_info(
            staker_id: &T::AccountId,
            contract_id: &T::SmartContract,
            era: EraIndex,
        ) -> StakerInfo<BalanceOf<T>> {
            let key = (staker_id, contract_id);
            if let Some(staking_info) = StakerContractEraInfo::<T>::get(key, era) {
                staking_info
            } else {
                let avail_era = StakerContractEraInfo::<T>::iter_key_prefix(key)
                    .filter(|x| *x <= era)
                    .max()
                    .unwrap_or(Zero::zero());

                let mut info = StakerContractEraInfo::<T>::get(key, avail_era).unwrap_or_default();
                info.claimed_rewards = Zero::zero();
                info
            }
        }

        /// Check that contract is registered and active.
        fn is_active(contract_id: &T::SmartContract) -> bool {
            if let Some(dapp_info) = RegisteredDapps::<T>::get(contract_id) {
                dapp_info.state == DAppState::Registered
            } else {
                false
            }
        }
    }
}
