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

use frame_support::dispatch::Encode;
use pallet_contracts::chain_extension::{ChainExtension, Environment, Ext, InitState, RetVal};
use pallet_xvm::XvmContext;
use sp_runtime::DispatchError;
use sp_std::marker::PhantomData;
use xvm_chain_extension_types::{XvmCallArgs, XvmExecutionResult};

enum XvmFuncId {
    XvmCall,
    // TODO: expand with other calls too
}

impl TryFrom<u16> for XvmFuncId {
    type Error = DispatchError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(XvmFuncId::XvmCall),
            _ => Err(DispatchError::Other(
                "Unsupported func id in Xvm chain extension",
            )),
        }
    }
}

/// XVM chain extension.
pub struct XvmExtension<T>(PhantomData<T>);

impl<T> Default for XvmExtension<T> {
    fn default() -> Self {
        XvmExtension(PhantomData)
    }
}

impl<T> ChainExtension<T> for XvmExtension<T>
where
    T: pallet_contracts::Config + pallet_xvm::Config,
{
    fn call<E: Ext>(&mut self, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
    {
        let func_id = env.func_id().try_into()?;
        let mut env = env.buf_in_buf_out();

        match func_id {
            XvmFuncId::XvmCall => {
                // We need to immediately charge for the worst case scenario. Gas equals Weight in pallet-contracts context.
                let remaining_weight = env.ext().gas_meter().gas_left();
                // We don't track used proof size, so we can't refund after.
                // So we will charge a 32KB dummy value as a temporary replacement.
                let charged_weight =
                    env.charge_weight(remaining_weight.set_proof_size(32 * 1024))?;

                let caller = env.ext().caller().clone();

                let XvmCallArgs { vm_id, to, input } = env.read_as_unbounded(env.in_len())?;

                let _origin_address = env.ext().address().clone();
                let _value = env.ext().value_transferred();
                let xvm_context = XvmContext {
                    id: vm_id,
                    max_weight: remaining_weight,
                    env: None,
                };

                let call_result =
                    pallet_xvm::Pallet::<T>::xvm_bare_call(xvm_context, caller, to, input);

                let actual_weight = pallet_xvm::consumed_weight(&call_result);
                env.adjust_weight(charged_weight, actual_weight);

                match call_result {
                    Ok(success) => {
                        log::trace!(
                            target: "xvm-extension::xvm_call",
                            "success: {:?}", success
                        );

                        let buffer: sp_std::vec::Vec<_> = success.output().encode();
                        env.write(&buffer, false, None)?;
                        Ok(RetVal::Converging(XvmExecutionResult::Success as u32))
                    }

                    Err(failure) => {
                        log::trace!(
                            target: "xvm-extension::xvm_call",
                            "failure: {:?}", failure
                        );

                        // TODO Propagate error
                        Ok(RetVal::Converging(XvmExecutionResult::UnknownError as u32))
                    }
                }
            }
        }
    }
}
