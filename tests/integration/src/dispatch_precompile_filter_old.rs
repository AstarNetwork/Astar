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
#![cfg(test)]

use crate::setup::*;
use astar_primitives::precompiles::DispatchFilterValidate;
use fp_evm::{ExitError, PrecompileFailure};
use frame_support::{
    dispatch::{DispatchClass, DispatchInfo, GetDispatchInfo, Pays},
    traits::Contains,
};
use pallet_evm_precompile_dispatch::DispatchValidateT;
use parity_scale_codec::Compact;

/// Whitelisted Calls are defined in the runtime
#[test]
fn filter_accepts_batch_call_with_whitelisted_calls() {
    ExtBuilder::default().build().execute_with(|| {
        let contract = SmartContract::Evm(H160::repeat_byte(0x01));
        let inner_call = RuntimeCall::DappsStaking(DappsStakingCall::Call::claim_staker {
            contract_id: contract.clone(),
        });
        let call = RuntimeCall::Utility(UtilityCall::batch {
            calls: vec![inner_call],
        });
        assert!(WhitelistedCalls::contains(&call));
    });
}

#[test]
fn filter_rejects_non_whitelisted_batch_calls() {
    ExtBuilder::default().build().execute_with(|| {
        // CASE1 - only non whitelisted calls
        let transfer_call = RuntimeCall::Balances(BalancesCall::transfer {
            dest: MultiAddress::Id(CAT),
            value: 100_000_000_000,
        });
        let transfer = Box::new(transfer_call);
        let call = Box::new(RuntimeCall::Utility(UtilityCall::batch {
            calls: vec![*transfer.clone()],
        }));

        // Utility call containing Balances Call
        assert!(!WhitelistedCalls::contains(&call));

        // CASE 2 - now whitelisted mixed with whitelisted calls

        let contract = SmartContract::Evm(H160::repeat_byte(0x01));
        let staking_call = RuntimeCall::DappsStaking(DappsStakingCall::Call::claim_staker {
            contract_id: contract.clone(),
        });
        let staking = Box::new(staking_call);

        let call = Box::new(RuntimeCall::Utility(UtilityCall::batch {
            calls: vec![*transfer, *staking.clone()],
        }));

        // Utility call containing Balances Call and Dappsstaking Call Fails filter
        assert!(!WhitelistedCalls::contains(&call));
    });
}

#[test]
fn filter_accepts_whitelisted_calls() {
    ExtBuilder::default().build().execute_with(|| {
        // Dappstaking call works
        let contract = SmartContract::Evm(H160::repeat_byte(0x01));
        let stake_call = RuntimeCall::DappsStaking(DappsStakingCall::Call::claim_staker {
            contract_id: contract.clone(),
        });
        assert!(WhitelistedCalls::contains(&stake_call));

        // Pallet::Assets transfer call works
        let transfer_call = RuntimeCall::Assets(pallet_assets::Call::transfer {
            id: Compact(0),
            target: MultiAddress::Address20(H160::repeat_byte(0x01).into()),
            amount: 100,
        });
        assert!(WhitelistedCalls::contains(&transfer_call));
    });
}

#[test]
fn filter_rejects_non_whitelisted_calls() {
    ExtBuilder::default().build().execute_with(|| {
        // Random call from non whitelisted pallet doesn't work
        let transfer_call = RuntimeCall::Balances(BalancesCall::transfer {
            dest: MultiAddress::Id(CAT),
            value: 100_000_000_000,
        });
        assert!(!WhitelistedCalls::contains(&transfer_call));

        // Only `transfer` call from pallet assets work
        // Other random call from Pallet Assets doesn't work
        let thaw_asset_call =
            RuntimeCall::Assets(pallet_assets::Call::thaw_asset { id: Compact(0) });
        assert!(!WhitelistedCalls::contains(&thaw_asset_call));
    })
}

#[test]
fn filter_accepts_whitelisted_batch_all_calls() {
    ExtBuilder::default().build().execute_with(|| {
        let contract = SmartContract::Evm(H160::repeat_byte(0x01));
        let inner_call1 = RuntimeCall::DappsStaking(DappsStakingCall::Call::claim_staker {
            contract_id: contract.clone(),
        });
        let inner_call2 = RuntimeCall::DappsStaking(DappsStakingCall::Call::claim_staker {
            contract_id: contract.clone(),
        });
        let transfer_call = RuntimeCall::Assets(pallet_assets::Call::transfer {
            id: Compact(0),
            target: MultiAddress::Address20(H160::repeat_byte(0x01).into()),
            amount: 100,
        });
        let call = RuntimeCall::Utility(UtilityCall::batch_all {
            calls: vec![inner_call1, inner_call2, transfer_call],
        });
        assert!(WhitelistedCalls::contains(&call));
    });
}

#[test]
fn test_correct_dispatch_info_works() {
    ExtBuilder::default().build().execute_with(|| {
        // Mock implementation
        struct Filter;
        struct AccountId;
        enum RuntimeCall {
            System,
            DappsStaking,
        }
        impl GetDispatchInfo for RuntimeCall {
            fn get_dispatch_info(&self) -> DispatchInfo {
                // Default is Pays::Yes and DispatchCall::Normal
                DispatchInfo::default()
            }
        }
        impl Contains<RuntimeCall> for Filter {
            fn contains(t: &RuntimeCall) -> bool {
                match t {
                    RuntimeCall::DappsStaking => true,
                    _ => false,
                }
            }
        }
        // Case 1: Whitelisted Call with correct Dispatch info
        assert_eq!(
            DispatchFilterValidate::<RuntimeCall, Filter>::validate_before_dispatch(
                &AccountId,
                &RuntimeCall::DappsStaking
            ),
            Option::None
        );
        // Case 2: Non-Whitelisted Call with correct Dispatch Info
        assert_eq!(
            DispatchFilterValidate::<RuntimeCall, Filter>::validate_before_dispatch(
                &AccountId,
                &RuntimeCall::System
            ),
            Option::Some(PrecompileFailure::Error {
                exit_status: ExitError::Other("call filtered out".into()),
            })
        );
    });
}

#[test]
fn test_incorrect_dispatch_info_fails() {
    ExtBuilder::default().build().execute_with(|| {
        // Mock implementation
        struct Filter;
        struct AccountId;
        enum RuntimeCall {
            DappsStaking,
        }
        impl GetDispatchInfo for RuntimeCall {
            fn get_dispatch_info(&self) -> DispatchInfo {
                DispatchInfo {
                    weight: Weight::default(),
                    class: DispatchClass::Normal,
                    // Should have been Pays::Yes for call to pass
                    pays_fee: Pays::No,
                }
            }
        }
        impl Contains<RuntimeCall> for Filter {
            fn contains(t: &RuntimeCall) -> bool {
                match t {
                    RuntimeCall::DappsStaking => true,
                }
            }
        }

        // WhiteListed Call fails because of incorrect DispatchInfo
        assert_eq!(
            DispatchFilterValidate::<RuntimeCall, Filter>::validate_before_dispatch(
                &AccountId,
                &RuntimeCall::DappsStaking
            ),
            Option::Some(PrecompileFailure::Error {
                exit_status: ExitError::Other("invalid call".into()),
            })
        );
    })
}
