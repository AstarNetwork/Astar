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

use super::*;
use frame_benchmarking::v2::*;
use frame_support::{
    assert_ok,
    traits::{Get, OnFinalize},
    BoundedVec,
};
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn feed_values(x: Linear<1, { <T as orml_oracle::Config>::MaxFeedValues::get() }>) {
        // Prepare account and add it as member
        let account: T::AccountId = whitelisted_caller();
        T::AddMember::add_member(account.clone());

        // Get base feed value
        let currency_id_price_pair = T::BenchmarkCurrencyIdValuePair::get();

        // Prepare feed values vector
        let mut key_value_pairs =
            BoundedVec::<_, <T as orml_oracle::Config>::MaxFeedValues>::default();
        for _ in 0..x {
            key_value_pairs
                .try_push(currency_id_price_pair.clone())
                .unwrap();
        }

        #[block]
        {
            assert_ok!(orml_oracle::Pallet::<T>::feed_values(
                RawOrigin::Signed(account.clone()).into(),
                key_value_pairs
            ));
        }
    }

    #[benchmark]
    fn on_finalize() {
        #[block]
        {
            orml_oracle::Pallet::<T>::on_finalize(1_u32.into());
        }
    }
}
