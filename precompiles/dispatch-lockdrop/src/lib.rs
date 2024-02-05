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

use core::marker::PhantomData;
use fp_evm::PrecompileHandle;
use frame_support::pallet_prelude::IsType;
use frame_support::weights::Weight;
use frame_support::{codec::DecodeLimit as _, traits::Get};
use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    traits::ConstU32,
};
use frame_system::Config;
use pallet_evm::GasWeightMapping;
use pallet_evm_precompile_dispatch::DispatchValidateT;
use precompile_utils::prelude::{revert, BoundedBytes, RuntimeHelper, UnboundedBytes};
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

        // Record a fixed amount of weight to ensure there is no free execution
        handle.record_cost(Runtime::GasWeightMapping::weight_to_gas(
            Weight::from_parts(1_000_000_000u64, 0),
        ))?;

        // Ensure that the caller matches the public key
        if caller != Self::get_evm_address_from_pubkey(pubkey.as_bytes()) {
            let message: &str = "caller does not match the public key";
            log::trace!(target: LOG_TARGET, "{}", message);
            return Err(revert(message));
        }

        // Derive the account id from the public key
        let origin = Self::get_account_id_from_pubkey(pubkey.as_bytes())
            .ok_or(revert("could not derive AccountId from pubkey"))?;

        // Decode the call
        let call = Runtime::RuntimeCall::decode_with_depth_limit(DecodeLimit::get(), &mut &*input)
            .map_err(|_| revert("could not decode call"))?;

        // Validate the call - ensure that the call is allowed in filter
        DispatchValidator::validate_before_dispatch(&origin, &call)
            .map_or_else(|| Ok(()), |_| Err(revert("invalid Call")))?;

        // Dispatch the call and handle the cost
        RuntimeHelper::<Runtime>::try_dispatch::<Runtime::RuntimeCall>(
            handle,
            Some(origin).into(),
            call,
        )?;

        Ok(true)
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
