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

use astar_primitives::{
    xvm::{FailureError, FailureReason, FailureRevert},
    Balance,
};
use parity_scale_codec::{Decode, Encode};
use sp_std::vec::Vec;

#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum XvmExecutionResult {
    /// Success
    Ok,
    /// Failure
    Err(u32),
}

impl From<FailureReason> for XvmExecutionResult {
    fn from(input: FailureReason) -> Self {
        // `0` is reserved for `Ok`
        let error_code = match input {
            // Revert failure: 1 - 127
            FailureReason::Revert(FailureRevert::InvalidTarget) => 1,
            FailureReason::Revert(FailureRevert::InputTooLarge) => 2,
            FailureReason::Revert(FailureRevert::VmRevert(_)) => 3,

            // Error failure: 128 - 255
            FailureReason::Error(FailureError::InvalidVmId) => 128,
            FailureReason::Error(FailureError::SameVmCallDenied) => 129,
            FailureReason::Error(FailureError::ReentranceDenied) => 130,
            FailureReason::Error(FailureError::VmError(_)) => 131,
            FailureReason::Error(FailureError::OutOfGas) => 132,
        };
        Self::Err(error_code)
    }
}

impl From<XvmExecutionResult> for u32 {
    fn from(input: XvmExecutionResult) -> Self {
        match input {
            XvmExecutionResult::Ok => 0,
            XvmExecutionResult::Err(code) => code,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
pub struct XvmCallArgs {
    /// virtual machine identifier
    pub vm_id: u8,
    /// Call destination (e.g. address)
    pub to: Vec<u8>,
    /// Encoded call params
    pub input: Vec<u8>,
    /// Value to transfer
    pub value: Balance,
}
