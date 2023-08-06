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
//! A module to provide
//!
//! ## Overview
//!
//! The XVM pallet provides a runtime interface to call different VMs. It currently
//! supports two VMs: EVM and WASM. With further development, more VMs can be added.
//!
//! Together with other functionalities like Chain Extension and precompiles,
//! the XVM pallet enables the runtime to support cross-VM calls.
//!
//! ## Interface
//!
//! ### Implementation
//!
//! - Implements `XvmCall` trait.
//!

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{ensure, traits::Currency};
use pallet_contracts::{CollectEvents, DebugInfo, Determinism};
use pallet_evm::GasWeightMapping;
use parity_scale_codec::Decode;
use sp_core::{H160, U256};
use sp_runtime::traits::StaticLookup;
use sp_std::{marker::PhantomData, prelude::*};

use astar_primitives::{
    ethereum_checked::{
        AccountMapping, CheckedEthereumTransact, CheckedEthereumTx, EthereumTxInput,
    },
    xvm::{CallError, CallErrorWithWeight, CallInfo, CallResult, Context, VmId, XvmCall},
    Balance,
};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

mod mock;
mod tests;

pub use pallet::*;

pub type WeightInfoOf<T> = <T as Config>::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_contracts::Config {
        /// Mapping from `Account` to `H160`.
        type AccountMapping: AccountMapping<Self::AccountId>;

        /// Mapping from Ethereum gas to Substrate weight.
        type GasWeightMapping: GasWeightMapping;

        /// `CheckedEthereumTransact` implementation.
        type EthereumTransact: CheckedEthereumTransact;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }
}

impl<T> XvmCall<T::AccountId> for Pallet<T>
where
    T: Config,
    T::Currency: Currency<T::AccountId, Balance = Balance>,
{
    fn call(
        context: Context,
        vm_id: VmId,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
        value: Balance,
    ) -> CallResult {
        Pallet::<T>::do_call(context, vm_id, source, target, input, value, false)
    }
}

impl<T> Pallet<T>
where
    T: Config,
    T::Currency: Currency<T::AccountId, Balance = Balance>,
{
    fn do_call(
        context: Context,
        vm_id: VmId,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
        value: Balance,
        skip_execution: bool,
    ) -> CallResult {
        ensure!(
            context.source_vm_id != vm_id,
            CallErrorWithWeight {
                error: CallError::SameVmCallNotAllowed,
                used_weight: match vm_id {
                    VmId::Evm => WeightInfoOf::<T>::evm_call_overheads(),
                    VmId::Wasm => WeightInfoOf::<T>::wasm_call_overheads(),
                },
            }
        );

        match vm_id {
            VmId::Evm => {
                Pallet::<T>::evm_call(context, source, target, input, value, skip_execution)
            }
            VmId::Wasm => {
                Pallet::<T>::wasm_call(context, source, target, input, value, skip_execution)
            }
        }
    }

    fn evm_call(
        context: Context,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
        value: Balance,
        skip_execution: bool,
    ) -> CallResult {
        log::trace!(
            target: "xvm::evm_call",
            "Calling EVM: {:?} {:?}, {:?}, {:?}, {:?}",
            context, source, target, input, value,
        );

        ensure!(
            target.len() == H160::len_bytes(),
            CallErrorWithWeight {
                error: CallError::InvalidTarget,
                used_weight: WeightInfoOf::<T>::evm_call_overheads(),
            }
        );
        let target_decoded =
            Decode::decode(&mut target.as_ref()).map_err(|_| CallErrorWithWeight {
                error: CallError::InvalidTarget,
                used_weight: WeightInfoOf::<T>::evm_call_overheads(),
            })?;
        let bounded_input = EthereumTxInput::try_from(input).map_err(|_| CallErrorWithWeight {
            error: CallError::InputTooLarge,
            used_weight: WeightInfoOf::<T>::evm_call_overheads(),
        })?;

        let value_u256 = U256::from(value);
        // With overheads, less weight is available.
        let weight_limit = context
            .weight_limit
            .saturating_sub(WeightInfoOf::<T>::evm_call_overheads());
        let gas_limit = U256::from(T::GasWeightMapping::weight_to_gas(weight_limit));

        let source = T::AccountMapping::into_h160(source);
        let tx = CheckedEthereumTx {
            gas_limit,
            target: target_decoded,
            value: value_u256,
            input: bounded_input,
            maybe_access_list: None,
        };

        // Note the skip execution check should be exactly before `T::EthereumTransact::xvm_transact`
        // to benchmark the correct overheads.
        if skip_execution {
            return Ok(CallInfo {
                output: vec![],
                used_weight: WeightInfoOf::<T>::evm_call_overheads(),
            });
        }

        let transact_result = T::EthereumTransact::xvm_transact(source, tx);
        log::trace!(
            target: "xvm::evm_call",
            "EVM call result: {:?}", transact_result,
        );

        transact_result
            .map(|(post_dispatch_info, call_info)| {
                let used_weight = post_dispatch_info
                    .actual_weight
                    .unwrap_or_default()
                    .saturating_add(WeightInfoOf::<T>::evm_call_overheads());
                CallInfo {
                    output: call_info.value,
                    used_weight,
                }
            })
            .map_err(|e| {
                let used_weight = e
                    .post_info
                    .actual_weight
                    .unwrap_or_default()
                    .saturating_add(WeightInfoOf::<T>::evm_call_overheads());
                CallErrorWithWeight {
                    error: CallError::ExecutionFailed(Into::<&str>::into(e.error).into()),
                    used_weight,
                }
            })
    }

    fn wasm_call(
        context: Context,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
        value: Balance,
        skip_execution: bool,
    ) -> CallResult {
        log::trace!(
            target: "xvm::wasm_call",
            "Calling WASM: {:?} {:?}, {:?}, {:?}, {:?}",
            context, source, target, input, value,
        );

        let dest = {
            let error = CallErrorWithWeight {
                error: CallError::InvalidTarget,
                used_weight: WeightInfoOf::<T>::wasm_call_overheads(),
            };
            let decoded = Decode::decode(&mut target.as_ref()).map_err(|_| error.clone())?;
            T::Lookup::lookup(decoded).map_err(|_| error)
        }?;

        // With overheads, less weight is available.
        let weight_limit = context
            .weight_limit
            .saturating_sub(WeightInfoOf::<T>::wasm_call_overheads());

        // Note the skip execution check should be exactly before `pallet_contracts::bare_call`
        // to benchmark the correct overheads.
        if skip_execution {
            return Ok(CallInfo {
                output: vec![],
                used_weight: WeightInfoOf::<T>::wasm_call_overheads(),
            });
        }

        let call_result = pallet_contracts::Pallet::<T>::bare_call(
            source,
            dest,
            value,
            weight_limit,
            None,
            input,
            DebugInfo::Skip,
            CollectEvents::Skip,
            Determinism::Enforced,
        );
        log::trace!(target: "xvm::wasm_call", "WASM call result: {:?}", call_result);

        let used_weight = call_result
            .gas_consumed
            .saturating_add(WeightInfoOf::<T>::wasm_call_overheads());
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
    pub fn call_without_execution(
        context: Context,
        vm_id: VmId,
        source: T::AccountId,
        target: Vec<u8>,
        input: Vec<u8>,
        value: Balance,
    ) -> CallResult {
        Self::do_call(context, vm_id, source, target, input, value, true)
    }
}
