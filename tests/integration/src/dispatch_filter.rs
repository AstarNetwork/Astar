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

#[test]
fn filter_accepts_batch_call_with_dappsstaking() {
    ExtBuilder::default().build().execute_with(|| {
        let contract = SmartContract::Evm(H160::repeat_byte(0x01));
        let inner_call = RuntimeCall::DappsStaking(DappStakingCall::Call::claim_staker{
            contract_id : contract.clone(),
        });
        let call = RuntimeCall::Utility(UtilityCall::batch {
            calls : vec![inner_call]
        });
        assert!(DispatchPrecompileFilter.filter(&call));
    });
}