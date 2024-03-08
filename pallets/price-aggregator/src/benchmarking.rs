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

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn process_block_aggregated_values() {
        // Fill up the current block buffer with some values
        let size_limit = <T as Config>::MaxValuesPerBlock::get();
        let mut result = BoundedVec::<CurrencyAmount, <T as Config>::MaxValuesPerBlock>::default();
        for x in 1..=size_limit {
            let value = CurrencyAmount::from_rational(x as u128 + 3, 10);
            result
                .try_push(value)
                .expect("Must succeed since we are iterating to the limit");
        }
        CurrentBlockValues::<T>::put(result);

        #[block]
        {
            Pallet::<T>::process_block_aggregated_values();
        }

        assert!(
            CurrentBlockValues::<T>::get().is_empty(),
            "Should have been cleaned up."
        );
    }

    #[benchmark]
    fn process_intermediate_aggregated_values() {
        // 1. Fill up the current aggregator and make it trigger on the current block end
        IntermediateValueAggregator::<T>::mutate(|a| {
            a.limit_block = frame_system::Pallet::<T>::block_number().saturated_into();

            a.total = CurrencyAmount::from_rational(1234, 10);
            a.count = 19;
        });

        // 2. Fill up the circular buffer with some values
        let buffer_length = <T as Config>::CircularBufferLength::get();
        ValuesCircularBuffer::<T>::mutate(|b| {
            for x in 1..=buffer_length {
                b.add(CurrencyAmount::from_rational(x as u128 + 3, 10));
            }
        });
        assert_eq!(
            ValuesCircularBuffer::<T>::get().buffer.len(),
            buffer_length as usize,
            "Sanity check."
        );

        // 3. Prepare local variables
        let buffer_snapshot = ValuesCircularBuffer::<T>::get();
        let current_block = frame_system::Pallet::<T>::block_number();

        #[block]
        {
            Pallet::<T>::process_intermediate_aggregated_values(current_block);
        }

        assert!(ValuesCircularBuffer::<T>::get() != buffer_snapshot);
    }

    impl_benchmark_test_suite!(
        Pallet,
        crate::benchmarking::tests::new_test_ext(),
        crate::mock::Test,
    );
}

#[cfg(test)]
mod tests {
    use crate::mock;
    use sp_io::TestExternalities;

    pub fn new_test_ext() -> TestExternalities {
        mock::ExtBuilder::build()
    }
}
