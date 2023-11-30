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

use crate::Balance;

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
    type Error = FailureReason;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == VmId::Evm as u8 {
            Ok(VmId::Evm)
        } else if value == VmId::Wasm as u8 {
            Ok(VmId::Wasm)
        } else {
            Err(FailureReason::Error(FailureError::InvalidVmId))
        }
    }
}

/// XVM call info on success.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct CallOutput {
    /// Output of the call.
    pub output: Vec<u8>,
    /// Actual used weight.
    pub used_weight: Weight,
}

impl CallOutput {
    /// Create a new `CallOutput`.
    pub fn new(output: Vec<u8>, used_weight: Weight) -> Self {
        Self {
            output,
            used_weight,
        }
    }
}

/// XVM call failure.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct CallFailure {
    /// Failure reason.
    pub reason: FailureReason,
    /// Actual used weight.
    pub used_weight: Weight,
}

impl CallFailure {
    /// Create a new `CallFailure` on revert.
    pub fn revert(details: FailureRevert, used_weight: Weight) -> Self {
        Self {
            reason: FailureReason::Revert(details),
            used_weight,
        }
    }

    /// Create a new `CallFailure` on error.
    pub fn error(details: FailureError, used_weight: Weight) -> Self {
        Self {
            reason: FailureReason::Error(details),
            used_weight,
        }
    }
}

/// Failure reason of XVM calls.
///
/// `Error` vs `Revert`:
/// - `Error` is for execution failed and the VM must stop. It maps to EVM
///  `ExitError/ExistFatal` and WASM `DispatchError`.
/// - `Revert` is for execution succeeded but the callee explicitly asked to
///  revert. It maps to EVM `ExitRevert` and WASM `REVERT` flag. It also includes
///  the case that wrong input was passed to XVM call, for instance invalid target,
///  as from VM/WASM perspective, it's an input guard condition failure.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum FailureReason {
    /// XVM call failed with explicit revert.
    Revert(FailureRevert),
    /// XVM call failed with error.
    Error(FailureError),
}

/// Failure reason on revert.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum FailureRevert {
    /// Target contract address is invalid.
    InvalidTarget,
    /// Input is too large.
    InputTooLarge,
    /// VM execution exit with revert.
    VmRevert(Vec<u8>),
}

/// Failure reason on error.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum FailureError {
    /// Invalid VM id.
    InvalidVmId,
    /// Calling the contracts in the same VM is not allowed.
    SameVmCallDenied,
    /// Reentrance is not allowed.
    ReentranceDenied,
    /// The call failed with error on EVM or WASM execution.
    VmError(Vec<u8>),
    /// Out of gas.
    OutOfGas,
}

/// XVM call result.
pub type CallResult = Result<CallOutput, CallFailure>;

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
    /// - `value`: value to transfer.
    /// - `storage_deposit_limit`: storage deposit limit for wasm calls.
    fn call(
        context: Context,
        vm_id: VmId,
        source: AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
        value: Balance,
        storage_deposit_limit: Option<Balance>,
    ) -> CallResult;
}
