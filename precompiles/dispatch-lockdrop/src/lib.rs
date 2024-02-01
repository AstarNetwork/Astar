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
use core::marker::PhantomData;
use fp_evm::{ExitError, PrecompileFailure, PrecompileHandle};
use frame_support::pallet_prelude::IsType;
use frame_support::{codec::DecodeLimit as _, dispatch::Pays, traits::Get};
use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    traits::ConstU32,
};
use frame_system::Config;
use pallet_evm::GasWeightMapping;
use pallet_evm_precompile_dispatch::DispatchValidateT;
use precompile_utils::prelude::{revert, BoundedBytes, UnboundedBytes};
use precompile_utils::EvmResult;
use sp_core::{crypto::AccountId32, H160, H256};
use sp_io::hashing::keccak_256;
use sp_std::vec::Vec;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub const LOG_TARGET: &str = "precompile::dispatch-lockdrop";

// ECDSA PublicKey
type ECDSAPublic = ConstU32<64>;

// `DecodeLimit` specifies the max depth a call can use when decoding, as unbounded depth
// can be used to overflow the stack.
// Default value is 8, which is the same as in XCM call decoding.
pub struct DispatchLockdrop<Runtime, DispatchValidator, DecodeLimit = ConstU32<8>>(
    PhantomData<(Runtime, DispatchValidator, DecodeLimit)>,
);

#[precompile_utils::precompile]
impl<Runtime, DispatchValidator, DecodeLimit>
    DispatchLockdrop<Runtime, DispatchValidator, DecodeLimit>
where
    Runtime: pallet_evm::Config,
    <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    <Runtime as Config>::AccountId: IsType<AccountId32>,
    <Runtime as Config>::AccountId: From<[u8; 32]>,
    DispatchValidator:
        DispatchValidateT<<Runtime as Config>::AccountId, <Runtime as Config>::RuntimeCall>,
    DecodeLimit: Get<u32>,
{
    #[precompile::public("dispatch_lockdrop_call(bytes,bytes)")]
    fn dispatch_lockdrop_call(
        handle: &mut impl PrecompileHandle,
        call: UnboundedBytes,
        pubkey: BoundedBytes<ECDSAPublic>,
    ) -> EvmResult<bool> {
        log::trace!(
            target: LOG_TARGET,
            "raw arguments: call: {:?}, pubkey: {:?}",
            call,
            pubkey
        );

        let caller: H160 = handle.context().caller.into();
        let input: Vec<u8> = call.into();

        // 1. Decode the call
        let call = Runtime::RuntimeCall::decode_with_depth_limit(DecodeLimit::get(), &mut &*input)
            .map_err(|_| revert("could not decode call"))?;

        // 2. Check if dispatching the call will not exceed the gas limit
        let mut gas_limit = handle.remaining_gas();
        // If caller specified a gas limit, make sure it's not exceeded.
        if let Some(user_limit) = handle.gas_limit() {
            gas_limit = gas_limit.min(user_limit);
        }

        let info = call.get_dispatch_info();

        // Charge the weight of the call to dispatch AND the overhead weight
        // corresponding to the blake2b Hash and the keccak256 Hash
        // based on the weight of UA::claim_default_evm_address()
        let weight = info.weight.ref_time().saturating_add(40_000_000u64);
        if !(weight <= Runtime::GasWeightMapping::gas_to_weight(gas_limit, false).ref_time()) {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::OutOfGas,
            });
        }
        handle.record_external_cost(Some(weight), None)?;

        // 3. Ensure that the caller matches the public key
        if caller != Self::get_evm_address_from_pubkey(pubkey.as_bytes()) {
            let message: &str = "caller does not match the public key";
            log::trace!(target: LOG_TARGET, "{}", message);
            return Err(revert(message));
        }

        // 4. Derive the AccountId from the ECDSA compressed Public key
        let origin = Self::get_account_id_from_pubkey(pubkey.as_bytes())
            .ok_or(revert("could not derive AccountId from pubkey"))?;

        // 5. validate the call
        DispatchValidator::validate_before_dispatch(&origin, &call)
            .map_or_else(|| Ok(()), |_| Err(revert("could not validate call")))?;

        // 6. Dispatch the call
        match call.dispatch(Some(origin).into()) {
            Ok(post_info) => {
                if post_info.pays_fee(&info) == Pays::Yes {
                    let actual_weight = post_info.actual_weight.unwrap_or(info.weight);
                    let cost = Runtime::GasWeightMapping::weight_to_gas(actual_weight);
                    handle.record_external_cost(None, Some(info.weight.proof_size()))?;

                    handle.refund_external_cost(
                        Some(
                            info.weight
                                .ref_time()
                                .saturating_sub(actual_weight.ref_time()),
                        ),
                        None,
                    );
                }

                Ok(true)
            }
            Err(e) => {
                log::trace!(target: LOG_TARGET, "{:?}", e);
                Err(revert(format!(
                    "dispatch execution failed: {}",
                    <&'static str>::from(e)
                )))
            }
        }
    }

    fn get_account_id_from_pubkey(pubkey: &[u8]) -> Option<<Runtime as Config>::AccountId> {
        libsecp256k1::PublicKey::parse_slice(pubkey, None)
            .map(|k| sp_io::hashing::blake2_256(k.serialize_compressed().as_ref()).into())
            .ok()
    }

    fn get_evm_address_from_pubkey(pubkey: &[u8]) -> H160 {
        H160::from(H256::from_slice(&keccak_256(pubkey)))
    }
}
