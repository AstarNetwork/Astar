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

use astar_primitives::xvm::{Context, VmId, XvmCall};
use frame_support::dispatch::Encode;
use pallet_contracts::chain_extension::{ChainExtension, Environment, Ext, InitState, RetVal};
use sp_runtime::DispatchError;
use sp_std::marker::PhantomData;
use xvm_chain_extension_types::{XvmCallArgs, XvmExecutionResult};

enum XvmFuncId {
    Call,
}

impl TryFrom<u16> for XvmFuncId {
    type Error = DispatchError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(XvmFuncId::Call),
            _ => Err(DispatchError::Other(
                "Unsupported func id in Xvm chain extension",
            )),
        }
    }
}

/// XVM chain extension.
pub struct XvmExtension<T, XC>(PhantomData<(T, XC)>);

impl<T, XC> Default for XvmExtension<T, XC> {
    fn default() -> Self {
        XvmExtension(PhantomData)
    }
}

impl<T, XC> ChainExtension<T> for XvmExtension<T, XC>
where
    T: pallet_contracts::Config,
    XC: XvmCall<T::AccountId>,
{
    fn call<E: Ext>(&mut self, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
    {
        let func_id = env.func_id().try_into()?;
        let mut env = env.buf_in_buf_out();

        match func_id {
            XvmFuncId::Call => {
                // We need to immediately charge for the worst case scenario. Gas equals Weight in pallet-contracts context.
                let weight_limit = env.ext().gas_meter().gas_left();
                // TODO: track proof size in align fees ticket
                // We don't track used proof size, so we can't refund after.
                // So we will charge a 32KB dummy value as a temporary replacement.
                let charged_weight = env.charge_weight(weight_limit.set_proof_size(32 * 1024))?;

                let XvmCallArgs {
                    vm_id,
                    to,
                    input,
                    value,
                } = env.read_as_unbounded(env.in_len())?;

                // Similar to EVM behavior, the `source` should be (limited to) the
                // contract address. Otherwise contracts would be able to do arbitrary
                // things on behalf of the caller via XVM.
                let source = env.ext().address();

                let xvm_context = Context {
                    source_vm_id: VmId::Wasm,
                    weight_limit,
                };
                let vm_id = {
                    match TryInto::<VmId>::try_into(vm_id) {
                        Ok(id) => id,
                        Err(err) => {
                            // TODO: Propagate error
                            let result = Into::<XvmExecutionResult>::into(err);
                            return Ok(RetVal::Converging(result.into()));
                        }
                    }
                };
                let call_result = XC::call(xvm_context, vm_id, source.clone(), to, input, value);

                let actual_weight = match call_result {
                    Ok(ref info) => info.used_weight,
                    Err(ref err) => err.used_weight,
                };
                env.adjust_weight(charged_weight, actual_weight);

                match call_result {
                    Ok(info) => {
                        log::trace!(
                            target: "xvm-extension::xvm_call",
                            "info: {:?}", info
                        );

                        let buffer: sp_std::vec::Vec<_> = info.output.encode();
                        env.write(&buffer, false, None)?;
                        Ok(RetVal::Converging(XvmExecutionResult::Ok.into()))
                    }

                    Err(err) => {
                        log::trace!(
                            target: "xvm-extension::xvm_call",
                            "err: {:?}", err
                        );

                        // TODO Propagate error
                        let result = Into::<XvmExecutionResult>::into(err.error);
                        Ok(RetVal::Converging(result.into()))
                    }
                }
            }
        }
    }
}
