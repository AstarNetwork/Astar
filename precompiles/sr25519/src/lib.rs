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

use fp_evm::PrecompileHandle;
use sp_core::ConstU32;
use sp_core::{crypto::UncheckedFrom, sr25519, H256};
use sp_std::marker::PhantomData;

use precompile_utils::prelude::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// SR25519 signature bytes
type SR25519SignatureBytes = ConstU32<64>;

/// A precompile to wrap substrate sr25519 functions.
pub struct Sr25519Precompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime: pallet_evm::Config> Sr25519Precompile<Runtime> {
    #[precompile::public("verify(bytes32,bytes,bytes)")]
    #[precompile::view]
    fn verify(
        _: &mut impl PrecompileHandle,
        public: H256,
        signature: BoundedBytes<SR25519SignatureBytes>,
        message: UnboundedBytes,
    ) -> EvmResult<bool> {
        // Parse pub key
        let public = sr25519::Public::unchecked_from(public);
        // Parse signature
        let signature = if let Some(sig) = sr25519::Signature::from_slice(&signature.as_bytes()) {
            sig
        } else {
            // Return `false` if signature length is wrong
            return Ok(false);
        };

        log::trace!(
            target: "sr25519-precompile",
            "Verify signature {:?} for public {:?} and message {:?}",
            signature, public, message,
        );

        let is_confirmed =
            sp_io::crypto::sr25519_verify(&signature, &message.as_bytes(), &public.into());

        log::trace!(
            target: "sr25519-precompile",
            "Verified signature {:?} is {:?}",
            signature, is_confirmed,
        );

        Ok(is_confirmed)
    }
}
