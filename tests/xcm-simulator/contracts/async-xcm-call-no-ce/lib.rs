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

#![cfg_attr(not(feature = "std"), no_std, no_main)]

use async_xcm_call_no_ce::*;
use ink::codegen::Env;
use ink::prelude::{boxed::Box, vec, vec::Vec};
use scale::{Decode, Encode};
use xcm::{prelude::*, v3::Weight};

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
        pub result: Option<bool>,
        pub here_para_id: u32,
    }

    #[derive(Encode, Decode, Debug, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct WeightsAndFees {
        pub foreign_base_fee: MultiAsset,
        pub foreign_transact_weight: Weight,
        pub foreign_transcat_pallet_xcm: Weight,
        pub here_callback_base_fee: MultiAsset,
        pub here_callback_transact_weight: Weight,
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

        #[ink(message, selector = 0x0000BBBB)]
        pub fn handle_response(&mut self, success: bool) {
            ink::env::debug_println!("[1/1] Inside handle_response...");

            self.result = Some(success);
        }

        #[ink(message, selector = 0x0000CCCC)]
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
                    data: [[0x00, 0x00, 0xBB, 0xBB].to_vec(), success.encode()].concat(),
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
