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

use super::{Pallet as AstarBenchmarks, *};
use crate::WrappedBenchmark;
use frame_benchmarking::v2::*;
use frame_support::dispatch::Weight;
use pallet_xcm_benchmarks::{generic::Pallet as PalletXcmBenchmarks, new_executor, XcmCallOf};
use sp_std::vec;
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
            fees: (fee_asset, u128::MAX).into(), // should be something inside of holding
            // this should not be Unlimited, as xcm-executor will skip buying the
            // exceution altogether.
            weight_limit: WeightLimit::Limited(Weight::from_parts(1u64, 1024)),
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

pub type XcmGenericBenchmarks<T> = WrappedBenchmark<AstarBenchmarks<T>, PalletXcmBenchmarks<T>>;
