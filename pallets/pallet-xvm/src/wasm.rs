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

//! WASM (substrate contracts) support for XVM pallet.

use crate::*;
use frame_support::traits::Currency;
use parity_scale_codec::HasCompact;
use scale_info::TypeInfo;
use sp_runtime::traits::Get;
use sp_runtime::traits::StaticLookup;
use sp_std::fmt::Debug;
pub struct WASM<I, T>(sp_std::marker::PhantomData<(I, T)>);

type BalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

impl<I, T> SyncVM<T::AccountId> for WASM<I, T>
where
    I: Get<VmId>,
    T: pallet_contracts::Config + frame_system::Config,
    <BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
{
    fn id() -> VmId {
        I::get()
    }

    fn xvm_call(context: XvmContext, from: T::AccountId, to: Vec<u8>, input: Vec<u8>) -> XvmResult {
        log::trace!(
            target: "xvm::WASM::xvm_call",
            "Start WASM XVM: {:?}, {:?}, {:?}",
            from, to, input,
        );
        let gas_limit = context.max_weight;
        log::trace!(
            target: "xvm::WASM::xvm_call",
            "WASM xvm call gas (weight) limit: {:?}", gas_limit);
        let dest = Decode::decode(&mut to.as_ref()).map_err(|_| XvmCallError {
            error: XvmError::EncodingFailure,
            consumed_weight: PLACEHOLDER_WEIGHT,
        })?;

        let dest = T::Lookup::lookup(dest).map_err(|error| XvmCallError {
            error: XvmError::ExecutionError(Into::<&str>::into(error).into()),
            consumed_weight: PLACEHOLDER_WEIGHT,
        })?;
        let call_result = pallet_contracts::Pallet::<T>::bare_call(
            from, // no need to check origin, we consider it signed here
            dest,
            Default::default(),
            gas_limit.into(),
            None,
            input,
            false,
            pallet_contracts::Determinism::Deterministic,
        );

        log::trace!(
            target: "xvm::WASM::xvm_call",
            "WASM XVM call result: {:?}", call_result
        );

        let consumed_weight = call_result.gas_consumed.ref_time();

        match call_result.result {
            Ok(success) => Ok(XvmCallOk {
                output: success.data,
                consumed_weight,
            }),

            Err(error) => Err(XvmCallError {
                error: XvmError::ExecutionError(Into::<&str>::into(error).into()),
                consumed_weight,
            }),
        }
    }
}
