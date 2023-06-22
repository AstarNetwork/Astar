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
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::{PrecompileHandle, PrecompileOutput};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{AddressMapping, Precompile};
use pallet_xvm::XvmContext;
use parity_scale_codec::Decode;
use sp_runtime::codec::Encode;
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use precompile_utils::{
    revert, succeed, Bytes, EvmDataWriter, EvmResult, FunctionModifier, PrecompileHandleExt,
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    XvmCall = "xvm_call(bytes,bytes,bytes)",
}

/// A precompile that expose XVM related functions.
pub struct XvmPrecompile<T>(PhantomData<T>);

impl<R> Precompile for XvmPrecompile<R>
where
    R: pallet_evm::Config + pallet_xvm::Config,
    <<R as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<R::AccountId>>,
    <R as frame_system::Config>::RuntimeCall:
        From<pallet_xvm::Call<R>> + Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
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

impl<R> XvmPrecompile<R>
where
    R: pallet_evm::Config + pallet_xvm::Config,
    <<R as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<R::AccountId>>,
    <R as frame_system::Config>::RuntimeCall:
        From<pallet_xvm::Call<R>> + Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
{
    fn xvm_call(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(4)?;

        // Read arguments and check it
        // TODO: This approach probably needs to be revised - does contract call need to specify gas/weight? Usually it is implicit.
        let context_raw = input.read::<Bytes>()?;
        let context: XvmContext = Decode::decode(&mut context_raw.0.as_ref())
            .map_err(|_| revert("can not decode XVM context"))?;

        // Fetch the remaining gas (weight) available for execution
        // TODO: rework
        //let remaining_gas = handle.remaining_gas();
        //let remaining_weight = R::GasWeightMapping::gas_to_weight(remaining_gas);
        //context.max_weight = remaining_weight;

        let call_to = input.read::<Bytes>()?.0;
        let call_input = input.read::<Bytes>()?.0;

        let from = R::AddressMapping::into_account_id(handle.context().caller);
        match &pallet_xvm::Pallet::<R>::xvm_bare_call(context, from, call_to, call_input) {
            Ok(success) => {
                log::trace!(
                    target: "xvm-precompile::xvm_call",
                    "success: {:?}", success
                );

                Ok(succeed(
                    EvmDataWriter::new()
                        .write(true)
                        .write(Bytes(success.output().to_vec())) // TODO redundant clone
                        .build(),
                ))
            }

            Err(failure) => {
                log::trace!(
                    target: "xvm-precompile::xvm_call",
                    "failure: {:?}", failure
                );

                let mut error_buffer = Vec::new();
                failure.error().encode_to(&mut error_buffer);

                Ok(succeed(
                    EvmDataWriter::new()
                        .write(false)
                        .write(Bytes(error_buffer))
                        .build(),
                ))
            }
        }
    }
}
