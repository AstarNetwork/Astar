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

pub mod fungible;
pub mod generic;

#[cfg(test)]
mod mock;

use sp_std::vec::Vec;

/// A base trait for all individual pallets
pub trait Config: frame_system::Config + pallet_xcm_benchmarks::Config {}

/// This is a wrapper benchmark implementation over `Inner` by `Outer` by merging
/// the benches from `Inner` if they don't exist in `Outer`.
pub struct WrappedBenchmark<Outer, Inner>(core::marker::PhantomData<(Outer, Inner)>);
impl<Outer, Inner> frame_benchmarking::Benchmarking for WrappedBenchmark<Outer, Inner>
where
    Outer: frame_benchmarking::Benchmarking,
    Inner: frame_benchmarking::Benchmarking,
{
    fn benchmarks(extra: bool) -> Vec<frame_benchmarking::BenchmarkMetadata> {
        let mut outer = Outer::benchmarks(extra);
        let inner = Inner::benchmarks(extra);

        for meta in inner {
            if !outer.iter().any(|m| m.name == meta.name) {
                outer.push(meta)
            }
        }
        outer
    }

    fn run_benchmark(
        name: &[u8],
        c: &[(frame_benchmarking::BenchmarkParameter, u32)],
        whitelist: &[frame_benchmarking::TrackedStorageKey],
        verify: bool,
        internal_repeats: u32,
    ) -> Result<Vec<frame_benchmarking::BenchmarkResult>, frame_benchmarking::BenchmarkError> {
        if Outer::benchmarks(true).iter().any(|x| x.name == name) {
            Outer::run_benchmark(name, c, whitelist, verify, internal_repeats)
        } else {
            Inner::run_benchmark(name, c, whitelist, verify, internal_repeats)
        }
    }
}
