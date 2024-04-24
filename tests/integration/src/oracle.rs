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

use crate::setup::*;

use astar_primitives::oracle::{CurrencyAmount, PriceProvider};
use pallet_price_aggregator::{IntermediateValueAggregator, ValueAggregator};

#[test]
fn price_submission_works() {
    new_test_ext().execute_with(|| {
        let native_currency_id =
            <Runtime as pallet_price_aggregator::Config>::NativeCurrencyId::get();
        assert_eq!(PriceAggregator::average_price(), INIT_PRICE, "Sanity check");

        // 0. Need to set limit block to something sensible, otherwise we'll waste time on many redundant iterations
        let limit_block = 10;
        IntermediateValueAggregator::<Runtime>::put(ValueAggregator::new(limit_block));

        // 1. Submit a price for a valid asset - the native currency
        let price_1 = CurrencyAmount::from_rational(15, 100);
        assert_ok!(Oracle::feed_values(
            RuntimeOrigin::signed(ALICE.clone()),
            vec![(native_currency_id, price_1)].try_into().unwrap()
        ));

        let price_2 = CurrencyAmount::from_rational(17, 100);
        assert_ok!(Oracle::feed_values(
            RuntimeOrigin::signed(BOB.clone()),
            vec![(native_currency_id, price_2)].try_into().unwrap()
        ));

        // 2. Advance a block, and check price aggregator intermediate state is as expected
        // (perhaps a bit detailed, but still good to check whether it's integrated)
        run_for_blocks(1);
        let expected_average = (price_1 + price_2) * CurrencyAmount::from_rational(1, 2);
        assert_eq!(
            IntermediateValueAggregator::<Runtime>::get().average(),
            expected_average
        );

        // 3. Keep advancing blocks, adding new values only each other block, and verify the average is as expected at the end
        for i in System::block_number() + 1..limit_block {
            if i % 2 == 0 {
                let step = CurrencyAmount::from_rational(i as u128 % 5, 100);

                assert_ok!(Oracle::feed_values(
                    RuntimeOrigin::signed(ALICE.clone()),
                    vec![(native_currency_id, price_1 + step)]
                        .try_into()
                        .unwrap()
                ));
                assert_ok!(Oracle::feed_values(
                    RuntimeOrigin::signed(BOB.clone()),
                    vec![(native_currency_id, price_2 - step)]
                        .try_into()
                        .unwrap()
                ));
            }
            run_for_blocks(1);
        }

        // 4. Execute limit block and verify state is updated as expected
        run_for_blocks(2); // Need to run on_finalize of the limit block
        let expected_moving_average =
            (expected_average + INIT_PRICE) * CurrencyAmount::from_rational(1, 2);
        assert_eq!(PriceAggregator::average_price(), expected_moving_average);

        // 5. Run until next limit block without any transactions, don't expect any changes
        let limit_block = limit_block * 2;
        IntermediateValueAggregator::<Runtime>::put(ValueAggregator::new(limit_block));

        run_to_block(limit_block + 1);
        assert_eq!(PriceAggregator::average_price(), expected_moving_average);
    })
}
