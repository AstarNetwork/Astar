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

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

use ethereum::{
    AccessListItem, EIP1559Transaction, TransactionAction, TransactionV2 as Transaction,
};
use ethereum_types::{H160, H256, U256};
use fp_evm::CallInfo;
use frame_support::{
    dispatch::{DispatchErrorWithPostInfo, PostDispatchInfo},
    pallet_prelude::*,
    traits::ConstU32,
    BoundedVec,
};
use sp_core::Hasher;
use sp_std::{prelude::*, result::Result};

use crate::AccountId;

/// Max Ethereum tx input size: 65_536 bytes
pub const MAX_ETHEREUM_TX_INPUT_SIZE: u32 = 2u32.pow(16);

pub type EthereumTxInput = BoundedVec<u8, ConstU32<MAX_ETHEREUM_TX_INPUT_SIZE>>;

/// The checked Ethereum transaction. Only contracts `call` is support(no `create`).
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct CheckedEthereumTx {
    /// Gas limit.
    pub gas_limit: U256,
    /// Contract address to call.
    pub target: H160,
    /// Amount to transfer.
    pub value: U256,
    /// Input of a contract call.
    pub input: EthereumTxInput,
    /// Optional access list, specified in EIP-2930.
    pub maybe_access_list: Option<Vec<(H160, Vec<H256>)>>,
}

impl CheckedEthereumTx {
    pub fn into_ethereum_tx(&self, nonce: U256, chain_id: u64) -> Transaction {
        let access_list = if let Some(ref list) = self.maybe_access_list {
            list.iter()
                .map(|(address, storage_keys)| AccessListItem {
                    address: *address,
                    storage_keys: storage_keys.clone(),
                })
                .collect()
        } else {
            Vec::new()
        };

        Transaction::EIP1559(EIP1559Transaction {
            chain_id,
            nonce,
            max_fee_per_gas: U256::zero(),
            max_priority_fee_per_gas: U256::zero(),
            gas_limit: self.gas_limit,
            value: self.value,
            action: TransactionAction::Call(self.target),
            input: self.input.to_vec(),
            access_list,
            odd_y_parity: true,
            r: dummy_rs(),
            s: dummy_rs(),
        })
    }
}

/// Dummy signature for all transactions.
fn dummy_rs() -> H256 {
    H256::from_low_u64_be(1u64)
}

/// Transact an checked Ethereum transaction. Similar to `pallet_ethereum::Transact` but
/// doesn't require tx signature.
pub trait CheckedEthereumTransact {
    /// Transact an checked Ethereum transaction in XVM.
    fn xvm_transact(
        source: H160,
        checked_tx: CheckedEthereumTx,
    ) -> Result<(PostDispatchInfo, CallInfo), DispatchErrorWithPostInfo>;
}

/// Mapping from `Account` to `H160`.
pub trait AccountMapping<AccountId> {
    fn into_h160(account: AccountId) -> H160;
}

/// Hashed derive mapping for converting account id to evm address
pub struct HashedAccountMapping<H>(sp_std::marker::PhantomData<H>);
impl<H: Hasher<Out = H256>> AccountMapping<AccountId> for HashedAccountMapping<H> {
    fn into_h160(account: AccountId) -> H160 {
        let payload = (b"evm:", account);
        H160::from_slice(&payload.using_encoded(H::hash)[0..20])
    }
}
