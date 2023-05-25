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

//! Async XCM Call Contracrt
//! This PoC contract showcase the scenario of sending a XCM to another parachain
//! and get the results back of XCM execution, aka async operation.
//! - Contract will send below XCM to foreign parachain using `call_runtime` to perform
//!   some operation (`Transact(Remark)`).
//! - If Error, a `Transact` instr will be used to send an XCM back to origin chain which
//!   will call `handle_response(false)` contract method.
//! - If Success, same as above but call `handle_response(true)` method.
//!
//! ```no_run
//! Xcm(vec![
//!     WithdrawAsset(..)
//!     BuyExecution(..)
//!     SetAppendix(Xcm(vec![
//!         Transcat { /* pallet_xcm::send() runtime call to call SUCCESS contract method in ParaA */ }
//!     ]))
//!     SetErrorHandler(Xcm(vec![
//!         Transcat { /* pallet_xcm::send() runtime call to call ERROR contract method in ParaA */ }
//!     ]))
//!     ...
//! ])
//! ```
//!
//! # Methods
//! - attempt_remark_via_xcm: This method when called will build the XCM call that
//!   will be sent to given parachain. The XCM call will include necessary handler
//!   to send the results back via calling the `handle_response` contract method
//! - handle_response: This method will be called with operation result initiated by
//!   XCM by foreign parachain.
//! - result: This is a getter method to get current stored result.

#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::large_enum_variant)]

use async_xcm_call_no_ce::*;
use ink::codegen::Env;
use ink::prelude::{boxed::Box, vec, vec::Vec};
use scale::{Decode, Encode};
use xcm::{prelude::*, v3::Weight};

/// Foreign parachain types
mod foreign {
    use super::*;

    #[derive(scale::Encode)]
    pub enum RuntimeCall {
        #[codec(index = 0)]
        System(SystemCall),
        #[codec(index = 3)]
        PolkadotXcm(PolkadotXcmCall),
    }

    #[derive(scale::Encode)]
    pub enum SystemCall {
        #[codec(index = 7)]
        RemarkWithEvent { remark: Vec<u8> },
    }

    #[derive(scale::Encode)]
    pub enum PolkadotXcmCall {
        #[codec(index = 0)]
        Send {
            dest: Box<VersionedMultiLocation>,
            message: Box<VersionedXcm<()>>,
        },
    }
}

/// parachain types
mod here {
    use super::*;
    use ink::primitives::AccountId;

    #[derive(scale::Encode)]
    pub enum RuntimeCall {
        #[codec(index = 3)]
        PolkadotXcm(PolkadotXcmCall),
        #[codec(index = 12)]
        Contracts(ContractsCall),
    }

    #[derive(scale::Encode)]
    pub enum PolkadotXcmCall {
        #[codec(index = 0)]
        Send {
            dest: Box<VersionedMultiLocation>,
            message: Box<VersionedXcm<()>>,
        },
    }

    #[derive(scale::Encode)]
    pub enum ContractsCall {
        #[codec(index = 6u8)]
        Call {
            dest: AccountId,
            #[codec(compact)]
            value: u128,
            gas_limit: Weight,
            storage_deposit_limit: Option<<u128 as scale::HasCompact>::Type>,
            data: Vec<u8>,
        },
    }
}

#[ink::contract]
mod async_xcm_call_no_ce {
    use super::*;

    #[ink(storage)]
    #[derive(Default)]
    pub struct AsyncCall {
        /// store the result of async XCM operation
        pub result: Option<bool>,
        /// Parachain's Id on which contract is deployed
        pub here_para_id: u32,
    }

