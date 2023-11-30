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
use frame_support::weights::Weight;
use parity_scale_codec::Encode;
use sp_core::H160;
use sp_runtime::MultiAddress;

use astar_primitives::Balance;

#[benchmarks(
    where <T as pallet_contracts::Config>::Currency: Currency<T::AccountId, Balance = Balance>,
)]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn evm_call_overheads() {
        let context = Context {
            source_vm_id: VmId::Wasm,
            weight_limit: Weight::from_parts(1_000_000, 1_000_000),
        };
        let vm_id = VmId::Evm;
        let source = whitelisted_caller();
        let target = H160::repeat_byte(1).encode();
        let input = vec![1, 2, 3];
        let value = 1_000_000u128;

        #[block]
        {
            Pallet::<T>::call_without_execution(context, vm_id, source, target, input, value, None)
                .unwrap();
        }
    }

    #[benchmark]
    fn wasm_call_overheads() {
        let context = Context {
            source_vm_id: VmId::Evm,
            weight_limit: Weight::from_parts(1_000_000, 1_000_000),
        };
        let vm_id = VmId::Wasm;
        let source = whitelisted_caller();
        let target = MultiAddress::<T::AccountId, ()>::Id(whitelisted_caller()).encode();
        let input = vec![1, 2, 3];
        let value = 1_000_000u128;

        #[block]
        {
            Pallet::<T>::call_without_execution(context, vm_id, source, target, input, value, None)
                .unwrap();
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
