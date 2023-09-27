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

use crate::setup::*;
pub use sp_io::hashing::keccak_256;

#[test]
fn transfer_to_h160_via_lookup() {
    new_test_ext().execute_with(|| {
        let eth_address = H160::from_slice(&keccak_256(b"Alice")[0..20]);

        // make sure account is empty
        assert!(EVM::is_account_empty(&eth_address));

        // tranfer to evm account
        assert_ok!(Balances::transfer(
            RuntimeOrigin::signed(ALICE),
            MultiAddress::Address20(eth_address.clone().into()),
            UNIT,
        ));

        // evm account should have recieved the funds
        let (account, _) = EVM::account_basic(&eth_address);
        assert_eq!(account.balance, (UNIT - ExistentialDeposit::get()).into());
    });
}
