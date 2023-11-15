
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

//! Autogenerated weights for pallet_dapp_staking_v3
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-11-14, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Dinos-MBP`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/astar-collator
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_dapp_staking-v3
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=dapp_staking_v3.rs
// --template=./scripts/templates/weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for pallet_dapp_staking_v3.
pub trait WeightInfo {
	fn dapp_tier_assignment(x: u32, ) -> Weight;
}

/// Weights for pallet_dapp_staking_v3 using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: DappStaking CounterForIntegratedDApps (r:1 w:0)
	/// Proof: DappStaking CounterForIntegratedDApps (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	/// Storage: DappStaking ContractStake (r:101 w:0)
	/// Proof: DappStaking ContractStake (max_values: Some(65535), max_size: Some(93), added: 2073, mode: MaxEncodedLen)
	/// Storage: DappStaking TierConfig (r:1 w:0)
	/// Proof: DappStaking TierConfig (max_values: Some(1), max_size: Some(161), added: 656, mode: MaxEncodedLen)
	/// The range of component `x` is `[0, 100]`.
	fn dapp_tier_assignment(x: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `449 + x * (33 ±0)`
		//  Estimated: `3063 + x * (2073 ±0)`
		// Minimum execution time: 9_000_000 picoseconds.
		Weight::from_parts(16_776_512, 3063)
			// Standard Error: 3_400
			.saturating_add(Weight::from_parts(2_636_298, 0).saturating_mul(x.into()))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(x.into())))
			.saturating_add(Weight::from_parts(0, 2073).saturating_mul(x.into()))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: DappStaking CounterForIntegratedDApps (r:1 w:0)
	/// Proof: DappStaking CounterForIntegratedDApps (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	/// Storage: DappStaking ContractStake (r:101 w:0)
	/// Proof: DappStaking ContractStake (max_values: Some(65535), max_size: Some(93), added: 2073, mode: MaxEncodedLen)
	/// Storage: DappStaking TierConfig (r:1 w:0)
	/// Proof: DappStaking TierConfig (max_values: Some(1), max_size: Some(161), added: 656, mode: MaxEncodedLen)
	/// The range of component `x` is `[0, 100]`.
	fn dapp_tier_assignment(x: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `449 + x * (33 ±0)`
		//  Estimated: `3063 + x * (2073 ±0)`
		// Minimum execution time: 9_000_000 picoseconds.
		Weight::from_parts(16_776_512, 3063)
			// Standard Error: 3_400
			.saturating_add(Weight::from_parts(2_636_298, 0).saturating_mul(x.into()))
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(x.into())))
			.saturating_add(Weight::from_parts(0, 2073).saturating_mul(x.into()))
	}
}