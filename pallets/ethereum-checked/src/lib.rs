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
//! A `pallet-ethereum like pallet that execute transactions from checked source,
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
    CallInfo, CallOrCreateInfo, CheckEvmTransaction, CheckEvmTransactionConfig, ExitReason,
    ExitSucceed, InvalidEvmTransactionError,
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

use astar_primitives::{
    ethereum_checked::{CheckedEthereumTransact, CheckedEthereumTx},
    evm::UnifiedAddressMapper,
};

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// TODO: after integrated into Astar/Shiden runtime, redo benchmarking with them.
// The reason is that `EVMChainId` storage read only happens in Shibuya
pub mod weights;
pub use weights::WeightInfo;

mod mock;
mod tests;

pub type WeightInfoOf<T> = <T as Config>::WeightInfo;

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
        type AddressMapper: UnifiedAddressMapper<Self::AccountId>;

        /// Origin for `transact` call.
        type XcmTransactOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
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
            weight_limit.saturating_add(WeightInfoOf::<T>::transact_without_apply())
        })]
        pub fn transact(origin: OriginFor<T>, tx: CheckedEthereumTx) -> DispatchResultWithPostInfo {
            let source = T::XcmTransactOrigin::ensure_origin(origin)?;
            Self::do_transact(
                T::AddressMapper::to_h160_or_default(&source).into_address(),
                tx.into(),
                CheckedEthereumTxKind::Xcm,
                false,
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
        skip_apply: bool,
    ) -> Result<(PostDispatchInfo, CallInfo), DispatchErrorWithPostInfo> {
        let chain_id = T::ChainId::get();
        let nonce = Nonce::<T>::get();
        let tx = checked_tx.into_ethereum_tx(Nonce::<T>::get(), chain_id);
        let tx_data: TransactionData = (&tx).into();

        let (weight_limit, proof_size_base_cost) =
            match <T as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
                tx_data.gas_limit.unique_saturated_into(),
                true,
            ) {
                weight_limit if weight_limit.proof_size() > 0 => (
                    Some(weight_limit),
                    // measured PoV should be correct to use here
                    Some(WeightInfoOf::<T>::transact_without_apply().proof_size()),
                ),
                _ => (None, None),
            };

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
            weight_limit,
            proof_size_base_cost,
        )
        // Gas limit validation. The fee payment has been validated as the tx is `checked`.
        .validate_common()
        .map_err(|_| DispatchErrorWithPostInfo {
            post_info: PostDispatchInfo {
                // actual_weight = overhead - nonce_write_1
                actual_weight: Some(
                    WeightInfoOf::<T>::transact_without_apply()
                        .saturating_sub(T::DbWeight::get().writes(1)),
                ),
                pays_fee: Pays::Yes,
            },
            error: DispatchError::Other("Failed to validate Ethereum tx"),
        })?;

        Nonce::<T>::put(nonce.saturating_add(U256::one()));

        if skip_apply {
            return Ok((
                PostDispatchInfo {
                    actual_weight: Some(WeightInfoOf::<T>::transact_without_apply()),
                    pays_fee: Pays::Yes,
                },
                CallInfo {
                    exit_reason: ExitReason::Succeed(ExitSucceed::Returned),
                    value: Default::default(),
                    used_gas: fp_evm::UsedGas {
                        standard: checked_tx.gas_limit,
                        effective: checked_tx.gas_limit,
                    },
                    weight_info: None,
                    logs: Default::default(),
                },
            ));
        }

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

    /// Similar to `transact` dispatch-able call that transacts an Ethereum transaction,
    /// but not to apply it. This is to benchmark the weight overhead in addition to `gas_limit`.
    #[cfg(feature = "runtime-benchmarks")]
    pub fn transact_without_apply(
        origin: OriginFor<T>,
        tx: CheckedEthereumTx,
    ) -> DispatchResultWithPostInfo {
        let source = T::XcmTransactOrigin::ensure_origin(origin)?;
        Self::do_transact(
            T::AddressMapper::to_h160_or_default(&source).into_address(),
            tx.into(),
            CheckedEthereumTxKind::Xcm,
            true,
        )
        .map(|(post_info, _)| post_info)
    }
}

impl<T: Config> CheckedEthereumTransact for Pallet<T> {
    fn xvm_transact(
        source: H160,
        checked_tx: CheckedEthereumTx,
    ) -> Result<(PostDispatchInfo, CallInfo), DispatchErrorWithPostInfo> {
        Self::do_transact(source, checked_tx, CheckedEthereumTxKind::Xvm, false)
    }
}
