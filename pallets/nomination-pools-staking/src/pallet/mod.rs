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
    pallet_prelude::*,
    traits::{Currency, Get, LockIdentifier, LockableCurrency, ReservableCurrency},
    weights::Weight,
    PalletId,
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::StaticLookup;
use sp_std::convert::From;

const _NOMINATION_POOL_STAKING_ID: LockIdentifier = *b"np_stake";

#[frame_support::pallet]
#[allow(clippy::module_inception)]
pub mod pallet {
    use super::*;
    use sp_std::vec::Vec;
    use xcm::v3::{Instruction, Junctions::Here, MultiLocation, Xcm};

    /// The balance type of this pallet.
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[derive(Encode, Decode, RuntimeDebug)]
    pub enum NominationPoolsCall<T: Config> {
        #[codec(index = 6)] // same to call index
        Create(
            #[codec(compact)] BalanceOf<T>,
            <T::Lookup as StaticLookup>::Source,
            <T::Lookup as StaticLookup>::Source,
            <T::Lookup as StaticLookup>::Source,
        ),
    }

    #[derive(Encode, Decode, RuntimeDebug)]
    pub enum RelayChainCall<T: Config> {
        // https://github.com/paritytech/polkadot/blob/7a19bf09147605f185421a51ec254c51d2c7d060/runtime/polkadot/src/lib.rs#L1414
        #[codec(index = 39)]
        NominationPools(NominationPoolsCall<T>),
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_xcm::Config {
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

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Account has bonded and staked funds on a smart contract.
        BondAndStake(T::AccountId, T::SmartContract),
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
        /// Failed to send XCM transaction
        FailedXcmTransaction,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
            // As long as pallet is disabled, we shouldn't allow any storage modifications.
            // This means we might prolong an era but it's acceptable.
            // Runtime upgrade should be timed so we ensure that we complete it before
            // a new era is triggered. This code is just a safety net to ensure nothing is broken
            // if we fail to do that.
            if PalletDisabled::<T>::get() {
                return T::DbWeight::get().reads(1);
            }

            T::DbWeight::get().reads(1)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Lock up and stake balance of the origin account.
        ///
        /// `value` must be more than the `minimum_balance` specified by `MinimumStakingAmount`
        /// unless account already has bonded value equal or more than 'minimum_balance'.
        ///
        /// The dispatch origin for this call must be _Signed_ by the staker's account.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as pallet::pallet::Config>::WeightInfo::create_nomination_pool())]
        pub fn create_nomination_pool(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;

            let staker: <T as frame_system::Config>::AccountId = ensure_signed(origin)?;

            let location: MultiLocation = MultiLocation {
                parents: 1,
                interior: Here,
            };

            let staker_multi_address = T::Lookup::unlookup(staker.clone());

            let create_nomination_pool: NominationPoolsCall<T> = NominationPoolsCall::Create(
                value,
                staker_multi_address.clone(),
                staker_multi_address.clone(),
                staker_multi_address.clone(),
            );

            let mut calls = Vec::new();

            calls.push(Instruction::WithdrawAsset(
                (Here, 10000000000000u128).into(),
            ));
            calls.push(Instruction::BuyExecution {
                fees: (Here, 10000000000000u128).into(),
                weight_limit: xcm::v3::WeightLimit::Unlimited,
            });
            calls.push(Instruction::Transact {
                origin_kind: xcm::v3::OriginKind::Native,
                require_weight_at_most: Weight::from_parts(4_000_0000u64, 1024 * 1024),
                call: create_nomination_pool.encode().into(),
            });

            let messages = Xcm(calls);

            match pallet_xcm::Pallet::<T>::send_xcm(Here, location, messages) {
                Ok(_) => {
                    Self::deposit_event(Event::<T>::BondAndStake(staker, contract_id));
                    Ok(().into())
                }
                Err(_err) => Err(Error::<T>::FailedXcmTransaction.into()),
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    /// `Err` if pallet disabled for maintenance, `Ok` otherwise
    pub fn ensure_pallet_enabled() -> Result<(), Error<T>> {
        if PalletDisabled::<T>::get() {
            Err(Error::<T>::Disabled)
        } else {
            Ok(())
        }
    }
}
