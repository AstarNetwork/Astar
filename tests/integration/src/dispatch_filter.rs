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
use frame_support::traits::InstanceFilter;

/// Whitelisted Calls are defined in the runtime
#[test]
fn filter_accepts_batch_call_with_whitelisted_calls() {
    ExtBuilder::default().build().execute_with(|| {
        let contract = SmartContract::Evm(H160::repeat_byte(0x01));
        let inner_call = RuntimeCall::DappsStaking(DappStakingCall::Call::claim_staker {
            contract_id: contract.clone(),
        });
        let call = RuntimeCall::Utility(UtilityCall::batch {
            calls: vec![inner_call],
        });
        assert!(DispatchPrecompileFilter.filter(&call));
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
        assert!(!DispatchPrecompileFilter.filter(&call));

        // CASE 2 - now whitelisted mixed with whitelisted calls

        let contract = SmartContract::Evm(H160::repeat_byte(0x01));
        let staking_call = RuntimeCall::DappsStaking(DappStakingCall::Call::claim_staker {
            contract_id: contract.clone(),
        });
        let staking = Box::new(staking_call);

        let call = Box::new(RuntimeCall::Utility(UtilityCall::batch {
            calls: vec![*transfer, *staking.clone()],
        }));

        // Utility call containing Balances Call and Dappsstaking Call Fails filter
        assert!(!DispatchPrecompileFilter.filter(&call));
    });
}

#[test]
fn filter_accepts_whitelisted_calls() {
    ExtBuilder::default().build().execute_with(|| {
        let contract = SmartContract::Evm(H160::repeat_byte(0x01));
        let stake_call = RuntimeCall::DappsStaking(DappStakingCall::Call::claim_staker {
            contract_id: contract.clone(),
        });
        assert!(DispatchPrecompileFilter.filter(&stake_call));
    });
}

#[test]
fn filter_rejects_non_whitelisted_calls() {
    ExtBuilder::default().build().execute_with(|| {
        let transfer_call = RuntimeCall::Balances(BalancesCall::transfer {
            dest: MultiAddress::Id(CAT),
            value: 100_000_000_000,
        });
        assert!(!DispatchPrecompileFilter.filter(&transfer_call));
    })
}
