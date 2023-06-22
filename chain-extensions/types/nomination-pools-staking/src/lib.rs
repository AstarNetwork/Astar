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

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[repr(u32)]
pub enum NPSError {
    /// Success
    Success = 0,
    /// Not enough balance
    NotEnoughBalance = 1,
    /// Unknown error
    UnknownError = 99,
}

impl From<NPSError> for u32 {
    fn from(error: NPSError) -> Self {
        match error {
            NPSError::Success => 0,
            NPSError::NotEnoughBalance => 1,
            NPSError::UnknownError => 99,
        }
    }
}

// impl From<PausableError> for NPSError {
//     fn from(pausable: PausableError) -> Self {
//         match pausable {
//             PausableError::Paused => NPSError::Custom(String::from("P::Paused")),
//             PausableError::NotPaused => NPSError::Custom(String::from("P::NotPaused")),
//         }
//     }
// }

// impl ink_env::chain_extension::FromStatusCode for NPSError {
//     fn from_status_code(status_code: u32) -> Result<(), Self> {
//         match status_code {
//             0 => Ok(()),
//             1 => Err(NPSError::NotEnoughBalance),
//             99 => Err(NPSError::UnknownError),
//             _ => Err(NPSError::UnknownError),
//         }
//     }
// }

impl TryFrom<DispatchError> for NPSError {
    type Error = DispatchError;

    fn try_from(input: DispatchError) -> Result<Self, Self::Error> {
        let error_text = match input {
            DispatchError::Module(ModuleError { message, .. }) => message,
            _ => Some("No module error Info"),
        };
        return match error_text {
            _ => Ok(NPSError::UnknownError),
        };
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen)]
pub struct NominationPoolStakingValueInput<Balance> {
    pub contract: [u8; 32],
    pub value: Balance,
}
