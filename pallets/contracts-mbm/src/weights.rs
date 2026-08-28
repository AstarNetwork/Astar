// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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

//! Weights for `contracts_mbm`
//!
//! ⚠️ PLACEHOLDER - hand written and MUST be regenerated before this upgrade is proposed. The
//! `proof_size` component is the only thing keeping the purge from producing a PoV oversized
//! block, so it has to reflect reality rather than a model:
//!
//! ```text
//! frame-omni-bencher v1 benchmark pallet \
//!   --runtime=./target/release/wbuild/astar-runtime/astar_runtime.compact.compressed.wasm \
//!   --steps=50 --repeat=20 --pallet=contracts_mbm --extrinsic='*' \
//!   --wasm-execution=compiled --heap-pages=4096 \
//!   --output=./benchmark-results/astar/pallet/mbm_weights.rs \
//!   --template=./scripts/templates/pallet-weight-template.hbs
//! ```

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]
#![allow(dead_code)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight info trait.
pub trait WeightInfo {
	/// Seek to, measure and remove one top level storage key holding an `x` byte value.
	fn remove_key(x: u32) -> Weight;
	/// Seek to, measure and remove one child trie key holding an `x` byte value.
	fn remove_child_key(x: u32) -> Weight;
	/// Hand back the consumer reference `pallet-contracts` took on one contract account.
	fn release_contract_consumer() -> Weight;
}

/// Deliberately pessimistic model:
/// * `ref_time`: the `next_key` seek plus the length probe plus the removal, and 1 µs per KiB of
///   value that has to be moved.
/// * `proof_size`: one trie node worth of overhead per key, plus the value itself.
///
/// `x` ranges over `[0, 128 KiB]` - `pallet_contracts::Config::MaxCodeLen` was 123 KiB on every
/// Astar runtime and `PristineCode` blobs are the largest values involved.
const PROOF_OVERHEAD_PER_KEY: u64 = 1_024;
const PICOSECONDS_PER_BYTE: u64 = 1_000;

/// Weight functions for `contracts_mbm`.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn remove_key(x: u32, ) -> Weight {
		Weight::from_parts(0, PROOF_OVERHEAD_PER_KEY)
			.saturating_add(Weight::from_parts(PICOSECONDS_PER_BYTE, 1).saturating_mul(x.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}

	fn remove_child_key(x: u32, ) -> Weight {
		Weight::from_parts(0, PROOF_OVERHEAD_PER_KEY)
			.saturating_add(Weight::from_parts(PICOSECONDS_PER_BYTE, 1).saturating_mul(x.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}

	/// Storage: `System::Account` (r:1 w:1)
	fn release_contract_consumer() -> Weight {
		Weight::from_parts(0, PROOF_OVERHEAD_PER_KEY)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
