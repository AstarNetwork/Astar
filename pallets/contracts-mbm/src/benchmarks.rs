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

use crate::{Config, Pallet, ReleaseContractsDeposits};
use frame_benchmarking::v2::*;
use frame_support::{
    migrations::SteppedMigration,
    traits::{
        fungible::{InspectHold, Mutate, MutateHold},
        Get,
    },
    weights::WeightMeter,
};
use sp_runtime::traits::Zero;
use sp_std::marker::PhantomData;

/// Supplies the benchmark's hold reason as the `Get` bound the migration expects.
pub struct BenchHoldReason<T>(PhantomData<T>);
impl<T: Config> Get<<T as pallet_balances::Config>::RuntimeHoldReason> for BenchHoldReason<T> {
    fn get() -> <T as pallet_balances::Config>::RuntimeHoldReason {
        T::BenchmarkHoldReason::get()
    }
}

/// Destination of the swept deposit.
pub struct BenchEscrow<T>(PhantomData<T>);
impl<T: Config> Get<T::AccountId> for BenchEscrow<T> {
    fn get() -> T::AccountId {
        account("escrow", 0, 0)
    }
}

#[benchmarks]
mod benches {
    use super::*;

    /// Cost of settling one account.
    ///
    /// Runs the real `step` against storage holding exactly one hold-bearing account, so the
    /// `Holds` seek, the decode and the `transfer_on_hold` are all measured. The terminal
    /// `next_key` probe that ends the loop is included too, which errs on the safe side.
    #[benchmark]
    fn release_deposit() {
        let contract: T::AccountId = account("contract", 0, 0);
        let reason = T::BenchmarkHoldReason::get();
        let amount: <T as pallet_balances::Config>::Balance = 1_000_000u32.into();

        pallet_balances::Pallet::<T>::set_balance(&contract, amount * 2u32.into());
        pallet_balances::Pallet::<T>::hold(&reason, &contract, amount)
            .expect("account was just funded");

        let mut meter = WeightMeter::new();

        #[block]
        {
            ReleaseContractsDeposits::<
                T,
                BenchHoldReason<T>,
                BenchHoldReason<T>,
                BenchEscrow<T>,
                crate::weights::SubstrateWeight<T>,
            >::step(None, &mut meter)
            .expect("migration step succeeds");
        }

        assert!(pallet_balances::Pallet::<T>::balance_on_hold(&reason, &contract).is_zero());
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Runtime);
}
