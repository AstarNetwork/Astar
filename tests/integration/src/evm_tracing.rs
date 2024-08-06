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

#[cfg(test)]
#[cfg(feature = "evm-tracing")]
mod tests {
    use crate::setup::*;

    use astar_primitives::Header;
    use fp_rpc::ConvertTransaction;
    use moonbeam_rpc_primitives_debug::runtime_decl_for_debug_runtime_api::DebugRuntimeApi;
    use sp_core::U256;

    // A valid signed Alice transfer.
    pub const VALID_ETH_TX: &str =
        "02f869820501808085e8d4a51000825208943cd0a705a2dc65e5b1e1205896baa2be8a07c6e00180c\
	001a061087911e877a5802142a89a40d231d50913db399eb50839bb2d04e612b22ec8a01aa313efdf2\
	793bea76da6813bda611444af16a6207a8cfef2d9c8aa8f8012f7";

    pub struct TransactionConverter;

    impl ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
        fn convert_transaction(
            &self,
            transaction: pallet_ethereum::Transaction,
        ) -> UncheckedExtrinsic {
            UncheckedExtrinsic::new_unsigned(
                pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
            )
        }
    }

    pub fn unchecked_eth_tx(raw_hex_tx: &str) -> UncheckedExtrinsic {
        let converter = TransactionConverter;
        converter.convert_transaction(ethereum_transaction(raw_hex_tx))
    }

    pub fn ethereum_transaction(raw_hex_tx: &str) -> pallet_ethereum::Transaction {
        let bytes = hex::decode(raw_hex_tx).expect("Transaction bytes.");
        let transaction = ethereum::EnvelopedDecodable::decode(&bytes[..]);
        assert!(transaction.is_ok());
        transaction.unwrap()
    }

    #[test]
    fn debug_runtime_api_trace_transaction() {
        new_test_ext().execute_with(|| {
            let non_eth_uxt = UncheckedExtrinsic::new_unsigned(
                pallet_balances::Call::<Runtime>::transfer_allow_death {
                    dest: MultiAddress::Id(AccountId::from(BOB)),
                    value: 1 * UNIT,
                }
                .into(),
            );
            let transaction = ethereum_transaction(VALID_ETH_TX);
            let eth_uxt = unchecked_eth_tx(VALID_ETH_TX);
            let block = Header {
                digest: Default::default(),
                extrinsics_root: Default::default(),
                number: 1,
                parent_hash: Default::default(),
                state_root: Default::default(),
            };
            assert_ok!(Runtime::trace_transaction(
                vec![non_eth_uxt.clone(), eth_uxt, non_eth_uxt.clone()],
                &transaction,
                &block
            ));
        });
    }

    #[test]
    fn debug_runtime_api_trace_block() {
        new_test_ext().execute_with(|| {
            let non_eth_uxt = UncheckedExtrinsic::new_unsigned(
                pallet_balances::Call::<Runtime>::transfer_allow_death {
                    dest: MultiAddress::Id(AccountId::from(BOB)),
                    value: 1 * UNIT,
                }
                .into(),
            );
            let eth_uxt = unchecked_eth_tx(VALID_ETH_TX);
            let eth_tx = ethereum_transaction(VALID_ETH_TX);
            let eth_extrinsic_hash = eth_tx.hash();
            let block = Header {
                digest: Default::default(),
                extrinsics_root: Default::default(),
                number: 1,
                parent_hash: Default::default(),
                state_root: Default::default(),
            };
            assert_ok!(Runtime::trace_block(
                vec![non_eth_uxt.clone(), eth_uxt.clone(), non_eth_uxt, eth_uxt],
                vec![eth_extrinsic_hash, eth_extrinsic_hash],
                &block
            ));
        });
    }

    #[test]
    fn debug_runtime_api_trace_call() {
        new_test_ext().execute_with(|| {
            let block = Header {
                digest: Default::default(),
                extrinsics_root: Default::default(),
                number: 1,
                parent_hash: Default::default(),
                state_root: Default::default(),
            };

            assert_ok!(Runtime::trace_call(
                &block,
                H160::repeat_byte(0x01),
                H160::repeat_byte(0x02),
                vec![0x03, 0x04],
                U256::from(0x12345678),
                U256::from(0x123),
                Some(U256::from(0x456)),
                Some(U256::from(0x789)),
                Some(U256::from(1)),
                Some(vec![]),
            ));
        });
    }
}
