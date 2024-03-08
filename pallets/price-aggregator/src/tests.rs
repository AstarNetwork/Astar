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

use crate::mock::*;
use crate::{
    pallet::Config, AverageBlockValue, CircularBuffer, CurrentBlockValues, Event,
    IntermediateValueAggregator, MedianBlockValue, ProcessBlockValues, ValueAggregator,
    ValuesCircularBuffer,
};

use astar_primitives::oracle::{CurrencyAmount, CurrencyId};

use orml_traits::OnNewData;

use frame_support::{
    assert_storage_noop,
    traits::{Get, Hooks},
    BoundedVec,
};
use sp_runtime::{traits::Zero, Saturating};

pub use num_traits::Bounded;

#[test]
fn average_block_value_works() {
    // 0. Empty vec check
    let empty_vec: Vec<CurrencyAmount> = vec![];
    assert!(AverageBlockValue::process(&empty_vec).is_err());

    // 1. Single value check
    let single_value_vec = vec![CurrencyAmount::from_rational(15, 10)];
    assert_eq!(
        AverageBlockValue::process(&single_value_vec),
        Ok(single_value_vec[0])
    );

    // 2. Multiple values check
    let multiple_values_vec = vec![
        CurrencyAmount::from_rational(5, 10),
        CurrencyAmount::from_rational(15, 10),
    ];
    assert_eq!(
        AverageBlockValue::process(&multiple_values_vec),
        Ok(CurrencyAmount::from_rational(10, 10))
    );
}

#[test]
fn median_block_value_works() {
    // 0. Empty vec check
    let empty_vec: Vec<CurrencyAmount> = vec![];
    assert!(MedianBlockValue::process(&empty_vec).is_err());

    // 1. Single value check
    let single_value_vec = vec![CurrencyAmount::from_rational(7, 10)];
    assert_eq!(
        MedianBlockValue::process(&single_value_vec),
        Ok(single_value_vec[0])
    );

    // 2. Odd number values check
    let odd_values_vec = vec![
        CurrencyAmount::from_rational(3, 10),
        CurrencyAmount::from_rational(7, 10),
        CurrencyAmount::from_rational(9, 10),
    ];
    assert_eq!(
        MedianBlockValue::process(&odd_values_vec),
        Ok(CurrencyAmount::from_rational(7, 10))
    );

    // 3.1. Even number values check
    let even_values_vec_1 = vec![
        CurrencyAmount::from_rational(4, 10),
        CurrencyAmount::from_rational(6, 10),
    ];
    assert_eq!(
        MedianBlockValue::process(&even_values_vec_1),
        Ok(CurrencyAmount::from_rational(5, 10))
    );

    // 3.1. Even number values check
    let even_values_vec_2 = vec![
        CurrencyAmount::from_rational(1, 10),
        CurrencyAmount::from_rational(4, 10),
        CurrencyAmount::from_rational(6, 10),
        CurrencyAmount::from_rational(23, 10),
    ];
    assert_eq!(
        MedianBlockValue::process(&even_values_vec_2),
        Ok(CurrencyAmount::from_rational(5, 10))
    );
}

#[test]
fn value_aggregator_basic_checks() {
    let limit_block = 10;
    let value_aggregator = ValueAggregator::new(limit_block);

    // 0. Sanity checks
    assert!(value_aggregator.total.is_zero());
    assert!(value_aggregator.count.is_zero());
    assert_eq!(value_aggregator.limit_block, limit_block);
    assert!(value_aggregator.average().is_zero());

    // 1. Add a value, verify state is as expected
    let amount_1 = CurrencyAmount::from_rational(15, 10);
    let result = value_aggregator.clone().try_add(amount_1);
    assert_eq!(
        result,
        Ok(ValueAggregator {
            total: amount_1,
            count: 1,
            limit_block,
        })
    );
    assert_eq!(result.unwrap().average(), amount_1);

    // 2. Add another value, verify state is as expected
    let value_aggregator = result.unwrap();
    let amount_2 = CurrencyAmount::from_rational(5, 10);
    let result = value_aggregator.clone().try_add(amount_2);
    assert_eq!(
        result,
        Ok(ValueAggregator {
            total: amount_1 + amount_2,
            count: 2,
            limit_block,
        })
    );
    assert_eq!(
        result.unwrap().average(),
        CurrencyAmount::from_rational(10, 10)
    );
}

#[test]
fn value_aggregator_overflow_checks() {
    // 1. Currency overflow check
    let max_currency_aggregator = ValueAggregator {
        total: CurrencyAmount::max_value(),
        count: 10,
        limit_block: 10,
    };

    let amount = CurrencyAmount::from_rational(1, 10);
    let result = max_currency_aggregator.clone().try_add(amount);
    assert!(result.is_err());

    // 2. Counter overflow check
    let max_count_aggregator = ValueAggregator {
        total: CurrencyAmount::zero(),
        count: u32::MAX,
        limit_block: 10,
    };
    let result = max_count_aggregator.clone().try_add(amount);
    assert!(result.is_err());
}

