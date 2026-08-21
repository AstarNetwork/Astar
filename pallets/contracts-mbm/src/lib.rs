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

//! # Contracts MBM
//!
//! Multi block migrations decommissioning `pallet-contracts` (ink!/Wasm smart contracts) from the
//! Astar, Shiden & Shibuya runtimes. Staged over two runtime upgrades:
//!
//! 1. [`ReleaseContractsDeposits`] - settles every balance the pallet still holds. Must complete
//!    before the pallet index is dropped, otherwise those holds become undecodable and the funds
//!    are frozen for good.
//! 2. *(next runtime)* purging the contract child tries and the pallet prefix.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarks;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod release;
pub use release::ReleaseContractsDeposits;

pub mod weights;
pub use weights::WeightInfo;

pub use pallet::*;

pub(crate) const LOG_TARGET: &str = "mbm::contracts";

#[frame_support::pallet]
pub mod pallet {
    use frame_support::traits::Get;

    /// Carries no storage and no calls. It exists only so that the migrations in this crate can
    /// be benchmarked through the standard `benchmark pallet` tooling, and is therefore added to
    /// the runtimes under `runtime-benchmarks` only.
    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_balances::Config {
        /// Hold reason the `release_deposit` benchmark places on its sample account.
        type BenchmarkHoldReason: Get<<Self as pallet_balances::Config>::RuntimeHoldReason>;
    }
}
