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

use parity_scale_codec::{Decode, Encode};
use sp_runtime::{DispatchError, ModuleError};
use sp_std::vec::Vec;

#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum XvmExecutionResult {
    /// Success
    Success = 0,
    // TODO: expand this with concrete XVM errors
    /// Error not (yet) covered by a dedidacted code
    UnknownError = 255,
}

impl TryFrom<DispatchError> for XvmExecutionResult {
    type Error = DispatchError;

    fn try_from(input: DispatchError) -> Result<Self, Self::Error> {
        let _error_text = match input {
            DispatchError::Module(ModuleError { message, .. }) => message,
            _ => Some("No module error Info"),
        };

        // TODO: expand this with concrete XVM errors (see dapps-staking types for example)
        Ok(XvmExecutionResult::UnknownError)
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
}

pub const FRONTIER_VM_ID: u8 = 0x0F;
pub const PARITY_WASM_VM_ID: u8 = 0x1F;
