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

#![cfg_attr(not(feature = "std"), no_std)]

use num_enum::{IntoPrimitive, TryFromPrimitive};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};

#[repr(u16)]
#[derive(TryFromPrimitive, IntoPrimitive, Decode, Encode)]
pub enum Command {
    /// Get the mapped Evm address if any
    GetEvmAddress = 0,
    /// Get the mapped Evm address if any otheriwse default associated Evm address
    GetEvmAddressOrDefault = 1,
    /// Get the mapped Native address if any
    GetNativeAddress = 2,
    /// Get the mapped Native address if any otheriwse default associated Native address
    GetNativeAddressOrDefault = 3,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum UnifiedAddress<T: Encode + Decode> {
    /// The address fetched from the mappings and the account
    /// is unified
    Mapped(T),
    /// The default address associated with account as there
    /// is no mapping found and accounts are not unified
    Default(T),
}
