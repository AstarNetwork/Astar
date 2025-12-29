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

#![cfg(test)]

use super::*;
use mock::*;

use astar_primitives::ethereum_checked::EthereumTxInput;
use ethereum::{ReceiptV4 as Receipt, TransactionV3 as Transaction};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError;

fn bounded_input(data: &'static str) -> EthereumTxInput {
    EthereumTxInput::try_from(hex::decode(data).expect("invalid input hex"))
        .expect("input too large")
}

#[test]
fn transact_works() {
    ExtBuilder::default().build().execute_with(|| {
        let store_tx = CheckedEthereumTx {
            gas_limit: U256::from(1_000_000),
            target: contract_address(),
            value: U256::zero(),
            // Calling `store(3)`
            input: bounded_input(
                "6057361d0000000000000000000000000000000000000000000000000000000000000003",
            ),
            maybe_access_list: None,
        };
        assert_ok!(EthereumChecked::transact(
            RawOrigin::XcmEthereumTx(ALICE).into(),
            store_tx.clone()
        ));
        assert_ok!(EthereumChecked::transact(
            RawOrigin::XcmEthereumTx(ALICE).into(),
            store_tx
        ));
        let pending0 = pallet_ethereum::Pending::<TestRuntime>::get(0);
        let pending1 = pallet_ethereum::Pending::<TestRuntime>::get(1);
        assert_eq!(pallet_ethereum::Pending::<TestRuntime>::count(), 2);

        match pending0 {
            Some((Transaction::EIP1559(ref t), _, Receipt::EIP1559(ref r))) => {
                // nonce 0, status code 1 (success)
                assert_eq!(t.nonce, U256::zero());
                assert_eq!(r.status_code, 1);
            }
            _ => panic!("unexpected transaction type"),
        }
        match pending1 {
            Some((Transaction::EIP1559(ref t), _, Receipt::EIP1559(ref r))) => {
                // nonce 1, status code 1 (success)
                assert_eq!(t.nonce, U256::one());
                assert_eq!(r.status_code, 1);
            }
            _ => panic!("unexpected transaction type"),
        }
        assert_eq!(Nonce::<TestRuntime>::get(), U256::from(2));
    });
}

#[test]
fn origin_check_works() {
    ExtBuilder::default().build().execute_with(|| {
        let store_tx = CheckedEthereumTx {
            gas_limit: U256::from(1_000_000),
            target: contract_address(),
            value: U256::zero(),
            // Calling `store(3)`
            input: bounded_input(
                "6057361d0000000000000000000000000000000000000000000000000000000000000003",
            ),
            maybe_access_list: None,
        };
        assert_noop!(
            EthereumChecked::transact(RuntimeOrigin::signed(ALICE), store_tx.clone()),
            DispatchError::BadOrigin
        );
        assert_noop!(
            EthereumChecked::transact(RuntimeOrigin::root(), store_tx.clone()),
            DispatchError::BadOrigin
        );
        assert_noop!(
            EthereumChecked::transact(RuntimeOrigin::none(), store_tx),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn no_hash_collision() {
    ExtBuilder::default().build().execute_with(|| {
        let store_tx = CheckedEthereumTx {
            gas_limit: U256::from(1_000_000),
            target: contract_address(),
            value: U256::zero(),
            // Calling `store(3)`
            input: bounded_input(
                "6057361d0000000000000000000000000000000000000000000000000000000000000003",
            ),
            maybe_access_list: None,
        };
        for _ in 0..5 {
            assert_ok!(EthereumChecked::transact(
                RawOrigin::XcmEthereumTx(ALICE).into(),
                store_tx.clone()
            ));
        }

        let mut tx_hashes = pallet_ethereum::Pending::<TestRuntime>::iter()
            .map(|(_, (tx, _, _))| tx.hash())
            .collect::<Vec<_>>();
        tx_hashes.dedup();
        assert_eq!(tx_hashes.len(), 5);
    });
}
