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
//! TODO
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
    traits::{BadOrigin, One, Saturating, Zero},
    Perbill, Permill,
};
pub use sp_std::vec::Vec;

use astar_primitives::Balance;

pub use pallet::*;

#[cfg(test)]
mod test;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod types;
use types::*;
pub use types::{PriceProvider, RewardPoolProvider, TierThreshold};

mod dsv3_weight;

const STAKING_ID: LockIdentifier = *b"dapstake";

// TODO: add tracing!

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    /// The current storage version.
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(5);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[cfg(feature = "runtime-benchmarks")]
    pub trait BenchmarkHelper<SmartContract> {
        fn get_smart_contract(id: u32) -> SmartContract;
    }

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

        /// Used to provide price information about the native token.
        type NativePriceProvider: PriceProvider;

        /// Used to provide reward pools amount.
        type RewardPoolProvider: RewardPoolProvider;

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

        /// Number of standard eras that need to pass before unlocking chunk can be claimed.
        /// Even though it's expressed in 'eras', it's actually measured in number of blocks.
        #[pallet::constant]
        type UnlockingPeriod: Get<EraNumber>;

        /// Maximum amount of stake entries contract is allowed to have at once.
        #[pallet::constant]
        type MaxNumberOfStakedContracts: Get<u32>;

        /// Minimum amount staker can stake on a contract.
        #[pallet::constant]
        type MinimumStakeAmount: Get<Balance>;

        /// Number of different tiers.
        #[pallet::constant]
        type NumberOfTiers: Get<u32>;

        /// Helper trait for benchmarks.
        #[cfg(feature = "runtime-benchmarks")]
        type BenchmarkHelper: BenchmarkHelper<Self::SmartContract>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New era has started.
        NewEra { era: EraNumber },
        /// New period has started.
        NewPeriod {
            subperiod: Subperiod,
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
            smart_contract: T::SmartContract,
            period: PeriodNumber,
            amount: Balance,
        },
        DAppReward {
            beneficiary: T::AccountId,
            smart_contract: T::SmartContract,
            tier_id: TierId,
            era: EraNumber,
            amount: Balance,
        },
        UnstakeFromUnregistered {
            account: T::AccountId,
            smart_contract: T::SmartContract,
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
        RewardExpired,
        /// There are no claimable rewards.
        NoClaimableRewards,
        /// An unexpected error occured while trying to claim staker rewards.
        InternalClaimStakerError,
        /// Account is has no eligible stake amount for bonus reward.
        NotEligibleForBonusReward,
        /// An unexpected error occured while trying to claim bonus reward.
        InternalClaimBonusError,
        /// Claim era is invalid - it must be in history, and rewards must exist for it.
        InvalidClaimEra,
        /// No dApp tier info exists for the specified era. This can be because era has expired
        /// or because during the specified era there were no eligible rewards or protocol wasn't active.
        NoDAppTierInfo,
        /// dApp reward has already been claimed for this era.
        DAppRewardAlreadyClaimed,
        /// An unexpected error occured while trying to claim dApp reward.
        InternalClaimDAppError,
        /// Contract is still active, not unregistered.
        ContractStillActive,
        /// There are too many contract stake entries for the account. This can be cleaned up by either unstaking or cleaning expired entries.
        TooManyStakedContracts,
    }

    /// General information about dApp staking protocol state.
    #[pallet::storage]
    pub type ActiveProtocolState<T: Config> =
        StorageValue<_, ProtocolState<BlockNumberFor<T>>, ValueQuery>;

    /// Counter for unique dApp identifiers.
    #[pallet::storage]
    pub type NextDAppId<T: Config> = StorageValue<_, DAppId, ValueQuery>;

    // TODO: where to track TierLabels? E.g. a label to bootstrap a dApp into a specific tier.
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
        StorageMap<_, Blake2_128Concat, T::SmartContract, ContractStakeAmount, ValueQuery>;

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

    /// Static tier parameters used to calculate tier configuration.
    #[pallet::storage]
    pub type StaticTierParams<T: Config> =
        StorageValue<_, TierParameters<T::NumberOfTiers>, ValueQuery>;

    /// Tier configuration to be used during the newly started period
    #[pallet::storage]
    pub type NextTierConfig<T: Config> =
        StorageValue<_, TiersConfiguration<T::NumberOfTiers>, ValueQuery>;

    /// Tier configuration user for current & preceding eras.
    #[pallet::storage]
    pub type TierConfig<T: Config> =
        StorageValue<_, TiersConfiguration<T::NumberOfTiers>, ValueQuery>;

    /// Information about which tier a dApp belonged to in a specific era.
    #[pallet::storage]
    pub type DAppTiers<T: Config> =
        StorageMap<_, Twox64Concat, EraNumber, DAppTierRewardsFor<T>, OptionQuery>;

    // TODO: this is experimental, please don't review
    #[pallet::storage]
    pub type ExperimentalContractEntries<T: Config> =
        StorageMap<_, Twox64Concat, EraNumber, ContractEntriesFor<T>, OptionQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig {
        pub reward_portion: Vec<Permill>,
        pub slot_distribution: Vec<Permill>,
        pub tier_thresholds: Vec<TierThreshold>,
        pub slots_per_tier: Vec<u16>,
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            // Prepare tier parameters & verify their correctness
            let tier_params = TierParameters::<T::NumberOfTiers> {
                reward_portion: BoundedVec::<Permill, T::NumberOfTiers>::try_from(
                    self.reward_portion.clone(),
                )
                .expect("Invalid number of reward portions provided."),
                slot_distribution: BoundedVec::<Permill, T::NumberOfTiers>::try_from(
                    self.slot_distribution.clone(),
                )
                .expect("Invalid number of slot distributions provided."),
                tier_thresholds: BoundedVec::<TierThreshold, T::NumberOfTiers>::try_from(
                    self.tier_thresholds.clone(),
                )
                .expect("Invalid number of tier thresholds provided."),
            };
            assert!(
                tier_params.is_valid(),
                "Invalid tier parameters values provided."
            );

            // Prepare tier configuration and verify its correctness
            let number_of_slots = self
                .slots_per_tier
                .iter()
                .fold(0, |acc, &slots| acc + slots);
            let tier_config = TiersConfiguration::<T::NumberOfTiers> {
                number_of_slots,
                slots_per_tier: BoundedVec::<u16, T::NumberOfTiers>::try_from(
                    self.slots_per_tier.clone(),
                )
                .expect("Invalid number of slots per tier entries provided."),
                reward_portion: tier_params.reward_portion.clone(),
                tier_thresholds: tier_params.tier_thresholds.clone(),
            };
            assert!(
                tier_params.is_valid(),
                "Invalid tier config values provided."
            );

            // Prepare initial protocol state
            let protocol_state = ProtocolState {
                era: 1,
                next_era_start: Pallet::<T>::blocks_per_voting_period() + 1_u32.into(),
                period_info: PeriodInfo {
                    number: 1,
                    subperiod: Subperiod::Voting,
                    subperiod_end_era: 2,
                },
                maintenance: false,
            };

            // Initialize necessary storage items
            ActiveProtocolState::<T>::put(protocol_state);
            StaticTierParams::<T>::put(tier_params);
            TierConfig::<T>::put(tier_config.clone());
            NextTierConfig::<T>::put(tier_config);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let mut protocol_state = ActiveProtocolState::<T>::get();

            // TODO: maybe do lazy history cleanup in this function?

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
            let (maybe_period_event, era_reward) = match protocol_state.subperiod() {
                Subperiod::Voting => {
                    // For the sake of consistency, we put zero reward into storage
                    let era_reward = EraReward {
                        staker_reward_pool: Balance::zero(),
                        staked: era_info.total_staked_amount(),
                        dapp_reward_pool: Balance::zero(),
                    };

                    let subperiod_end_era =
                        next_era.saturating_add(T::StandardErasPerBuildAndEarnPeriod::get());
                    let build_and_earn_start_block =
                        now.saturating_add(T::StandardEraLength::get());
                    protocol_state
                        .into_next_subperiod(subperiod_end_era, build_and_earn_start_block);

                    era_info.migrate_to_next_era(Some(protocol_state.subperiod()));

                    // Update tier configuration to be used when calculating rewards for the upcoming eras
                    let next_tier_config = NextTierConfig::<T>::take();
                    TierConfig::<T>::put(next_tier_config);

                    (
                        Some(Event::<T>::NewPeriod {
                            subperiod: protocol_state.subperiod(),
                            number: protocol_state.period_number(),
                        }),
                        era_reward,
                    )
                }
                Subperiod::BuildAndEarn => {
                    let (staker_reward_pool, dapp_reward_pool) =
                        T::RewardPoolProvider::normal_reward_pools();
                    let era_reward = EraReward {
                        staker_reward_pool,
                        staked: era_info.total_staked_amount(),
                        dapp_reward_pool,
                    };

                    // Distribute dapps into tiers, write it into storage
                    let dapp_tier_rewards = Self::get_dapp_tier_assignment(
                        current_era,
                        protocol_state.period_number(),
                        dapp_reward_pool,
                    );
                    DAppTiers::<T>::insert(&current_era, dapp_tier_rewards);

                    // Switch to `Voting` period if conditions are met.
                    if protocol_state.period_info.is_next_period(next_era) {
                        // Store info about period end
                        let bonus_reward_pool = T::RewardPoolProvider::bonus_reward_pool();
                        PeriodEnd::<T>::insert(
                            &protocol_state.period_number(),
                            PeriodEndInfo {
                                bonus_reward_pool,
                                total_vp_stake: era_info.staked_amount(Subperiod::Voting),
                                final_era: current_era,
                            },
                        );

                        // For the sake of consistency we treat the whole `Voting` period as a single era.
                        // This means no special handling is required for this period, it only lasts potentially longer than a single standard era.
                        let subperiod_end_era = next_era.saturating_add(1);
                        let voting_period_length = Self::blocks_per_voting_period();
                        let next_era_start_block = now.saturating_add(voting_period_length);

                        protocol_state.into_next_subperiod(subperiod_end_era, next_era_start_block);

                        era_info.migrate_to_next_era(Some(protocol_state.subperiod()));

                        // Re-calculate tier configuration for the upcoming new period
                        let tier_params = StaticTierParams::<T>::get();
                        let average_price = T::NativePriceProvider::average_price();
                        let new_tier_config =
                            TierConfig::<T>::get().calculate_new(average_price, &tier_params);
                        NextTierConfig::<T>::put(new_tier_config);

                        (
                            Some(Event::<T>::NewPeriod {
                                subperiod: protocol_state.subperiod(),
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
            // TODO: Error "cannot" happen here. Log an error if it does though.
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

        /// Used to modify the reward beneficiary account for a dApp.
        ///
        /// Caller has to be dApp owner.
        /// If set to `None`, rewards will be deposited to the dApp owner.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::zero())]
        pub fn set_dapp_reward_beneficiary(
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

            ContractStake::<T>::remove(&smart_contract);

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
            let unlock_block = current_block.saturating_add(Self::unlock_period());
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

            // In case it's full unlock, account is exiting dApp staking, ensure all storage is cleaned up.
            // TODO: will be used after benchmarks
            let _removed_entries = if ledger.is_empty() {
                let _ = StakerInfo::<T>::clear_prefix(&account, ledger.contract_stake_count, None);
                ledger.contract_stake_count
            } else {
                0
            };

            // TODO: discussion point - this will "kill" users ability to withdraw past rewards.
            // This can be handled by the frontend though.

            Self::update_ledger(&account, ledger);
            CurrentEraInfo::<T>::mutate(|era_info| {
                era_info.unlocking_removed(amount);
            });

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

            // TODO: suggestion is to change this a bit so we clean up ledger if rewards have expired
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

            // TODO: also change this to check if rewards have expired

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
            let (mut new_staking_info, is_new_entry) =
                match StakerInfo::<T>::get(&account, &smart_contract) {
                    // Entry with matching period exists
                    Some(staking_info)
                        if staking_info.period_number() == protocol_state.period_number() =>
                    {
                        (staking_info, false)
                    }
                    // Entry exists but period doesn't match. Either reward should be claimed or cleaned up.
                    Some(_) => {
                        return Err(Error::<T>::UnclaimedRewardsFromPastPeriods.into());
                    }
                    // No entry exists
                    None => (
                        SingularStakingInfo::new(
                            protocol_state.period_number(),
                            protocol_state.subperiod(),
                        ),
                        true,
                    ),
                };
            new_staking_info.stake(amount, protocol_state.subperiod());
            ensure!(
                new_staking_info.total_staked_amount() >= T::MinimumStakeAmount::get(),
                Error::<T>::InsufficientStakeAmount
            );

            if is_new_entry {
                ledger.contract_stake_count.saturating_inc();
                ensure!(
                    ledger.contract_stake_count <= T::MaxNumberOfStakedContracts::get(),
                    Error::<T>::TooManyStakedContracts
                );
            }

            // 3.
            // Update `ContractStake` storage with the new stake amount on the specified contract.
            let mut contract_stake_info = ContractStake::<T>::get(&smart_contract);
            contract_stake_info.stake(amount, protocol_state.period_info, stake_era);

            // 4.
            // Update total staked amount for the next era.
            CurrentEraInfo::<T>::mutate(|era_info| {
                era_info.add_stake_amount(amount, protocol_state.subperiod());
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

                    staking_info.unstake(amount, protocol_state.subperiod());
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
                    // These are all defensive checks, which should never happen since we already checked them above.
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
            contract_stake_info.unstake(amount, protocol_state.period_info, unstake_era);

            // 4.
            // Update total staked amount for the next era.
            CurrentEraInfo::<T>::mutate(|era_info| {
                era_info.unstake_amount(amount, protocol_state.subperiod());
            });

            // 5.
            // Update remaining storage entries
            ContractStake::<T>::insert(&smart_contract, contract_stake_info);

            if new_staking_info.is_empty() {
                ledger.contract_stake_count.saturating_dec();
                StakerInfo::<T>::remove(&account, &smart_contract);
            } else {
                StakerInfo::<T>::insert(&account, &smart_contract, new_staking_info);
            }

            Self::update_ledger(&account, ledger);

            Self::deposit_event(Event::<T>::Unstake {
                account,
                smart_contract,
                amount,
            });

            Ok(())
        }

        /// Claims some staker rewards, if user has any.
        /// In the case of a successfull call, at least one era will be claimed, with the possibility of multiple claims happening
        /// if appropriate entries exist in account's ledger.
        #[pallet::call_index(11)]
        #[pallet::weight(Weight::zero())]
        pub fn claim_staker_rewards(origin: OriginFor<T>) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            let account = ensure_signed(origin)?;

            let mut ledger = Ledger::<T>::get(&account);
            let staked_period = ledger
                .staked_period()
                .ok_or(Error::<T>::NoClaimableRewards)?;

            // Check if the rewards have expired
            let protocol_state = ActiveProtocolState::<T>::get();
            ensure!(
                staked_period >= Self::oldest_claimable_period(protocol_state.period_number()),
                Error::<T>::RewardExpired
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
                let era_reward = era_rewards
                    .get(era)
                    .ok_or(Error::<T>::InternalClaimStakerError)?;

                // Optimization, and zero-division protection
                if amount.is_zero() || era_reward.staked.is_zero() {
                    continue;
                }
                let staker_reward = Perbill::from_rational(amount, era_reward.staked)
                    * era_reward.staker_reward_pool;

                rewards.push((era, staker_reward));
                reward_sum.saturating_accrue(staker_reward);
            }

            // Account exists since it has locked funds.
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

        /// Used to claim bonus reward for a smart contract, if eligible.
        #[pallet::call_index(12)]
        #[pallet::weight(Weight::zero())]
        pub fn claim_bonus_reward(
            origin: OriginFor<T>,
            smart_contract: T::SmartContract,
        ) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            let account = ensure_signed(origin)?;

            let staker_info = StakerInfo::<T>::get(&account, &smart_contract)
                .ok_or(Error::<T>::NoClaimableRewards)?;
            let protocol_state = ActiveProtocolState::<T>::get();

            // Ensure:
            // 1. Period for which rewards are being claimed has ended.
            // 2. Account has been a loyal staker.
            // 3. Rewards haven't expired.
            let staked_period = staker_info.period_number();
            ensure!(
                staked_period < protocol_state.period_number(),
                Error::<T>::NoClaimableRewards
            );
            ensure!(
                staker_info.is_loyal(),
                Error::<T>::NotEligibleForBonusReward
            );
            ensure!(
                staker_info.period_number()
                    >= Self::oldest_claimable_period(protocol_state.period_number()),
                Error::<T>::RewardExpired
            );

            let period_end_info =
                PeriodEnd::<T>::get(&staked_period).ok_or(Error::<T>::InternalClaimBonusError)?;
            // Defensive check - we should never get this far in function if no voting period stake exists.
            ensure!(
                !period_end_info.total_vp_stake.is_zero(),
                Error::<T>::InternalClaimBonusError
            );

            let eligible_amount = staker_info.staked_amount(Subperiod::Voting);
            let bonus_reward =
                Perbill::from_rational(eligible_amount, period_end_info.total_vp_stake)
                    * period_end_info.bonus_reward_pool;

            // Account exists since it has locked funds.
            T::Currency::deposit_into_existing(&account, bonus_reward)
                .map_err(|_| Error::<T>::InternalClaimStakerError)?;

            // Cleanup entry since the reward has been claimed
            StakerInfo::<T>::remove(&account, &smart_contract);

            Self::deposit_event(Event::<T>::BonusReward {
                account: account.clone(),
                smart_contract,
                period: staked_period,
                amount: bonus_reward,
            });

            Ok(())
        }

        /// Used to claim dApp reward for the specified era.
        #[pallet::call_index(13)]
        #[pallet::weight(Weight::zero())]
        pub fn claim_dapp_reward(
            origin: OriginFor<T>,
            smart_contract: T::SmartContract,
            #[pallet::compact] era: EraNumber,
        ) -> DispatchResult {
            Self::ensure_pallet_enabled()?;
            let _ = ensure_signed(origin)?;

            let dapp_info =
                IntegratedDApps::<T>::get(&smart_contract).ok_or(Error::<T>::ContractNotFound)?;

            // Make sure provided era has ended
            let protocol_state = ActiveProtocolState::<T>::get();
            ensure!(era < protocol_state.era, Error::<T>::InvalidClaimEra);

            // 'Consume' dApp reward for the specified era, if possible.
            let mut dapp_tiers = DAppTiers::<T>::get(&era).ok_or(Error::<T>::NoDAppTierInfo)?;
            ensure!(
                Self::oldest_claimable_period(dapp_tiers.period) <= protocol_state.period_number(),
                Error::<T>::RewardExpired
            );

            let (amount, tier_id) =
                dapp_tiers
                    .try_consume(dapp_info.id)
                    .map_err(|error| match error {
                        DAppTierError::NoDAppInTiers => Error::<T>::NoClaimableRewards,
                        DAppTierError::RewardAlreadyClaimed => Error::<T>::DAppRewardAlreadyClaimed,
                        _ => Error::<T>::InternalClaimDAppError,
                    })?;

            // Get reward destination, and deposit the reward.
            let beneficiary = dapp_info.reward_beneficiary();
            T::Currency::deposit_creating(beneficiary, amount);

            // Write back updated struct to prevent double reward claims
            DAppTiers::<T>::insert(&era, dapp_tiers);

            Self::deposit_event(Event::<T>::DAppReward {
                beneficiary: beneficiary.clone(),
                smart_contract,
                tier_id,
                era,
                amount,
            });

            Ok(())
        }

        /// Used to unstake funds from a contract that was unregistered after an account staked on it.
        #[pallet::call_index(14)]
        #[pallet::weight(Weight::zero())]
        pub fn unstake_from_unregistered(
            origin: OriginFor<T>,
            smart_contract: T::SmartContract,
        ) -> DispatchResult {
            // TODO: tests are missing but will be added later.
            Self::ensure_pallet_enabled()?;
            let account = ensure_signed(origin)?;

            ensure!(
                !Self::is_active(&smart_contract),
                Error::<T>::ContractStillActive
            );

            let protocol_state = ActiveProtocolState::<T>::get();
            let unstake_era = protocol_state.era;

            // Extract total staked amount on the specified unregistered contract
            let amount = match StakerInfo::<T>::get(&account, &smart_contract) {
                Some(staking_info) => {
                    ensure!(
                        staking_info.period_number() == protocol_state.period_number(),
                        Error::<T>::UnstakeFromPastPeriod
                    );

                    staking_info.total_staked_amount()
                }
                None => {
                    return Err(Error::<T>::NoStakingInfo.into());
                }
            };

            // Reduce stake amount in ledger
            let mut ledger = Ledger::<T>::get(&account);
            ledger
                .unstake_amount(amount, unstake_era, protocol_state.period_info)
                .map_err(|err| match err {
                    // These are all defensive checks, which should never happen since we already checked them above.
                    AccountLedgerError::InvalidPeriod | AccountLedgerError::InvalidEra => {
                        Error::<T>::UnclaimedRewardsFromPastPeriods
                    }
                    _ => Error::<T>::InternalUnstakeError,
                })?;

            // Update total staked amount for the next era.
            // This means 'fake' stake total amount has been kept until now, even though contract was unregistered.
            CurrentEraInfo::<T>::mutate(|era_info| {
                era_info.unstake_amount(amount, protocol_state.subperiod());
            });

            // Update remaining storage entries
            Self::update_ledger(&account, ledger);
            StakerInfo::<T>::remove(&account, &smart_contract);

            Self::deposit_event(Event::<T>::UnstakeFromUnregistered {
                account,
                smart_contract,
                amount,
            });

            Ok(())
        }

        // TODO: an alternative to this could would be to allow `unstake` call to cleanup old entries, however that means more complexity in that call
        /// Used to unstake funds from a contract that was unregistered after an account staked on it.
        #[pallet::call_index(15)]
        #[pallet::weight(Weight::zero())]
        pub fn cleanup_expired_entries(origin: OriginFor<T>) -> DispatchResult {
            // TODO: tests are missing but will be added later.
            Self::ensure_pallet_enabled()?;
            let account = ensure_signed(origin)?;

            let protocol_state = ActiveProtocolState::<T>::get();
            let current_period = protocol_state.period_number();

            // Find all entries which have expired. This is bounded by max allowed number of entries.
            let to_be_deleted: Vec<T::SmartContract> = StakerInfo::<T>::iter_prefix(&account)
                .filter_map(|(smart_contract, stake_info)| {
                    if stake_info.period_number() < current_period {
                        Some(smart_contract)
                    } else {
                        None
                    }
                })
                .collect();

            // Remove all expired entries.
            for smart_contract in to_be_deleted {
                StakerInfo::<T>::remove(&account, &smart_contract);
            }

            // Remove expired ledger stake entries, if needed.
            let threshold_period = Self::oldest_claimable_period(current_period);
            let mut ledger = Ledger::<T>::get(&account);
            if ledger.maybe_cleanup_expired(threshold_period) {
                Self::update_ledger(&account, ledger);
            }

            Ok(())
        }

        /// Used to enable or disable maintenance mode.
        /// Can only be called by manager origin.
        #[pallet::call_index(16)]
        #[pallet::weight(Weight::zero())]
        pub fn force(origin: OriginFor<T>, force_type: ForcingType) -> DispatchResult {
            // TODO: tests are missing but will be added later.
            Self::ensure_pallet_enabled()?;
            T::ManagerOrigin::ensure_origin(origin)?;

            // Ensure a 'change' happens on the next block
            ActiveProtocolState::<T>::mutate(|state| {
                let current_block = frame_system::Pallet::<T>::block_number();
                state.next_era_start = current_block.saturating_add(One::one());

                match force_type {
                    ForcingType::Era => (),
                    ForcingType::Subperiod => {
                        state.period_info.subperiod_end_era = state.era.saturating_add(1);
                    }
                }
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
        pub(crate) fn is_active(smart_contract: &T::SmartContract) -> bool {
            IntegratedDApps::<T>::get(smart_contract)
                .map_or(false, |dapp_info| dapp_info.state == DAppState::Registered)
        }

        /// Calculates the `EraRewardSpan` index for the specified era.
        pub(crate) fn era_reward_span_index(era: EraNumber) -> EraNumber {
            era.saturating_sub(era % T::EraRewardSpanLength::get())
        }

        /// Return the oldest period for which rewards can be claimed.
        /// All rewards before that period are considered to be expired.
        pub(crate) fn oldest_claimable_period(current_period: PeriodNumber) -> PeriodNumber {
            current_period.saturating_sub(T::RewardRetentionInPeriods::get())
        }

        /// Unlocking period expressed in the number of blocks.
        pub(crate) fn unlock_period() -> BlockNumberFor<T> {
            T::StandardEraLength::get().saturating_mul(T::UnlockingPeriod::get().into())
        }

        /// Assign eligible dApps into appropriate tiers, and calculate reward for each tier.
        ///
        /// The returned object contains information about each dApp that made it into a tier.
        pub(crate) fn get_dapp_tier_assignment(
            era: EraNumber,
            period: PeriodNumber,
            dapp_reward_pool: Balance,
        ) -> DAppTierRewardsFor<T> {
            // TODO - by breaking this into multiple steps, if they are too heavy for a single block, we can distribute them between multiple blocks.
            // Benchmarks will show this, but I don't believe it will be needed, especially with increased block capacity we'll get with async backing.
            // Even without async backing though, we should have enough capacity to handle this.
            // UPDATE: might work with async backing, but right now we could handle up to 150 dApps before exceeding the PoV size.

            // UPDATE2: instead of taking the approach of reading an ever increasing amount of entries from storage,  we can instead adopt an approach
            // of eficiently storing composite information into `BTreeMap`. The approach is essentially the same as the one used below to store rewards.
            // Each time `stake` or `unstake` are called, corresponding entries are updated. This way we can keep track of all the contract stake in a single DB entry.
            // To make the solution more scalable, we could 'split' stake entries into spans, similar as rewards are handled now.
            //
            // Experiment with an 'experimental' entry shows PoV size of ~7kB induced for entry that can hold up to 100 entries.

            let mut dapp_stakes = Vec::with_capacity(IntegratedDApps::<T>::count() as usize);

            // 1.
            // Iterate over all registered dApps, and collect their stake amount.
            // This is bounded by max amount of dApps we allow to be registered.
            for (smart_contract, dapp_info) in IntegratedDApps::<T>::iter() {
                // Skip unregistered dApps
                if dapp_info.state != DAppState::Registered {
                    continue;
                }

                // Skip dApps which don't have ANY amount staked (TODO: potential improvement is to prune all dApps below minimum threshold)
                let stake_amount = match ContractStake::<T>::get(&smart_contract).get(era, period) {
                    Some(stake_amount) if !stake_amount.total().is_zero() => stake_amount,
                    _ => continue,
                };

                // TODO: Need to handle labels!
                // Proposition for label handling:
                // Split them into 'musts' and 'good-to-have'
                // In case of 'must', reduce appropriate tier size, and insert them at the end
                // For good to have, we can insert them immediately, and then see if we need to adjust them later.
                // Anyhow, labels bring complexity. For starters, we should only deliver the one for 'bootstraping' purposes.
                dapp_stakes.push((dapp_info.id, stake_amount.total()));
            }

            // 2.
            // Sort by amount staked, in reverse - top dApp will end in the first place, 0th index.
            dapp_stakes.sort_unstable_by(|(_, amount_1), (_, amount_2)| amount_2.cmp(amount_1));

            // 3.
            // Iterate over configured tier and potential dApps.
            // Each dApp will be assigned to the best possible tier if it satisfies the required condition,
            // and tier capacity hasn't been filled yet.
            let mut dapp_tiers = Vec::with_capacity(dapp_stakes.len());
            let tier_config = TierConfig::<T>::get();

            let mut global_idx = 0;
            let mut tier_id = 0;
            for (tier_capacity, tier_threshold) in tier_config
                .slots_per_tier
                .iter()
                .zip(tier_config.tier_thresholds.iter())
            {
                let max_idx = global_idx
                    .saturating_add(*tier_capacity as usize)
                    .min(dapp_stakes.len());

                // Iterate over dApps until one of two conditions has been met:
                // 1. Tier has no more capacity
                // 2. dApp doesn't satisfy the tier threshold (since they're sorted, none of the following dApps will satisfy the condition either)
                for (dapp_id, stake_amount) in dapp_stakes[global_idx..max_idx].iter() {
                    if tier_threshold.is_satisfied(*stake_amount) {
                        global_idx.saturating_inc();
                        dapp_tiers.push(DAppTier {
                            dapp_id: *dapp_id,
                            tier_id: Some(tier_id),
                        });
                    } else {
                        break;
                    }
                }

                tier_id.saturating_inc();
            }

            // 4.
            // Sort by dApp ID, in ascending order (unstable sort should be faster, and stability is guaranteed due to lack of duplicated Ids).
            // TODO & Idea: perhaps use BTreeMap instead? It will "sort" automatically based on dApp Id, and we can efficiently remove entries,
            // reducing PoV size step by step.
            // It's a trade-off between speed and PoV size. Although both are quite minor, so maybe it doesn't matter that much.
            dapp_tiers.sort_unstable_by(|first, second| first.dapp_id.cmp(&second.dapp_id));

            // 5. Calculate rewards.
            let tier_rewards = tier_config
                .reward_portion
                .iter()
                .map(|percent| *percent * dapp_reward_pool)
                .collect::<Vec<_>>();

            // 6.
            // Prepare and return tier & rewards info.
            // In case rewards creation fails, we just write the default value. This should never happen though.
            DAppTierRewards::<MaxNumberOfContractsU32<T>, T::NumberOfTiers>::new(
                dapp_tiers,
                tier_rewards,
                period,
            )
            .unwrap_or_default()
        }
    }
}
