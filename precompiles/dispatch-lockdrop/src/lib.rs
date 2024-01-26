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
use frame_support::{codec::DecodeLimit as _, dispatch::Pays, traits::Get};
use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    traits::ConstU32,
};
use frame_system::Config;
use pallet_evm::GasWeightMapping;
use pallet_evm_precompile_dispatch::DispatchValidateT;
use parity_scale_codec::Encode;
use precompile_utils::prelude::{BoundedBytes, UnboundedBytes};
use precompile_utils::{keccak256, EvmResult};
use sp_core::ecdsa::Signature;
use sp_core::{crypto::AccountId32, H160, H256};
use sp_core::{ecdsa, U256};
use sp_io::hashing::keccak_256;
use sp_runtime::traits::Zero;
use sp_std::vec::Vec;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub const LOG_TARGET: &str = "dispatch-lockdrop-precompile";

// ECDSA signature bytes
type ECDSASignatureBytes = ConstU32<65>;

// `DecodeLimit` specifies the max depth a call can use when decoding, as unbounded depth
// can be used to overflow the stack.
// Default value is 8, which is the same as in XCM call decoding.
pub struct DispatchLockdrop<Runtime, DispatchValidator = (), DecodeLimit = ConstU32<8>>(
    PhantomData<(Runtime, DispatchValidator, DecodeLimit)>,
);

#[precompile_utils::precompile]
impl<Runtime, DispatchValidator, DecodeLimit>
    DispatchLockdrop<Runtime, DispatchValidator, DecodeLimit>
