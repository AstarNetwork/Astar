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

use super::{pallet::Error, Event, *};
use frame_support::{assert_noop, assert_ok};
use mock::*;
use sp_runtime::traits::{BadOrigin, Zero};

#[test]
fn force_set_price_works() {
    ExternalityBuilder::build().execute_with(|| {
        assert!(!ActivePrice::<Test>::get().is_zero(), "Sanity check");

        let new_price = ActivePrice::<Test>::get() * 2.into();
        assert_ok!(StaticPriceProvider::force_set_price(
            RuntimeOrigin::root(),
            new_price
        ));
        System::assert_last_event(RuntimeEvent::StaticPriceProvider(Event::PriceSet {
            price: new_price,
        }));
        assert_eq!(ActivePrice::<Test>::get(), new_price);
        assert_eq!(StaticPriceProvider::average_price(), new_price);
    })
}

#[test]
fn force_set_zero_price_fails() {
    ExternalityBuilder::build().execute_with(|| {
        assert_noop!(
            StaticPriceProvider::force_set_price(RuntimeOrigin::root(), 0.into()),
            Error::<Test>::ZeroPrice
        );
    })
}

#[test]
fn force_set_price_with_invalid_origin_fails() {
    ExternalityBuilder::build().execute_with(|| {
        assert_noop!(
            StaticPriceProvider::force_set_price(RuntimeOrigin::signed(1), 1.into()),
            BadOrigin
        );
    })
}
