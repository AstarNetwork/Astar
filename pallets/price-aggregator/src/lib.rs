// This file is part of Astar.

// Copyright (C) 2019-2024 Stake Technologies Pte.Ltd.
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

//! # Price Aggregator Pallet
//!
//! ## Overview
//!
//! ##

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade, DefaultNoBound};
use frame_system::{ensure_root, pallet_prelude::*};
pub use pallet::*;
use sp_arithmetic::{
    fixed_point::FixedU128,
    traits::{CheckedAdd, SaturatedConversion, Saturating, Zero},
};
use sp_std::marker::PhantomData;

pub use orml_traits::OnNewData;

use astar_primitives::{oracle::PriceProvider, BlockNumber};

// TODO: move to primitives
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum CurrencyId {
    ASTR,
}
pub type CurrencyAmount = FixedU128;

/// Trait for processing accumulated currency values within a single block.
///
/// This can be anything from median, average, or more complex calculation.
pub trait ProcessBlockValues {
    /// Process the accumulated values and return the result.
    ///
    /// In case of an error, return an error message.
    fn process(values: &[CurrencyAmount]) -> Result<CurrencyAmount, &'static str>;
}

const LOG_TARGET: &str = "price-aggregator";

/// Used to aggregate the accumulated values over some time period.
///
/// To avoid having a large memory footprint, values are summed up into a single accumulator.
/// Number of summed up values is tracked in a separate field.
#[derive(Encode, Decode, MaxEncodedLen, Default, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct ValueAggregator {
    /// Total accumulated value amount.
    total: CurrencyAmount,
    /// Number of values accumulated.
    count: u32,
    /// Block number at which aggregation should reset.
    limit_block: BlockNumber,
}

impl ValueAggregator {
    /// New value aggregator, with the given block number as the new limit.
    pub fn new(limit_block: BlockNumber) -> Self {
        Self {
            limit_block,
            ..Default::default()
        }
    }

    /// Attempts to add a value to the aggregator, consuming `self` in the process.
    ///
    /// Returns an error if the addition would cause an overflow in the accumulator or the counter.
    /// Otherwise returns the updated aggregator.
    pub fn try_add(mut self, value: CurrencyAmount) -> Result<Self, &'static str> {
        self.total = self
            .total
            .checked_add(&value)
            .ok_or("Failed to add value to the aggregator due to overflow.")?;

        self.count = self
            .count
            .checked_add(1)
            .ok_or("Failed to increment count in the aggregator due to overflow.")?;

        Ok(self)
    }

    /// Returns the average of the accumulated values.
    pub fn average(&self) -> CurrencyAmount {
        if self.count == 0 {
            return CurrencyAmount::zero();
        }

        // TODO: maybe this can be written in a way that preserves more precision?
        self.total
            .saturating_mul(FixedU128::from_rational(1, self.count.into()))
    }
}

/// Used to store the aggregated intermediate values into a circular buffer.
///
/// Inserts values sequentially into the buffer, until the buffer has been filled out.
/// After that, the oldest value is always overwritten with the new value.
#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    RuntimeDebugNoBound,
    PartialEqNoBound,
    EqNoBound,
    CloneNoBound,
    TypeInfo,
    DefaultNoBound,
)]
#[scale_info(skip_type_params(L))]
pub struct CircularBuffer<L: Get<u32>> {
    /// Next index to write to.
    next_index: u32,
    /// Currency values store.
    buffer: BoundedVec<CurrencyAmount, L>,
}

impl<L: Get<u32>> CircularBuffer<L> {
    /// Adds a new value to the circular buffer, possibly overriding the oldest value if capacity is filled.
    pub fn add(&mut self, value: CurrencyAmount) {
        // This can never happen, parameters must ensure that.
        // But we still check it and log an error if it does.
        if self.next_index >= L::get() || self.next_index as usize > self.buffer.len() {
            log::error!(
                target: LOG_TARGET,
                "Failed to push value to the circular buffer due to invalid next index. \
                Next index: {:?}, Buffer length: {:?}, Buffer capacity: {:?}",
                self.next_index,
                self.buffer.len(),
                L::get()
            );
            return;
        }

        let _infallible = self.buffer.try_insert(self.next_index as usize, value);
        self.next_index = self.next_index.saturating_add(1) % L::get();
    }

