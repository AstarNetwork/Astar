
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

//! Autogenerated weights for pallet_dapp_staking
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-05-01, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `gh-runner-01-ovh`, CPU: `Intel(R) Xeon(R) E-2236 CPU @ 3.40GHz`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("shiden-dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/astar-collator
// benchmark
// pallet
// --chain=shiden-dev
// --steps=50
// --repeat=20
// --pallet=pallet_dapp_staking
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./benchmark-results/shiden-dev/dapp_staking_weights.rs
// --template=./scripts/templates/weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;
use pallet_dapp_staking::WeightInfo;

/// Weights for pallet_dapp_staking using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn maintenance_mode() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_968_000 picoseconds.
		Weight::from_parts(6_128_000, 0)
	}
	/// Storage: `DappStaking::IntegratedDApps` (r:1 w:1)
	/// Proof: `DappStaking::IntegratedDApps` (`max_values`: Some(65535), `max_size`: Some(116), added: 2096, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::CounterForIntegratedDApps` (r:1 w:1)
	/// Proof: `DappStaking::CounterForIntegratedDApps` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::NextDAppId` (r:1 w:1)
	/// Proof: `DappStaking::NextDAppId` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	fn register() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `3086`
		// Minimum execution time: 12_156_000 picoseconds.
		Weight::from_parts(12_383_000, 3086)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: `DappStaking::IntegratedDApps` (r:1 w:1)
	/// Proof: `DappStaking::IntegratedDApps` (`max_values`: Some(65535), `max_size`: Some(116), added: 2096, mode: `MaxEncodedLen`)
	fn set_dapp_reward_beneficiary() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `97`
		//  Estimated: `3086`
		// Minimum execution time: 10_444_000 picoseconds.
		Weight::from_parts(10_815_000, 3086)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: `DappStaking::IntegratedDApps` (r:1 w:1)
	/// Proof: `DappStaking::IntegratedDApps` (`max_values`: Some(65535), `max_size`: Some(116), added: 2096, mode: `MaxEncodedLen`)
	fn set_dapp_owner() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `97`
		//  Estimated: `3086`
		// Minimum execution time: 10_370_000 picoseconds.
		Weight::from_parts(10_652_000, 3086)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: `DappStaking::IntegratedDApps` (r:1 w:1)
	/// Proof: `DappStaking::IntegratedDApps` (`max_values`: Some(65535), `max_size`: Some(116), added: 2096, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::CounterForIntegratedDApps` (r:1 w:1)
	/// Proof: `DappStaking::CounterForIntegratedDApps` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::ContractStake` (r:0 w:1)
	/// Proof: `DappStaking::ContractStake` (`max_values`: Some(65535), `max_size`: Some(91), added: 2071, mode: `MaxEncodedLen`)
	fn unregister() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `97`
		//  Estimated: `3086`
		// Minimum execution time: 14_692_000 picoseconds.
		Weight::from_parts(14_973_000, 3086)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: `DappStaking::Ledger` (r:1 w:1)
	/// Proof: `DappStaking::Ledger` (`max_values`: None, `max_size`: Some(310), added: 2785, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::Candidates` (r:1 w:0)
	/// Proof: `CollatorSelection::Candidates` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(67), added: 2542, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::CurrentEraInfo` (r:1 w:1)
	/// Proof: `DappStaking::CurrentEraInfo` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	fn lock_new_account() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `138`
		//  Estimated: `4764`
		// Minimum execution time: 27_703_000 picoseconds.
		Weight::from_parts(28_088_000, 4764)
			.saturating_add(T::DbWeight::get().reads(5_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: `DappStaking::Ledger` (r:1 w:1)
	/// Proof: `DappStaking::Ledger` (`max_values`: None, `max_size`: Some(310), added: 2785, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(67), added: 2542, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::CurrentEraInfo` (r:1 w:1)
	/// Proof: `DappStaking::CurrentEraInfo` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	fn lock_existing_account() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `158`
		//  Estimated: `4764`
		// Minimum execution time: 31_136_000 picoseconds.
		Weight::from_parts(31_749_000, 4764)
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: `DappStaking::Ledger` (r:1 w:1)
	/// Proof: `DappStaking::Ledger` (`max_values`: None, `max_size`: Some(310), added: 2785, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(67), added: 2542, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::CurrentEraInfo` (r:1 w:1)
	/// Proof: `DappStaking::CurrentEraInfo` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	fn unlock() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `158`
		//  Estimated: `4764`
		// Minimum execution time: 28_484_000 picoseconds.
		Weight::from_parts(28_994_000, 4764)
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: `DappStaking::Ledger` (r:1 w:1)
	/// Proof: `DappStaking::Ledger` (`max_values`: None, `max_size`: Some(310), added: 2785, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(67), added: 2542, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::CurrentEraInfo` (r:1 w:1)
	/// Proof: `DappStaking::CurrentEraInfo` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// The range of component `x` is `[0, 16]`.
	fn claim_unlocked(x: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `191`
		//  Estimated: `4764`
		// Minimum execution time: 28_418_000 picoseconds.
		Weight::from_parts(29_649_616, 4764)
			// Standard Error: 2_468
			.saturating_add(Weight::from_parts(117_983, 0).saturating_mul(x.into()))
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: `DappStaking::Ledger` (r:1 w:1)
	/// Proof: `DappStaking::Ledger` (`max_values`: None, `max_size`: Some(310), added: 2785, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(67), added: 2542, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::CurrentEraInfo` (r:1 w:1)
	/// Proof: `DappStaking::CurrentEraInfo` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	fn relock_unlocking() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `200`
		//  Estimated: `4764`
		// Minimum execution time: 25_786_000 picoseconds.
		Weight::from_parts(26_585_000, 4764)
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: `DappStaking::IntegratedDApps` (r:1 w:0)
	/// Proof: `DappStaking::IntegratedDApps` (`max_values`: Some(65535), `max_size`: Some(116), added: 2096, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::Ledger` (r:1 w:1)
	/// Proof: `DappStaking::Ledger` (`max_values`: None, `max_size`: Some(310), added: 2785, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::StakerInfo` (r:1 w:1)
	/// Proof: `DappStaking::StakerInfo` (`max_values`: None, `max_size`: Some(178), added: 2653, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::ContractStake` (r:1 w:1)
	/// Proof: `DappStaking::ContractStake` (`max_values`: Some(65535), `max_size`: Some(91), added: 2071, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::CurrentEraInfo` (r:1 w:1)
	/// Proof: `DappStaking::CurrentEraInfo` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(67), added: 2542, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	fn stake() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `274`
		//  Estimated: `4764`
		// Minimum execution time: 38_893_000 picoseconds.
		Weight::from_parts(39_246_000, 4764)
			.saturating_add(T::DbWeight::get().reads(7_u64))
			.saturating_add(T::DbWeight::get().writes(5_u64))
	}
	/// Storage: `DappStaking::IntegratedDApps` (r:1 w:0)
	/// Proof: `DappStaking::IntegratedDApps` (`max_values`: Some(65535), `max_size`: Some(116), added: 2096, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::Ledger` (r:1 w:1)
	/// Proof: `DappStaking::Ledger` (`max_values`: None, `max_size`: Some(310), added: 2785, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::StakerInfo` (r:1 w:1)
	/// Proof: `DappStaking::StakerInfo` (`max_values`: None, `max_size`: Some(178), added: 2653, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::ContractStake` (r:1 w:1)
	/// Proof: `DappStaking::ContractStake` (`max_values`: Some(65535), `max_size`: Some(91), added: 2071, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::CurrentEraInfo` (r:1 w:1)
	/// Proof: `DappStaking::CurrentEraInfo` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(67), added: 2542, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	fn unstake() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `459`
		//  Estimated: `4764`
		// Minimum execution time: 43_357_000 picoseconds.
		Weight::from_parts(43_678_000, 4764)
			.saturating_add(T::DbWeight::get().reads(7_u64))
			.saturating_add(T::DbWeight::get().writes(5_u64))
	}
	/// Storage: `DappStaking::Ledger` (r:1 w:1)
	/// Proof: `DappStaking::Ledger` (`max_values`: None, `max_size`: Some(310), added: 2785, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::EraRewards` (r:1 w:0)
	/// Proof: `DappStaking::EraRewards` (`max_values`: None, `max_size`: Some(789), added: 3264, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::PeriodEnd` (r:1 w:0)
	/// Proof: `DappStaking::PeriodEnd` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(67), added: 2542, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// The range of component `x` is `[1, 16]`.
	fn claim_staker_rewards_past_period(x: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `541`
		//  Estimated: `4764`
		// Minimum execution time: 48_239_000 picoseconds.
		Weight::from_parts(47_380_121, 4764)
			// Standard Error: 3_818
			.saturating_add(Weight::from_parts(1_938_013, 0).saturating_mul(x.into()))
			.saturating_add(T::DbWeight::get().reads(5_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: `DappStaking::Ledger` (r:1 w:1)
	/// Proof: `DappStaking::Ledger` (`max_values`: None, `max_size`: Some(310), added: 2785, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::EraRewards` (r:1 w:0)
	/// Proof: `DappStaking::EraRewards` (`max_values`: None, `max_size`: Some(789), added: 3264, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(67), added: 2542, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// The range of component `x` is `[1, 16]`.
	fn claim_staker_rewards_ongoing_period(x: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `519`
		//  Estimated: `4764`
		// Minimum execution time: 45_792_000 picoseconds.
		Weight::from_parts(45_193_051, 4764)
			// Standard Error: 5_334
			.saturating_add(Weight::from_parts(1_945_694, 0).saturating_mul(x.into()))
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: `DappStaking::StakerInfo` (r:1 w:1)
	/// Proof: `DappStaking::StakerInfo` (`max_values`: None, `max_size`: Some(178), added: 2653, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::PeriodEnd` (r:1 w:0)
	/// Proof: `DappStaking::PeriodEnd` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::Ledger` (r:1 w:1)
	/// Proof: `DappStaking::Ledger` (`max_values`: None, `max_size`: Some(310), added: 2785, mode: `MaxEncodedLen`)
	fn claim_bonus_reward() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `275`
		//  Estimated: `3775`
		// Minimum execution time: 35_426_000 picoseconds.
		Weight::from_parts(35_776_000, 3775)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: `DappStaking::IntegratedDApps` (r:1 w:0)
	/// Proof: `DappStaking::IntegratedDApps` (`max_values`: Some(65535), `max_size`: Some(116), added: 2096, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::DAppTiers` (r:1 w:1)
	/// Proof: `DappStaking::DAppTiers` (`max_values`: None, `max_size`: Some(1648), added: 4123, mode: `MaxEncodedLen`)
	fn claim_dapp_reward() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2672`
		//  Estimated: `5113`
		// Minimum execution time: 48_587_000 picoseconds.
		Weight::from_parts(49_645_000, 5113)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: `DappStaking::IntegratedDApps` (r:1 w:0)
	/// Proof: `DappStaking::IntegratedDApps` (`max_values`: Some(65535), `max_size`: Some(116), added: 2096, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::StakerInfo` (r:1 w:1)
	/// Proof: `DappStaking::StakerInfo` (`max_values`: None, `max_size`: Some(178), added: 2653, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::Ledger` (r:1 w:1)
	/// Proof: `DappStaking::Ledger` (`max_values`: None, `max_size`: Some(310), added: 2785, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::CurrentEraInfo` (r:1 w:1)
	/// Proof: `DappStaking::CurrentEraInfo` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(67), added: 2542, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	fn unstake_from_unregistered() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `322`
		//  Estimated: `4764`
		// Minimum execution time: 35_966_000 picoseconds.
		Weight::from_parts(36_190_000, 4764)
			.saturating_add(T::DbWeight::get().reads(6_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
	/// Storage: `DappStaking::StakerInfo` (r:17 w:16)
	/// Proof: `DappStaking::StakerInfo` (`max_values`: None, `max_size`: Some(178), added: 2653, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::Ledger` (r:1 w:1)
	/// Proof: `DappStaking::Ledger` (`max_values`: None, `max_size`: Some(310), added: 2785, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(67), added: 2542, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// The range of component `x` is `[1, 16]`.
	fn cleanup_expired_entries(x: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `256 + x * (73 ±0)`
		//  Estimated: `4764 + x * (2653 ±0)`
		// Minimum execution time: 35_797_000 picoseconds.
		Weight::from_parts(32_229_929, 4764)
			// Standard Error: 6_544
			.saturating_add(Weight::from_parts(4_898_852, 0).saturating_mul(x.into()))
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(x.into())))
			.saturating_add(T::DbWeight::get().writes(2_u64))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(x.into())))
			.saturating_add(Weight::from_parts(0, 2653).saturating_mul(x.into()))
	}
	/// Storage: `DappStaking::Safeguard` (r:1 w:0)
	/// Proof: `DappStaking::Safeguard` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn force() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `1486`
		// Minimum execution time: 8_530_000 picoseconds.
		Weight::from_parts(8_785_000, 1486)
			.saturating_add(T::DbWeight::get().reads(1_u64))
	}
	/// Storage: `DappStaking::CurrentEraInfo` (r:1 w:1)
	/// Proof: `DappStaking::CurrentEraInfo` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::EraRewards` (r:1 w:1)
	/// Proof: `DappStaking::EraRewards` (`max_values`: None, `max_size`: Some(789), added: 3264, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::StaticTierParams` (r:1 w:0)
	/// Proof: `DappStaking::StaticTierParams` (`max_values`: Some(1), `max_size`: Some(167), added: 662, mode: `MaxEncodedLen`)
	/// Storage: `PriceAggregator::ValuesCircularBuffer` (r:1 w:0)
	/// Proof: `PriceAggregator::ValuesCircularBuffer` (`max_values`: Some(1), `max_size`: Some(117), added: 612, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::TierConfig` (r:1 w:1)
	/// Proof: `DappStaking::TierConfig` (`max_values`: Some(1), `max_size`: Some(161), added: 656, mode: `MaxEncodedLen`)
	fn on_initialize_voting_to_build_and_earn() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `334`
		//  Estimated: `4254`
		// Minimum execution time: 24_436_000 picoseconds.
		Weight::from_parts(25_190_000, 4254)
			.saturating_add(T::DbWeight::get().reads(5_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: `DappStaking::CurrentEraInfo` (r:1 w:1)
	/// Proof: `DappStaking::CurrentEraInfo` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::PeriodEnd` (r:1 w:2)
	/// Proof: `DappStaking::PeriodEnd` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::HistoryCleanupMarker` (r:1 w:1)
	/// Proof: `DappStaking::HistoryCleanupMarker` (`max_values`: Some(1), `max_size`: Some(12), added: 507, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::EraRewards` (r:1 w:1)
	/// Proof: `DappStaking::EraRewards` (`max_values`: None, `max_size`: Some(789), added: 3264, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::StaticTierParams` (r:1 w:0)
	/// Proof: `DappStaking::StaticTierParams` (`max_values`: Some(1), `max_size`: Some(167), added: 662, mode: `MaxEncodedLen`)
	/// Storage: `PriceAggregator::ValuesCircularBuffer` (r:1 w:0)
	/// Proof: `PriceAggregator::ValuesCircularBuffer` (`max_values`: Some(1), `max_size`: Some(117), added: 612, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::TierConfig` (r:1 w:1)
	/// Proof: `DappStaking::TierConfig` (`max_values`: Some(1), `max_size`: Some(161), added: 656, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::DAppTiers` (r:0 w:1)
	/// Proof: `DappStaking::DAppTiers` (`max_values`: None, `max_size`: Some(1648), added: 4123, mode: `MaxEncodedLen`)
	fn on_initialize_build_and_earn_to_voting() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `843`
		//  Estimated: `4254`
		// Minimum execution time: 39_979_000 picoseconds.
		Weight::from_parts(41_027_000, 4254)
			.saturating_add(T::DbWeight::get().reads(7_u64))
			.saturating_add(T::DbWeight::get().writes(7_u64))
	}
	/// Storage: `DappStaking::CurrentEraInfo` (r:1 w:1)
	/// Proof: `DappStaking::CurrentEraInfo` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::EraRewards` (r:1 w:1)
	/// Proof: `DappStaking::EraRewards` (`max_values`: None, `max_size`: Some(789), added: 3264, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::StaticTierParams` (r:1 w:0)
	/// Proof: `DappStaking::StaticTierParams` (`max_values`: Some(1), `max_size`: Some(167), added: 662, mode: `MaxEncodedLen`)
	/// Storage: `PriceAggregator::ValuesCircularBuffer` (r:1 w:0)
	/// Proof: `PriceAggregator::ValuesCircularBuffer` (`max_values`: Some(1), `max_size`: Some(117), added: 612, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::TierConfig` (r:1 w:1)
	/// Proof: `DappStaking::TierConfig` (`max_values`: Some(1), `max_size`: Some(161), added: 656, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::DAppTiers` (r:0 w:1)
	/// Proof: `DappStaking::DAppTiers` (`max_values`: None, `max_size`: Some(1648), added: 4123, mode: `MaxEncodedLen`)
	fn on_initialize_build_and_earn_to_build_and_earn() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `386`
		//  Estimated: `4254`
		// Minimum execution time: 27_746_000 picoseconds.
		Weight::from_parts(27_938_000, 4254)
			.saturating_add(T::DbWeight::get().reads(5_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
	/// Storage: `DappStaking::ContractStake` (r:101 w:0)
	/// Proof: `DappStaking::ContractStake` (`max_values`: Some(65535), `max_size`: Some(91), added: 2071, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::TierConfig` (r:1 w:0)
	/// Proof: `DappStaking::TierConfig` (`max_values`: Some(1), `max_size`: Some(161), added: 656, mode: `MaxEncodedLen`)
	/// The range of component `x` is `[0, 100]`.
	fn dapp_tier_assignment(x: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `152 + x * (32 ±0)`
		//  Estimated: `3061 + x * (2071 ±0)`
		// Minimum execution time: 6_531_000 picoseconds.
		Weight::from_parts(10_637_970, 3061)
			// Standard Error: 3_112
			.saturating_add(Weight::from_parts(2_395_087, 0).saturating_mul(x.into()))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(x.into())))
			.saturating_add(Weight::from_parts(0, 2071).saturating_mul(x.into()))
	}
	/// Storage: `DappStaking::HistoryCleanupMarker` (r:1 w:1)
	/// Proof: `DappStaking::HistoryCleanupMarker` (`max_values`: Some(1), `max_size`: Some(12), added: 507, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::EraRewards` (r:1 w:1)
	/// Proof: `DappStaking::EraRewards` (`max_values`: None, `max_size`: Some(789), added: 3264, mode: `MaxEncodedLen`)
	/// Storage: `DappStaking::DAppTiers` (r:0 w:1)
	/// Proof: `DappStaking::DAppTiers` (`max_values`: None, `max_size`: Some(1648), added: 4123, mode: `MaxEncodedLen`)
	fn on_idle_cleanup() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `293`
		//  Estimated: `4254`
		// Minimum execution time: 8_145_000 picoseconds.
		Weight::from_parts(8_379_000, 4254)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: `DappStaking::Ledger` (r:2 w:1)
	/// Proof: `DappStaking::Ledger` (`max_values`: None, `max_size`: Some(310), added: 2785, mode: `MaxEncodedLen`)
	fn step() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `76`
		//  Estimated: `6560`
		// Minimum execution time: 10_060_000 picoseconds.
		Weight::from_parts(10_314_000, 6560)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn move_stake() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `76`
		//  Estimated: `6560`
		// Minimum execution time: 10_060_000 picoseconds.
		Weight::from_parts(10_314_000, 6560)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
}