#[test]
fn circular_buffer_basic_checks() {
    // 0. Buffer size prep
    const BUFFER_SIZE: u32 = 16;
    struct BufferSize;
    impl Get<u32> for BufferSize {
        fn get() -> u32 {
            BUFFER_SIZE
        }
    }

    // 1. Sanity checks
    let mut circular_buffer = CircularBuffer::<BufferSize>::default();
    assert!(circular_buffer.buffer.is_empty());
    assert!(circular_buffer.head.is_zero());

    // 2. Add a value, verify state is as expected
    let amount_1 = CurrencyAmount::from_rational(19, 10);
    let mut expected_buffer = vec![amount_1];
    circular_buffer.add(amount_1);
    assert_eq!(circular_buffer.buffer.clone().into_inner(), expected_buffer);
    assert_eq!(circular_buffer.head, 1);
    assert_eq!(circular_buffer.average(), amount_1);

    // 3. Add another value, verify state is as expected
    let amount_2 = CurrencyAmount::from_rational(7, 10);
    circular_buffer.add(amount_2);
    expected_buffer.push(amount_2);
    assert_eq!(circular_buffer.buffer.clone().into_inner(), expected_buffer);
    assert_eq!(circular_buffer.head, 2);
    assert_eq!(
        circular_buffer.average(),
        CurrencyAmount::from_rational(13, 10)
    );

    // 4. Fill up the buffer, verify state is as expected
    let amount_3 = CurrencyAmount::from_rational(27, 10);
    for _ in 2..BUFFER_SIZE {
        circular_buffer.add(amount_3);
        expected_buffer.push(amount_3);
    }
    assert_eq!(circular_buffer.buffer.clone().into_inner(), expected_buffer);
    assert!(circular_buffer.head.is_zero());

    // 5. Add another value, verify 0-th element is replaced
    let amount_4 = CurrencyAmount::from_rational(9, 10);
    circular_buffer.add(amount_4);
    expected_buffer[0] = amount_4;
    assert_eq!(circular_buffer.buffer.clone().into_inner(), expected_buffer);
    assert_eq!(circular_buffer.head, 1);

    // 6. Repeat the cycle few more times, expect it works as expected
    for x in 0..BUFFER_SIZE * 5 {
        // Store head for the next check
        let init_head = circular_buffer.head;

        // Generate a new amount
        let amount = amount_3 * CurrencyAmount::from_rational(x as u128 + 1, 1);

        assert!(circular_buffer.buffer[init_head as usize] != amount);
        circular_buffer.add(amount);
        assert_eq!(circular_buffer.buffer[init_head as usize], amount);
        assert_eq!(circular_buffer.head, (init_head + 1) % BUFFER_SIZE);
    }
}

#[test]
fn circular_buffer_inconsistency_safeguard_checks() {
    // 0. Buffer size prep
    const BUFFER_SIZE: u32 = 4;
    struct BufferSize;
    impl Get<u32> for BufferSize {
        fn get() -> u32 {
            BUFFER_SIZE
        }
    }

    // 1. Check that if head is ahead of length, operation does nothing
    let amount = CurrencyAmount::from_rational(15, 100);
    let mut inconsistent_buffer = CircularBuffer::<BufferSize> {
        buffer: Default::default(),
        head: 1,
    };

    inconsistent_buffer.add(amount);
    assert!(inconsistent_buffer.buffer.is_empty());
    assert_eq!(inconsistent_buffer.head, 1);

    // 2. Check that when head equals length, operation does nothing
    let init_buffer = BoundedVec::try_from(vec![amount, amount, amount, amount])
        .expect("Must work since size matches the bound.");
    let mut inconsistent_buffer = CircularBuffer::<BufferSize> {
        buffer: init_buffer.clone(),
        head: BUFFER_SIZE,
    };

    inconsistent_buffer.add(amount);
    assert_eq!(inconsistent_buffer.buffer, init_buffer);
    assert_eq!(inconsistent_buffer.head, BUFFER_SIZE);
}