where
    Runtime: pallet_evm::Config + pallet_unified_accounts::Config,
    <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    <Runtime as Config>::AccountId: IsType<AccountId32>,
    <Runtime as Config>::AccountId: From<[u8; 32]>,
    DispatchValidator:
        DispatchValidateT<<Runtime as Config>::AccountId, <Runtime as Config>::RuntimeCall>,
    DecodeLimit: Get<u32>,
{
    #[precompile::public("dispatch_lockdrop_call(bytes,bytes32,bytes)")]
    fn dispatch_lockdrop_call(
        handle: &mut impl PrecompileHandle,
        call: UnboundedBytes,
        account_id: H256,
        signature: BoundedBytes<ECDSASignatureBytes>,
    ) -> EvmResult<bool> {
        log::trace!(
            target: LOG_TARGET,
            "raw arguments: call: {:?}, account_id: {:?}, signature: {:?}",
            call,
            account_id,
            signature
        );

        let target_gas = handle.gas_limit();
        let caller: H160 = handle.context().caller.into();
        let input: Vec<u8> = call.into();
        let signature_bytes: Vec<u8> = signature.into();
        let account_id = AccountId32::new(account_id.into()).into();

        // 1. Decode the call
        let call =
            match Runtime::RuntimeCall::decode_with_depth_limit(DecodeLimit::get(), &mut &*input) {
                Ok(c) => c,
                Err(_) => {
                    log::trace!(target: LOG_TARGET, "Error: could not decode call");
                    return Ok(false);
                }
            };

        // 2. Check that gas limit is not exceeded
        let info = call.get_dispatch_info();
        if let Some(gas) = target_gas {
            let valid_weight = info.weight.ref_time()
                <= Runtime::GasWeightMapping::gas_to_weight(gas, false).ref_time();
            if !valid_weight {
                log::trace!(target: LOG_TARGET, "Error: gas limit exceeded");
                return Ok(false);
            }
        }

        // 3. Recover the ECDSA Public key from the signature
        let signature_opt = match Self::parse_signature(&signature_bytes) {
            Some(s) => s,
            None => {
                log::trace!(target: LOG_TARGET, "Error: could not parse signature");
                return Ok(false);
            }
        };
        let payload_hash = Self::build_signing_payload(&account_id);
        let pubkey = match <pallet_unified_accounts::Pallet<Runtime>>::recover_pubkey(
            signature_opt.as_ref(),
            payload_hash,
        ) {
            Some(k) => k,
            None => {
                log::trace!(
                    target: LOG_TARGET,
                    "Error: could not recover pubkey from signature"
                );
                return Ok(false);
            }
        };

        // 4. Ensure that the caller matches the recovered EVM address from the signature
        if caller != Self::get_evm_address_from_pubkey(&pubkey) {
            log::trace!(
                target: LOG_TARGET,
                "Error: caller does not match calculated EVM address"
            );
            return Ok(false);
        }

        // 5. Derive the AccountId from the ECDSA compressed Public key
        let origin = match Self::get_account_id_from_pubkey(pubkey) {
            Some(a) => a,
            None => {
                log::trace!(
                    target: LOG_TARGET,
                    "Error: could not derive AccountId from pubkey"
                );
                return Ok(false);
            }
        };

        // 6. validate the call
        if let Some(_) = DispatchValidator::validate_before_dispatch(&origin, &call) {
            return Ok(false);
        }

        handle
            .record_external_cost(Some(info.weight.ref_time()), Some(info.weight.proof_size()))?;

        match call.dispatch(Some(origin).into()) {
            Ok(post_info) => {
                if post_info.pays_fee(&info) == Pays::Yes {
                    let actual_weight = post_info.actual_weight.unwrap_or(info.weight);
                    let cost = Runtime::GasWeightMapping::weight_to_gas(actual_weight);
                    handle.record_cost(cost)?;

                    handle.refund_external_cost(
                        Some(
                            info.weight
                                .ref_time()
                                .saturating_sub(actual_weight.ref_time()),
                        ),
                        Some(
                            info.weight
                                .proof_size()
                                .saturating_sub(actual_weight.proof_size()),
                        ),
                    );
                }

                Ok(true)
            }
            Err(e) => {
                log::trace!(target: LOG_TARGET, "Error: {:?}", e);
                Ok(false)
            }
        }
    }

    fn get_account_id_from_pubkey(pubkey: [u8; 64]) -> Option<<Runtime as Config>::AccountId> {
        libsecp256k1::PublicKey::parse_slice(&pubkey, None)
            .map(|k| sp_io::hashing::blake2_256(k.serialize_compressed().as_ref()).into())
            .ok()
    }

    fn parse_signature(signature_bytes: &Vec<u8>) -> Option<Signature> {
        ecdsa::Signature::from_slice(&signature_bytes[..])
    }

    fn get_evm_address_from_pubkey(pubkey: &[u8]) -> H160 {
        H160::from(H256::from_slice(&keccak_256(pubkey)))
    }

    fn build_signing_payload(who: &<Runtime as Config>::AccountId) -> [u8; 32] {
        let domain_separator = Self::build_domain_separator();
        let args_hash = Self::build_args_hash(who);

        let mut payload = b"\x19\x01".to_vec();
        payload.extend_from_slice(&domain_separator);
        payload.extend_from_slice(&args_hash);
        keccak_256(&payload)
    }

    fn build_domain_separator() -> [u8; 32] {
        let mut domain =
            keccak256!("EIP712Domain(string name,string version,uint256 chainId,bytes32 salt)")
                .to_vec();
        domain.extend_from_slice(&keccak256!("Astar EVM dispatch")); // name
        domain.extend_from_slice(&keccak256!("1")); // version
        domain.extend_from_slice(
            &(<[u8; 32]>::from(U256::from(
                <Runtime as pallet_unified_accounts::Config>::ChainId::get(),
            ))),
        ); // chain id
        domain.extend_from_slice(
            frame_system::Pallet::<Runtime>::block_hash(<Runtime as Config>::BlockNumber::zero())
                .as_ref(),
        ); // genesis block hash
        keccak_256(domain.as_slice())
    }

    fn build_args_hash(account: &<Runtime as Config>::AccountId) -> [u8; 32] {
        let mut args_hash = keccak256!("Dispatch(bytes substrateAddress)").to_vec();
        args_hash.extend_from_slice(&keccak_256(&account.encode()));
        keccak_256(args_hash.as_slice())
    }
}
