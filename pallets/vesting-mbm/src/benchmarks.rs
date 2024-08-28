// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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

use crate::{Config, Pallet};
use frame_benchmarking::v2::*;
use frame_support::{
    assert_ok, migrations::SteppedMigration, traits::Currency, weights::WeightMeter,
};
use pallet_vesting::VestingInfo;
use sp_runtime::traits::StaticLookup;
use sp_std::vec;

#[benchmarks]
mod benches {
    use super::*;

    /// Benchmark a single step of vesting migration.
    #[benchmark]
    fn step(x: Linear<1u32, { <T as pallet_vesting::Config>::MAX_VESTING_SCHEDULES }>) {
        let alice: T::AccountId = account("alice", 0, 1);
        let bob: T::AccountId = account("bob", 0, 2);

        for _ in 0..x {
            let _ = T::Currency::make_free_balance_be(&alice, 1_000_000u32.into());
            assert_ok!(pallet_vesting::Pallet::<T>::vested_transfer(
                frame_system::RawOrigin::Signed(alice.clone()).into(),
                T::Lookup::unlookup(bob.clone()),
                VestingInfo::new(1_000_000u32.into(), 10u32.into(), 0u32.into()),
            ));
        }

        let mut meter = WeightMeter::new();

        #[block]
        {
            crate::LazyMigration::<T, crate::weights::SubstrateWeight<T>>::step(None, &mut meter)
                .unwrap();
        }

        let mut expected = vec![];
        for _ in 0..x {
            expected.push(VestingInfo::new(
                999_990u32.into(),
                5u32.into(),
                1u32.into(),
            ));
        }

        assert_eq!(
            pallet_vesting::Vesting::<T>::get(&bob).unwrap().to_vec(),
            expected
        );
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Runtime);
}
