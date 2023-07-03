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

use super::*;
use mock::*;

use ethereum::ReceiptV3;
use frame_support::assert_ok;

/* Testing contract

pragma solidity >=0.8.2 <0.9.0;

contract Storage {
    uint256 number;

    /**
     * @dev Store value in variable
     * @param num value to store
     */
    function store(uint256 num) public {
        number = num;
    }

    /**
     * @dev Return value
     * @return value of 'number'
     */
    function retrieve() public view returns (uint256){
        return number;
    }
}
*/
const STORAGE_CONTRACT: &str = "608060405234801561001057600080fd5b50610150806100206000396000f3fe608060405234801561001057600080fd5b50600436106100365760003560e01c80632e64cec11461003b5780636057361d14610059575b600080fd5b610043610075565b60405161005091906100a1565b60405180910390f35b610073600480360381019061006e91906100ed565b61007e565b005b60008054905090565b8060008190555050565b6000819050919050565b61009b81610088565b82525050565b60006020820190506100b66000830184610092565b92915050565b600080fd5b6100ca81610088565b81146100d557600080fd5b50565b6000813590506100e7816100c1565b92915050565b600060208284031215610103576101026100bc565b5b6000610111848285016100d8565b9150509291505056fea2646970667358221220322c78243e61b783558509c9cc22cb8493dde6925aa5e89a08cdf6e22f279ef164736f6c63430008120033";

#[test]
fn transact_checked_works() {
    ExtBuilder::default().build().execute_with(|| {
        // Deploy testing contract.
        assert_ok!(Evm::create2(
            RuntimeOrigin::root(),
            ALICE_H160,
            hex::decode(STORAGE_CONTRACT).unwrap(),
            H256::zero(),
            U256::zero(),
            1_000_000,
            U256::one(),
            None,
            Some(U256::zero()),
            vec![],
        ));
        let address =
            H160::from_slice(&hex::decode("dfb975d018f03994a3b943808e3aa0964bd78463").unwrap());
        System::assert_last_event(RuntimeEvent::Evm(pallet_evm::Event::Created { address }));

        let store_tx = CheckedEthereumTx {
            gas_limit: U256::from(1_000_000),
            action: TransactionAction::Call(address),
            value: U256::zero(),
            // Calling `store(3)`
            input: hex::decode(
                "6057361d0000000000000000000000000000000000000000000000000000000000000003",
            )
            .unwrap(),
            maybe_access_list: None,
            kind: CheckedEthereumTxKind::Xcm,
        };
        assert_ok!(EthereumChecked::transact_checked(
            ALICE_H160,
            store_tx.clone()
        ));
        assert_ok!(EthereumChecked::transact_checked(ALICE_H160, store_tx));
        let pending = Ethereum::pending();
        assert_eq!(pending.len(), 2);

        match pending[0] {
            (Transaction::EIP1559(ref t), _, ReceiptV3::EIP1559(ref r)) => {
                // nonce 0, status code 1 (success)
                assert_eq!(t.nonce, U256::zero());
                assert_eq!(r.status_code, 1);
            }
            _ => panic!("unexpected transaction type"),
        }
        match pending[1] {
            (Transaction::EIP1559(ref t), _, ReceiptV3::EIP1559(ref r)) => {
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
fn no_hash_collision() {
    ExtBuilder::default().build().execute_with(|| {
        // Deploy testing contract.
        assert_ok!(Evm::create2(
            RuntimeOrigin::root(),
            ALICE_H160,
            hex::decode(STORAGE_CONTRACT).unwrap(),
            H256::zero(),
            U256::zero(),
            1_000_000,
            U256::one(),
            None,
            Some(U256::zero()),
            vec![],
        ));
        let address =
            H160::from_slice(&hex::decode("dfb975d018f03994a3b943808e3aa0964bd78463").unwrap());
        System::assert_last_event(RuntimeEvent::Evm(pallet_evm::Event::Created { address }));

        let store_tx = CheckedEthereumTx {
            gas_limit: U256::from(1_000_000),
            action: TransactionAction::Call(address),
            value: U256::zero(),
            // Calling `store(3)`
            input: hex::decode(
                "6057361d0000000000000000000000000000000000000000000000000000000000000003",
            )
            .unwrap(),
            maybe_access_list: None,
            kind: CheckedEthereumTxKind::Xcm,
        };
        for _ in 0..5 {
            assert_ok!(EthereumChecked::transact_checked(
                ALICE_H160,
                store_tx.clone()
            ));
            assert_ok!(EthereumChecked::transact_checked(
                BOB_H160,
                store_tx.clone()
            ));
            assert_ok!(EthereumChecked::transact_checked(
                CHARLIE_H160,
                store_tx.clone()
            ));
        }

        let pending = Ethereum::pending();
        let mut tx_hashes = pending
            .iter()
            .map(|(tx, _, _)| tx.hash())
            .collect::<Vec<_>>();
        tx_hashes.dedup();
        assert_eq!(tx_hashes.len(), 15);
    });
}
