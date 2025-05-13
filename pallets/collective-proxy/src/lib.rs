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

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    dispatch::GetDispatchInfo,
    pallet_prelude::*,
    traits::{InstanceFilter, IsType, OriginTrait},
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::Dispatchable;
use sp_std::prelude::*;

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // TODO: The pallet is intentionally very basic. It could be improved to handle more origins, more aliases, etc.
    // There could also be different instances, if such approach was needed.
    // However, it's supposed to be the simplest solution possible to cover a specific scenario.
    // Pallet is stateless and can easily be upgraded in the future.

    /// Configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The overarching call type.
        type RuntimeCall: Parameter
            + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
            + GetDispatchInfo
            + From<frame_system::Call<Self>>
            + IsType<<Self as frame_system::Config>::RuntimeCall>;

        /// Origin that can act on behalf of the collective.
        type CollectiveProxy: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

        /// Account representing the collective treasury.
        type ProxyAccountId: Get<Self::AccountId>;

        /// Filter to determine whether a call can be executed or not.
        type CallFilter: InstanceFilter<<Self as Config>::RuntimeCall> + Default;

        /// Weight info
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Community proxy call executed successfully.
        CollectiveProxyExecuted { result: DispatchResult },
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Executes the call on a behalf of an aliased account.
        ///
        /// The `origin` of the call is supposed to be a _collective_ (but can be anything) which can dispatch `call` on behalf of the aliased account.
        /// It's essentially a proxy call that can be made by arbitrary origin type.
        #[pallet::call_index(0)]
        #[pallet::weight({
			let di = call.get_dispatch_info();
			(T::WeightInfo::execute_call().saturating_add(di.total_weight()), di.class)
		})]
        pub fn execute_call(
            origin: OriginFor<T>,
            call: Box<<T as Config>::RuntimeCall>,
        ) -> DispatchResult {
            // Ensure origin is valid.
            T::CollectiveProxy::ensure_origin(origin)?;

            // Account authentication is ensured by the `CollectiveProxy` origin check.
            let mut origin: T::RuntimeOrigin =
                frame_system::RawOrigin::Signed(T::ProxyAccountId::get()).into();

            // Ensure custom filter is applied.
            origin.add_filter(move |c: &<T as frame_system::Config>::RuntimeCall| {
                let c = <T as Config>::RuntimeCall::from_ref(c);
                T::CallFilter::default().filter(c)
            });

            // Dispatch the call.
            let e = call.dispatch(origin);
            Self::deposit_event(Event::CollectiveProxyExecuted {
                result: e.map(|_| ()).map_err(|e| e.error),
            });

            Ok(())
        }
    }
}
