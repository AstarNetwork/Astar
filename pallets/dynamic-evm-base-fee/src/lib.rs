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

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{traits::Get, weights::Weight};
use sp_core::U256;
use sp_runtime::{traits::Convert, traits::UniqueSaturatedInto};

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type DefaultBaseFeePerGas: Get<U256>;
        type MinBaseFeePerGas: Get<U256>;
        type MaxBaseFeePerGas: Get<U256>;
        type AdjustmentLogic: Convert<u128, u128>;
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub base_fee_per_gas: U256,
        _marker: PhantomData<T>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                base_fee_per_gas: T::DefaultBaseFeePerGas::get(),
                _marker: PhantomData,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            BaseFeePerGas::<T>::put(self.base_fee_per_gas);
        }
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
        NewBaseFeePerGas { fee: U256 },
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_: T::BlockNumber) -> Weight {
            // TODO: benchmark this!
            let db_weight = <T as frame_system::Config>::DbWeight::get();
            db_weight.reads_writes(2, 1)
        }

        fn on_finalize(_n: <T as frame_system::Config>::BlockNumber) {
            BaseFeePerGas::<T>::mutate(|base_fee_per_gas| {
                let new_base_fee_per_gas =
                    T::AdjustmentLogic::convert(base_fee_per_gas.clone().unique_saturated_into());

                *base_fee_per_gas = U256::from(new_base_fee_per_gas)
                    .clamp(T::MinBaseFeePerGas::get(), T::MaxBaseFeePerGas::get());
            })
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
        pub fn set_base_fee_per_gas(origin: OriginFor<T>, fee: U256) -> DispatchResult {
            ensure_root(origin)?;
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
