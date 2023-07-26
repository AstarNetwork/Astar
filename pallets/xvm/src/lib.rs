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

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{ensure, pallet_prelude::*, traits::ConstU32, BoundedVec};
use pallet_evm::GasWeightMapping;
use parity_scale_codec::Decode;
use sp_core::U256;
use sp_runtime::traits::StaticLookup;
use sp_std::{marker::PhantomData, prelude::*};

use astar_primitives::{
    ethereum_checked::{
        AccountMapping, CheckedEthereumTransact, CheckedEthereumTx, MAX_ETHEREUM_TX_INPUT_SIZE,
    },
    xvm::{CallError, CallErrorWithWeight, CallInfo, Context, VmId, XvmCall, XvmCallResult},
};

pub use pallet::*;

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

impl<T: Config> XvmCall<T::AccountId> for Pallet<T> {
    fn call(
        context: Context,
        vm_id: VmId,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
    ) -> XvmCallResult {
        Pallet::<T>::do_call(context, vm_id, source, target, input, false)
    }
}

// TODO: benchmark XVM calls overhead
pub const PLACEHOLDER_WEIGHT: Weight = Weight::from_parts(1_000_000, 1024);

impl<T: Config> Pallet<T> {
    fn do_call(
        context: Context,
        vm_id: VmId,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
        skip_execution: bool,
    ) -> XvmCallResult {
        ensure!(
            context.source_vm_id != vm_id,
            CallErrorWithWeight {
                error: CallError::SameVmCallNotAllowed,
                used_weight: PLACEHOLDER_WEIGHT,
            }
        );

        match context.source_vm_id {
            VmId::Evm => Pallet::<T>::evm_call(context, source, target, input, skip_execution),
            VmId::Wasm => Pallet::<T>::wasm_call(context, source, target, input, skip_execution),
        }
    }

    fn evm_call(
        context: Context,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
        skip_execution: bool,
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

        if skip_execution {
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
        context: Context,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
        skip_execution: bool,
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

        if skip_execution {
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

    #[cfg(feature = "runtime-benchmarks")]
    pub fn xvm_call_without_execution(
        context: Context,
        vm_id: VmId,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
    ) -> XvmCallResult {
        Self::do_call(context, vm_id, source, target, input, true)
    }
}
