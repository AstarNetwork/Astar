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

//! # dApp Staking v3 Pallet
//!
//! - [`Config`]
//!
//! ## Overview
//!
//! Pallet that implements dapps staking protocol.
//!
//! <>
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//! <>
//!
//! ### Other
//!
//! <>
//!

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    pallet_prelude::*,
    traits::{Currency, LockIdentifier, LockableCurrency, StorageVersion, WithdrawReasons},
    weights::Weight,
};
use frame_system::pallet_prelude::*;
use sp_runtime::{
    traits::{BadOrigin, Saturating, Zero},
    Perbill,
};

use astar_primitives::Balance;

use crate::types::*;
pub use pallet::*;

#[cfg(test)]
mod test;

mod types;

const STAKING_ID: LockIdentifier = *b"dapstake";

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    /// The current storage version.
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(5);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency used for staking.
        /// TODO: remove usage of deprecated LockableCurrency trait and use the new freeze approach. Might require some renaming of Lock to Freeze :)
        type Currency: LockableCurrency<
            Self::AccountId,
            Moment = Self::BlockNumber,
            Balance = Balance,
        >;

        /// Describes smart contract in the context required by dApp staking.
        type SmartContract: Parameter + Member + MaxEncodedLen;

        /// Privileged origin for managing dApp staking pallet.
        type ManagerOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

        /// Length of a standard era in block numbers.
        #[pallet::constant]
        type StandardEraLength: Get<Self::BlockNumber>;

        /// Length of the `Voting` period in standard eras.
        /// Although `Voting` period only consumes one 'era', we still measure its length in standard eras
        /// for the sake of simplicity & consistency.
        #[pallet::constant]
        type StandardErasPerVotingPeriod: Get<EraNumber>;

        /// Length of the `Build&Earn` period in standard eras.
        /// Each `Build&Earn` period consists of one or more distinct standard eras.
        #[pallet::constant]
        type StandardErasPerBuildAndEarnPeriod: Get<EraNumber>;

        /// Maximum length of a single era reward span length entry.
        #[pallet::constant]
        type EraRewardSpanLength: Get<u32>;

        /// Number of periods for which we keep rewards available for claiming.
        /// After that period, they are no longer claimable.
        #[pallet::constant]
        type RewardRetentionInPeriods: Get<PeriodNumber>;

        /// Maximum number of contracts that can be integrated into dApp staking at once.
        #[pallet::constant]
        type MaxNumberOfContracts: Get<DAppId>;

        /// Maximum number of unlocking chunks that can exist per account at a time.
        #[pallet::constant]
        type MaxUnlockingChunks: Get<u32>;

        /// Minimum amount an account has to lock in dApp staking in order to participate.
        #[pallet::constant]
        type MinimumLockedAmount: Get<Balance>;

        /// Amount of blocks that need to pass before unlocking chunks can be claimed by the owner.
        #[pallet::constant]
        type UnlockingPeriod: Get<BlockNumberFor<Self>>;

        /// Minimum amount staker can stake on a contract.
        #[pallet::constant]
        type MinimumStakeAmount: Get<Balance>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New era has started.
        NewEra { era: EraNumber },
        /// New period has started.
        NewPeriod {
            period_type: PeriodType,
            number: PeriodNumber,
        },
        /// A smart contract has been registered for dApp staking
        DAppRegistered {
            owner: T::AccountId,
            smart_contract: T::SmartContract,
            dapp_id: DAppId,
        },
        /// dApp reward destination has been updated.
        DAppRewardDestinationUpdated {
            smart_contract: T::SmartContract,
            beneficiary: Option<T::AccountId>,
        },
        /// dApp owner has been changed.
        DAppOwnerChanged {
            smart_contract: T::SmartContract,
            new_owner: T::AccountId,
        },
        /// dApp has been unregistered
        DAppUnregistered {
            smart_contract: T::SmartContract,
            era: EraNumber,
        },
        /// Account has locked some amount into dApp staking.
        Locked {
            account: T::AccountId,
            amount: Balance,
        },
        /// Account has started the unlocking process for some amount.
        Unlocking {
            account: T::AccountId,
            amount: Balance,
        },
        /// Account has claimed unlocked amount, removing the lock from it.
        ClaimedUnlocked {
            account: T::AccountId,
            amount: Balance,
        },
        /// Account has relocked all of the unlocking chunks.
        Relock {
            account: T::AccountId,
            amount: Balance,
        },
        /// Account has staked some amount on a smart contract.
        Stake {
            account: T::AccountId,
            smart_contract: T::SmartContract,
            amount: Balance,
        },
        /// Account has unstaked some amount from a smart contract.
        Unstake {
            account: T::AccountId,
            smart_contract: T::SmartContract,
            amount: Balance,
        },
        /// Account has claimed some stake rewards.
        Reward {
            account: T::AccountId,
            era: EraNumber,
            amount: Balance,
        },
        BonusReward {
            account: T::AccountId,
            period: PeriodNumber,
            amount: Balance,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Pallet is disabled/in maintenance mode.
        Disabled,
        /// Smart contract already exists within dApp staking protocol.
        ContractAlreadyExists,
        /// Maximum number of smart contracts has been reached.
        ExceededMaxNumberOfContracts,
        /// Not possible to assign a new dApp Id.
        /// This should never happen since current type can support up to 65536 - 1 unique dApps.
        NewDAppIdUnavailable,
        /// Specified smart contract does not exist in dApp staking.
        ContractNotFound,
        /// Call origin is not dApp owner.
        OriginNotOwner,
        /// dApp is part of dApp staking but isn't active anymore.
        NotOperatedDApp,
        /// Performing locking or staking with 0 amount.
        ZeroAmount,
        /// Total locked amount for staker is below minimum threshold.
        LockedAmountBelowThreshold,
        /// Cannot add additional unlocking chunks due to capacity limit.
        TooManyUnlockingChunks,
        /// Remaining stake prevents entire balance of starting the unlocking process.
        RemainingStakePreventsFullUnlock,
        /// There are no eligible unlocked chunks to claim. This can happen either if no eligible chunks exist, or if user has no chunks at all.
        NoUnlockedChunksToClaim,
        /// There are no unlocking chunks available to relock.
        NoUnlockingChunks,
        /// The amount being staked is too large compared to what's available for staking.
        UnavailableStakeFunds,
        /// There are unclaimed rewards remaining from past periods. They should be claimed before staking again.
        UnclaimedRewardsFromPastPeriods,
        /// An unexpected error occured while trying to stake.
        InternalStakeError,
        /// Total staked amount on contract is below the minimum required value.
        InsufficientStakeAmount,
        /// Stake operation is rejected since period ends in the next era.
        PeriodEndsInNextEra,
        /// Unstaking is rejected since the period in which past stake was active has passed.
        UnstakeFromPastPeriod,
        /// Unstake amount is greater than the staked amount.
        UnstakeAmountTooLarge,
        /// Account has no staking information for the contract.
        NoStakingInfo,
        /// An unexpected error occured while trying to unstake.
        InternalUnstakeError,
        /// Rewards are no longer claimable since they are too old.
        StakerRewardsExpired,
        /// There are no claimable rewards for the account.
        NoClaimableRewards,
        /// An unexpected error occured while trying to claim staker rewards.
        InternalClaimStakerError,
        /// Bonus rewards have already been claimed.
        BonusRewardAlreadyClaimed,
        /// Account is has no eligible stake amount for bonus reward.
        NotEligibleForBonusReward,
        /// An unexpected error occured while trying to claim bonus reward.
        InternalClaimBonusError,
    }

    /// General information about dApp staking protocol state.
    #[pallet::storage]
    pub type ActiveProtocolState<T: Config> =
        StorageValue<_, ProtocolState<BlockNumberFor<T>>, ValueQuery>;

    /// Counter for unique dApp identifiers.
    #[pallet::storage]
    pub type NextDAppId<T: Config> = StorageValue<_, DAppId, ValueQuery>;

    /// Map of all dApps integrated into dApp staking protocol.
    #[pallet::storage]
    pub type IntegratedDApps<T: Config> = CountedStorageMap<
        _,
        Blake2_128Concat,
        T::SmartContract,
        DAppInfo<T::AccountId>,
        OptionQuery,
    >;

    /// General locked/staked information for each account.
    #[pallet::storage]
    pub type Ledger<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, AccountLedgerFor<T>, ValueQuery>;

    /// Information about how much each staker has staked for each smart contract in some period.
    #[pallet::storage]
    pub type StakerInfo<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        T::SmartContract,
        SingularStakingInfo,
        OptionQuery,
    >;

    /// Information about how much has been staked on a smart contract in some era or period.
    #[pallet::storage]
    pub type ContractStake<T: Config> =
        StorageMap<_, Blake2_128Concat, T::SmartContract, ContractStakeAmountSeries, ValueQuery>;

    /// General information about the current era.
    #[pallet::storage]
    pub type CurrentEraInfo<T: Config> = StorageValue<_, EraInfo, ValueQuery>;

    /// Information about rewards for each era.
    ///
    /// Since each entry is a 'span', covering up to `T::EraRewardSpanLength` entries, only certain era value keys can exist in storage.
    /// For the sake of simplicity, valid `era` keys are calculated as:
    ///
    /// era_key = era - (era % T::EraRewardSpanLength)
    ///
    /// This means that e.g. in case `EraRewardSpanLength = 8`, only era values 0, 8, 16, 24, etc. can exist in storage.
    /// Eras 1-7 will be stored in the same entry as era 0, eras 9-15 will be stored in the same entry as era 8, etc.
    #[pallet::storage]
    pub type EraRewards<T: Config> =
        StorageMap<_, Twox64Concat, EraNumber, EraRewardSpan<T::EraRewardSpanLength>, OptionQuery>;

    /// Information about period's end.
    #[pallet::storage]
    pub type PeriodEnd<T: Config> =
        StorageMap<_, Twox64Concat, PeriodNumber, PeriodEndInfo, OptionQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let mut protocol_state = ActiveProtocolState::<T>::get();

            // We should not modify pallet storage while in maintenance mode.
            // This is a safety measure, since maintenance mode is expected to be
            // enabled in case some misbehavior or corrupted storage is detected.
            if protocol_state.maintenance {
                return T::DbWeight::get().reads(1);
            }

            // Nothing to do if it's not new era
            if !protocol_state.is_new_era(now) {
                return T::DbWeight::get().reads(1);
            }

            let mut era_info = CurrentEraInfo::<T>::get();

            let current_era = protocol_state.era;
            let next_era = current_era.saturating_add(1);
            let (maybe_period_event, era_reward) = match protocol_state.period_type() {
                PeriodType::Voting => {
                    // For the sake of consistency, we put zero reward into storage
                    let era_reward =
                        EraReward::new(Balance::zero(), era_info.total_staked_amount());

                    let ending_era =
                        next_era.saturating_add(T::StandardErasPerBuildAndEarnPeriod::get());
                    let build_and_earn_start_block =
                        now.saturating_add(T::StandardEraLength::get());
                    protocol_state.next_period_type(ending_era, build_and_earn_start_block);

                    era_info.migrate_to_next_era(Some(protocol_state.period_type()));

                    (
                        Some(Event::<T>::NewPeriod {
                            period_type: protocol_state.period_type(),
                            number: protocol_state.period_number(),
                        }),
                        era_reward,
                    )
                }
                PeriodType::BuildAndEarn => {
                    // TODO: trigger dAPp tier reward calculation here. This will be implemented later.

                    let staker_reward_pool = Balance::from(1_000_000_000_000u128); // TODO: calculate this properly, inject it from outside (Tokenomics 2.0 pallet?)
                    let era_reward =
                        EraReward::new(staker_reward_pool, era_info.total_staked_amount());

                    // Switch to `Voting` period if conditions are met.
                    if protocol_state.period_info.is_next_period(next_era) {
                        // Store info about period end
                        let bonus_reward_pool = Balance::from(3_000_000_u32); // TODO: get this from Tokenomics 2.0 pallet
                        PeriodEnd::<T>::insert(
                            &protocol_state.period_number(),
                            PeriodEndInfo {
                                bonus_reward_pool,
                                total_vp_stake: era_info.staked_amount(PeriodType::Voting),
                                final_era: current_era,
                            },
                        );

                        // For the sake of consistency we treat the whole `Voting` period as a single era.
                        // This means no special handling is required for this period, it only lasts potentially longer than a single standard era.
                        let ending_era = next_era.saturating_add(1);
                        let voting_period_length = Self::blocks_per_voting_period();
                        let next_era_start_block = now.saturating_add(voting_period_length);

                        protocol_state.next_period_type(ending_era, next_era_start_block);

                        era_info.migrate_to_next_era(Some(protocol_state.period_type()));

                        // TODO: trigger tier configuration calculation based on internal & external params.

                        (
                            Some(Event::<T>::NewPeriod {
                                period_type: protocol_state.period_type(),
                                number: protocol_state.period_number(),
                            }),
                            era_reward,
                        )
                    } else {
                        let next_era_start_block = now.saturating_add(T::StandardEraLength::get());
                        protocol_state.next_era_start = next_era_start_block;

                        era_info.migrate_to_next_era(None);

                        (None, era_reward)
                    }
                }
            };

            // Update storage items

            protocol_state.era = next_era;
            ActiveProtocolState::<T>::put(protocol_state);

            CurrentEraInfo::<T>::put(era_info);

            let era_span_index = Self::era_reward_span_index(current_era);
            let mut span = EraRewards::<T>::get(&era_span_index).unwrap_or(EraRewardSpan::new());
            // TODO: error must not happen here. Log an error if it does.
            // The consequence will be that some rewards will be temporarily lost/unavailable, but nothing protocol breaking.
            // Will require a fix from the runtime team though.
            let _ = span.push(current_era, era_reward);
            EraRewards::<T>::insert(&era_span_index, span);

            Self::deposit_event(Event::<T>::NewEra { era: next_era });
            if let Some(period_event) = maybe_period_event {
                Self::deposit_event(period_event);
            }

            // TODO: benchmark later
            T::DbWeight::get().reads_writes(3, 3)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Used to enable or disable maintenance mode.
        /// Can only be called by manager origin.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::zero())]
        pub fn maintenance_mode(origin: OriginFor<T>, enabled: bool) -> DispatchResult {
            T::ManagerOrigin::ensure_origin(origin)?;
            ActiveProtocolState::<T>::mutate(|state| state.maintenance = enabled);
            Ok(())
        }

        /// Used to register a new contract for dApp staking.
        ///
        /// If successful, smart contract will be assigned a simple, unique numerical identifier.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::zero())]
        pub fn register(
            origin: OriginFor<T>,
            owner: T::AccountId,
            smart_contract: T::SmartContract,
        ) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            T::ManagerOrigin::ensure_origin(origin)?;

            ensure!(
                !IntegratedDApps::<T>::contains_key(&smart_contract),
                Error::<T>::ContractAlreadyExists,
            );

            ensure!(
                IntegratedDApps::<T>::count() < T::MaxNumberOfContracts::get().into(),
                Error::<T>::ExceededMaxNumberOfContracts
            );

            let dapp_id = NextDAppId::<T>::get();
            // MAX value must never be assigned as a dApp Id since it serves as a sentinel value.
            ensure!(dapp_id < DAppId::MAX, Error::<T>::NewDAppIdUnavailable);

            IntegratedDApps::<T>::insert(
                &smart_contract,
                DAppInfo {
                    owner: owner.clone(),
                    id: dapp_id,
                    state: DAppState::Registered,
                    reward_destination: None,
                },
            );

            NextDAppId::<T>::put(dapp_id.saturating_add(1));

            Self::deposit_event(Event::<T>::DAppRegistered {
                owner,
                smart_contract,
                dapp_id,
            });

            Ok(())
        }

        /// Used to modify the reward destination account for a dApp.
        ///
        /// Caller has to be dApp owner.
        /// If set to `None`, rewards will be deposited to the dApp owner.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::zero())]
        pub fn set_dapp_reward_destination(
            origin: OriginFor<T>,
            smart_contract: T::SmartContract,
            beneficiary: Option<T::AccountId>,
        ) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            let dev_account = ensure_signed(origin)?;

            IntegratedDApps::<T>::try_mutate(
                &smart_contract,
                |maybe_dapp_info| -> DispatchResult {
                    let dapp_info = maybe_dapp_info
                        .as_mut()
                        .ok_or(Error::<T>::ContractNotFound)?;

                    ensure!(dapp_info.owner == dev_account, Error::<T>::OriginNotOwner);

                    dapp_info.reward_destination = beneficiary.clone();

                    Ok(())
                },
            )?;

            Self::deposit_event(Event::<T>::DAppRewardDestinationUpdated {
                smart_contract,
                beneficiary,
            });

            Ok(())
        }

        /// Used to change dApp owner.
        ///
        /// Can be called by dApp owner or dApp staking manager origin.
        /// This is useful in two cases:
        /// 1. when the dApp owner account is compromised, manager can change the owner to a new account
        /// 2. if project wants to transfer ownership to a new account (DAO, multisig, etc.).
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::zero())]
        pub fn set_dapp_owner(
            origin: OriginFor<T>,
            smart_contract: T::SmartContract,
            new_owner: T::AccountId,
        ) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            let origin = Self::ensure_signed_or_manager(origin)?;

            IntegratedDApps::<T>::try_mutate(
                &smart_contract,
                |maybe_dapp_info| -> DispatchResult {
                    let dapp_info = maybe_dapp_info
                        .as_mut()
                        .ok_or(Error::<T>::ContractNotFound)?;

                    // If manager origin, `None`, no need to check if caller is the owner.
                    if let Some(caller) = origin {
                        ensure!(dapp_info.owner == caller, Error::<T>::OriginNotOwner);
                    }

                    dapp_info.owner = new_owner.clone();

                    Ok(())
                },
            )?;

            Self::deposit_event(Event::<T>::DAppOwnerChanged {
                smart_contract,
                new_owner,
            });

            Ok(())
        }

        /// Unregister dApp from dApp staking protocol, making it ineligible for future rewards.
        /// This doesn't remove the dApp completely from the system just yet, but it can no longer be used for staking.
        ///
        /// Can be called by dApp owner or dApp staking manager origin.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::zero())]
        pub fn unregister(
            origin: OriginFor<T>,
            smart_contract: T::SmartContract,
        ) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            T::ManagerOrigin::ensure_origin(origin)?;

            let current_era = ActiveProtocolState::<T>::get().era;

            IntegratedDApps::<T>::try_mutate(
                &smart_contract,
                |maybe_dapp_info| -> DispatchResult {
                    let dapp_info = maybe_dapp_info
                        .as_mut()
                        .ok_or(Error::<T>::ContractNotFound)?;

                    ensure!(
                        dapp_info.state == DAppState::Registered,
                        Error::<T>::NotOperatedDApp
                    );

                    dapp_info.state = DAppState::Unregistered(current_era);

                    Ok(())
                },
            )?;

            // TODO: might require some modification later on, like additional checks to ensure contract can be unregistered.

            // TODO2: we should remove staked amount from appropriate entries, since contract has been 'invalidated'

            // TODO3: will need to add a call similar to what we have in DSv2, for stakers to 'unstake_from_unregistered_contract'

            Self::deposit_event(Event::<T>::DAppUnregistered {
                smart_contract,
                era: current_era,
            });

            Ok(())
        }

        /// Locks additional funds into dApp staking.
        ///
        /// In case caller account doesn't have sufficient balance to cover the specified amount, everything is locked.
        /// After adjustment, lock amount must be greater than zero and in total must be equal or greater than the minimum locked amount.
        ///
        /// It is possible for call to fail due to caller account already having too many locked balance chunks in storage. To solve this,
        /// caller should claim pending rewards, before retrying to lock additional funds.
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::zero())]
        pub fn lock(origin: OriginFor<T>, #[pallet::compact] amount: Balance) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            let account = ensure_signed(origin)?;

            let mut ledger = Ledger::<T>::get(&account);

            // Calculate & check amount available for locking
            let available_balance =
                T::Currency::free_balance(&account).saturating_sub(ledger.active_locked_amount());
            let amount_to_lock = available_balance.min(amount);
            ensure!(!amount_to_lock.is_zero(), Error::<T>::ZeroAmount);

            ledger.add_lock_amount(amount_to_lock);

            ensure!(
                ledger.active_locked_amount() >= T::MinimumLockedAmount::get(),
                Error::<T>::LockedAmountBelowThreshold
            );

            Self::update_ledger(&account, ledger);
            CurrentEraInfo::<T>::mutate(|era_info| {
                era_info.add_locked(amount_to_lock);
            });

            Self::deposit_event(Event::<T>::Locked {
                account,
                amount: amount_to_lock,
            });

            Ok(())
        }

        /// Attempts to start the unlocking process for the specified amount.
        ///
        /// Only the amount that isn't actively used for staking can be unlocked.
        /// If the amount is greater than the available amount for unlocking, everything is unlocked.
        /// If the remaining locked amount would take the account below the minimum locked amount, everything is unlocked.
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::zero())]
        pub fn unlock(origin: OriginFor<T>, #[pallet::compact] amount: Balance) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            let account = ensure_signed(origin)?;

            let state = ActiveProtocolState::<T>::get();
            let mut ledger = Ledger::<T>::get(&account);

            let available_for_unlocking = ledger.unlockable_amount(state.period_info.number);
            let amount_to_unlock = available_for_unlocking.min(amount);

            // Ensure we unlock everything if remaining amount is below threshold.
            let remaining_amount = ledger
                .active_locked_amount()
                .saturating_sub(amount_to_unlock);
            let amount_to_unlock = if remaining_amount < T::MinimumLockedAmount::get() {
                ensure!(
                    ledger.staked_amount(state.period_info.number).is_zero(),
                    Error::<T>::RemainingStakePreventsFullUnlock
                );
                ledger.active_locked_amount()
            } else {
                amount_to_unlock
            };

            // Sanity check
            ensure!(!amount_to_unlock.is_zero(), Error::<T>::ZeroAmount);

            // Update ledger with new lock and unlocking amounts
            ledger.subtract_lock_amount(amount_to_unlock);

            let current_block = frame_system::Pallet::<T>::block_number();
            let unlock_block = current_block.saturating_add(T::UnlockingPeriod::get());
            ledger
                .add_unlocking_chunk(amount_to_unlock, unlock_block)
                .map_err(|_| Error::<T>::TooManyUnlockingChunks)?;

            // Update storage
            Self::update_ledger(&account, ledger);
            CurrentEraInfo::<T>::mutate(|era_info| {
                era_info.unlocking_started(amount_to_unlock);
            });

            Self::deposit_event(Event::<T>::Unlocking {
                account,
                amount: amount_to_unlock,
            });

            Ok(())
        }

        /// Claims all of fully unlocked chunks, removing the lock from them.
        #[pallet::call_index(7)]
        #[pallet::weight(Weight::zero())]
        pub fn claim_unlocked(origin: OriginFor<T>) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            let account = ensure_signed(origin)?;

            let mut ledger = Ledger::<T>::get(&account);

            let current_block = frame_system::Pallet::<T>::block_number();
            let amount = ledger.claim_unlocked(current_block);
            ensure!(amount > Zero::zero(), Error::<T>::NoUnlockedChunksToClaim);

            Self::update_ledger(&account, ledger);
            CurrentEraInfo::<T>::mutate(|era_info| {
                era_info.unlocking_removed(amount);
            });

            // TODO: We should ensure user doesn't unlock everything if they still have storage leftovers (e.g. unclaimed rewards?)

            // TODO2: to make it more  bounded, we could add a limit to how much distinct stake entries a user can have

            Self::deposit_event(Event::<T>::ClaimedUnlocked { account, amount });

            Ok(())
        }

        #[pallet::call_index(8)]
        #[pallet::weight(Weight::zero())]
        pub fn relock_unlocking(origin: OriginFor<T>) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            let account = ensure_signed(origin)?;

            let mut ledger = Ledger::<T>::get(&account);

            ensure!(!ledger.unlocking.is_empty(), Error::<T>::NoUnlockingChunks);

            let amount = ledger.consume_unlocking_chunks();

            ledger.add_lock_amount(amount);
            ensure!(
                ledger.active_locked_amount() >= T::MinimumLockedAmount::get(),
                Error::<T>::LockedAmountBelowThreshold
            );

            Self::update_ledger(&account, ledger);
            CurrentEraInfo::<T>::mutate(|era_info| {
                era_info.add_locked(amount);
                era_info.unlocking_removed(amount);
            });

            Self::deposit_event(Event::<T>::Relock { account, amount });

            Ok(())
        }

        /// Stake the specified amount on a smart contract.
        /// The `amount` specified **must** be available for staking and meet the required minimum, otherwise the call will fail.
        ///
        /// Depending on the period type, appropriate stake amount will be updated.
        #[pallet::call_index(9)]
        #[pallet::weight(Weight::zero())]
        pub fn stake(
            origin: OriginFor<T>,
            smart_contract: T::SmartContract,
            #[pallet::compact] amount: Balance,
        ) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            let account = ensure_signed(origin)?;

            ensure!(amount > 0, Error::<T>::ZeroAmount);

            ensure!(
                Self::is_active(&smart_contract),
                Error::<T>::NotOperatedDApp
            );

            let protocol_state = ActiveProtocolState::<T>::get();
            let stake_era = protocol_state.era;
            ensure!(
                !protocol_state
                    .period_info
                    .is_next_period(stake_era.saturating_add(1)),
                Error::<T>::PeriodEndsInNextEra
            );

            let mut ledger = Ledger::<T>::get(&account);

            // 1.
            // Increase stake amount for the next era & current period in staker's ledger
            ledger
                .add_stake_amount(amount, stake_era, protocol_state.period_info)
                .map_err(|err| match err {
                    AccountLedgerError::InvalidPeriod | AccountLedgerError::InvalidEra => {
                        Error::<T>::UnclaimedRewardsFromPastPeriods
                    }
                    AccountLedgerError::UnavailableStakeFunds => Error::<T>::UnavailableStakeFunds,
                    // Defensive check, should never happen
                    _ => Error::<T>::InternalStakeError,
                })?;

            // 2.
            // Update `StakerInfo` storage with the new stake amount on the specified contract.
            //
            // There are two distinct scenarios:
            // 1. Existing entry matches the current period number - just update it.
            // 2. Entry doesn't exist or it's for an older period - create a new one.
            //
            // This is ok since we only use this storage entry to keep track of how much each staker
            // has staked on each contract in the current period. We only ever need the latest information.
            // This is because `AccountLedger` is the one keeping information about how much was staked when.
            let new_staking_info = match StakerInfo::<T>::get(&account, &smart_contract) {
                Some(mut staking_info)
                    if staking_info.period_number() == protocol_state.period_number() =>
                {
                    staking_info.stake(amount, protocol_state.period_info.period_type);
                    staking_info
                }
                _ => {
                    ensure!(
                        amount >= T::MinimumStakeAmount::get(),
                        Error::<T>::InsufficientStakeAmount
                    );
                    let mut staking_info = SingularStakingInfo::new(
                        protocol_state.period_info.number,
                        protocol_state.period_info.period_type,
                    );
                    staking_info.stake(amount, protocol_state.period_info.period_type);
                    staking_info
                }
            };

            // 3.
            // Update `ContractStake` storage with the new stake amount on the specified contract.
            let mut contract_stake_info = ContractStake::<T>::get(&smart_contract);
            ensure!(
                contract_stake_info
                    .stake(amount, protocol_state.period_info, stake_era)
                    .is_ok(),
                Error::<T>::InternalStakeError
            );

            // 4.
            // Update total staked amount for the next era.
            CurrentEraInfo::<T>::mutate(|era_info| {
                era_info.add_stake_amount(amount, protocol_state.period_type());
            });

            // 5.
            // Update remaining storage entries
            Self::update_ledger(&account, ledger);
            StakerInfo::<T>::insert(&account, &smart_contract, new_staking_info);
            ContractStake::<T>::insert(&smart_contract, contract_stake_info);

            Self::deposit_event(Event::<T>::Stake {
                account,
                smart_contract,
                amount,
            });

            Ok(())
        }

        /// Unstake the specified amount from a smart contract.
        /// The `amount` specified **must** not exceed what's staked, otherwise the call will fail.
        ///
        /// Depending on the period type, appropriate stake amount will be updated.
        #[pallet::call_index(10)]
        #[pallet::weight(Weight::zero())]
        pub fn unstake(
            origin: OriginFor<T>,
            smart_contract: T::SmartContract,
            #[pallet::compact] amount: Balance,
        ) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            let account = ensure_signed(origin)?;

            ensure!(amount > 0, Error::<T>::ZeroAmount);

            ensure!(
                Self::is_active(&smart_contract),
                Error::<T>::NotOperatedDApp
            );

            let protocol_state = ActiveProtocolState::<T>::get();
            let unstake_era = protocol_state.era;

            let mut ledger = Ledger::<T>::get(&account);

            // 1.
            // Update `StakerInfo` storage with the reduced stake amount on the specified contract.
            let (new_staking_info, amount) = match StakerInfo::<T>::get(&account, &smart_contract) {
                Some(mut staking_info) => {
                    ensure!(
                        staking_info.period_number() == protocol_state.period_number(),
                        Error::<T>::UnstakeFromPastPeriod
                    );
                    ensure!(
                        staking_info.total_staked_amount() >= amount,
                        Error::<T>::UnstakeAmountTooLarge
                    );

                    // If unstaking would take the total staked amount below the minimum required value,
                    // unstake everything.
                    let amount = if staking_info.total_staked_amount().saturating_sub(amount)
                        < T::MinimumStakeAmount::get()
                    {
                        staking_info.total_staked_amount()
                    } else {
                        amount
                    };

                    staking_info.unstake(amount, protocol_state.period_type());
                    (staking_info, amount)
                }
                None => {
                    return Err(Error::<T>::NoStakingInfo.into());
                }
            };

            // 2.
            // Reduce stake amount
            ledger
                .unstake_amount(amount, unstake_era, protocol_state.period_info)
                .map_err(|err| match err {
                    AccountLedgerError::InvalidPeriod | AccountLedgerError::InvalidEra => {
                        Error::<T>::UnclaimedRewardsFromPastPeriods
                    }
                    AccountLedgerError::UnstakeAmountLargerThanStake => {
                        Error::<T>::UnstakeAmountTooLarge
                    }
                    _ => Error::<T>::InternalUnstakeError,
                })?;

            // 3.
            // Update `ContractStake` storage with the reduced stake amount on the specified contract.
            let mut contract_stake_info = ContractStake::<T>::get(&smart_contract);
            ensure!(
                contract_stake_info
                    .unstake(amount, protocol_state.period_info, unstake_era)
                    .is_ok(),
                Error::<T>::InternalUnstakeError
            );

            // 4.
            // Update total staked amount for the next era.
            CurrentEraInfo::<T>::mutate(|era_info| {
                era_info.unstake_amount(amount, protocol_state.period_type());
            });

            // 5.
            // Update remaining storage entries
            Self::update_ledger(&account, ledger);
            ContractStake::<T>::insert(&smart_contract, contract_stake_info);

            if new_staking_info.is_empty() {
                StakerInfo::<T>::remove(&account, &smart_contract);
            } else {
                StakerInfo::<T>::insert(&account, &smart_contract, new_staking_info);
            }

            Self::deposit_event(Event::<T>::Unstake {
                account,
                smart_contract,
                amount,
            });

            Ok(())
        }

        /// TODO
        #[pallet::call_index(11)]
        #[pallet::weight(Weight::zero())]
        pub fn claim_staker_rewards(origin: OriginFor<T>) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            let account = ensure_signed(origin)?;

            let protocol_state = ActiveProtocolState::<T>::get();
            let mut ledger = Ledger::<T>::get(&account);

            // TODO: how do we handle expired rewards? Add an additional call to clean them up?
            // Putting this logic inside existing calls will add even more complexity.

            ensure!(
                !ledger.staker_rewards_claimed,
                Error::<T>::NoClaimableRewards
            ); // TODO: maybe different error type?
               // Check if the rewards have expired
            let staked_period = ledger
                .staked_period()
                .ok_or(Error::<T>::NoClaimableRewards)?;
            ensure!(
                staked_period
                    >= protocol_state
                        .period_number()
                        .saturating_sub(T::RewardRetentionInPeriods::get()),
                Error::<T>::StakerRewardsExpired
            );

            // Calculate the reward claim span
            let earliest_staked_era = ledger
                .earliest_staked_era()
                .ok_or(Error::<T>::InternalClaimStakerError)?;
            let era_rewards =
                EraRewards::<T>::get(Self::era_reward_span_index(earliest_staked_era))
                    .ok_or(Error::<T>::NoClaimableRewards)?;

            // The last era for which we can theoretically claim rewards.
            // And indicator if we know the period's ending era.
            let (last_period_era, period_end) = if staked_period == protocol_state.period_number() {
                (protocol_state.era.saturating_sub(1), None)
            } else {
                PeriodEnd::<T>::get(&staked_period)
                    .map(|info| (info.final_era, Some(info.final_era)))
                    .ok_or(Error::<T>::InternalClaimStakerError)?
            };

            // The last era for which we can claim rewards for this account.
            let last_claim_era = era_rewards.last_era().min(last_period_era);

            // Get chunks for reward claiming
            let rewards_iter =
                ledger
                    .claim_up_to_era(last_claim_era, period_end)
                    .map_err(|err| match err {
                        AccountLedgerError::NothingToClaim => Error::<T>::NoClaimableRewards,
                        _ => Error::<T>::InternalClaimStakerError,
                    })?;

            // Calculate rewards
            let mut rewards: Vec<_> = Vec::new();
            let mut reward_sum = Balance::zero();
            for (era, amount) in rewards_iter {
                // TODO: this should be zipped, and values should be fetched only once
                let era_reward = era_rewards
                    .get(era)
                    .ok_or(Error::<T>::InternalClaimStakerError)?;

                // Optimization, and zero-division protection
                if amount.is_zero() || era_reward.staked().is_zero() {
                    continue;
                }
                let staker_reward = Perbill::from_rational(amount, era_reward.staked())
                    * era_reward.staker_reward_pool();

                rewards.push((era, staker_reward));
                reward_sum.saturating_accrue(staker_reward);
            }

            // TODO: add negative test for this?
            T::Currency::deposit_into_existing(&account, reward_sum)
                .map_err(|_| Error::<T>::InternalClaimStakerError)?;

            Self::update_ledger(&account, ledger);

            rewards.into_iter().for_each(|(era, reward)| {
                Self::deposit_event(Event::<T>::Reward {
                    account: account.clone(),
                    era,
                    amount: reward,
                });
            });

            Ok(())
        }

        /// TODO
        #[pallet::call_index(12)]
        #[pallet::weight(Weight::zero())]
        pub fn claim_bonus_reward(origin: OriginFor<T>) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            let account = ensure_signed(origin)?;

            let protocol_state = ActiveProtocolState::<T>::get();
            let mut ledger = Ledger::<T>::get(&account);

            ensure!(
                !ledger.staker_rewards_claimed,
                Error::<T>::BonusRewardAlreadyClaimed
            );
            // Check if the rewards have expired
            let staked_period = ledger
                .staked_period()
                .ok_or(Error::<T>::NoClaimableRewards)?;
            ensure!(
                staked_period
                    >= protocol_state
                        .period_number()
                        .saturating_sub(T::RewardRetentionInPeriods::get()),
                Error::<T>::StakerRewardsExpired
            );

            // Check if period has ended
            ensure!(
                staked_period < protocol_state.period_number(),
                Error::<T>::NoClaimableRewards
            );

            // Check if user is applicable for bonus reward
            let eligible_amount =
                ledger
                    .claim_bonus_reward(staked_period)
                    .map_err(|err| match err {
                        AccountLedgerError::NothingToClaim => Error::<T>::NoClaimableRewards,
                        _ => Error::<T>::InternalClaimBonusError,
                    })?;
            ensure!(
                !eligible_amount.is_zero(),
                Error::<T>::NotEligibleForBonusReward
            );

            let period_end_info =
                PeriodEnd::<T>::get(&staked_period).ok_or(Error::<T>::InternalClaimBonusError)?;
            // Defensive check, situation should never happen.
            ensure!(
                !period_end_info.total_vp_stake.is_zero(),
                Error::<T>::InternalClaimBonusError
            );

            let bonus_reward =
                Perbill::from_rational(eligible_amount, period_end_info.total_vp_stake)
                    * period_end_info.bonus_reward_pool;

            // TODO: add negative test for this?
            T::Currency::deposit_into_existing(&account, bonus_reward)
                .map_err(|_| Error::<T>::InternalClaimStakerError)?;

            Self::update_ledger(&account, ledger);

            Self::deposit_event(Event::<T>::BonusReward {
                account: account.clone(),
                period: staked_period,
                amount: bonus_reward,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// `Err` if pallet disabled for maintenance, `Ok` otherwise.
        pub(crate) fn ensure_pallet_enabled() -> Result<(), Error<T>> {
            if ActiveProtocolState::<T>::get().maintenance {
                Err(Error::<T>::Disabled)
            } else {
                Ok(())
            }
        }

        /// Ensure that the origin is either the `ManagerOrigin` or a signed origin.
        ///
        /// In case of manager, `Ok(None)` is returned, and if signed origin `Ok(Some(AccountId))` is returned.
        pub(crate) fn ensure_signed_or_manager(
            origin: T::RuntimeOrigin,
        ) -> Result<Option<T::AccountId>, BadOrigin> {
            if T::ManagerOrigin::ensure_origin(origin.clone()).is_ok() {
                return Ok(None);
            }
            let who = ensure_signed(origin)?;
            Ok(Some(who))
        }

        /// Update the account ledger, and dApp staking balance lock.
        ///
        /// In case account ledger is empty, entries from the DB are removed and lock is released.
        pub(crate) fn update_ledger(account: &T::AccountId, ledger: AccountLedgerFor<T>) {
            if ledger.is_empty() {
                Ledger::<T>::remove(&account);
                T::Currency::remove_lock(STAKING_ID, account);
            } else {
                T::Currency::set_lock(
                    STAKING_ID,
                    account,
                    ledger.active_locked_amount(),
                    WithdrawReasons::all(),
                );
                Ledger::<T>::insert(account, ledger);
            }
        }

        /// Returns the number of blocks per voting period.
        pub(crate) fn blocks_per_voting_period() -> BlockNumberFor<T> {
            T::StandardEraLength::get().saturating_mul(T::StandardErasPerVotingPeriod::get().into())
        }

        /// `true` if smart contract is active, `false` if it has been unregistered.
        fn is_active(smart_contract: &T::SmartContract) -> bool {
            IntegratedDApps::<T>::get(smart_contract)
                .map_or(false, |dapp_info| dapp_info.state == DAppState::Registered)
        }

        /// Calculates the `EraRewardSpan` index for the specified era.
        pub fn era_reward_span_index(era: EraNumber) -> EraNumber {
            era.saturating_sub(era % T::EraRewardSpanLength::get())
        }
    }
}
