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

use astar_primitives::ethereum_checked::MAX_ETHEREUM_TX_INPUT_SIZE;
use frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn transact_without_apply() {
        let origin = T::XcmTransactOrigin::try_successful_origin().unwrap();
        let target =
            H160::from_slice(&hex::decode("dfb975d018f03994a3b943808e3aa0964bd78463").unwrap());
        // Calling `store(3)`
        let input = BoundedVec::<u8, ConstU32<MAX_ETHEREUM_TX_INPUT_SIZE>>::try_from(
            hex::decode("6057361d0000000000000000000000000000000000000000000000000000000000000003")
                .unwrap(),
        )
        .unwrap();
        let checked_tx = CheckedEthereumTx {
            gas_limit: U256::from(1_000_000),
            target,
            value: U256::zero(),
            input,
            maybe_access_list: None,
        };

        #[block]
        {
            Pallet::<T>::transact_without_apply(origin, checked_tx).unwrap();
        }

        assert_eq!(Nonce::<T>::get(), U256::one())
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
    use sp_io::TestExternalities;

    pub fn new_test_ext() -> TestExternalities {
        mock::ExtBuilder::default().build()
    }
}
