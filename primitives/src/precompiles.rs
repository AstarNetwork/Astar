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

use fp_evm::{ExitError, PrecompileFailure};
use pallet_evm_precompile_dispatch::DispatchValidateT;

pub struct BlockAllDispatchValidate;

/// The default implementation of `DispatchValidateT`.
impl<AccountId, RuntimeCall> DispatchValidateT<AccountId, RuntimeCall>
    for BlockAllDispatchValidate
{
    fn validate_before_dispatch(
        _origin: &AccountId,
        _call: &RuntimeCall,
    ) -> Option<PrecompileFailure> {
        Some(PrecompileFailure::Error {
            exit_status: ExitError::Other("invalid call".into()),
        })
    }
}