    /// All the fees and weights values required for the whole
    /// operation.
    #[derive(Encode, Decode, Debug, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct WeightsAndFees {
        /// Max fee for whole XCM operation in foreign chain
        /// This includes fees for sending XCM back to original
        /// chain via Transact(pallet_xcm::send).
        pub foreign_base_fee: MultiAsset,
        /// Max weight for operation (remark)
        pub foreign_transact_weight: Weight,
        /// Max weight for Transact(pallet_xcm::send) operation
        pub foreign_transcat_pallet_xcm: Weight,
        /// Max fee for the callback operation
        /// send by foreign chain
        pub here_callback_base_fee: MultiAsset,
        /// Max weight for Transact(pallet_contracts::call)
        pub here_callback_transact_weight: Weight,
        /// Max weight for contract call
        pub here_callback_contract_weight: Weight,
    }

    impl AsyncCall {
        #[ink(constructor, selector = 0x00001111)]
        pub fn new(here_para_id: u32) -> Self {
            Self {
                result: None,
                here_para_id,
            }
        }

        /// Attempt to perform remark operation on given parachain by
        /// sending a XCM using `call_runtime`.
        #[ink(message, selector = 0x00002222)]
        pub fn attempt_remark_via_xcm(
            &mut self,
            parachain_id: u32,
            remark: Vec<u8>,
            weight_and_fees: WeightsAndFees,
        ) -> bool {
            ink::env::debug_println!("[1/2] Start of attempt_remark_via_xcm");

            let dest: Box<VersionedMultiLocation> =
                Box::new((Parent, Parachain(parachain_id)).into());
            let message: Box<VersionedXcm<()>> =
                Box::new(VersionedXcm::V3(self.build_xcm(remark, &weight_and_fees)));

            ink::env::debug_println!("[2/2] XCM Build successfully, sending...");

            self.env()
                .call_runtime(&here::RuntimeCall::PolkadotXcm(
                    here::PolkadotXcmCall::Send { dest, message },
                ))
                .is_ok()
        }

        #[ink(message, selector = 0x00003333)]
        pub fn handle_response(&mut self, success: bool) {
            ink::env::debug_println!("[1/1] Inside handle_response...");

            self.result = Some(success);
        }

        #[ink(message, selector = 0x00004444)]
        pub fn result(&self) -> Option<bool> {
            self.result
        }
    }
}

impl AsyncCall {
    fn build_callback_sequence(&self, success: bool, weight_and_fees: &WeightsAndFees) -> Xcm<()> {
        let callback_xcm = Xcm(vec![
            // buy execution
            WithdrawAsset(weight_and_fees.here_callback_base_fee.clone().into()),
            BuyExecution {
                fees: weight_and_fees.here_callback_base_fee.clone(),
                weight_limit: Unlimited,
            },
            // transact call to contract method
            Transact {
                origin_kind: OriginKind::SovereignAccount,
                require_weight_at_most: weight_and_fees.here_callback_transact_weight,
                call: here::RuntimeCall::Contracts(here::ContractsCall::Call {
                    dest: self.env().account_id(),
                    value: 0u128,
                    gas_limit: weight_and_fees.here_callback_contract_weight,
                    storage_deposit_limit: None,
                    data: [[0x00, 0x00, 0x33, 0x33].to_vec(), success.encode()].concat(),
                })
                .encode()
                .into(),
            },
            ExpectTransactStatus(MaybeErrorCode::Success),
        ]);

        Xcm(vec![Transact {
            origin_kind: OriginKind::SovereignAccount,
            require_weight_at_most: weight_and_fees.foreign_transcat_pallet_xcm,
            call: foreign::RuntimeCall::PolkadotXcm(foreign::PolkadotXcmCall::Send {
                dest: Box::new((Parent, Parachain(self.here_para_id)).into()),
                message: Box::new(VersionedXcm::V3(callback_xcm)),
            })
            .encode()
            .into(),
        }])
    }

    fn build_xcm(&self, remark: Vec<u8>, weight_and_fees: &WeightsAndFees) -> Xcm<()> {
        Xcm(vec![
            // buy execution
            WithdrawAsset(weight_and_fees.foreign_base_fee.clone().into()),
            BuyExecution {
                fees: weight_and_fees.foreign_base_fee.clone(),
                weight_limit: Unlimited,
            },
            // set on error handler
            SetErrorHandler(self.build_callback_sequence(false, weight_and_fees)),
            // set on success handler
            SetAppendix(self.build_callback_sequence(true, weight_and_fees)),
            // perform operation - remark
            Transact {
                origin_kind: OriginKind::SovereignAccount,
                require_weight_at_most: weight_and_fees.foreign_transact_weight,
                call: foreign::RuntimeCall::System(foreign::SystemCall::RemarkWithEvent { remark })
                    .encode()
                    .into(),
            },
        ])
    }
}