#[test]
fn on_new_data_works_as_expected() {
    ExtBuilder::build().execute_with(|| {
        // 0. Initial sanity check
        assert!(
            CurrentBlockValues::<Test>::get().is_empty(),
            "Init state must be empty."
        );

        // 1. Inform pallet of a new piece of data, verify state is as expected
        let dummy_account_1 = 123;
        let native_currency_id = <Test as Config>::NativeCurrencyId::get();
        let amount_1 = CurrencyAmount::from_rational(15, 10);
        PriceAggregator::on_new_data(&dummy_account_1, &native_currency_id, &amount_1);
        assert_eq!(
            CurrentBlockValues::<Test>::get().into_inner(),
            vec![amount_1],
        );

        // 2. Try to add non-native currency, verify no state change
        let non_native_currency_id = CurrencyId::SDN;
        assert!(
            non_native_currency_id != native_currency_id,
            "Sanity check."
        );

        let non_native_amount = CurrencyAmount::from_rational(7, 10);
        assert_storage_noop!(PriceAggregator::on_new_data(
            &dummy_account_1,
            &non_native_currency_id,
            &non_native_amount
        ));

        // 3. Add additional amount, verify state is as expected
        let amount_2 = CurrencyAmount::from_rational(3, 10);
        PriceAggregator::on_new_data(&dummy_account_1, &native_currency_id, &amount_2);
        assert_eq!(
            CurrentBlockValues::<Test>::get().into_inner(),
            vec![amount_1, amount_2],
        );

        // 4. Fill up storage to the limit, verify state is as expected
        let limit = <Test as Config>::MaxValuesPerBlock::get();
        let mut result = vec![amount_1, amount_2];
        let amount_3 = CurrencyAmount::from_rational(19, 10);

        for _ in 2..limit {
            PriceAggregator::on_new_data(&dummy_account_1, &native_currency_id, &amount_3);
            result.push(amount_3);
        }

        assert_eq!(result.len(), limit as usize, "Sanity check.");
        assert_eq!(CurrentBlockValues::<Test>::get().into_inner(), result);

        // 5. Try to add one more value, overflowing the buffer, verify no state change
        assert_storage_noop!(PriceAggregator::on_new_data(
            &dummy_account_1,
            &native_currency_id,
            &amount_3
        ));
    });
}

#[test]
fn on_finalize_updates_aggregated_data() {
    ExtBuilder::build().execute_with(|| {
        // 1. Store some data into the current block values buffer
        let dummy_account_1 = 123;
        let native_currency_id = <Test as Config>::NativeCurrencyId::get();
        let amount_1 = CurrencyAmount::from_rational(13, 10);
        let amount_2 = CurrencyAmount::from_rational(17, 10);
        PriceAggregator::on_new_data(&dummy_account_1, &native_currency_id, &amount_1);
        PriceAggregator::on_new_data(&dummy_account_1, &native_currency_id, &amount_2);

        // 2. Finalize the block, verify state is as expected.
        let block_number_1 = System::block_number();
        PriceAggregator::on_finalize(block_number_1);

        assert!(
            CurrentBlockValues::<Test>::get().is_empty(),
            "Buffer must be empty after the finalization."
        );
        let intermediate_value_aggregator = IntermediateValueAggregator::<Test>::get();
        assert_eq!(intermediate_value_aggregator.count, 1);

        let average_amount_1 = CurrencyAmount::from_rational(15, 10);
        assert_eq!(intermediate_value_aggregator.total, average_amount_1);

        // 3. Move to the next block, but for this one no new data is added
        let intermediate_value_snapshot = IntermediateValueAggregator::<Test>::get();

        let block_number_2 = block_number_1 + 1;
        System::set_block_number(block_number_2);
        PriceAggregator::on_initialize(block_number_2);

        // No new data is added, everything must still work without breaking
        PriceAggregator::on_finalize(block_number_2);
        assert_eq!(
            IntermediateValueAggregator::<Test>::get(),
            intermediate_value_snapshot,
            "No new data was added, so the state must remain the same."
        );

        // 4. Add new data, verify state is updated as expected, i.e. nothing was broken by the previous step
        let block_number_3 = block_number_2 + 1;
        System::set_block_number(block_number_3);
        PriceAggregator::on_initialize(block_number_3);

        let amount_3 = CurrencyAmount::from_rational(19, 10);
        PriceAggregator::on_new_data(&dummy_account_1, &native_currency_id, &amount_3);
        PriceAggregator::on_new_data(&dummy_account_1, &native_currency_id, &amount_3);
        PriceAggregator::on_finalize(block_number_3);

        let intermediate_value_aggregator = IntermediateValueAggregator::<Test>::get();
        assert_eq!(
            intermediate_value_aggregator.count, 2,
            "Count must be 2 since we added only 2 new values."
        );
        assert_eq!(
            intermediate_value_aggregator.total,
            average_amount_1 + amount_3,
            "New entry must have been added, increasing the total."
        );
    })
}

