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

//! # Static Price Provider Pallet
//!
//! A simple pallet that provides a static price for the native currency.
//! This is a temporary solution before oracle is implemented & operational.
//!
//! ## Overview
//!
//! The Static Price Provider pallet provides functionality for setting the active native currency price via privileged call.
//! Only the root can set the price.
//!
//! Network maintainers must ensure to update the price at appropriate times so that inflation & dApp Staking rewards are calculated correctly.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade};
use frame_system::{ensure_root, pallet_prelude::*};
pub use pallet::*;
use sp_arithmetic::{fixed_point::FixedU64, traits::Zero, FixedPointNumber};
use sp_std::marker::PhantomData;

use astar_primitives::oracle::PriceProvider;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {

    use super::*;

    /// The current storage version.
    pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New static native currency price has been set.
        PriceSet { price: FixedU64 },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Zero is invalid value for the price (hopefully).
        ZeroPrice,
    }

    /// Default value handler for active price.
    /// This pallet is temporary and it's not worth bothering with genesis config.
    pub struct DefaultActivePrice;
    impl Get<FixedU64> for DefaultActivePrice {
        fn get() -> FixedU64 {
            FixedU64::from_rational(1, 10)
        }
    }

    /// Current active native currency price.
    #[pallet::storage]
    #[pallet::whitelist_storage]
    pub type ActivePrice<T: Config> = StorageValue<_, FixedU64, ValueQuery, DefaultActivePrice>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Privileged action used to set the active native currency price.
        ///
        /// This is a temporary solution before oracle is implemented & operational.
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().writes(1))]
        pub fn force_set_price(origin: OriginFor<T>, price: FixedU64) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(!price.is_zero(), Error::<T>::ZeroPrice);

            ActivePrice::<T>::put(price);

            Self::deposit_event(Event::<T>::PriceSet { price });

            Ok(().into())
        }
    }

    impl<T: Config> PriceProvider for Pallet<T> {
        fn average_price() -> FixedU64 {
            ActivePrice::<T>::get()
        }
    }
}

/// `OnRuntimeUpgrade` logic for integrating this pallet into the live network.
pub struct InitActivePrice<T, P>(PhantomData<(T, P)>);
impl<T: Config, P: Get<FixedU64>> OnRuntimeUpgrade for InitActivePrice<T, P> {
    fn on_runtime_upgrade() -> Weight {
        let init_price = P::get().max(FixedU64::from_rational(1, FixedU64::DIV.into()));

        log::info!("Setting initial active price to {}", init_price);
        ActivePrice::<T>::put(init_price);

        STORAGE_VERSION.put::<Pallet<T>>();

        T::DbWeight::get().writes(2)
    }
}
