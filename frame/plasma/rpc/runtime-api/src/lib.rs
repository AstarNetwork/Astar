// Copyright 2019-2020 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Runtime API definition required by Plasma RPC extensions.
//!
//! This API should be imported and implemented by the runtime,
//! of a node that wants to use the custom RPC extension
//! adding Plasma access methods.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    /// The API to interact with contracts without using executive.
    pub trait PlasmaApi<AccountId, BlockNumber, Range, Hash, InclusionProof> where
        AccountId: Codec,
        BlockNumber: Codec,
        Range: Codec,
        Hash: Codec,
        InclusionProof: Codec,
    {
        fn retrieve(plapps_id: AccountId, block_number: BlockNumber) -> Hash;

        fn verify_inclusion(
            plapps_id: AccountId,
            leaf: Hash,
            token_address: AccountId,
            range: Range,
            inclusion_proof: InclusionProof,
            block_number: BlockNumber,
        ) -> bool;

        fn verify_inclusion_with_root(
            leaf: Hash,
            token_address: AccountId,
            range: Range,
            inclusion_proof: InclusionProof,
            root: Hash,
        ) -> bool;
    }
}
