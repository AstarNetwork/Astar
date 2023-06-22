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

#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::pallet_prelude::MaxEncodedLen;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::{DispatchError, ModuleError};

#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum DSError {
    /// Success
    Success = 0,
    /// Disabled
    Disabled = 1,
    /// No change in maintenance mode
    NoMaintenanceModeChange = 2,
    /// Upgrade is too heavy, reduce the weight parameter.
    UpgradeTooHeavy = 3,
    /// Can not stake with zero value.
    StakingWithNoValue = 4,
    /// Can not stake with value less than minimum staking value
    InsufficientValue = 5,
    /// Number of stakers per contract exceeded.
    MaxNumberOfStakersExceeded = 6,
    /// Targets must be operated contracts
    NotOperatedContract = 7,
    /// Contract isn't staked.
    NotStakedContract = 8,
    /// Contract isn't unregistered.
    NotUnregisteredContract = 9,
    /// Unclaimed rewards should be claimed before withdrawing stake.
    UnclaimedRewardsRemaining = 10,
    /// Unstaking a contract with zero value
    UnstakingWithNoValue = 11,
    /// There are no previously unbonded funds that can be unstaked and withdrawn.
    NothingToWithdraw = 12,
    /// The contract is already registered by other account
    AlreadyRegisteredContract = 13,
    /// User attempts to register with address which is not contract
    ContractIsNotValid = 14,
    /// This account was already used to register contract
    AlreadyUsedDeveloperAccount = 15,
    /// Smart contract not owned by the account id.
    NotOwnedContract = 16,
    /// Report issue on github if this is ever emitted
    UnknownEraReward = 17,
    /// Report issue on github if this is ever emitted
    UnexpectedStakeInfoEra = 18,
    /// Contract has too many unlocking chunks. Withdraw the existing chunks if possible
    /// or wait for current chunks to complete unlocking process to withdraw them.
    TooManyUnlockingChunks = 19,
    /// Contract already claimed in this era and reward is distributed
    AlreadyClaimedInThisEra = 20,
    /// Era parameter is out of bounds
    EraOutOfBounds = 21,
    /// Too many active `EraStake` values for (staker, contract) pairing.
    /// Claim existing rewards to fix this problem.
    TooManyEraStakeValues = 22,
    /// To register a contract, pre-approval is needed for this address
    RequiredContractPreApproval = 23,
    /// Developer's account is already part of pre-approved list
    AlreadyPreApprovedDeveloper = 24,
    /// Account is not actively staking
    NotActiveStaker = 25,
    /// Transfering nomination to the same contract
    NominationTransferToSameContract = 26,
    /// Unexpected reward destination value
    RewardDestinationValueOutOfBounds = 27,
    /// Unknown error
    UnknownError = 99,
}

impl TryFrom<DispatchError> for DSError {
    type Error = DispatchError;

    fn try_from(input: DispatchError) -> Result<Self, Self::Error> {
        let error_text = match input {
            DispatchError::Module(ModuleError { message, .. }) => message,
            _ => Some("No module error Info"),
        };
        return match error_text {
            Some("Disabled") => Ok(DSError::Disabled),
            Some("NoMaintenanceModeChange") => Ok(DSError::NoMaintenanceModeChange),
            Some("UpgradeTooHeavy") => Ok(DSError::UpgradeTooHeavy),
            Some("StakingWithNoValue") => Ok(DSError::StakingWithNoValue),
            Some("InsufficientValue") => Ok(DSError::InsufficientValue),
            Some("MaxNumberOfStakersExceeded") => Ok(DSError::MaxNumberOfStakersExceeded),
            Some("NotOperatedContract") => Ok(DSError::NotOperatedContract),
            Some("NotStakedContract") => Ok(DSError::NotStakedContract),
            Some("NotUnregisteredContract") => Ok(DSError::NotUnregisteredContract),
            Some("UnclaimedRewardsRemaining") => Ok(DSError::UnclaimedRewardsRemaining),
            Some("UnstakingWithNoValue") => Ok(DSError::UnstakingWithNoValue),
            Some("NothingToWithdraw") => Ok(DSError::NothingToWithdraw),
            Some("AlreadyRegisteredContract") => Ok(DSError::AlreadyRegisteredContract),
            Some("ContractIsNotValid") => Ok(DSError::ContractIsNotValid),
            Some("AlreadyUsedDeveloperAccount") => Ok(DSError::AlreadyUsedDeveloperAccount),
            Some("NotOwnedContract") => Ok(DSError::NotOwnedContract),
            Some("UnknownEraReward") => Ok(DSError::UnknownEraReward),
            Some("UnexpectedStakeInfoEra") => Ok(DSError::UnexpectedStakeInfoEra),
            Some("TooManyUnlockingChunks") => Ok(DSError::TooManyUnlockingChunks),
            Some("AlreadyClaimedInThisEra") => Ok(DSError::AlreadyClaimedInThisEra),
            Some("EraOutOfBounds") => Ok(DSError::EraOutOfBounds),
            Some("TooManyEraStakeValues") => Ok(DSError::TooManyEraStakeValues),
            Some("RequiredContractPreApproval") => Ok(DSError::RequiredContractPreApproval),
            Some("AlreadyPreApprovedDeveloper") => Ok(DSError::AlreadyPreApprovedDeveloper),
            Some("NotActiveStaker") => Ok(DSError::NotActiveStaker),
            Some("NominationTransferToSameContract") => {
                Ok(DSError::NominationTransferToSameContract)
            }
            _ => Ok(DSError::UnknownError),
        };
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen)]
pub struct DappsStakingValueInput<Balance> {
    pub contract: [u8; 32],
    pub value: Balance,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen)]
pub struct DappsStakingAccountInput {
    pub contract: [u8; 32],
    pub staker: [u8; 32],
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen)]
pub struct DappsStakingEraInput {
    pub contract: [u8; 32],
    pub era: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen)]
pub struct DappsStakingNominationInput<Balance> {
    pub origin_contract: [u8; 32],
    pub target_contract: [u8; 32],
    pub value: Balance,
}
