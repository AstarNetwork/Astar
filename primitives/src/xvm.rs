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

use frame_support::weights::Weight;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::{convert::TryFrom, prelude::*, result::Result};

/// Vm Id.
#[repr(u8)]
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum VmId {
    Evm = 0x0F,
    Wasm = 0x1F,
}

impl TryFrom<u8> for VmId {
    type Error = CallError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == VmId::Evm as u8 {
            Ok(VmId::Evm)
        } else if value == VmId::Wasm as u8 {
            Ok(VmId::Wasm)
        } else {
            Err(CallError::InvalidVmId)
        }
    }
}

/// XVM call info on success.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct CallInfo {
    /// Output of the call.
    pub output: Vec<u8>,
    /// Actual used weight.
    pub used_weight: Weight,
}

/// XVM call error on failure.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum CallError {
    /// Invalid VM id.
    InvalidVmId,
    /// Calling the contracts in the same VM is not allowed.
    SameVmCallNotAllowed,
    /// Target contract address is invalid.
    InvalidTarget,
    /// Input is too large.
    InputTooLarge,
    /// Bad origin.
    BadOrigin,
    /// The call failed on EVM or WASM execution.
    ExecutionFailed(Vec<u8>),
}

/// XVM call error with used weight info.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct CallErrorWithWeight {
    /// Error info.
    pub error: CallError,
    /// Actual used weight.
    pub used_weight: Weight,
}

/// XVM call result.
pub type CallResult = Result<CallInfo, CallErrorWithWeight>;

/// XVM context.
///
/// Note this should be set by runtime, instead of passed by callers.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Context {
    /// The source VM of the call.
    pub source_vm_id: VmId,
    /// Max weight limit.
    pub weight_limit: Weight,
}

pub trait XvmCall<AccountId> {
    /// Call a contract in XVM.
    ///
    /// Parameters:
    /// - `context`: XVM context.
    /// - `vm_id`: the VM Id of the target contract.
    /// - `source`: Caller Id.
    /// - `target`: Target contract address.
    /// - `input`: call input data.
    fn call(
        context: Context,
        vm_id: VmId,
        source: AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
    ) -> CallResult;
}
