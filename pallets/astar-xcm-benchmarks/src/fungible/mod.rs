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

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    #[pallet::config]
    pub trait Config<I: 'static = ()>:
        frame_system::Config + crate::Config + pallet_xcm_benchmarks::fungible::Config
    {
        /// A trusted location where reserve assets are stored, and the asset we allow to be
        /// reserves.
        type TrustedReserve: frame_support::traits::Get<
            Option<(xcm::latest::MultiLocation, xcm::latest::MultiAsset)>,
        >;
    }

    #[pallet::pallet]
    pub struct Pallet<T, I = ()>(_);
}
