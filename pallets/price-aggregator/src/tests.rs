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
use crate::{pallet::Config, CurrentBlockValues};

use astar_primitives::oracle::CurrencyId;

use orml_traits::OnNewData;

use frame_support::{assert_storage_noop, traits::Get};
use sp_arithmetic::FixedU128;

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
        let amount_1 = FixedU128::from_rational(15, 10);
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

        let non_native_amount = FixedU128::from_rational(7, 10);
        assert_storage_noop!(PriceAggregator::on_new_data(
            &dummy_account_1,
            &non_native_currency_id,
            &non_native_amount
        ));

        // 3. Add additional amount, verify state is as expected
        let amount_2 = FixedU128::from_rational(3, 10);
        PriceAggregator::on_new_data(&dummy_account_1, &native_currency_id, &amount_2);
        assert_eq!(
            CurrentBlockValues::<Test>::get().into_inner(),
            vec![amount_1, amount_2],
        );

        // 4. Fill up storage to the limit, verify state is as expected
        let limit = <Test as Config>::MaxValuesPerBlock::get();
        let mut result = vec![amount_1, amount_2];
        let amount_3 = FixedU128::from_rational(19, 10);

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
