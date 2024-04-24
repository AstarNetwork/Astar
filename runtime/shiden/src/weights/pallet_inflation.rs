
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

//! Autogenerated weights for pallet_inflation
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-02-04, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `gh-runner-01-ovh`, CPU: `Intel(R) Xeon(R) E-2236 CPU @ 3.40GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("shiden-dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/astar-collator
// benchmark
// pallet
// --chain=shiden-dev
// --steps=50
// --repeat=20
// --pallet=pallet_inflation
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./benchmark-results/shiden-dev/inflation_weights.rs
// --template=./scripts/templates/weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;
use pallet_inflation::WeightInfo;

/// Weights for pallet_inflation using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: Inflation InflationParams (r:0 w:1)
	/// Proof: Inflation InflationParams (max_values: Some(1), max_size: Some(64), added: 559, mode: MaxEncodedLen)
	fn force_set_inflation_params() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 9_034_000 picoseconds.
		Weight::from_parts(9_303_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: Inflation InflationParams (r:1 w:0)
	/// Proof: Inflation InflationParams (max_values: Some(1), max_size: Some(64), added: 559, mode: MaxEncodedLen)
	fn force_inflation_recalculation() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `40`
		//  Estimated: `1549`
		// Minimum execution time: 13_548_000 picoseconds.
		Weight::from_parts(13_796_000, 1549)
			.saturating_add(T::DbWeight::get().reads(1_u64))
	}
	/// Storage: Inflation InflationParams (r:1 w:0)
	/// Proof: Inflation InflationParams (max_values: Some(1), max_size: Some(64), added: 559, mode: MaxEncodedLen)
	fn recalculation() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `58`
		//  Estimated: `1549`
		// Minimum execution time: 13_421_000 picoseconds.
		Weight::from_parts(13_808_000, 1549)
			.saturating_add(T::DbWeight::get().reads(1_u64))
	}
	/// Storage: System Account (r:2 w:2)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn hooks_without_recalculation() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `39`
		//  Estimated: `6196`
		// Minimum execution time: 40_566_000 picoseconds.
		Weight::from_parts(40_980_000, 6196)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
}
