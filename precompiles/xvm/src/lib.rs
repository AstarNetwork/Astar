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

extern crate alloc;
use alloc::format;

use astar_primitives::{
    xvm::{Context, FailureReason, VmId, XvmCall},
    Balance,
};
use fp_evm::{PrecompileHandle, PrecompileOutput};
use frame_support::dispatch::Dispatchable;
use pallet_evm::{AddressMapping, GasWeightMapping, Precompile};
use sp_std::{marker::PhantomData, prelude::*};

use precompile_utils::{
    revert, succeed, Bytes, EvmDataWriter, EvmResult, FunctionModifier, PrecompileHandleExt,
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// The selector on EVM revert, calculated by: `Keccak256::digest(b"Error(string)")[..4]`
const EVM_ERROR_MSG_SELECTOR: [u8; 4] = [8, 195, 121, 160];

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    XvmCall = "xvm_call(uint8,bytes,bytes,uint256)",
}

/// A precompile that expose XVM related functions.
pub struct XvmPrecompile<T, XC>(PhantomData<(T, XC)>);

impl<R, XC> Precompile for XvmPrecompile<R, XC>
where
    R: pallet_evm::Config,
    <<R as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<R::AccountId>>,
    XC: XvmCall<R::AccountId>,
{
    fn execute(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "xvm-precompile", "In XVM precompile");

        let selector = handle.read_selector()?;

        handle.check_function_modifier(FunctionModifier::NonPayable)?;

        match selector {
            // Dispatchables
            Action::XvmCall => Self::xvm_call(handle),
        }
    }
}

impl<R, XC> XvmPrecompile<R, XC>
where
    R: pallet_evm::Config,
    <<R as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<R::AccountId>>,
    XC: XvmCall<R::AccountId>,
{
    fn xvm_call(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(4)?;

        let vm_id = {
            let id = input.read::<u8>()?;
            id.try_into().map_err(|_| revert("invalid vm id"))
        }?;

        let mut gas_limit = handle.remaining_gas();
        // If user specified a gas limit, make sure it's not exceeded.
        if let Some(user_limit) = handle.gas_limit() {
            gas_limit = gas_limit.min(user_limit);
        }
        let weight_limit = R::GasWeightMapping::gas_to_weight(gas_limit, true);
        let xvm_context = Context {
            source_vm_id: VmId::Evm,
            weight_limit,
        };

        let call_to = input.read::<Bytes>()?.0;
        let call_input = input.read::<Bytes>()?.0;
        let value = input.read::<Balance>()?;
        let from = R::AddressMapping::into_account_id(handle.context().caller);

        let call_result = XC::call(xvm_context, vm_id, from, call_to, call_input, value);

        let used_weight = match &call_result {
            Ok(s) => s.used_weight,
            Err(f) => f.used_weight,
        };
        handle.record_cost(R::GasWeightMapping::weight_to_gas(used_weight))?;
        handle
            .record_external_cost(Some(used_weight.ref_time()), Some(used_weight.proof_size()))?;

        match call_result {
            Ok(success) => {
                log::trace!(
                    target: "xvm-precompile::xvm_call",
                    "success: {:?}", success
                );

                Ok(succeed(
                    EvmDataWriter::new()
                        .write(true)
                        .write(Bytes(success.output))
                        .build(),
                ))
            }

            Err(failure) => {
                log::trace!(
                    target: "xvm-precompile::xvm_call",
                    "failure: {:?}", failure
                );

                // On `FailureReason::Error` cases, use `revert` instead of `error` to
                // allow error details propagate to caller. EVM implementation always reverts,
                // no matter which one is used.
                let message = match failure.reason {
                    FailureReason::Revert(failure_revert) => {
                        format!("{:?}", failure_revert)
                    }
                    FailureReason::Error(failure_error) => {
                        format!("{:?}", failure_error)
                    }
                };
                let data =
                    EvmDataWriter::new_with_selector(u32::from_be_bytes(EVM_ERROR_MSG_SELECTOR))
                        .write(Bytes(message.into_bytes()))
                        .build();
                Err(revert(data))
            }
        }
    }
}
