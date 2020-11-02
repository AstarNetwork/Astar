// Copyright 2019-2020 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! # Utility Module
//! A module with helpers for dispatch management.
//!
//! - [`utility::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//!
//! ## Overview
//!
//! This module contains three basic pieces of functionality, two of which are stateless:
//! - Batch dispatch: A stateless operation, allowing any origin to execute multiple calls in a
//!   single dispatch. This can be useful to amalgamate proposals, combining `set_code` with
//!   corresponding `set_storage`s, for efficient multiple payouts with just a single signature
//!   verify, or in combination with one of the other two dispatch functionality.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! #### For batch dispatch
//! * `batch` - Dispatch multiple calls from the sender's origin.
//!
//! [`Call`]: ./enum.Call.html
//! [`Trait`]: ./trait.Trait.html

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

mod tests;

use frame_support::{decl_event, decl_module, decl_storage, Parameter};
use frame_support::{
    dispatch::PostDispatchInfo,
    weights::{DispatchClass, FunctionOf, GetDispatchInfo},
};
use frame_system as system;
use sp_runtime::{traits::Dispatchable, DispatchError};
use sp_std::prelude::*;

/// Configuration trait.
pub trait Trait: frame_system::Trait {
    /// The overarching event type.
    type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;

    /// The overarching call type.
    type Call: Parameter
        + Dispatchable<Origin = Self::Origin, PostInfo = PostDispatchInfo>
        + GetDispatchInfo
        + From<frame_system::Call<Self>>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Utility {
    }
}

decl_event! {
    /// Events type.
    pub enum Event {
        /// Batch of dispatches did not complete fully. Index of first failing dispatch given, as
        /// well as the error.
        BatchInterrupted(u32, DispatchError),
        /// Batch of dispatches completed fully with no error.
        BatchCompleted,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Deposit one of this module's events by using the default implementation.
        fn deposit_event() = default;

        /// Send a batch of dispatch calls.
        ///
        /// This will execute until the first one fails and then stop.
        ///
        /// May be called from any origin.
        ///
        /// - `calls`: The calls to be dispatched from the same origin.
        ///
        /// # <weight>
        /// - The sum of the weights of the `calls`.
        /// - One event.
        /// # </weight>
        ///
        /// This will return `Ok` in all circumstances. To determine the success of the batch, an
        /// event is deposited. If a call failed and the batch was interrupted, then the
        /// `BatchInterrupted` event is deposited, along with the number of successful calls made
        /// and the error of the failed call. If all were successful, then the `BatchCompleted`
        /// event is deposited.
        #[weight = FunctionOf(
            |args: (&Vec<<T as Trait>::Call>,)| {
                args.0.iter()
                    .map(|call| call.get_dispatch_info().weight)
                    .fold(10_000, |a, n| a + n)
            },
            |args: (&Vec<<T as Trait>::Call>,)| {
                let all_operational = args.0.iter()
                    .map(|call| call.get_dispatch_info().class)
                    .all(|class| class == DispatchClass::Operational);
                if all_operational {
                    DispatchClass::Operational
                } else {
                    DispatchClass::Normal
                }
            },
            true
        )]
        fn batch(origin, calls: Vec<<T as Trait>::Call>) {
            for (index, call) in calls.into_iter().enumerate() {
                let result = call.dispatch(origin.clone());
                if let Err(e) = result {
                    Self::deposit_event(Event::BatchInterrupted(index as u32, e.error));
                    return Ok(());
                }
            }
            Self::deposit_event(Event::BatchCompleted);
        }
    }
}
