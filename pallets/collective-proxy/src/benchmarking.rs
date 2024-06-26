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

use super::{Pallet as CollectiveProxy, *};

use frame_benchmarking::v2::*;
use sp_std::prelude::*;

/// Assert that the last event equals the provided one.
pub(super) fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
    frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

#[benchmarks()]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn execute_call() {
        let origin = T::CollectiveProxy::try_successful_origin()
            .expect("Must succeed in order to run benchmarks.");

        // A bit dirty, but runtime should ensure to allow the `remark` call.
        let call: <T as Config>::RuntimeCall =
            frame_system::Call::<T>::remark { remark: vec![] }.into();

        #[extrinsic_call]
        _(origin as T::RuntimeOrigin, Box::new(call));

        assert_last_event::<T>(Event::<T>::CollectiveProxyExecuted { result: Ok(()) }.into());
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
