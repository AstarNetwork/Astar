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
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//!
//! ### Other
//!
//!

#[frame_support::pallet]
#[allow(clippy::module_inception)]
pub mod pallet {
    use crate::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Supported synchronous VM list, for example (EVM, WASM)
        type SyncVM: SyncVM<Self::AccountId>;
        /// Supported asynchronous VM list.
        type AsyncVM: AsyncVM<Self::AccountId>;
        /// General event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        XvmCall { result: Result<Vec<u8>, XvmError> },
        XvmSend { result: Result<Vec<u8>, XvmError> },
        XvmQuery { result: Result<Vec<u8>, XvmError> },
    }

    impl<T: Config> Pallet<T> {
        /// Internal interface for cross-pallet invocation.
        /// Essentially does the same thing as `xvm_call`, but a bit differently:
        ///   - It does not verify origin
        ///   - It does not use `Dispatchable` API (cannot be called from tx)
        ///   - It does not deposit event upon completion
        ///   - It returns `XvmResult` letting the caller get return data directly
        pub fn xvm_bare_call(
            context: XvmContext,
            from: T::AccountId,
            to: Vec<u8>,
            input: Vec<u8>,
        ) -> XvmResult {
            let result = T::SyncVM::xvm_call(context, from, to, input);

            log::trace!(
                target: "xvm::pallet::xvm_bare_call",
                "Execution result: {:?}", result
            );

            result
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(context.max_weight)]
        pub fn xvm_call(
            origin: OriginFor<T>,
            context: XvmContext,
            to: Vec<u8>,
            input: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;

            // Executing XVM call logic itself will consume some weight so that should be subtracted from the max allowed weight of XCM call
            // TODO: fix
            //context.max_weight = context.max_weight - PLACEHOLDER_WEIGHT;

            let result = T::SyncVM::xvm_call(context, from, to, input);
            let consumed_weight = consumed_weight(&result);

            log::trace!(
                target: "xvm::pallet::xvm_call",
                "Execution result: {:?}, consumed_weight: {:?}", result, consumed_weight,
            );

            Self::deposit_event(Event::<T>::XvmCall {
                result: match result {
                    Ok(result) => Ok(result.output),
                    Err(result) => Err(result.error),
                },
            });

            Ok(Some(consumed_weight).into())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(context.max_weight)]
        pub fn xvm_send(
            origin: OriginFor<T>,
            context: XvmContext,
            to: Vec<u8>,
            message: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;
            let result = T::AsyncVM::xvm_send(context, from, to, message);

            Self::deposit_event(Event::<T>::XvmSend {
                result: match result {
                    Ok(result) => Ok(result.output),
                    Err(result) => Err(result.error),
                },
            });

            Ok(().into())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(context.max_weight)]
        pub fn xvm_query(origin: OriginFor<T>, context: XvmContext) -> DispatchResultWithPostInfo {
            let inbox = ensure_signed(origin)?;
            let result = T::AsyncVM::xvm_query(context, inbox);

            Self::deposit_event(Event::<T>::XvmQuery {
                result: match result {
                    Ok(result) => Ok(result.output),
                    Err(result) => Err(result.error),
                },
            });

            Ok(().into())
        }
    }
}
