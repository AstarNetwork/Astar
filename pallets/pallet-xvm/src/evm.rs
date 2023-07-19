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

//! EVM support for XVM pallet.

use crate::*;
use frame_support::{traits::ConstU32, BoundedVec};
use pallet_evm::GasWeightMapping;
use sp_core::U256;
use sp_runtime::traits::Get;

use astar_primitives::ethereum_checked::{
    AccountMapping as AccountMappingT, CheckedEthereumTransact, CheckedEthereumTx,
    MAX_ETHEREUM_TX_INPUT_SIZE,
};

/// EVM adapter for XVM calls.
///
/// This adapter supports generic XVM calls and encode it into EVM native calls
/// using Solidity ABI codec (https://docs.soliditylang.org/en/v0.8.16/abi-spec.html).
pub struct EVM<I, T, Transact>(sp_std::marker::PhantomData<(I, T, Transact)>);

impl<I, T, Transact> SyncVM<T::AccountId> for EVM<I, T, Transact>
where
    I: Get<VmId>,
    T: frame_system::Config + pallet_evm::Config + pallet_ethereum_checked::Config,
    Transact: CheckedEthereumTransact,
{
    fn id() -> VmId {
        I::get()
    }

    fn xvm_call(context: XvmContext, from: T::AccountId, to: Vec<u8>, input: Vec<u8>) -> XvmResult {
        log::trace!(
            target: "xvm::EVM::xvm_call",
            "Start EVM XVM: {:?}, {:?}, {:?}",
            from, to, input,
        );

        let value = U256::zero();
        let gas_limit = T::GasWeightMapping::weight_to_gas(context.max_weight);

        let source = T::AccountMapping::into_h160(from);
        let target = Decode::decode(&mut to.as_ref()).map_err(|_| XvmCallError {
            error: XvmError::EncodingFailure,
            consumed_weight: PLACEHOLDER_WEIGHT,
        })?;
        let bounded_input = BoundedVec::<u8, ConstU32<MAX_ETHEREUM_TX_INPUT_SIZE>>::try_from(input)
            .map_err(|_| XvmCallError {
                error: XvmError::InputTooLarge,
                consumed_weight: PLACEHOLDER_WEIGHT,
            })?;

        let (post_dispatch_info, call_info) = Transact::xvm_transact(
            source,
            CheckedEthereumTx {
                gas_limit: U256::from(gas_limit),
                target,
                value,
                input: bounded_input,
                maybe_access_list: None,
            },
        )
        .map_err(|e| {
            let consumed_weight = e.post_info.actual_weight.unwrap_or_default();
            XvmCallError {
                error: XvmError::ExecutionError(Into::<&str>::into(e.error).into()),
                consumed_weight,
            }
        })?;

        log::trace!(
            target: "xvm::EVM::xvm_call",
            "EVM XVM call result: exit_reason: {:?}, used_gas: {:?}", call_info.exit_reason, call_info.used_gas,
        );

        Ok(XvmCallOk {
            output: call_info.value,
            consumed_weight: post_dispatch_info.actual_weight.unwrap_or_default(),
        })
    }
}
