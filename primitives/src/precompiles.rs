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

use core::marker::PhantomData;

use fp_evm::{ExitError, PrecompileFailure};
use frame_support::{
    dispatch::{DispatchClass, GetDispatchInfo, Pays},
    traits::Contains,
};
use pallet_evm_precompile_dispatch::DispatchValidateT;

/// Struct that allows only calls based on `Filter` to pass through.
pub struct DispatchFilterValidate<RuntimeCall, Filter: Contains<RuntimeCall>>(
    PhantomData<(RuntimeCall, Filter)>,
);

impl<AccountId, RuntimeCall: GetDispatchInfo, Filter: Contains<RuntimeCall>>
    DispatchValidateT<AccountId, RuntimeCall> for DispatchFilterValidate<RuntimeCall, Filter>
{
    fn validate_before_dispatch(
        _origin: &AccountId,
        call: &RuntimeCall,
    ) -> Option<PrecompileFailure> {
        let info = call.get_dispatch_info();
        let paid_normal_call = info.pays_fee == Pays::Yes && info.class == DispatchClass::Normal;
        if !paid_normal_call {
            return Some(PrecompileFailure::Error {
                exit_status: ExitError::Other("invalid call".into()),
            });
        }
        if Filter::contains(call) {
            None
        } else {
            Some(PrecompileFailure::Error {
                exit_status: ExitError::Other("call filtered out".into()),
            })
        }
    }
}
