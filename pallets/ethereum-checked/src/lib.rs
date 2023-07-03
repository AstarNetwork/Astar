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

//TODO: update docstring

//! # Ethereum Checked Pallet
//!
//! ## Overview
//!
//! A `pallet-ethererum` like pallet that execute transactions from checked source,
//! like XCM remote call, cross-VM call, etc.
//!
//! ## Interface
//!

#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

use ethereum::{
    AccessListItem, EIP1559Transaction, TransactionAction, TransactionV2 as Transaction,
};
use ethereum_types::{H160, H256, U256};
use fp_ethereum::{TransactionData, ValidatedTransaction};
use fp_evm::{CheckEvmTransaction, CheckEvmTransactionConfig, InvalidEvmTransactionError};
use pallet_evm::GasWeightMapping;

use frame_support::{
    dispatch::{DispatchErrorWithPostInfo, PostDispatchInfo},
    pallet_prelude::*,
};
use sp_std::prelude::*;

use pallet::*;

mod mock;
mod tests;

//TODO: move type definitions to primitives crate

/// Transaction kind.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum CheckedEthereumTxKind {
    /// The tx is from XCM remote call.
    Xcm,
    /// The tx is from cross-VM call.
    Xvm,
}

/// The checked Ethereum transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct CheckedEthereumTx {
    /// Gas limit.
    pub gas_limit: U256,
    /// Action type, either `Call` or `Create`.
    pub action: TransactionAction,
    /// Amount to transfer.
    pub value: U256,
    /// Input of a contract call.
    pub input: Vec<u8>,
    /// Optional access list specified in EIP-2930.
    pub maybe_access_list: Option<Vec<(H160, Vec<H256>)>>,
    /// Transaction kind. For instance, XCM or XVM.
    pub kind: CheckedEthereumTxKind,
}

impl CheckedEthereumTx {
    fn into_ethereum_tx(&self, nonce: U256, chain_id: u64) -> Transaction {
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
            action: self.action,
            input: self.input.clone(),
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

pub trait CheckedEthereumTransact {
    fn transact(source: H160, checked_tx: CheckedEthereumTx) -> DispatchResultWithPostInfo;
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_evm::Config {
        /// Reserved Xcmp weight for block gas limit calculation.
        type ReservedXcmpWeight: Get<Weight>;

        /// Xcm transaction weight limit, for block gas limit calculation.
        type XvmTxWeightLimit: Get<Weight>;

        /// Invalid tx error.
        type InvalidEvmTransactionError: From<InvalidEvmTransactionError>;

        /// Validated tx execution.
        type ValidatedTransaction: ValidatedTransaction;
    }

    /// Global nonce for all transactions to avoid hash collision, which is
    /// caused by the same dummy signatures for all transactions.
    #[pallet::storage]
    pub type Nonce<T: Config> = StorageValue<_, U256, ValueQuery>;
}

impl<T: Config> Pallet<T> {
    /// Validate and execute the checked tx.
    fn transact_checked(source: H160, checked_tx: CheckedEthereumTx) -> DispatchResultWithPostInfo {
        let chain_id = T::ChainId::get();
        let nonce = Nonce::<T>::get();
        let tx = checked_tx.into_ethereum_tx(Nonce::<T>::get(), chain_id);
        let tx_data: TransactionData = (&tx).into();

        // Validate the tx.
        let _ = CheckEvmTransaction::<T::InvalidEvmTransactionError>::new(
            CheckEvmTransactionConfig {
                evm_config: T::config(),
                block_gas_limit: U256::from(Self::block_gas_limit(&checked_tx.kind)),
                base_fee: U256::zero(),
                chain_id,
                is_transactional: true,
            },
            tx_data.into(),
        )
        // Gas limit validation. The fee payment has been validated as the tx is `checked`.
        .validate_common()
        .map_err(|_| DispatchErrorWithPostInfo {
            post_info: PostDispatchInfo {
                //TODO: calculate weight on error
                actual_weight: Some(Weight::default()),
                pays_fee: Pays::Yes,
            },
            error: DispatchError::Other("Failed to validate Ethereum tx"),
        })?;

        Nonce::<T>::put(nonce.saturating_add(U256::one()));

        // Execute the tx.
        T::ValidatedTransaction::apply(source, tx)
    }

    /// Block gas limit calculation based on the tx kind.
    fn block_gas_limit(tx_kind: &CheckedEthereumTxKind) -> u64 {
        let weight_limit = match tx_kind {
            CheckedEthereumTxKind::Xcm => T::ReservedXcmpWeight::get(),
            CheckedEthereumTxKind::Xvm => T::XvmTxWeightLimit::get(),
        };
        T::GasWeightMapping::weight_to_gas(weight_limit)
    }
}

impl<T: Config> CheckedEthereumTransact for Pallet<T> {
    fn transact(source: H160, checked_tx: CheckedEthereumTx) -> DispatchResultWithPostInfo {
        Self::transact_checked(source, checked_tx)
    }
}