    /// Returns the average of the accumulated values.
    pub fn average(&self) -> CurrencyAmount {
        if self.buffer.is_empty() {
            return CurrencyAmount::zero();
        }

        let sum = self
            .buffer
            .iter()
            .fold(CurrencyAmount::zero(), |acc, &value| {
                acc.saturating_add(value)
            });

        // At this point, length of the buffer is guaranteed to be greater than zero.
        sum.saturating_mul(FixedU128::from_rational(1, self.buffer.len() as u128))
    }
}

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

        /// Maximum number of distinct currency values we can store during a single block.
        #[pallet::constant]
        type MaxValuesPerBlock: Get<u32>;

        /// Used to process accumulated values in the current block.
        type ProcessBlockValues: ProcessBlockValues;

        /// Native currency ID that this pallet is supposed to track.
        type NativeCurrencyId: Get<CurrencyId>;

        /// Maximum length of the circular buffer used to calculate the moving average.
        #[pallet::constant]
        type CircularBufferLength: Get<u32>;

        /// Duration of aggregation period expressed in the number of blocks.
        /// During this time, currency values are aggregated, and are then used to calculate the average value.
        #[pallet::constant]
        type AggregationDuration: Get<BlockNumberFor<Self>>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        // TODO: do we want to emit some events?
        // Maybe when the aggregated average value is pushed to the buffer?
    }

    #[pallet::error]
    pub enum Error<T> {
        // TODO
    }

    /// Storage for the accumulated native currency price in the current block.
    #[pallet::storage]
    pub type CurrentBlockValues<T: Config> =
        StorageValue<_, BoundedVec<CurrencyAmount, T::MaxValuesPerBlock>, ValueQuery>;

    /// Used to store the aggregated processed block values during some time period.
    #[pallet::storage]
    pub type IntermediateValueAggregator<T: Config> = StorageValue<_, ValueAggregator, ValueQuery>;

    /// Used to store aggregated intermediate values for some time period.
    #[pallet::storage]
    pub type ValuesCircularBuffer<T: Config> =
        StorageValue<_, CircularBuffer<T::CircularBufferLength>, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // TODO
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
            // TODO: benchmarks & account for the possible changes in the on_finalize
            Weight::zero()
        }

        fn on_finalize(now: BlockNumberFor<T>) {
            // 1. Process the accumulated native currency values in the current block.
            Self::process_block_aggregated_values();

            // 2. Check if we need to push the average aggregated value to the storage.
            if IntermediateValueAggregator::<T>::get().limit_block >= now.saturated_into() {
                Self::process_intermediate_aggregated_values(now);
            }
        }
    }

    impl<T: Config> Pallet<T> {
        /// Used to process the native currency values accumulated in the current block.
        ///
        /// Guarantees that the accumulated values are cleared after processing.
        /// In case of an error during processing, intermediate aggregated value is not updated.
        fn process_block_aggregated_values() {
            // 1. Take the accumulated block values, clearing the existing storage.
            let accumulated_values = CurrentBlockValues::<T>::take();

            // 2. Attempt to process accumulated block values.
            let processed_value = match T::ProcessBlockValues::process(
                accumulated_values.as_slice(),
            ) {
                Ok(value) => value,
                Err(message) => {
                    log::error!(
                        target: LOG_TARGET,
                        "Failed to process the accumulated native currency values in the current block. \
                        Reason: {:?}",
                        message
                    );

                    // Nothing to do if we have no valid value to store.
                    return;
                }
            };

            // TODO: is it ok to ignore this? A bit confused what happens actually in the closure.
            // 3. Attempt to store the processed value.
            let _ignore = IntermediateValueAggregator::<T>::try_mutate(
                |aggregator| match aggregator.try_add(processed_value) {
                    Ok(new_aggregator) => Ok(new_aggregator),
                    Err(message) => {
                        log::error!(
                            target: LOG_TARGET,
                            "Failed to add the processed native currency value to the intermediate storage. \
                            Reason: {:?}",
                            message
                        );
                        Err(())
                    }
                },
            );
        }

        /// Used to process the intermediate aggregated values, and push them to the moving average storage.
        fn process_intermediate_aggregated_values(now: BlockNumberFor<T>) {
            // 1. Get the average value from the intermediate aggregator.
            let average_value = IntermediateValueAggregator::<T>::get().average();

            // 2. Reset the aggregator back to zero, and set the new limit block.
            IntermediateValueAggregator::<T>::put(ValueAggregator::new(
                now.saturating_add(T::AggregationDuration::get())
                    .saturated_into(),
            ));

            // 3. In case aggregated value equals 0, it means something has gone wrong since it's extremely unlikely
            // that price goes to absolute zero. The much more likely case is that there's a problem with the oracle data feed.
            if average_value.is_zero() {
                log::error!(
                    target: LOG_TARGET,
                    "The average aggregated price equals zero, which most likely means that oracle data feed is faulty. \
                    Not pushing the 'zero' value to the moving average storage."
                );
                return;
            }

            // 4. Push the 'valid' average aggregated value to the circular buffer.
            ValuesCircularBuffer::<T>::mutate(|buffer| buffer.add(average_value));
        }
    }

    // Make this pallet an 'observer' ('listener') of the new oracle data feed.
    impl<T: Config> OnNewData<T::AccountId, CurrencyId, CurrencyAmount> for Pallet<T> {
        fn on_new_data(who: &T::AccountId, key: &CurrencyId, value: &CurrencyAmount) {
            // TODO
            // Do we need to prevent same account posting multiple values in the same block? Or will the other pallet take care of that?

            // Ignore any currency that is not native currency.
            if T::NativeCurrencyId::get() != *key {
                return;
            }

            CurrentBlockValues::<T>::mutate(|v| match v.try_push(*value) {
                Ok(()) => {}
                Err(_) => {
                    log::error!(
                        target: LOG_TARGET,
                            "Failed to push native currency value into the ongoing block due to exceeded capacity. \
                            Value was submitted by: {:?}",
                            who
                        );
                }
            });
        }
    }

    impl<T: Config> PriceProvider for Pallet<T> {
        fn average_price() -> FixedU128 {
            ValuesCircularBuffer::<T>::get().average()
        }
    }
}
