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

//! # XVM pallet
//!
//! ## Overview
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//!
//! ### Other
//!
//!

use frame_support::{pallet_prelude::*, traits::ConstU32, BoundedVec};
use pallet_evm::GasWeightMapping;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::U256;
use sp_runtime::{traits::StaticLookup, RuntimeDebug};
use sp_std::{marker::PhantomData, prelude::*, result::Result};

use astar_primitives::ethereum_checked::{
    AccountMapping, CheckedEthereumTransact, CheckedEthereumTx, MAX_ETHEREUM_TX_INPUT_SIZE,
};

pub use pallet::*;

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
    /// The call failed on EVM or WASM execution.
    ExecutionFailed(Vec<u8>),
    /// Input is too large.
    InputTooLarge,
    /// Target contract address is invalid.
    InvalidTarget,
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
pub type XvmCallResult = Result<CallInfo, CallErrorWithWeight>;

/// XVM context.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct XvmContext {
    /// Max weight limit.
    pub weight_limit: Weight,
    /// Optional encoded execution environment.
    pub env: Option<Vec<u8>>,
}

pub trait XvmCall<AccountId> {
    /// Call a contract in XVM.
    ///
    /// Parameters:
    /// - `context`: XVM context.
    /// - `source`: Caller Id.
    /// - `target`: Target contract address.
    /// - `input`: call input data.
    fn xvm_call(
        context: XvmContext,
        source: AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
    ) -> XvmCallResult;
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + pallet_evm::Config
        + pallet_ethereum_checked::Config
        + pallet_contracts::Config
    {
        /// `CheckedEthereumTransact` implementation.
        type EthereumTransact: CheckedEthereumTransact;
    }
}

// TODO: benchmark XVM calls overhead
pub const PLACEHOLDER_WEIGHT: Weight = Weight::from_parts(1_000_000, 1024);

/// XVM call to EVM.
pub struct EvmCall<T>(PhantomData<T>);
impl<T: Config> XvmCall<T::AccountId> for EvmCall<T> {
    fn xvm_call(
        context: XvmContext,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
    ) -> XvmCallResult {
        Pallet::<T>::evm_call(context, source, target, input, false)
    }
}

/// XVM call to WASM.
pub struct WasmCall<T>(PhantomData<T>);
impl<T: Config> XvmCall<T::AccountId> for WasmCall<T> {
    fn xvm_call(
        context: XvmContext,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
    ) -> XvmCallResult {
        Pallet::<T>::wasm_call(context, source, target, input, false)
    }
}

impl<T: Config> Pallet<T> {
    fn evm_call(
        context: XvmContext,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
        skip_apply: bool,
    ) -> XvmCallResult {
        log::trace!(
            target: "xvm::evm_call",
            "Calling EVM: {:?} {:?}, {:?}, {:?}",
            context, source, target, input,
        );

        let value = U256::zero();
        let gas_limit = T::GasWeightMapping::weight_to_gas(context.weight_limit);

        let target_decoded =
            Decode::decode(&mut target.as_ref()).map_err(|_| CallErrorWithWeight {
                error: CallError::InvalidTarget,
                used_weight: PLACEHOLDER_WEIGHT,
            })?;
        let bounded_input = BoundedVec::<u8, ConstU32<MAX_ETHEREUM_TX_INPUT_SIZE>>::try_from(input)
            .map_err(|_| CallErrorWithWeight {
                error: CallError::InputTooLarge,
                used_weight: PLACEHOLDER_WEIGHT,
            })?;

        if skip_apply {
            return Ok(CallInfo {
                output: vec![],
                used_weight: PLACEHOLDER_WEIGHT,
            });
        }

        let (post_dispatch_info, call_info) = T::EthereumTransact::xvm_transact(
            T::AccountMapping::into_h160(source),
            CheckedEthereumTx {
                gas_limit: U256::from(gas_limit),
                target: target_decoded,
                value,
                input: bounded_input,
                maybe_access_list: None,
            },
        )
        .map_err(|e| {
            let used_weight = e.post_info.actual_weight.unwrap_or_default();
            CallErrorWithWeight {
                error: CallError::ExecutionFailed(Into::<&str>::into(e.error).into()),
                used_weight,
            }
        })?;

        log::trace!(
            target: "xvm::evm_call",
            "EVM call result: exit_reason: {:?}, used_gas: {:?}", call_info.exit_reason, call_info.used_gas,
        );

        // TODO: add overhead to used weight
        Ok(CallInfo {
            output: call_info.value,
            used_weight: post_dispatch_info.actual_weight.unwrap_or_default(),
        })
    }

    fn wasm_call(
        context: XvmContext,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
        skip_apply: bool,
    ) -> XvmCallResult {
        log::trace!(
            target: "xvm::wasm_call",
            "Calling WASM: {:?} {:?}, {:?}, {:?}",
            context, source, target, input,
        );

        let dest = {
            let error = CallErrorWithWeight {
                error: CallError::InvalidTarget,
                used_weight: PLACEHOLDER_WEIGHT,
            };
            let decoded = Decode::decode(&mut target.as_ref()).map_err(|_| error.clone())?;
            T::Lookup::lookup(decoded).map_err(|_| error)
        }?;

        if skip_apply {
            return Ok(CallInfo {
                output: vec![],
                used_weight: PLACEHOLDER_WEIGHT,
            });
        }

        let call_result = pallet_contracts::Pallet::<T>::bare_call(
            source,
            dest,
            Default::default(),
            context.weight_limit,
            None,
            input,
            false,
            pallet_contracts::Determinism::Deterministic,
        );
        log::trace!(target: "xvm::wasm_call", "WASM call result: {:?}", call_result);

        // TODO: add overhead to used weight
        let used_weight = call_result.gas_consumed;

        match call_result.result {
            Ok(success) => Ok(CallInfo {
                output: success.data,
                used_weight,
            }),

            Err(error) => Err(CallErrorWithWeight {
                error: CallError::ExecutionFailed(Into::<&str>::into(error).into()),
                used_weight,
            }),
        }
    }
}

#[cfg(feature = "runtime-benchmarks")]
impl<T: Config> Pallet<T> {
    pub fn evm_call_without_apply(
        context: XvmContext,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
    ) -> XvmCallResult {
        Self::evm_call(context, source, target, input, true)
    }

    pub fn wasm_call_without_apply(
        context: XvmContext,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
    ) -> XvmCallResult {
        Self::wasm_call(context, source, target, input, true)
    }
}
