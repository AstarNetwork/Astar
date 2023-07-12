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

//! # XVM pallet
//!
//! ## Overview
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//!
//! ### Other
//!
//!

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::weights::Weight;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::{traits::Member, RuntimeDebug};
use sp_std::prelude::*;

pub mod pallet;
pub use pallet::pallet::*;

/// EVM call adapter.
#[cfg(feature = "evm")]
pub mod evm;

/// Wasm call adapter.
#[cfg(feature = "wasm")]
pub mod wasm;

/// Unique VM identifier.
pub type VmId = u8;

// TODO: remove later after solution is properly benchmarked
// Just a arbitrary weight constant to avoid having ZERO weight in some parts of execution
pub const PLACEHOLDER_WEIGHT: Weight = Weight::from_parts(1_000_000, 0);

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub enum XvmError {
    VmNotRecognized,
    EncodingFailure,
    ContextConversionFailed,
    OutOfGas,
    ExecutionError(Vec<u8>),
    InputTooLarge,
    // extend this list as part of improved error handling
}

// TODO: Currently our precompile/chain-extension calls rely on direct `Call` usage of XVM pallet.
// This is perfectly fine when we're just calling a function in other VM and are interested whether the call was
// successful or not.
//
// Problem arises IF we want to get back arbitrary read value from the other VM - `DispatchResultWithPostInfo` isn't enough for this.
// We need to receive back a concrete value back from the other VM.

/// Denotes a successful XVM call execution
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub struct XvmCallOk {
    /// Output of XVM call. E.g. if call was a query, this will contain query response.
    output: Vec<u8>,
    /// Total consumed weight. This is in context of Substrate (1 unit of weight ~ 1 ps of execution time)
    consumed_weight: Weight,
}

impl XvmCallOk {
    pub fn output(&self) -> &[u8] {
        &self.output
    }
}

/// Denotes an successful XVM call execution
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub struct XvmCallError {
    /// Result of XVM call
    // TODO: use XvmError enum from pallet? Perhaps that's a better approach. Or at least provide mapping?
    error: XvmError,
    /// Total consumed weight. This is in context of Substrate (1 unit of weight ~ 1 ps of execution time)
    consumed_weight: Weight,
}

impl XvmCallError {
    pub fn error(&self) -> &XvmError {
        &self.error
    }
}

/// Result for executing X-VM calls
pub type XvmResult = Result<XvmCallOk, XvmCallError>;

pub fn consumed_weight(result: &XvmResult) -> Weight {
    match result {
        Ok(res) => res.consumed_weight,
        Err(err) => err.consumed_weight,
    }
}

/// XVM context consist of unique ID and optional execution arguments.
#[derive(Default, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub struct XvmContext {
    /// Identifier (should be unique for each VM in tuple).
    pub id: VmId,
    /// Max allowed weight for the call
    pub max_weight: Weight,
    /// Encoded VM execution environment.
    pub env: Option<Vec<u8>>,
}

/// The engine that support synchronous smart contract execution.
/// For example, EVM.
pub trait SyncVM<AccountId> {
    /// Unique VM identifier.
    fn id() -> VmId;

    /// Make a call to VM contract and return result or error.
    ///
    ///
    fn xvm_call(context: XvmContext, from: AccountId, to: Vec<u8>, input: Vec<u8>) -> XvmResult;
}

#[impl_trait_for_tuples::impl_for_tuples(30)]
impl<AccountId: Member> SyncVM<AccountId> for Tuple {
    fn id() -> VmId {
        Default::default()
    }

    fn xvm_call(context: XvmContext, from: AccountId, to: Vec<u8>, input: Vec<u8>) -> XvmResult {
        for_tuples!( #(
            if Tuple::id() == context.id {
                log::trace!(
                    target: "xvm::SyncVm::xvm_call",
                    "VM found, run XVM call: {:?}, {:?}, {:?}, {:?}",
                    context, from, to, input,
                );
                return Tuple::xvm_call(context, from, to, input)
            }
        )* );
        log::trace!(
            target: "xvm::SyncVm::xvm_call",
            "VM with ID {:?} not found", context.id
        );
        Err(XvmCallError {
            error: XvmError::VmNotRecognized,
            consumed_weight: PLACEHOLDER_WEIGHT,
        })
    }
}

/// The engine that support asynchronous smart contract execution.
/// For example, XCVM.
pub trait AsyncVM<AccountId> {
    /// Unique VM identifier.
    fn id() -> VmId;

    /// Send a message.
    fn xvm_send(context: XvmContext, from: AccountId, to: Vec<u8>, message: Vec<u8>) -> XvmResult;

    /// Query for incoming messages.
    fn xvm_query(context: XvmContext, inbox: AccountId) -> XvmResult;
}

#[impl_trait_for_tuples::impl_for_tuples(30)]
impl<AccountId: Member> AsyncVM<AccountId> for Tuple {
    fn id() -> VmId {
        Default::default()
    }

    fn xvm_send(context: XvmContext, from: AccountId, to: Vec<u8>, message: Vec<u8>) -> XvmResult {
        for_tuples!( #(
            if Tuple::id() == context.id {
                log::trace!(
                    target: "xvm::AsyncVM::xvm_send",
                    "VM found, send message: {:?}, {:?}, {:?}, {:?}",
                    context, from, to, message,
                );
                return Tuple::xvm_send(context, from, to, message)
            }
        )* );
        log::trace!(
            target: "xvm::AsyncVM::xvm_send",
            "VM with ID {:?} not found", context.id
        );

        Err(XvmCallError {
            error: XvmError::VmNotRecognized,
            consumed_weight: PLACEHOLDER_WEIGHT,
        })
    }

    fn xvm_query(context: XvmContext, inbox: AccountId) -> XvmResult {
        for_tuples!( #(
            if Tuple::id() == context.id {
                log::trace!(
                    target: "xvm::AsyncVM::xvm_query",
                    "VM found, query messages: {:?} {:?}",
                    context, inbox,
                );
                return Tuple::xvm_query(context, inbox)
            }
        )* );
        log::trace!(
            target: "xvm::AsyncVM::xvm_query",
            "VM with ID {:?} not found", context.id
        );

        Err(XvmCallError {
            error: XvmError::VmNotRecognized,
            consumed_weight: PLACEHOLDER_WEIGHT,
        })
    }
}
