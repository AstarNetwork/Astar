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
    traits::InstanceFilter,
};
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

pub struct DispatchFilterValidate<RuntimeCall, Filter: InstanceFilter<RuntimeCall> + Default>(
    PhantomData<(RuntimeCall, Filter)>,
);

impl<AccountId, RuntimeCall: GetDispatchInfo, Filter: InstanceFilter<RuntimeCall> + Default>
    DispatchValidateT<AccountId, RuntimeCall> for DispatchFilterValidate<RuntimeCall, Filter>
{
    fn validate_before_dispatch(
        _origin: &AccountId,
        call: &RuntimeCall,
    ) -> Option<PrecompileFailure> {
        let info = call.get_dispatch_info();
        if !(info.pays_fee == Pays::Yes && info.class == DispatchClass::Normal) {
            return Some(PrecompileFailure::Error {
                exit_status: ExitError::Other("invalid call".into()),
            });
        } else if Filter::default().filter(call) {
            return None;
        } else {
            return Some(PrecompileFailure::Error {
                exit_status: ExitError::Other("invalid call".into()),
            });
        }
    }
}

mod test {
    use super::*;
    #[test]
    fn all_non_whitelisted_call_should_fail() {
        assert_eq!(
            BlockAllDispatchValidate::validate_before_dispatch(&(), &()).unwrap(),
            PrecompileFailure::Error {
                exit_status: ExitError::Other("invalid call".into()),
            }
        )
    }
}
