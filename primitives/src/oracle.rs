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

use frame_support::{pallet_prelude::*, traits::Time};
use sp_arithmetic::fixed_point::FixedU128;
use sp_std::vec::Vec;

/// Interface for fetching price of the native token.
pub trait PriceProvider {
    /// Get the price of the native token.
    fn average_price() -> Price;
}

pub type Price = FixedU128;
pub type CurrencyAmount = FixedU128;

#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    TypeInfo,
)]
pub enum CurrencyId {
    ASTR,
    SDN,
}

type TimestampedValue<T, I = ()> =
    orml_oracle::TimestampedValue<Price, <<T as orml_oracle::Config<I>>::Time as Time>::Moment>;

/// A dummy implementation of `CombineData` trait that does nothing.
pub struct DummyCombineData<T, I = ()>(PhantomData<(T, I)>);
impl<T: orml_oracle::Config<I>, I> orml_traits::CombineData<CurrencyId, TimestampedValue<T, I>>
    for DummyCombineData<T, I>
{
    fn combine_data(
        _key: &CurrencyId,
        _values: Vec<TimestampedValue<T, I>>,
        _prev_value: Option<TimestampedValue<T, I>>,
    ) -> Option<TimestampedValue<T, I>> {
        None
    }
}
