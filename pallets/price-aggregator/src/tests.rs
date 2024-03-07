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
use crate::{pallet::Config, CircularBuffer, CurrentBlockValues, ValueAggregator};

use astar_primitives::oracle::{CurrencyAmount, CurrencyId};

use orml_traits::OnNewData;

use frame_support::{assert_storage_noop, traits::Get, BoundedVec};
use sp_runtime::traits::Zero;

pub use num_traits::Bounded;

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

    // 2. Repeat the check but on the other edge
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
