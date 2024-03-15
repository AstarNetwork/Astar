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

//! A temporary solution to benchmark the `orml-oracle` pallet.
//! Should be removed once `orml-oracle` pallet has its own benchmarking.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Get;
pub use pallet::*;
use sp_std::marker::PhantomData;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config:
        frame_system::Config + orml_oracle::Config + pallet_price_aggregator::Config
    {
        #[pallet::constant]
        type BenchmarkCurrencyIdValuePair: Get<(
            <Self as orml_oracle::Config>::OracleKey,
            <Self as orml_oracle::Config>::OracleValue,
        )>;
    }
}
