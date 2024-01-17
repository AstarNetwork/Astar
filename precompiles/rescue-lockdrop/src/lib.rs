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

use core::marker::PhantomData;
use fp_evm::PrecompileHandle;
use frame_support::pallet_prelude::IsType;
use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    traits::ConstU32,
};
use frame_system::Config;
use precompile_utils::prelude::BoundedBytes;
use precompile_utils::prelude::RuntimeHelper;
use precompile_utils::EvmResult;
use sp_core::ecdsa;
use sp_core::ecdsa::Signature;
use sp_core::{crypto::AccountId32, H160, H256};
use sp_io::hashing::keccak_256;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// ECDSA signature bytes
type ECDSASignatureBytes = ConstU32<65>;

/// A precompile to unify lock drop account.
pub struct UnifyLockdropPrecompile<R>(PhantomData<R>);

#[precompile_utils::precompile]
impl<R> UnifyLockdropPrecompile<R>
where
    R: pallet_evm::Config + pallet_unified_accounts::Config,
    <R::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<R::AccountId>>,
    R::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    R::RuntimeCall: From<pallet_unified_accounts::Call<R>>,
    <R as Config>::AccountId: IsType<AccountId32>,
    <R as Config>::AccountId: From<[u8; 32]>,
{
    #[precompile::public("claim_lock_drop_account(bytes32,bytes)")]
    fn claim_lock_drop_account(
        handle: &mut impl PrecompileHandle,
        account_id: H256,
        signature: BoundedBytes<ECDSASignatureBytes>,
    ) -> EvmResult<bool> {
        log::trace!(
            target: "rescue-lockdrop-precompile:claim_lock_drop_account",
            "raw arguments: account_id: {:?}, signature: {:?}",
            account_id,
            signature
        );

        let caller = handle.context().caller.into();
        let signature_bytes: Vec<u8> = signature.into();
        let account_id = AccountId32::new(account_id.into()).into();

        let signature_opt = Self::parse_signature(&signature_bytes);

        let pubkey = match <pallet_unified_accounts::Pallet<R>>::recover_pubkey(
            &account_id,
            signature_opt.as_ref(),
        ) {
            Some(k) => k,
            None => {
                log::trace!(
                    target: "rescue-lockdrop-precompile:claim_lock_drop_account",
                    "Error: could not recover pubkey from signature"
                );
                return Ok(false);
            }
        };

        if caller != Self::get_evm_address_from_pubkey(&pubkey) {
            log::trace!(
                target: "rescue-lockdrop-precompile:claim_lock_drop_account",
                "Error: caller does not match calculated EVM address"
            );
            return Ok(false);
        }

        let origin = Self::get_account_id_from_pubkey(pubkey);

        let call = pallet_unified_accounts::Call::<R>::claim_evm_address {
            evm_address: caller,
            signature: signature_opt.into(),
        };

        match RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call) {
            Ok(_) => Ok(true),
            Err(e) => {
                log::trace!(
                    target: "rescue-lockdrop-precompile:claim_lock_drop_account",
                    "Error: {:?}",
                    e
                );
                Ok(false)
            }
        }
    }

    fn get_account_id_from_pubkey(pubkey: [u8; 64]) -> <R as Config>::AccountId {
        let origin =
            sp_io::hashing::blake2_256(ecdsa::Public::from_full(pubkey.as_ref()).unwrap().as_ref())
                .into();
        origin
    }

    fn parse_signature(signature_bytes: &Vec<u8>) -> Signature {
        ecdsa::Signature::from_slice(&signature_bytes[..]).unwrap()
    }

    fn get_evm_address_from_pubkey(pubkey: &[u8]) -> H160 {
        H160::from(H256::from_slice(&keccak_256(pubkey)))
    }
}
