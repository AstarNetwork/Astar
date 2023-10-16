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

use fp_evm::FeeCalculator;
use frame_benchmarking::v2::*;
use frame_support::traits::Hooks;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn base_fee_per_gas_adjustment() {
        let (first_block, second_block) = (T::BlockNumber::from(1u32), T::BlockNumber::from(2u32));

        // Setup actions, should ensure some value is written to storage.
        Pallet::<T>::on_initialize(first_block);
        Pallet::<T>::on_finalize(first_block);
        assert!(
            BaseFeePerGas::<T>::exists(),
            "Value should exist in storage after first on_finalize call"
        );

        Pallet::<T>::on_initialize(second_block);
        let init_bfpg = BaseFeePerGas::<T>::get();

        #[block]
        {
            Pallet::<T>::on_finalize(second_block);
        }

        // Ensure that the value has changed.
        assert!(BaseFeePerGas::<T>::get() != init_bfpg);
    }

    #[benchmark]
    fn set_base_fee_per_gas() {
        let old_bfpg = BaseFeePerGas::<T>::get();
        let new_bfpg = old_bfpg + 1;

        #[extrinsic_call]
        _(RawOrigin::Root, new_bfpg);

        // Ensure that the value has changed.
        assert_eq!(BaseFeePerGas::<T>::get(), new_bfpg);
    }

    #[benchmark]
    fn min_gas_price() {
        let first_block = T::BlockNumber::from(1u32);

        // Setup actions, should ensure some value is written to storage.
        Pallet::<T>::on_initialize(first_block);
        Pallet::<T>::on_finalize(first_block);
        assert!(
            BaseFeePerGas::<T>::exists(),
            "Value should exist in storage after first on_finalize call"
        );

        #[block]
        {
            let _ = Pallet::<T>::min_gas_price();
        }
    }

    impl_benchmark_test_suite!(
        Pallet,
        crate::benchmarking::tests::new_test_ext(),
        crate::mock::TestRuntime,
    );
}

#[cfg(test)]
mod tests {
    use crate::mock;
    use frame_support::sp_io::TestExternalities;

    pub fn new_test_ext() -> TestExternalities {
        mock::ExtBuilder::build()
    }
}
