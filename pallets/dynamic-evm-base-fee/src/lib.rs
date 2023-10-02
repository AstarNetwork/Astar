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

//! Dynamic Evm Base Fee Pallet
//!
//! ## Overview
//!
//! The pallet is responsible for calculating `Base Fee Per Gas` value, according to the current system parameters.
//! This is not like `EIP-1559`, instead it's intended for `Astar` and `Astar-like` networks, which allow both
//! **Substrate native transactions** (which in `Astar` case reuse Polkadot transaction fee approach)
//! and **EVM transactions** (which use `Base Fee Per Gas`).
//!
//! For a more detailed description, reader is advised to refer to Astar Network forum post about [Tokenomics 2.0](https://forum.astar.network/t/astar-tokenomics-2-0-a-dynamically-adjusted-inflation/4924).
//!
//! ## Approach
//!
//! The core formula this pallet tries to satisfy is:
//!
//! base_fee_per_gas = adjustment_factor * weight_factor * 25 / 98974
//!
//! Where:
//! * **adjustment_factor** - is a value that changes in-between the blocks, related to the block fill ratio.
//! * **weight_factor** - fixed constant, used to convert consumed _weight_ to _fee_.
//!
//! The implementation doesn't make any hard requirements on these values, and only requires that a type implementing `Get<_>` provides them.
//!
//! ## Implementation
//!
//! The core logic is implemented in `on_finalize` hook, which is called at the end of each block.
//! This pallet's hook should be called AFTER whichever pallet's hook is responsible for updating **adjustment factor**.
//!
//! The hook will calculate the ideal new `base_fee_per_gas` value, and then clamp it in between the allowed limits.
//!
//! ## Interface
//!
//! Pallet provides an implementation of `FeeCalculator` trait. This makes it usable directly in `pallet-evm`.
//!
//! A _root-only_ extrinsic is provided to allow setting the `base_fee_per_gas` value manually.
//!
//! ## Practical Remarks
//!
//! According to the proposed **Tokenomics 2.0**, max amount that adjustment factor will be able to change on live networks in-between blocks is:
//!
//! adjustment_new = adjustment_old * (1 + adj + adj^2/2)
//!
//! adj = v * (s - s*)
//! --> recommended _v_ value: 0.000_015
//! --> largest 's' delta: (1 - 0.25) = **0.75**
//!
//! (for variable explanation please check the linked forum post above)
//! (in short: `v` - variability factor, `s` - current block fill ratio, `s*` - ideal block fill ratio)
//!
//! adj = 0.000015 * (1 - 0.25) = **0.000_011_25**
//! (1 + 0.000_011_25 + 0.000_011_25^2/2) = (1 + 0.000_011_25 + 0.000_000_000_063_281) = **1,000_011_250_063_281**
//!
//! Discarding the **1**, and only considering the decimals, this can be expressed as ratio:
//! Expressed as ratio: 11_250_063_281 / 1_000_000_000_000_000.
//! This is a much smaller change compared to the max step limit ratio we'll use to limit bfpg alignment.
//! This means that once equilibrium is reached (fees are aligned), the `StepLimitRatio` will be larger than the max possible adjustment, essentially eliminating its effect.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::weights::Weight;
use sp_core::U256;
use sp_runtime::{traits::UniqueSaturatedInto, FixedPointNumber, FixedU128, Perquintill};

pub use self::pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    use super::*;

    /// The current storage version.
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
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
        /// Weight information for extrinsics & functions of this pallet.
        type WeightInfo: WeightInfo;
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
            T::WeightInfo::base_fee_per_gas_adjustment()
        }

        fn on_finalize(_n: <T as frame_system::Config>::BlockNumber) {
            BaseFeePerGas::<T>::mutate(|base_fee_per_gas| {
                let old_bfpg = *base_fee_per_gas;

                // Maximum step we're allowed to move the base fee per gas by.
                let max_step = {
                    let old_bfpg_u128: u128 = old_bfpg.unique_saturated_into();
                    let step = T::StepLimitRatio::get() * old_bfpg_u128;
                    U256::from(step)
                };

                // It's possible current base fee per gas is outside of the allowed range.
                // This can & will happen when this solution is deployed on live networks.
                //
                // In such scenario, we will discard the lower & upper bounds configured in the runtime.
                // Once these bounds are reached ONCE, the runtime logic will prevent them from going out of bounds again.
                let apply_configured_bounds = old_bfpg >= T::MinBaseFeePerGas::get()
                    && old_bfpg <= T::MaxBaseFeePerGas::get();
                let (lower_limit, upper_limit) = if apply_configured_bounds {
                    (
                        T::MinBaseFeePerGas::get().max(old_bfpg.saturating_sub(max_step)),
                        T::MaxBaseFeePerGas::get().min(old_bfpg.saturating_add(max_step)),
                    )
                } else {
                    (
                        old_bfpg.saturating_sub(max_step),
                        old_bfpg.saturating_add(max_step),
                    )
                };

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
        /// `root-only` extrinsic to set the `base_fee_per_gas` value manually.
        /// The specified value has to respect min & max limits configured in the runtime.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::set_base_fee_per_gas())]
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
        (BaseFeePerGas::<T>::get(), T::WeightInfo::min_gas_price())
    }
}
