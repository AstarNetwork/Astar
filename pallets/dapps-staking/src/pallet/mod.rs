// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

//! Dapps staking FRAME Pallet.

use super::*;
use frame_support::{
    dispatch::DispatchResult,
    ensure,
    pallet_prelude::*,
    traits::{
        Currency, ExistenceRequirement, Get, Imbalance, LockIdentifier, LockableCurrency,
        ReservableCurrency, WithdrawReasons,
    },
    weights::Weight,
    PalletId,
};
use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
use sp_runtime::{
    traits::{AccountIdConversion, Saturating, Zero},
    Perbill,
};
use sp_std::{convert::From, mem};

const STAKING_ID: LockIdentifier = *b"dapstake";

#[frame_support::pallet]
#[allow(clippy::module_inception)]
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

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The staking balance.
        type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>
            + ReservableCurrency<Self::AccountId>;

        /// Describes smart contract in the context required by dapps staking.
        type SmartContract: Default + Parameter + Member + MaxEncodedLen;

        /// Number of blocks per era.
        #[pallet::constant]
        type BlockPerEra: Get<BlockNumberFor<Self>>;

        /// Deposit that will be reserved as part of new contract registration.
        #[pallet::constant]
        type RegisterDeposit: Get<BalanceOf<Self>>;

        /// Maximum number of unique stakers per contract.
        #[pallet::constant]
        type MaxNumberOfStakersPerContract: Get<u32>;

        /// Minimum amount user must have staked on contract.
        /// User can stake less if they already have the minimum staking amount staked on that particular contract.
        #[pallet::constant]
        type MinimumStakingAmount: Get<BalanceOf<Self>>;

        /// Dapps staking pallet Id
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Minimum amount that should be left on staker account after staking.
        /// Serves as a safeguard to prevent users from locking their entire free balance.
        #[pallet::constant]
        type MinimumRemainingAmount: Get<BalanceOf<Self>>;

        /// Max number of unlocking chunks per account Id <-> contract Id pairing.
        /// If value is zero, unlocking becomes impossible.
        #[pallet::constant]
        type MaxUnlockingChunks: Get<u32>;

        /// Number of eras that need to pass until unstaked value can be withdrawn.
        /// Current era is always counted as full era (regardless how much blocks are remaining).
        /// When set to `0`, it's equal to having no unbonding period.
        #[pallet::constant]
        type UnbondingPeriod: Get<u32>;

        /// Max number of unique `EraStake` values that can exist for a `(staker, contract)` pairing.
        /// When stakers claims rewards, they will either keep the number of `EraStake` values the same or they will reduce them by one.
        /// Stakers cannot add an additional `EraStake` value by calling `bond&stake` or `unbond&unstake` if they've reached the max number of values.
        ///
        /// This ensures that history doesn't grow indefinitely - if there are too many chunks, stakers should first claim their former rewards
        /// before adding additional `EraStake` values.
        #[pallet::constant]
        type MaxEraStakeValues: Get<u32>;

        /// Number of eras that need to pass until dApp rewards for the unregistered contracts can be burned.
        /// Developer can still claim rewards after this period has passed, iff it hasn't been burned yet.
        ///
        /// For example, if retention is set to `2` and current era is `10`, it means that all unclaimed rewards bellow era `8` can be burned.
        #[pallet::constant]
        type UnregisteredDappRewardRetention: Get<u32>;

        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    /// Denotes whether pallet is disabled (in maintenance mode) or not
    #[pallet::storage]
    #[pallet::whitelist_storage]
    #[pallet::getter(fn pallet_disabled)]
    pub type PalletDisabled<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// General information about the staker (non-smart-contract specific).
    #[pallet::storage]
    #[pallet::getter(fn ledger)]
    pub type Ledger<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, AccountLedger<BalanceOf<T>>, ValueQuery>;

    /// The current era index.
    #[pallet::storage]
    #[pallet::whitelist_storage]
    #[pallet::getter(fn current_era)]
    pub type CurrentEra<T> = StorageValue<_, EraIndex, ValueQuery>;

    /// Accumulator for block rewards during an era. It is reset at every new era
    #[pallet::storage]
    #[pallet::getter(fn block_reward_accumulator)]
    pub type BlockRewardAccumulator<T> = StorageValue<_, RewardInfo<BalanceOf<T>>, ValueQuery>;

    #[pallet::type_value]
    pub fn ForceEraOnEmpty() -> Forcing {
        Forcing::NotForcing
    }

    /// Mode of era forcing.
    #[pallet::storage]
    #[pallet::whitelist_storage]
    #[pallet::getter(fn force_era)]
    pub type ForceEra<T> = StorageValue<_, Forcing, ValueQuery, ForceEraOnEmpty>;

    /// Stores the block number of when the next era starts
    #[pallet::storage]
    #[pallet::whitelist_storage]
    #[pallet::getter(fn next_era_starting_block)]
    pub type NextEraStartingBlock<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    /// Simple map where developer account points to their smart contract
    #[pallet::storage]
    #[pallet::getter(fn registered_contract)]
    pub(crate) type RegisteredDevelopers<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, T::SmartContract>;

    /// Simple map where smart contract points to basic info about it (e.g. developer address, state)
    #[pallet::storage]
    #[pallet::getter(fn dapp_info)]
    pub(crate) type RegisteredDapps<T: Config> =
        StorageMap<_, Blake2_128Concat, T::SmartContract, DAppInfo<T::AccountId>>;

    /// General information about an era like TVL, total staked value, rewards.
    #[pallet::storage]
    #[pallet::getter(fn general_era_info)]
    pub type GeneralEraInfo<T: Config> =
        StorageMap<_, Twox64Concat, EraIndex, EraInfo<BalanceOf<T>>>;

    /// Staking information about contract in a particular era.
    #[pallet::storage]
    #[pallet::getter(fn contract_stake_info)]
    pub type ContractEraStake<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::SmartContract,
        Twox64Concat,
        EraIndex,
        ContractStakeInfo<BalanceOf<T>>,
    >;

    /// Info about stakers stakes on particular contracts.
    #[pallet::storage]
    #[pallet::getter(fn staker_info)]
    pub type GeneralStakerInfo<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        T::SmartContract,
        StakerInfo<BalanceOf<T>>,
        ValueQuery,
    >;

    /// Stores the current pallet storage version.
    #[pallet::storage]
    #[pallet::getter(fn storage_version)]
    pub(crate) type StorageVersion<T> = StorageValue<_, Version, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Account has bonded and staked funds on a smart contract.
        BondAndStake(T::AccountId, T::SmartContract, BalanceOf<T>),
        /// Account has unbonded & unstaked some funds. Unbonding process begins.
        UnbondAndUnstake(T::AccountId, T::SmartContract, BalanceOf<T>),
        /// Account has fully withdrawn all staked amount from an unregistered contract.
        WithdrawFromUnregistered(T::AccountId, T::SmartContract, BalanceOf<T>),
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
        /// Maintenance mode has been enabled or disabled
        MaintenanceMode(bool),
        /// Reward handling modified
        RewardDestination(T::AccountId, RewardDestination),
        /// Nomination part has been transfered from one contract to another.
        ///
        /// \(staker account, origin smart contract, amount, target smart contract\)
        NominationTransfer(
            T::AccountId,
            T::SmartContract,
            BalanceOf<T>,
            T::SmartContract,
        ),
        /// Stale, unclaimed reward from an unregistered contract has been burned.
        ///
        /// \(developer account, smart contract, era, amount burned\)
        StaleRewardBurned(T::AccountId, T::SmartContract, EraIndex, BalanceOf<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Disabled
        Disabled,
        /// No change in maintenance mode
        NoMaintenanceModeChange,
        /// Upgrade is too heavy, reduce the weight parameter.
        UpgradeTooHeavy,
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
        /// Unclaimed rewards should be claimed before withdrawing stake.
        UnclaimedRewardsRemaining,
        /// Unstaking a contract with zero value
        UnstakingWithNoValue,
        /// There are no previously unbonded funds that can be unstaked and withdrawn.
        NothingToWithdraw,
        /// The contract is already registered by other account
        AlreadyRegisteredContract,
        /// This account was already used to register contract
        AlreadyUsedDeveloperAccount,
        /// Smart contract not owned by the account id.
        NotOwnedContract,
        /// Report issue on github if this is ever emitted
        UnknownEraReward,
        /// Report issue on github if this is ever emitted
        UnexpectedStakeInfoEra,
        /// Contract has too many unlocking chunks. Withdraw the existing chunks if possible
        /// or wait for current chunks to complete unlocking process to withdraw them.
        TooManyUnlockingChunks,
        /// Contract already claimed in this era and reward is distributed
        AlreadyClaimedInThisEra,
        /// Era parameter is out of bounds
        EraOutOfBounds,
        /// Too many active `EraStake` values for (staker, contract) pairing.
        /// Claim existing rewards to fix this problem.
        TooManyEraStakeValues,
        /// Account is not actively staking
        NotActiveStaker,
        /// Transfering nomination to the same contract
        NominationTransferToSameContract,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            // As long as pallet is disabled, we shouldn't allow any storage modifications.
            // This means we might prolong an era but it's acceptable.
            // Runtime upgrade should be timed so we ensure that we complete it before
            // a new era is triggered. This code is just a safety net to ensure nothing is broken
            // if we fail to do that.
            if PalletDisabled::<T>::get() {
                return T::DbWeight::get().reads(1);
            }

            let force_new_era = Self::force_era().eq(&Forcing::ForceNew);
            let previous_era = Self::current_era();
            let next_era_starting_block = Self::next_era_starting_block();

            // Value is compared to 1 since genesis block is ignored
            if now >= next_era_starting_block || force_new_era || previous_era.is_zero() {
                let blocks_per_era = T::BlockPerEra::get();
                let next_era = previous_era + 1;
                CurrentEra::<T>::put(next_era);

                NextEraStartingBlock::<T>::put(now + blocks_per_era);

                let reward = BlockRewardAccumulator::<T>::take();
                Self::reward_balance_snapshot(previous_era, reward);
                let consumed_weight = Self::rotate_staking_info(previous_era);

                if force_new_era {
                    ForceEra::<T>::put(Forcing::NotForcing);
                }

                Self::deposit_event(Event::<T>::NewDappStakingEra(next_era));

                consumed_weight + T::DbWeight::get().reads_writes(5, 3)
            } else {
                T::DbWeight::get().reads(4)
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Used to register contract for dapps staking.
        /// The origin account used is treated as the `developer` account.
        ///
        /// Depending on the pallet configuration/state it is possible that developer needs to be whitelisted prior to registration.
        ///
        /// As part of this call, `RegisterDeposit` will be reserved from devs account.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register())]
        pub fn register(
            origin: OriginFor<T>,
            developer: T::AccountId,
            contract_id: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            ensure_root(origin)?;

            ensure!(
                !RegisteredDevelopers::<T>::contains_key(&developer),
                Error::<T>::AlreadyUsedDeveloperAccount,
            );
            ensure!(
                !RegisteredDapps::<T>::contains_key(&contract_id),
                Error::<T>::AlreadyRegisteredContract,
            );

            T::Currency::reserve(&developer, T::RegisterDeposit::get())?;

            RegisteredDapps::<T>::insert(contract_id.clone(), DAppInfo::new(developer.clone()));
            RegisteredDevelopers::<T>::insert(&developer, contract_id.clone());

            Self::deposit_event(Event::<T>::NewContract(developer, contract_id));

            Ok(().into())
        }

        /// Unregister existing contract from dapps staking, making it ineligible for rewards from current era onwards.
        /// This must be called by the root (at the moment).
        ///
        /// Deposit is returned to the developer but existing stakers should manually call `withdraw_from_unregistered` if they wish to to unstake.
        ///
        /// **Warning**: After this action ,contract can not be registered for dapps staking again.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::unregister())]
        pub fn unregister(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            ensure_root(origin)?;

            let mut dapp_info =
                RegisteredDapps::<T>::get(&contract_id).ok_or(Error::<T>::NotOperatedContract)?;
            ensure!(
                dapp_info.state == DAppState::Registered,
                Error::<T>::NotOperatedContract
            );
            let developer = dapp_info.developer.clone();

            let current_era = Self::current_era();
            dapp_info.state = DAppState::Unregistered(current_era);
            RegisteredDapps::<T>::insert(&contract_id, dapp_info);

            T::Currency::unreserve(&developer, T::RegisterDeposit::get());

            Self::deposit_event(Event::<T>::ContractRemoved(developer, contract_id));

            Ok(().into())
        }

        /// Withdraw locked funds from a contract that was unregistered.
        ///
        /// Funds don't need to undergo the unbonding period - they are returned immediately to the staker's free balance.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::withdraw_from_unregistered())]
        pub fn withdraw_from_unregistered(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            let staker = ensure_signed(origin)?;

            // dApp must exist and it has to be unregistered
            let dapp_info =
                RegisteredDapps::<T>::get(&contract_id).ok_or(Error::<T>::NotOperatedContract)?;

            let unregistered_era = if let DAppState::Unregistered(x) = dapp_info.state {
                x
            } else {
                return Err(Error::<T>::NotUnregisteredContract.into());
            };

            // There should be some leftover staked amount
            let mut staker_info = Self::staker_info(&staker, &contract_id);
            let staked_value = staker_info.latest_staked_value();
            ensure!(staked_value > Zero::zero(), Error::<T>::NotStakedContract);

            // Don't allow withdrawal until all rewards have been claimed.
            let (claimable_era, _) = staker_info.claim();
            ensure!(
                claimable_era >= unregistered_era || claimable_era.is_zero(),
                Error::<T>::UnclaimedRewardsRemaining
            );

            // Unlock the staked amount immediately. No unbonding period for this scenario.
            let mut ledger = Self::ledger(&staker);
            ledger.locked = ledger.locked.saturating_sub(staked_value);
            Self::update_ledger(&staker, ledger);

            Self::update_staker_info(&staker, &contract_id, Default::default());

            let current_era = Self::current_era();
            GeneralEraInfo::<T>::mutate(&current_era, |value| {
                if let Some(x) = value {
                    x.staked = x.staked.saturating_sub(staked_value);
                    x.locked = x.locked.saturating_sub(staked_value);
                }
            });

            Self::deposit_event(Event::<T>::WithdrawFromUnregistered(
                staker,
                contract_id,
                staked_value,
            ));

            Ok(().into())
        }

        /// Lock up and stake balance of the origin account.
        ///
        /// `value` must be more than the `minimum_balance` specified by `MinimumStakingAmount`
        /// unless account already has bonded value equal or more than 'minimum_balance'.
        ///
        /// The dispatch origin for this call must be _Signed_ by the staker's account.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::bond_and_stake())]
        pub fn bond_and_stake(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            let staker = ensure_signed(origin)?;

            // Check that contract is ready for staking.
            ensure!(
                Self::is_active(&contract_id),
                Error::<T>::NotOperatedContract
            );

            // Get the staking ledger or create an entry if it doesn't exist.
            let mut ledger = Self::ledger(&staker);
            let available_balance = Self::available_staking_balance(&staker, &ledger);
            let value_to_stake = value.min(available_balance);
            ensure!(
                value_to_stake > Zero::zero(),
                Error::<T>::StakingWithNoValue
            );

            let current_era = Self::current_era();
            let mut staking_info =
                Self::contract_stake_info(&contract_id, current_era).unwrap_or_default();
            let mut staker_info = Self::staker_info(&staker, &contract_id);

            Self::stake_on_contract(
                &mut staker_info,
                &mut staking_info,
                value_to_stake,
                current_era,
            )?;

            ledger.locked = ledger.locked.saturating_add(value_to_stake);

            // Update storage
            GeneralEraInfo::<T>::mutate(&current_era, |value| {
                if let Some(x) = value {
                    x.staked = x.staked.saturating_add(value_to_stake);
                    x.locked = x.locked.saturating_add(value_to_stake);
                }
            });

            Self::update_ledger(&staker, ledger);
            Self::update_staker_info(&staker, &contract_id, staker_info);
            ContractEraStake::<T>::insert(&contract_id, current_era, staking_info);

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
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::unbond_and_unstake())]
        pub fn unbond_and_unstake(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            let staker = ensure_signed(origin)?;

            ensure!(value > Zero::zero(), Error::<T>::UnstakingWithNoValue);
            ensure!(
                Self::is_active(&contract_id),
                Error::<T>::NotOperatedContract,
            );

            let current_era = Self::current_era();
            let mut staker_info = Self::staker_info(&staker, &contract_id);
            let mut contract_stake_info =
                Self::contract_stake_info(&contract_id, current_era).unwrap_or_default();

            let value_to_unstake = Self::unstake_from_contract(
                &mut staker_info,
                &mut contract_stake_info,
                value,
                current_era,
            )?;

            // Update the chunks and write them to storage
            let mut ledger = Self::ledger(&staker);
            ledger.unbonding_info.add(UnlockingChunk {
                amount: value_to_unstake,
                unlock_era: current_era + T::UnbondingPeriod::get(),
            });
            // This should be done AFTER insertion since it's possible for chunks to merge
            ensure!(
                ledger.unbonding_info.len() <= T::MaxUnlockingChunks::get(),
                Error::<T>::TooManyUnlockingChunks
            );

            Self::update_ledger(&staker, ledger);

            // Update total staked value in era.
            GeneralEraInfo::<T>::mutate(&current_era, |value| {
                if let Some(x) = value {
                    x.staked = x.staked.saturating_sub(value_to_unstake)
                }
            });
            Self::update_staker_info(&staker, &contract_id, staker_info);
            ContractEraStake::<T>::insert(&contract_id, current_era, contract_stake_info);

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
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::withdraw_unbonded())]
        pub fn withdraw_unbonded(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
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
            GeneralEraInfo::<T>::mutate(&current_era, |value| {
                if let Some(x) = value {
                    x.locked = x.locked.saturating_sub(withdraw_amount)
                }
            });

            Self::deposit_event(Event::<T>::Withdrawn(staker, withdraw_amount));

            Ok(().into())
        }

        /// Transfer nomination from one contract to another.
        ///
        /// Same rules as for `bond_and_stake` and `unbond_and_unstake` apply.
        /// Minor difference is that there is no unbonding period so this call won't
        /// check whether max number of unbonding chunks is exceeded.
        ///
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::nomination_transfer())]
        pub fn nomination_transfer(
            origin: OriginFor<T>,
            origin_contract_id: T::SmartContract,
            #[pallet::compact] value: BalanceOf<T>,
            target_contract_id: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            let staker = ensure_signed(origin)?;

            // Contracts must differ and both must be active
            ensure!(
                origin_contract_id != target_contract_id,
                Error::<T>::NominationTransferToSameContract
            );
            ensure!(
                Self::is_active(&origin_contract_id),
                Error::<T>::NotOperatedContract
            );
            ensure!(
                Self::is_active(&target_contract_id),
                Error::<T>::NotOperatedContract
            );

            // Validate origin contract related data & update it
            let current_era = Self::current_era();
            let mut origin_staker_info = Self::staker_info(&staker, &origin_contract_id);
            let mut origin_staking_info =
                Self::contract_stake_info(&origin_contract_id, current_era).unwrap_or_default();

            let origin_to_target_transfer_value = Self::unstake_from_contract(
                &mut origin_staker_info,
                &mut origin_staking_info,
                value,
                current_era,
            )?;

            // Validate target contract related data & update it
            let mut target_staker_info = Self::staker_info(&staker, &target_contract_id);
            let mut target_staking_info =
                Self::contract_stake_info(&target_contract_id, current_era).unwrap_or_default();

            Self::stake_on_contract(
                &mut target_staker_info,
                &mut target_staking_info,
                origin_to_target_transfer_value,
                current_era,
            )?;

            // Update origin data
            ContractEraStake::<T>::insert(&origin_contract_id, current_era, origin_staking_info);
            Self::update_staker_info(&staker, &origin_contract_id, origin_staker_info);

            // Update target data
            ContractEraStake::<T>::insert(&target_contract_id, current_era, target_staking_info);
            Self::update_staker_info(&staker, &target_contract_id, target_staker_info);

            Self::deposit_event(Event::<T>::NominationTransfer(
                staker,
                origin_contract_id,
                origin_to_target_transfer_value,
                target_contract_id,
            ));

            Ok(().into())
        }

        // TODO: do we need to add force methods or at least methods that allow others to claim for someone else?

        /// Claim earned staker rewards for the oldest unclaimed era.
        /// In order to claim multiple eras, this call has to be called multiple times.
        ///
        /// The rewards are always added to the staker's free balance (account) but depending on the reward destination configuration,
        /// they might be immediately re-staked.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::claim_staker_with_restake().max(T::WeightInfo::claim_staker_without_restake()))]
        pub fn claim_staker(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            let staker = ensure_signed(origin)?;

            // Ensure we have something to claim
            let mut staker_info = Self::staker_info(&staker, &contract_id);
            let (era, staked) = staker_info.claim();
            ensure!(staked > Zero::zero(), Error::<T>::NotStakedContract);

            let dapp_info =
                RegisteredDapps::<T>::get(&contract_id).ok_or(Error::<T>::NotOperatedContract)?;

            if let DAppState::Unregistered(unregister_era) = dapp_info.state {
                ensure!(era < unregister_era, Error::<T>::NotOperatedContract);
            }

            let current_era = Self::current_era();
            ensure!(era < current_era, Error::<T>::EraOutOfBounds);

            let staking_info = Self::contract_stake_info(&contract_id, era).unwrap_or_default();
            let reward_and_stake =
                Self::general_era_info(era).ok_or(Error::<T>::UnknownEraReward)?;

            let (_, stakers_joint_reward) =
                Self::dev_stakers_split(&staking_info, &reward_and_stake);
            let staker_reward =
                Perbill::from_rational(staked, staking_info.total) * stakers_joint_reward;

            let mut ledger = Self::ledger(&staker);

            let should_restake_reward = Self::should_restake_reward(
                ledger.reward_destination,
                dapp_info.state,
                staker_info.latest_staked_value(),
            );

            if should_restake_reward {
                staker_info
                    .stake(current_era, staker_reward)
                    .map_err(|_| Error::<T>::UnexpectedStakeInfoEra)?;

                // Restaking will, in the worst case, remove one, and add one record,
                // so it's fine if the vector is full
                ensure!(
                    staker_info.len() <= T::MaxEraStakeValues::get(),
                    Error::<T>::TooManyEraStakeValues
                );
            }

            // Withdraw reward funds from the dapps staking pot
            let reward_imbalance = T::Currency::withdraw(
                &Self::account_id(),
                staker_reward,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::AllowDeath,
            )?;

            if should_restake_reward {
                ledger.locked = ledger.locked.saturating_add(staker_reward);
                Self::update_ledger(&staker, ledger);

                // Update storage
                GeneralEraInfo::<T>::mutate(&current_era, |value| {
                    if let Some(x) = value {
                        x.staked = x.staked.saturating_add(staker_reward);
                        x.locked = x.locked.saturating_add(staker_reward);
                    }
                });

                ContractEraStake::<T>::mutate(contract_id.clone(), current_era, |staking_info| {
                    if let Some(x) = staking_info {
                        x.total = x.total.saturating_add(staker_reward);
                    }
                });

                Self::deposit_event(Event::<T>::BondAndStake(
                    staker.clone(),
                    contract_id.clone(),
                    staker_reward,
                ));
            }

            T::Currency::resolve_creating(&staker, reward_imbalance);
            Self::update_staker_info(&staker, &contract_id, staker_info);
            Self::deposit_event(Event::<T>::Reward(staker, contract_id, era, staker_reward));

            Ok(Some(if should_restake_reward {
                T::WeightInfo::claim_staker_with_restake()
            } else {
                T::WeightInfo::claim_staker_without_restake()
            })
            .into())
        }

        /// Claim earned dapp rewards for the specified era.
        ///
        /// Call must ensure that the specified era is eligible for reward payout and that it hasn't already been paid out for the dapp.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::claim_dapp())]
        pub fn claim_dapp(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            #[pallet::compact] era: EraIndex,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            let _ = ensure_signed(origin)?;

            let dapp_info =
                RegisteredDapps::<T>::get(&contract_id).ok_or(Error::<T>::NotOperatedContract)?;

            let mut contract_stake_info =
                Self::contract_stake_info(&contract_id, era).unwrap_or_default();

            let dapp_reward = Self::calculate_dapp_reward(&contract_stake_info, &dapp_info, era)?;

            // Withdraw reward funds from the dapps staking
            let reward_imbalance = T::Currency::withdraw(
                &Self::account_id(),
                dapp_reward,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::AllowDeath,
            )?;

            T::Currency::resolve_creating(&dapp_info.developer, reward_imbalance);
            Self::deposit_event(Event::<T>::Reward(
                dapp_info.developer,
                contract_id.clone(),
                era,
                dapp_reward,
            ));

            // updated counter for total rewards paid to the contract
            contract_stake_info.contract_reward_claimed = true;
            ContractEraStake::<T>::insert(&contract_id, era, contract_stake_info);

            Ok(().into())
        }

        /// Force a new era at the start of the next block.
        ///
        /// The dispatch origin must be Root.
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::force_new_era())]
        pub fn force_new_era(origin: OriginFor<T>) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            ensure_root(origin)?;
            ForceEra::<T>::put(Forcing::ForceNew);
            Ok(())
        }

        /// `true` will disable pallet, enabling maintenance mode. `false` will do the opposite.
        ///
        /// The dispatch origin must be Root.
        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::maintenance_mode())]
        pub fn maintenance_mode(
            origin: OriginFor<T>,
            enable_maintenance: bool,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            let is_disabled = PalletDisabled::<T>::get();

            ensure!(
                is_disabled ^ enable_maintenance,
                Error::<T>::NoMaintenanceModeChange
            );
            PalletDisabled::<T>::put(enable_maintenance);

            Self::deposit_event(Event::<T>::MaintenanceMode(enable_maintenance));
            Ok(().into())
        }

        /// Used to set reward destination for staker rewards.
        ///
        /// User must be an active staker in order to use this call.
        /// This will apply to all existing unclaimed rewards.
        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::set_reward_destination())]
        pub fn set_reward_destination(
            origin: OriginFor<T>,
            reward_destination: RewardDestination,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            let staker = ensure_signed(origin)?;
            let mut ledger = Self::ledger(&staker);

            ensure!(!ledger.is_empty(), Error::<T>::NotActiveStaker);

            // this is done directly instead of using update_ledger helper
            // because there's no need to interact with the Currency locks
            ledger.reward_destination = reward_destination;
            Ledger::<T>::insert(&staker, ledger);

            Self::deposit_event(Event::<T>::RewardDestination(staker, reward_destination));
            Ok(().into())
        }

        /// Used to force set `ContractEraStake` storage values.
        /// The purpose of this call is only for fixing one of the issues detected with dapps-staking.
        ///
        /// The dispatch origin must be Root.
        #[pallet::call_index(12)]
        #[pallet::weight(T::DbWeight::get().writes(1))]
        pub fn set_contract_stake_info(
            origin: OriginFor<T>,
            contract: T::SmartContract,
            era: EraIndex,
            contract_stake_info: ContractStakeInfo<BalanceOf<T>>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            ContractEraStake::<T>::insert(contract, era, contract_stake_info);

            Ok(().into())
        }

        /// Used to burn unclaimed & stale rewards from an unregistered contract.
        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::claim_dapp())]
        pub fn burn_stale_reward(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            #[pallet::compact] era: EraIndex,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            ensure_root(origin)?;

            let dapp_info =
                RegisteredDapps::<T>::get(&contract_id).ok_or(Error::<T>::NotOperatedContract)?;
            ensure!(
                dapp_info.is_unregistered(),
                Error::<T>::NotUnregisteredContract
            );

            let current_era = Self::current_era();

            let burn_era_limit =
                current_era.saturating_sub(T::UnregisteredDappRewardRetention::get());
            ensure!(era < burn_era_limit, Error::<T>::EraOutOfBounds);

            let mut contract_stake_info =
                Self::contract_stake_info(&contract_id, era).unwrap_or_default();

            let dapp_reward = Self::calculate_dapp_reward(&contract_stake_info, &dapp_info, era)?;

            // Withdraw reward funds from the dapps staking pot and burn them
            let imbalance_to_burn = T::Currency::withdraw(
                &Self::account_id(),
                dapp_reward,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::AllowDeath,
            )?;
            mem::drop(imbalance_to_burn);

            // mark entry as `claimed` but it means it's just handled (want to avoid rename since pallet will soon be redesigned).
            contract_stake_info.contract_reward_claimed = true;
            ContractEraStake::<T>::insert(&contract_id, era, contract_stake_info);

            Self::deposit_event(Event::<T>::StaleRewardBurned(
                dapp_info.developer,
                contract_id.clone(),
                era,
                dapp_reward,
            ));

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Calculate the dApp reward for the specified era.
        /// If successfull, returns reward amount.
        /// In case reward cannot be claimed or was already claimed, an error is raised.
        fn calculate_dapp_reward(
            contract_stake_info: &ContractStakeInfo<BalanceOf<T>>,
            dapp_info: &DAppInfo<T::AccountId>,
            era: EraIndex,
        ) -> Result<BalanceOf<T>, Error<T>> {
            let current_era = Self::current_era();
            if let DAppState::Unregistered(unregister_era) = dapp_info.state {
                ensure!(era < unregister_era, Error::<T>::NotOperatedContract);
            }
            ensure!(era < current_era, Error::<T>::EraOutOfBounds);

            ensure!(
                !contract_stake_info.contract_reward_claimed,
                Error::<T>::AlreadyClaimedInThisEra,
            );
            ensure!(
                contract_stake_info.total > Zero::zero(),
                Error::<T>::NotStakedContract,
            );

            let reward_and_stake =
                Self::general_era_info(era).ok_or(Error::<T>::UnknownEraReward)?;

            // Calculate the contract reward for this era.
            let (dapp_reward, _) = Self::dev_stakers_split(&contract_stake_info, &reward_and_stake);

            Ok(dapp_reward)
        }

        /// An utility method used to stake specified amount on an arbitrary contract.
        ///
        /// `StakerInfo` and `ContractStakeInfo` are provided and all checks are made to ensure that it's possible to
        /// complete staking operation.
        ///
        /// # Arguments
        ///
        /// * `staker_info` - info about staker's stakes on the contract up to current moment
        /// * `staking_info` - general info about contract stakes up to current moment
        /// * `value` - value which is being bonded & staked
        /// * `current_era` - current dapps-staking era
        ///
        /// # Returns
        ///
        /// If stake operation was successful, given structs are properly modified.
        /// If not, an error is returned and structs are left in an undefined state.
        ///
        fn stake_on_contract(
            staker_info: &mut StakerInfo<BalanceOf<T>>,
            staking_info: &mut ContractStakeInfo<BalanceOf<T>>,
            value: BalanceOf<T>,
            current_era: EraIndex,
        ) -> Result<(), Error<T>> {
            ensure!(
                !staker_info.latest_staked_value().is_zero()
                    || staking_info.number_of_stakers < T::MaxNumberOfStakersPerContract::get(),
                Error::<T>::MaxNumberOfStakersExceeded
            );
            if staker_info.latest_staked_value().is_zero() {
                staking_info.number_of_stakers = staking_info.number_of_stakers.saturating_add(1);
            }

            staker_info
                .stake(current_era, value)
                .map_err(|_| Error::<T>::UnexpectedStakeInfoEra)?;
            ensure!(
                // One spot should remain for compounding reward claim call
                staker_info.len() < T::MaxEraStakeValues::get(),
                Error::<T>::TooManyEraStakeValues
            );
            ensure!(
                staker_info.latest_staked_value() >= T::MinimumStakingAmount::get(),
                Error::<T>::InsufficientValue,
            );

            // Increment ledger and total staker value for contract.
            staking_info.total = staking_info.total.saturating_add(value);

            Ok(())
        }

        /// An utility method used to unstake specified amount from an arbitrary contract.
        ///
        /// The amount unstaked can be different in case staked amount would fall bellow `MinimumStakingAmount`.
        /// In that case, entire staked amount will be unstaked.
        ///
        /// `StakerInfo` and `ContractStakeInfo` are provided and all checks are made to ensure that it's possible to
        /// complete unstake operation.
        ///
        /// # Arguments
        ///
        /// * `staker_info` - info about staker's stakes on the contract up to current moment
        /// * `staking_info` - general info about contract stakes up to current moment
        /// * `value` - value which should be unstaked
        /// * `current_era` - current dapps-staking era
        ///
        /// # Returns
        ///
        /// If unstake operation was successful, given structs are properly modified and total unstaked value is returned.
        /// If not, an error is returned and structs are left in an undefined state.
        ///
        fn unstake_from_contract(
            staker_info: &mut StakerInfo<BalanceOf<T>>,
            contract_stake_info: &mut ContractStakeInfo<BalanceOf<T>>,
            value: BalanceOf<T>,
            current_era: EraIndex,
        ) -> Result<BalanceOf<T>, Error<T>> {
            let staked_value = staker_info.latest_staked_value();
            ensure!(staked_value > Zero::zero(), Error::<T>::NotStakedContract);

            // Calculate the value which will be unstaked.
            let remaining = staked_value.saturating_sub(value);
            let value_to_unstake = if remaining < T::MinimumStakingAmount::get() {
                contract_stake_info.number_of_stakers =
                    contract_stake_info.number_of_stakers.saturating_sub(1);
                staked_value
            } else {
                value
            };
            contract_stake_info.total = contract_stake_info.total.saturating_sub(value_to_unstake);

            // Sanity check
            ensure!(
                value_to_unstake > Zero::zero(),
                Error::<T>::UnstakingWithNoValue
            );

            staker_info
                .unstake(current_era, value_to_unstake)
                .map_err(|_| Error::<T>::UnexpectedStakeInfoEra)?;
            ensure!(
                // One spot should remain for compounding reward claim call
                staker_info.len() < T::MaxEraStakeValues::get(),
                Error::<T>::TooManyEraStakeValues
            );

            Ok(value_to_unstake)
        }

        /// Get AccountId assigned to the pallet.
        pub(crate) fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        /// `Err` if pallet disabled for maintenance, `Ok` otherwise
        pub fn ensure_pallet_enabled() -> Result<(), Error<T>> {
            if PalletDisabled::<T>::get() {
                Err(Error::<T>::Disabled)
            } else {
                Ok(())
            }
        }

        /// Update the ledger for a staker. This will also update the stash lock.
        /// This lock will lock the entire funds except paying for further transactions.
        fn update_ledger(staker: &T::AccountId, ledger: AccountLedger<BalanceOf<T>>) {
            if ledger.is_empty() {
                Ledger::<T>::remove(&staker);
                T::Currency::remove_lock(STAKING_ID, staker);
            } else {
                T::Currency::set_lock(STAKING_ID, staker, ledger.locked, WithdrawReasons::all());
                Ledger::<T>::insert(staker, ledger);
            }
        }

        /// Update the staker info for the `(staker, contract_id)` pairing.
        /// If staker_info is empty, remove it from the DB. Otherwise, store it.
        fn update_staker_info(
            staker: &T::AccountId,
            contract_id: &T::SmartContract,
            staker_info: StakerInfo<BalanceOf<T>>,
        ) {
            if staker_info.is_empty() {
                GeneralStakerInfo::<T>::remove(staker, contract_id)
            } else {
                GeneralStakerInfo::<T>::insert(staker, contract_id, staker_info)
            }
        }

        /// The block rewards are accumulated on the pallets's account during an era.
        /// This function takes a snapshot of the pallet's balance accrued during current era
        /// and stores it for future distribution
        ///
        /// This is called just at the beginning of an era.
        fn reward_balance_snapshot(era: EraIndex, rewards: RewardInfo<BalanceOf<T>>) {
            // Get the reward and stake information for previous era
            let mut era_info = Self::general_era_info(era).unwrap_or_default();

            // Prepare info for the next era
            GeneralEraInfo::<T>::insert(
                era + 1,
                EraInfo {
                    rewards: Default::default(),
                    staked: era_info.staked,
                    locked: era_info.locked,
                },
            );

            // Set the reward for the previous era.
            era_info.rewards = rewards;

            GeneralEraInfo::<T>::insert(era, era_info);
        }

        /// Used to copy all `ContractStakeInfo` from the ending era over to the next era.
        /// This is the most primitive solution since it scales with number of dApps.
        /// It is possible to provide a hybrid solution which allows laziness but also prevents
        /// a situation where we don't have access to the required data.
        fn rotate_staking_info(current_era: EraIndex) -> Weight {
            let next_era = current_era + 1;

            let mut consumed_weight = Weight::zero();

            for (contract_id, dapp_info) in RegisteredDapps::<T>::iter() {
                // Ignore dapp if it was unregistered
                consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads(1));
                if let DAppState::Unregistered(_) = dapp_info.state {
                    continue;
                }

                // Copy data from era `X` to era `X + 1`
                if let Some(mut staking_info) = Self::contract_stake_info(&contract_id, current_era)
                {
                    staking_info.contract_reward_claimed = false;
                    ContractEraStake::<T>::insert(&contract_id, next_era, staking_info);

                    consumed_weight =
                        consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
                } else {
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads(1));
                }
            }

            consumed_weight
        }

        /// Returns available staking balance for the potential staker
        fn available_staking_balance(
            staker: &T::AccountId,
            ledger: &AccountLedger<BalanceOf<T>>,
        ) -> BalanceOf<T> {
            // Ensure that staker has enough balance to bond & stake.
            let free_balance =
                T::Currency::free_balance(staker).saturating_sub(T::MinimumRemainingAmount::get());

            // Remove already locked funds from the free balance
            free_balance.saturating_sub(ledger.locked)
        }

        /// `true` if contract is active, `false` if it has been unregistered
        fn is_active(contract_id: &T::SmartContract) -> bool {
            RegisteredDapps::<T>::get(contract_id)
                .map_or(false, |dapp_info| dapp_info.state == DAppState::Registered)
        }

        /// `true` if all the conditions for restaking the reward have been met, `false` otherwise
        pub(crate) fn should_restake_reward(
            reward_destination: RewardDestination,
            dapp_state: DAppState,
            latest_staked_value: BalanceOf<T>,
        ) -> bool {
            reward_destination == RewardDestination::StakeBalance
                && dapp_state == DAppState::Registered
                && latest_staked_value > Zero::zero()
        }

        /// Calculate reward split between developer and stakers.
        ///
        /// Returns (developer reward, joint stakers reward)
        pub(crate) fn dev_stakers_split(
            contract_info: &ContractStakeInfo<BalanceOf<T>>,
            era_info: &EraInfo<BalanceOf<T>>,
        ) -> (BalanceOf<T>, BalanceOf<T>) {
            let contract_stake_portion =
                Perbill::from_rational(contract_info.total, era_info.staked);

            let developer_reward_part = contract_stake_portion * era_info.rewards.dapps;
            let stakers_joint_reward = contract_stake_portion * era_info.rewards.stakers;

            (developer_reward_part, stakers_joint_reward)
        }

        /// Adds `stakers` and `dapps` rewards to the reward pool.
        ///
        /// - `stakers` - portion of the reward that will be distributed to stakers
        /// - `dapps` - portion of the reward that will be distributed to dapps
        pub fn rewards(stakers: NegativeImbalanceOf<T>, dapps: NegativeImbalanceOf<T>) {
            BlockRewardAccumulator::<T>::mutate(|accumulated_reward| {
                accumulated_reward.dapps = accumulated_reward.dapps.saturating_add(dapps.peek());
                accumulated_reward.stakers =
                    accumulated_reward.stakers.saturating_add(stakers.peek());
            });

            T::Currency::resolve_creating(&Self::account_id(), stakers.merge(dapps));
        }

        /// Returns total value locked by dapps-staking.
        ///
        /// Note that this can differ from _total staked value_ since some funds might be undergoing the unbonding period.
        pub fn tvl() -> BalanceOf<T> {
            let current_era = Self::current_era();
            if let Some(era_info) = Self::general_era_info(current_era) {
                era_info.locked
            } else {
                // Should never happen since era info for current era must always exist
                Zero::zero()
            }
        }
    }
}