#[test]
fn on_finalize_updates_circular_buffer() {
    ExtBuilder::build().execute_with(|| {
        let dummy_account = 456;
        let native_currency_id = <Test as Config>::NativeCurrencyId::get();

        // 1. Advance just until limit block is reached, checking appropriate storage items along the way
        let mut total = CurrencyAmount::zero();
        let current_block = System::block_number();
        let limit_block = IntermediateValueAggregator::<Test>::get().limit_block;

        for block in current_block..limit_block {
            // Add new data
            let amount = CurrencyAmount::from_rational(block.into(), 10);
            PriceAggregator::on_new_data(&dummy_account, &native_currency_id, &amount);
            total.saturating_accrue(amount);

            // Finalize the block
            PriceAggregator::on_finalize(block);
            assert_eq!(
                IntermediateValueAggregator::<Test>::get().total,
                total,
                "Check total is updated as expected."
            );
            assert!(
                ValuesCircularBuffer::<Test>::get().buffer.is_empty(),
                "Circular buffer is expected to remain empty until limit block is reached."
            );

            let new_block = block + 1;
            System::set_block_number(new_block);
            PriceAggregator::on_initialize(new_block);
        }

        // 2. Move over to the next block, expect circular buffer to be updated since limit block will be reached.
        let current_block = System::block_number();
        assert_eq!(current_block, limit_block, "Sanity check.");

        // Don't add any new data, just finalize the block. This is neat since we already know the exact 'total' amount
        // but also get to test that circular buffer update doesn't break due to missing value.
        PriceAggregator::on_finalize(current_block);

        // Check that value aggregator is reset & new block limit is correct
        let reset_intermediate_aggregator = IntermediateValueAggregator::<Test>::get();
        assert_eq!(reset_intermediate_aggregator.total, CurrencyAmount::zero());
        assert_eq!(reset_intermediate_aggregator.count, 0);
        assert_eq!(
            reset_intermediate_aggregator.limit_block,
            current_block + <Test as Config>::AggregationDuration::get()
        );

        // Check that circular buffer was updated as expected
        let circular_buffer = ValuesCircularBuffer::<Test>::get();
        let expected_average = total * CurrencyAmount::from_rational(1, limit_block as u128 - 1);
        assert_eq!(
            circular_buffer.buffer.clone().into_inner(),
            vec![expected_average]
        );
        assert_eq!(circular_buffer.head, 1);

        // Verify deposited event
        System::assert_last_event(RuntimeEvent::PriceAggregator(
            Event::AverageAggregatedValue {
                value: expected_average,
            },
        ));
    })
}

#[test]
fn circular_buffer_really_is_circular() {
    ExtBuilder::build().execute_with(|| {
        // 0. Init data
        let aggregation_duration = <Test as Config>::AggregationDuration::get();
        let circular_buffer_length: u32 = <Test as Config>::CircularBufferLength::get();

        fn advance_to_block(block: u32) {
            let dummy_account = 456;
            let native_currency_id = <Test as Config>::NativeCurrencyId::get();
            let init_block = System::block_number();

            for block in init_block..block {
                // Submit some amount to prevent error spam
                let amount = CurrencyAmount::from_rational(block as u128, 10);
                PriceAggregator::on_new_data(&dummy_account, &native_currency_id, &amount);

                PriceAggregator::on_finalize(block);

                let new_block = block + 1;
                System::set_block_number(new_block);
                PriceAggregator::on_initialize(new_block);
            }
        }

        // 1. Fill up the circular buffer
        for x in 0..circular_buffer_length {
            // Advance until circular buffer is updated
            let intermediate_aggregator = IntermediateValueAggregator::<Test>::get();
            advance_to_block(intermediate_aggregator.limit_block + 1);

            // Check that circular buffer is updated as expected
            let circular_buffer = ValuesCircularBuffer::<Test>::get();
            assert_eq!(circular_buffer.buffer.len(), x as usize + 1);
            assert_eq!(circular_buffer.head, (x + 1) % circular_buffer_length);

            // Check that intermediate aggregator is reset & limit block is updated
            let reset_intermediate_aggregator = IntermediateValueAggregator::<Test>::get();
            assert_eq!(reset_intermediate_aggregator.total, CurrencyAmount::zero());
            assert_eq!(reset_intermediate_aggregator.count, 0);
            assert_eq!(
                reset_intermediate_aggregator.limit_block,
                intermediate_aggregator.limit_block + aggregation_duration
            );
        }

        // 2. Continue adding the data, verify circular buffer is updated as expected
        for x in 0..circular_buffer_length * 3 {
            // Advance until circular buffer is updated
            let intermediate_aggregator = IntermediateValueAggregator::<Test>::get();
            advance_to_block(intermediate_aggregator.limit_block + 1);

            // Check that circular buffer is updated as expected
            let circular_buffer = ValuesCircularBuffer::<Test>::get();
            assert_eq!(
                circular_buffer.buffer.len(),
                circular_buffer_length as usize
            );
            assert_eq!(circular_buffer.head, (x + 1) % circular_buffer_length);
        }
    })
}
