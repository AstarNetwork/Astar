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

use crate::{mock::*, Event};

use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use pallet_balances::Call as BalancesCall;

#[test]
fn execute_call_fails_for_invalid_origin() {
    ExtBuilder::build().execute_with(|| {
        assert_noop!(
            CollectiveProxy::execute_call(
                RuntimeOrigin::signed(1),
                Box::new(RuntimeCall::Balances(BalancesCall::transfer_allow_death {
                    dest: 2,
                    value: 10
                }))
            ),
            BadOrigin
        );
    });
}

#[test]
fn execute_call_filters_not_allowed_call() {
    ExtBuilder::build().execute_with(|| {
        let init_balance = Balances::free_balance(COMMUNITY_ACCOUNT);

        // Call is filtered, but `execute_call` succeeds.
        assert_ok!(CollectiveProxy::execute_call(
            RuntimeOrigin::signed(PRIVILEGED_ACCOUNT),
            Box::new(RuntimeCall::Balances(BalancesCall::transfer_keep_alive {
                dest: 2,
                value: 10
            }))
        ));

        // Ensure event with error is emitted.
        System::assert_last_event(
            Event::<Test>::CollectiveProxyExecuted {
                result: Err(frame_system::Error::<Test>::CallFiltered.into()),
            }
            .into(),
        );

        // Balance must have remained unchanged.
        let after_balance = Balances::free_balance(COMMUNITY_ACCOUNT);
        assert_eq!(
            init_balance, after_balance,
            "Since transfer_keep_alive is filtered out, no balance change is expected."
        );
    });
}

#[test]
fn execute_call_succeeds() {
    ExtBuilder::build().execute_with(|| {
        let init_balance = Balances::free_balance(COMMUNITY_ACCOUNT);
        let transfer_value = init_balance / 3;

        assert_ok!(CollectiveProxy::execute_call(
            RuntimeOrigin::signed(PRIVILEGED_ACCOUNT),
            Box::new(RuntimeCall::Balances(BalancesCall::transfer_allow_death {
                dest: 2,
                value: transfer_value
            }))
        ));

        System::assert_last_event(
            Event::<Test>::CollectiveProxyExecuted {
                result: Ok(().into()),
            }
            .into(),
        );

        // Balance must have been updated.
        let after_balance = Balances::free_balance(COMMUNITY_ACCOUNT);
        assert_eq!(init_balance, after_balance + transfer_value,);
    });
}
