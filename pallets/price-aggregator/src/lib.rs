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
//! ## TODO

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade, DefaultNoBound};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use sp_arithmetic::{
    fixed_point::FixedU128,
    traits::{CheckedAdd, SaturatedConversion, Saturating, Zero},
};
use sp_std::marker::PhantomData;

use orml_traits::OnNewData;

use astar_primitives::{
    oracle::{CurrencyAmount, CurrencyId, PriceProvider},
    BlockNumber,
};

// TODO: perhaps make CurrencyAmount and CurrencyId generic?

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Trait for processing accumulated currency values within a single block.
///
/// This can be anything from median, average, or more complex calculation.
pub trait ProcessBlockValues {
    /// Process the accumulated values and return the result.
    ///
    /// In case of an error, return an error message.
    fn process(values: &[CurrencyAmount]) -> Result<CurrencyAmount, &'static str>;
}

/// Used to calculate the simple average of the accumulated values.
pub struct AverageBlockValue;
impl ProcessBlockValues for AverageBlockValue {
    fn process(values: &[CurrencyAmount]) -> Result<CurrencyAmount, &'static str> {
        if values.is_empty() {
            return Err("No values exist for the current block.");
        }

        let sum = values.iter().fold(CurrencyAmount::zero(), |acc, &value| {
            acc.saturating_add(value)
        });

        Ok(sum.saturating_mul(FixedU128::from_rational(1, values.len() as u128)))
    }
}

const LOG_TARGET: &str = "price-aggregator";

/// Used to aggregate the accumulated values over some time period.
///
/// To avoid having a large memory footprint, values are summed up into a single accumulator.
/// Number of summed up values is tracked in a separate field.
#[derive(Encode, Decode, MaxEncodedLen, Default, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct ValueAggregator {
    /// Total accumulated value amount.
    #[codec(compact)]
    pub(crate) total: CurrencyAmount,
    /// Number of values accumulated.
    #[codec(compact)]
    pub(crate) count: u32,
    /// Block number at which aggregation should reset.
    #[codec(compact)]
    pub(crate) limit_block: BlockNumber,
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
        if self.count.is_zero() {
            CurrencyAmount::zero()
        } else {
            self.total
                .saturating_mul(FixedU128::from_rational(1, self.count.into()))
        }
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
    /// Currency values store.
    pub(crate) buffer: BoundedVec<CurrencyAmount, L>,
    /// Next index to write to.
    #[codec(compact)]
    pub(crate) head: u32,
}

impl<L: Get<u32>> CircularBuffer<L> {
    /// Adds a new value to the circular buffer, possibly overriding the oldest value if capacity is filled.
    pub fn add(&mut self, value: CurrencyAmount) {
        // This can never happen, parameters must ensure that.
        // But we still check it and log an error if it does.
        if self.head >= L::get() || self.head as usize > self.buffer.len() {
            log::error!(
                target: LOG_TARGET,
                "Failed to push value to the circular buffer due to invalid next index. \
                Next index: {:?}, Buffer length: {:?}, Buffer capacity: {:?}",
                self.head,
                self.buffer.len(),
                L::get()
            );
            return;
        }

        if self.buffer.len() > self.head as usize {
            // Vec has been filled out, so we need to override the 'head' value
            self.buffer[self.head as usize] = value;
        } else {
            // Vec is not full yet, so we can just push the value
            let _ignorable = self.buffer.try_push(value);
        }
        self.head = self.head.saturating_add(1) % L::get();
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
        /// New average native currency value has been calculated and pushed into the moving average buffer.
        AverageAggregatedValue(CurrencyAmount),
    }

    /// Storage for the accumulated native currency price in the current block.
    #[pallet::storage]
    #[pallet::whitelist_storage]
    pub type CurrentBlockValues<T: Config> =
        StorageValue<_, BoundedVec<CurrencyAmount, T::MaxValuesPerBlock>, ValueQuery>;

    /// Used to store the aggregated processed block values during some time period.
    #[pallet::storage]
    #[pallet::whitelist_storage]
    pub type IntermediateValueAggregator<T: Config> = StorageValue<_, ValueAggregator, ValueQuery>;

    /// Used to store aggregated intermediate values for some time period.
    #[pallet::storage]
    pub type ValuesCircularBuffer<T: Config> =
        StorageValue<_, CircularBuffer<T::CircularBufferLength>, ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
            // Need to account for the reads and writes of:
            // - CurrentBlockValues
            // - IntermediateValueAggregator
            T::DbWeight::get().reads_writes(2, 2)

            // TODO + the weight of the actual processing time, needs to be benchmarked
        }

        fn on_finalize(now: BlockNumberFor<T>) {
            // 1. Process the accumulated native currency values in the current block.
            Self::process_block_aggregated_values();

            // 2. Check if we need to push the average aggregated value to the storage.
            if IntermediateValueAggregator::<T>::get().limit_block >= now.saturated_into() {
                Self::process_intermediate_aggregated_values(now);
            }
        }

        // TODO: integration tests!
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
            Self::deposit_event(Event::AverageAggregatedValue(average_value));
        }
    }

    // Make this pallet an 'observer' ('listener') of the new oracle data feed.
    impl<T: Config> OnNewData<T::AccountId, CurrencyId, CurrencyAmount> for Pallet<T> {
        fn on_new_data(who: &T::AccountId, key: &CurrencyId, value: &CurrencyAmount) {
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

    // Make this pallet a `price provider` for the native currency.
    //
    // For this particular implementation, a simple moving average is used to calculate the average price.
    impl<T: Config> PriceProvider for Pallet<T> {
        fn average_price() -> FixedU128 {
            ValuesCircularBuffer::<T>::get().average()
        }
    }
}

/// Used to update static price due to storage schema change.
pub struct PriceAggregatorInitializer<T, P>(PhantomData<(T, P)>);
impl<T: Config, P: Get<FixedU128>> OnRuntimeUpgrade for PriceAggregatorInitializer<T, P> {
    fn on_runtime_upgrade() -> Weight {
        if Pallet::<T>::on_chain_storage_version() > 0 {
            return Weight::zero();
        }

        // 1. Prepare price aggregator storage.
        let now = frame_system::Pallet::<T>::block_number();
        let limit_block = now.saturating_add(T::AggregationDuration::get().saturated_into());
        IntermediateValueAggregator::<T>::put(ValueAggregator::new(limit_block.saturated_into()));

        // 2. Put the initial value into the circular buffer so it's not empty.
        use sp_arithmetic::FixedPointNumber;
        let init_price = P::get().max(FixedU128::from_rational(1, FixedU128::DIV.into()));
        log::info!(
            "Pushing initial price value into moving average buffer: {}",
            init_price
        );
        ValuesCircularBuffer::<T>::mutate(|buffer| buffer.add(init_price));

        // 3. Set the initial storage version.
        STORAGE_VERSION.put::<Pallet<T>>();

        // Reading block number is 'free' in the terms of weight.
        T::DbWeight::get().writes(3)
    }
}
