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

//! TODO: Rustdoc!!!

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{traits::Get, weights::Weight};
use sp_core::U256;
use sp_runtime::{traits::UniqueSaturatedInto, FixedPointNumber, FixedU128, Perquintill};

pub use self::pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Overarching event type
        type RuntimeEvent: From<Event> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Default base fee per gas value. Used in genesis if no other value specified explicitly.
        type DefaultBaseFeePerGas: Get<U256>;
        /// Minimum value 'base fee per gas' can be adjusted to. This is a defensive measure to prevent the fee from being too low.
        type MinBaseFeePerGas: Get<U256>;
        /// Maximum value 'base fee per gas' can be adjusted to. This is a defensive measure to prevent the fee from being too high.
        type MaxBaseFeePerGas: Get<U256>;
        /// Getter for the fee adjustment factor used in 'base fee per gas' formula. This is expected to change in-between the blocks (doesn't have to though).
        type AdjustmentFactor: Get<FixedU128>;
        /// The so-called `weight_factor` in the 'base fee per gas' formula.
        type WeightFactor: Get<u128>;
        /// Ratio limit on how much the 'base fee per gas' can change in-between two blocks.
        /// It's expressed as percentage, and used to calculate the delta between the old and new value.
        /// E.g. if the current 'base fee per gas' is 100, and the limit is 10%, then the new base fee per gas can be between 90 and 110.
        type StepLimitRatio: Get<Perquintill>;
    }

    #[pallet::type_value]
    pub fn DefaultBaseFeePerGas<T: Config>() -> U256 {
        T::DefaultBaseFeePerGas::get()
    }

    #[pallet::storage]
    pub type BaseFeePerGas<T> = StorageValue<_, U256, ValueQuery, DefaultBaseFeePerGas<T>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event {
        /// New `base fee per gas` value has been force-set.
        NewBaseFeePerGas { fee: U256 },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Specified value is outside of the allowed range.
        ValueOutOfBounds,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_: T::BlockNumber) -> Weight {
            // TODO: benchmark this!
            let db_weight = <T as frame_system::Config>::DbWeight::get();
            db_weight.reads_writes(2, 1)
        }

        // TODO: it's super important to do double-check possible loss of precision here.
        // Do some tests, compare to benchmark values.
        fn on_finalize(_n: <T as frame_system::Config>::BlockNumber) {
            BaseFeePerGas::<T>::mutate(|base_fee_per_gas| {
                let old_bfpg = *base_fee_per_gas;

                // Maximum step we're allowed to move the base fee per gas by.
                let max_step = {
                    let old_bfpg_u128: u128 = old_bfpg.unique_saturated_into();
                    let step = T::StepLimitRatio::get() * old_bfpg_u128;
                    U256::from(step)
                };

                // TODO: maybe add a DB entry to check until when should we apply max step adjustment?
                // Once 'equilibrium' is reached, it's safe to just follow the formula without limit updates.
                // Or we could abuse the sudo for this.

                // Lower & upper limit between which the new base fee per gas should be clamped.
                let lower_limit = T::MinBaseFeePerGas::get().max(old_bfpg.saturating_sub(max_step));
                let upper_limit = T::MaxBaseFeePerGas::get().min(old_bfpg.saturating_add(max_step));

                // Calculate ideal new 'base_fee_per_gas' according to the formula
                let ideal_new_bfpg = T::AdjustmentFactor::get()
                    // Weight factor should be multiplied first since it's a larger number, to avoid precision loss.
                    .saturating_mul_int(T::WeightFactor::get())
                    .saturating_mul(25)
                    .saturating_div(98974);

                // Clamp the ideal value in between the allowed limits
                *base_fee_per_gas = U256::from(ideal_new_bfpg).clamp(lower_limit, upper_limit);
            })
        }

        fn integrity_test() {
            assert!(T::MinBaseFeePerGas::get() <= T::MaxBaseFeePerGas::get(),
                "Minimum base fee per gas has to be equal or lower than maximum allowed base fee per gas.");

            assert!(T::DefaultBaseFeePerGas::get() >= T::MinBaseFeePerGas::get(),
                "Default base fee per gas has to be equal or higher than minimum allowed base fee per gas.");
            assert!(T::DefaultBaseFeePerGas::get() <= T::MaxBaseFeePerGas::get(),
                "Default base fee per gas has to be equal or lower than maximum allowed base fee per gas.");

            assert!(T::MaxBaseFeePerGas::get() <= U256::from(u128::MAX),
                "Maximum base fee per gas has to be equal or lower than u128::MAX, otherwise precision loss will occur.");
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())] // TODO: weight!
        pub fn set_base_fee_per_gas(origin: OriginFor<T>, fee: U256) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(
                fee >= T::MinBaseFeePerGas::get() && fee <= T::MaxBaseFeePerGas::get(),
                Error::<T>::ValueOutOfBounds
            );

            BaseFeePerGas::<T>::put(fee);
            Self::deposit_event(Event::NewBaseFeePerGas { fee });
            Ok(())
        }
    }
}

impl<T: Config> fp_evm::FeeCalculator for Pallet<T> {
    fn min_gas_price() -> (U256, Weight) {
        (BaseFeePerGas::<T>::get(), T::DbWeight::get().reads(1))
    }
}
