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
use astar_primitives::Balance;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn dapp_tier_assignment() {
        let era = 10;
        let period = 1;
        let reward_pool = Balance::from(1e30 as u128);

        TierConfig:<T>::put()
        // TODO: TierConfig setting
        // TODO: dApp registration
        // TODO: ContractStake filling


        #[block]
        {
            let _ = Pallet::<T>::get_dapp_tier_assignment(era, period, reward_pool);
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
    use sp_io::TestExternalities;

    pub fn new_test_ext() -> TestExternalities {
        mock::ExtBuilder::default().build()
    }
}
