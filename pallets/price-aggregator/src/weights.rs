
// This file is part of Astar.

// Copyright (C) 2019-2024 Stake Technologies Pte.Ltd.
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

// This is just a dummy file.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

pub trait WeightInfo {
	fn process_block_aggregated_values() -> Weight;
	fn process_intermediate_aggregated_values() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn process_block_aggregated_values() -> Weight {
        Weight::from_parts(1_000_000_000, 1024 * 32)
	}
	fn process_intermediate_aggregated_values() -> Weight {
		Weight::from_parts(1_000_000_000, 1024 * 32)
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {

	fn process_block_aggregated_values() -> Weight {
        Weight::from_parts(1_000_000_000, 1024 * 32)
	}
	fn process_intermediate_aggregated_values() -> Weight {
        Weight::from_parts(1_000_000_000, 1024 * 32)
	}
}
