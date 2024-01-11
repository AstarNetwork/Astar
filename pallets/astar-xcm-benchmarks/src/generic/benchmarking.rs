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

use super::*;
use frame_benchmarking::v2::*;
// use frame_benchmarking::{benchmarks, BenchmarkError, BenchmarkResult};
use frame_support::dispatch::Weight;
use pallet_xcm_benchmarks::{new_executor, XcmCallOf};
use sp_std::vec;
use sp_std::vec::Vec;
use xcm::latest::prelude::*;

#[benchmarks]
mod benchmarks {
    use super::*;

    /// We need re-write buy_execution benchmark becuase our runtime
    /// needs 1 additional DB read (XcAssetConfig) for fetching unit per sec
    /// for a fungible asset. The upstream benchmark use native assets thus
    /// won't accout for it.
    #[benchmark]
    fn buy_execution() -> Result<(), BenchmarkError> {
        let holding = T::worst_case_holding(0).into();

        let mut executor = new_executor::<T>(Default::default());
        executor.set_holding(holding);

        // A fungible asset
        let fee_asset = Concrete(MultiLocation::parent());

        let instruction = Instruction::<XcmCallOf<T>>::BuyExecution {
            fees: (fee_asset, 100_000_000u128).into(), // should be something inside of holding
            weight_limit: WeightLimit::Unlimited,
        };

        let xcm = Xcm(vec![instruction]);

        #[block]
        {
            executor.bench_process(xcm)?;
        }
        // The completion of execution above is enough to validate this is completed.
        Ok(())
    }

    /// Re-write as upstream one has hardcoded system pallet index as 1 whereas our runtimes
    /// uses index 10.
    #[benchmark]
    fn expect_pallet() -> Result<(), BenchmarkError> {
        let mut executor = new_executor::<T>(Default::default());

        let instruction = Instruction::ExpectPallet {
            // used index 10 for our runtimes
            index: 10,
            name: b"System".to_vec(),
            module_name: b"frame_system".to_vec(),
            crate_major: 4,
            min_crate_minor: 0,
        };
        let xcm = Xcm(vec![instruction]);

        #[block]
        {
            executor.bench_process(xcm)?;
        }
        Ok(())
    }

    #[benchmark]
    fn exchange_asset() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    #[benchmark]
    fn export_message() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    #[benchmark]
    fn lock_asset() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    #[benchmark]
    fn unlock_asset() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    #[benchmark]
    fn note_unlockable() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    #[benchmark]
    fn request_unlock() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    #[benchmark]
    fn universal_origin() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}

pub struct XcmGenericBenchmarks<T>(sp_std::marker::PhantomData<T>);
// Benchmarks wrapper
impl<T: Config> frame_benchmarking::Benchmarking for XcmGenericBenchmarks<T> {
    fn benchmarks(extra: bool) -> Vec<frame_benchmarking::BenchmarkMetadata> {
        // all the generic xcm benchmarks
        use pallet_xcm_benchmarks::generic::Pallet as PalletXcmGenericBench;
        PalletXcmGenericBench::<T>::benchmarks(extra)
    }
    fn run_benchmark(
        extrinsic: &[u8],
        c: &[(frame_benchmarking::BenchmarkParameter, u32)],
        whitelist: &[frame_benchmarking::TrackedStorageKey],
        verify: bool,
        internal_repeats: u32,
    ) -> Result<Vec<frame_benchmarking::BenchmarkResult>, frame_benchmarking::BenchmarkError> {
        use pallet_xcm_benchmarks::generic::Pallet as PalletXcmGenericBench;

        use crate::generic::Pallet as AstarXcmGenericBench;
        if AstarXcmGenericBench::<T>::benchmarks(true)
            .iter()
            .any(|x| x.name == extrinsic)
        {
            AstarXcmGenericBench::<T>::run_benchmark(
                extrinsic,
                c,
                whitelist,
                verify,
                internal_repeats,
            )
        } else {
            PalletXcmGenericBench::<T>::run_benchmark(
                extrinsic,
                c,
                whitelist,
                verify,
                internal_repeats,
            )
        }
    }
}
