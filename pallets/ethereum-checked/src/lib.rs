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

//! # Ethereum Checked Pallet
//!
//! ## Overview
//!
//! A `pallet-ethererum` like pallet that execute transactions from checked source,
//! like XCM remote call, cross-VM call, etc. Only `Call` transactions are supported
//! (no `Create`).
//!
//! The checked source guarantees that transactions are valid with prior checks, so these
//! transactions are not required to include valid signatures. Instead, `pallet-ethereum-checked`
//! will add the same dummy signature to them. To avoid transaction hash collisions, a global
//! nonce shared with all users are used.
//!
//! ## Interface
//!
//! ### Dispatch-able calls
//!
//! - `transact`: transact an Ethereum transaction. Similar to `pallet_ethereum::Transact`,
//! but is only for XCM remote call.
//!
//! ### Implementation
//!
//! - Implements `CheckedEthereumTransact` trait.
//!

#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

use ethereum_types::{H160, U256};
use fp_ethereum::{TransactionData, ValidatedTransaction};
use fp_evm::{
    CallInfo, CallOrCreateInfo, CheckEvmTransaction, CheckEvmTransactionConfig,
    InvalidEvmTransactionError,
};
use pallet_evm::GasWeightMapping;

use frame_support::{
    dispatch::{DispatchErrorWithPostInfo, PostDispatchInfo},
    pallet_prelude::*,
};
use frame_system::pallet_prelude::*;
#[cfg(feature = "runtime-benchmarks")]
use sp_runtime::traits::TrailingZeroInput;
use sp_runtime::traits::UniqueSaturatedInto;
use sp_std::{marker::PhantomData, result::Result};

use astar_primitives::ethereum_checked::{
    AccountMapping, CheckedEthereumTransact, CheckedEthereumTx,
};

pub use pallet::*;

mod mock;
mod tests;

/// Origin for dispatch-able calls.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum RawOrigin<AccountId> {
    XcmEthereumTx(AccountId),
}

/// Ensure the origin is with XCM calls.
pub struct EnsureXcmEthereumTx<AccountId>(PhantomData<AccountId>);
impl<O: Into<Result<RawOrigin<AccountId>, O>> + From<RawOrigin<AccountId>>, AccountId: Decode>
    EnsureOrigin<O> for EnsureXcmEthereumTx<AccountId>
{
    type Success = AccountId;

    fn try_origin(o: O) -> Result<Self::Success, O> {
        o.into().map(|o| match o {
            RawOrigin::XcmEthereumTx(account_id) => account_id,
        })
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin() -> Result<O, ()> {
        let zero_account_id =
            AccountId::decode(&mut TrailingZeroInput::zeroes()).map_err(|_| ())?;
        Ok(O::from(RawOrigin::XcmEthereumTx(zero_account_id)))
    }
}

/// Transaction kind.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum CheckedEthereumTxKind {
    /// The tx is from XCM remote call.
    Xcm,
    /// The tx is from cross-VM call.
    Xvm,
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

        /// Account mapping.
        type AccountMapping: AccountMapping<Self::AccountId>;

        /// Origin for `transact` call.
        type XcmTransactOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;
    }

    #[pallet::origin]
    pub type Origin<T> = RawOrigin<<T as frame_system::Config>::AccountId>;

    /// Global nonce for all transactions to avoid hash collision, which is
    /// caused by the same dummy signatures for all transactions.
    #[pallet::storage]
    pub type Nonce<T: Config> = StorageValue<_, U256, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Transact an Ethereum transaction. Similar to `pallet_ethereum::Transact`,
        /// but is only for XCM remote call.
        #[pallet::call_index(0)]
        #[pallet::weight({
            let weight_limit = T::GasWeightMapping::gas_to_weight(tx.gas_limit.unique_saturated_into(), false);
            // `Nonce` storage read 1, write 1.
            weight_limit.saturating_add(T::DbWeight::get().reads_writes(1, 1))
        })]
        pub fn transact(origin: OriginFor<T>, tx: CheckedEthereumTx) -> DispatchResultWithPostInfo {
            let source = T::XcmTransactOrigin::ensure_origin(origin)?;
            Self::do_transact(
                T::AccountMapping::into_h160(source),
                tx.into(),
                CheckedEthereumTxKind::Xcm,
            )
            .map(|(post_info, _)| post_info)
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Validate and execute the checked tx. Only `Call` transaction action is allowed.
    fn do_transact(
        source: H160,
        checked_tx: CheckedEthereumTx,
        tx_kind: CheckedEthereumTxKind,
    ) -> Result<(PostDispatchInfo, CallInfo), DispatchErrorWithPostInfo> {
        let chain_id = T::ChainId::get();
        let nonce = Nonce::<T>::get();
        let tx = checked_tx.into_ethereum_tx(Nonce::<T>::get(), chain_id);
        let tx_data: TransactionData = (&tx).into();

        // Validate the tx.
        let _ = CheckEvmTransaction::<T::InvalidEvmTransactionError>::new(
            CheckEvmTransactionConfig {
                evm_config: T::config(),
                block_gas_limit: U256::from(Self::block_gas_limit(&tx_kind)),
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
                // `Nonce` storage read 1.
                actual_weight: Some(T::DbWeight::get().reads(1)),
                pays_fee: Pays::Yes,
            },
            error: DispatchError::Other("Failed to validate Ethereum tx"),
        })?;

        Nonce::<T>::put(nonce.saturating_add(U256::one()));

        // Execute the tx.
        let (post_info, apply_info) = T::ValidatedTransaction::apply(source, tx)?;
        match apply_info {
            CallOrCreateInfo::Call(info) => Ok((post_info, info)),
            // It is not possible to have a `Create` transaction via `CheckedEthereumTx`.
            CallOrCreateInfo::Create(_) => {
                unreachable!("Cannot create a 'Create' transaction; qed")
            }
        }
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
    fn xvm_transact(
        source: H160,
        checked_tx: CheckedEthereumTx,
    ) -> Result<(PostDispatchInfo, CallInfo), DispatchErrorWithPostInfo> {
        Self::do_transact(source, checked_tx, CheckedEthereumTxKind::Xvm)
    }
}
