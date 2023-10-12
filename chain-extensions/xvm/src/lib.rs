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
    evm::UnifiedAddressMapper,
    xvm::{Context, VmId, XvmCall},
};
use frame_support::{dispatch::Encode, traits::Get, weights::Weight};
use frame_system::RawOrigin;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, ReturnFlags,
};
use pallet_unified_accounts::WeightInfo;
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
pub struct XvmExtension<T, XC, UA>(PhantomData<(T, XC, UA)>);

impl<T, XC, UA> Default for XvmExtension<T, XC, UA> {
    fn default() -> Self {
        XvmExtension(PhantomData)
    }
}

impl<T, XC, UA> ChainExtension<T> for XvmExtension<T, XC, UA>
where
    T: pallet_contracts::Config + pallet_unified_accounts::Config,
    XC: XvmCall<T::AccountId>,
    UA: UnifiedAddressMapper<T::AccountId>,
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
                let source = env.ext().address().clone();

                // Claim the default evm address if needed.
                let mut actual_weight = Weight::zero();
                if value > 0 {
                    // `UA::to_h160` 1 DB read.
                    actual_weight.saturating_accrue(T::DbWeight::get().reads(1));

                    if UA::to_h160(&source).is_none() {
                        let weight_of_claim = <T as pallet_unified_accounts::Config>::WeightInfo::claim_default_evm_address();
                        actual_weight.saturating_accrue(weight_of_claim);

                        let claim_result =
                            pallet_unified_accounts::Pallet::<T>::claim_default_evm_address(
                                RawOrigin::Signed(source.clone()).into(),
                            );
                        if claim_result.is_err() {
                            return Ok(RetVal::Diverging {
                                flags: ReturnFlags::REVERT,
                                data: format!("{:?}", claim_result.err()).into(),
                            });
                        }
                    }
                }

                let xvm_context = Context {
                    source_vm_id: VmId::Wasm,
                    weight_limit,
                };
                let vm_id = {
                    match TryInto::<VmId>::try_into(vm_id) {
                        Ok(id) => id,
                        Err(err) => {
                            return Ok(RetVal::Diverging {
                                flags: ReturnFlags::REVERT,
                                data: format!("{:?}", err).into(),
                            });
                        }
                    }
                };
                let call_result = XC::call(xvm_context, vm_id, source, to, input, value, None);

                let used_weight = match call_result {
                    Ok(ref info) => info.used_weight,
                    Err(ref err) => err.used_weight,
                };
                actual_weight.saturating_accrue(used_weight);
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

                        // `Diverging` is used instead of `Err` to make sure the control
                        // doesn't return to the caller.
                        Ok(RetVal::Diverging {
                            flags: ReturnFlags::REVERT,
                            data: format!("{:?}", err).into(),
                        })
                    }
                }
            }
        }
    }
}
