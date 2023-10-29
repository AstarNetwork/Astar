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

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;

/// Assert that the last event equals the provided one.
fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
    frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn claim_evm_address() {
        let caller: T::AccountId = whitelisted_caller();
        let eth_secret_key = libsecp256k1::SecretKey::parse(&keccak_256(b"Alice")).unwrap();
        let evm_address = Pallet::<T>::eth_address(&eth_secret_key);
        let signature = Pallet::<T>::eth_sign_prehash(
            &Pallet::<T>::build_signing_payload(&caller),
            &eth_secret_key,
        )
        .into();

        assert_ok!(T::Currency::mint_into(
            &caller,
            T::AccountMappingStorageFee::get()
        ));
        let caller_clone = caller.clone();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), evm_address, signature);

        assert_last_event::<T>(
            Event::<T>::AccountClaimed {
                account_id: caller_clone,
                evm_address,
            }
            .into(),
        );
    }

    #[benchmark]
    fn claim_default_evm_address() {
        let caller: T::AccountId = whitelisted_caller();
        let caller_clone = caller.clone();
        let evm_address = T::DefaultNativeToEvm::into_h160(caller.clone());

        assert_ok!(T::Currency::mint_into(
            &caller,
            T::AccountMappingStorageFee::get()
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller));

        assert_last_event::<T>(
            Event::<T>::AccountClaimed {
                account_id: caller_clone,
                evm_address,
            }
            .into(),
        );
    }

    #[benchmark]
    fn to_account_id() {
        let caller: T::AccountId = whitelisted_caller();
        let evm_address = T::DefaultNativeToEvm::into_h160(caller.clone());
        assert_ok!(T::Currency::mint_into(
            &caller,
            T::AccountMappingStorageFee::get()
        ));
        // claim mapping
        assert_ok!(Pallet::<T>::claim_default_evm_address(
            RawOrigin::Signed(caller.clone()).into()
        ));

        #[block]
        {
            let _ = <Pallet<T> as UnifiedAddressMapper<T::AccountId>>::to_account_id(&evm_address);
        }
    }

    #[benchmark]
    fn to_account_id_or_default() {
        let caller: T::AccountId = whitelisted_caller();
        let evm_address = T::DefaultNativeToEvm::into_h160(caller.clone());
        assert_ok!(T::Currency::mint_into(
            &caller,
            T::AccountMappingStorageFee::get()
        ));
        // claim mapping
        assert_ok!(Pallet::<T>::claim_default_evm_address(
            RawOrigin::Signed(caller.clone()).into()
        ));

        #[block]
        {
            let _ = <Pallet<T> as UnifiedAddressMapper<T::AccountId>>::to_account_id_or_default(
                &evm_address,
            );
        }
    }

    #[benchmark]
    fn to_h160() {
        let caller: T::AccountId = whitelisted_caller();
        assert_ok!(T::Currency::mint_into(
            &caller,
            T::AccountMappingStorageFee::get()
        ));
        // claim mapping
        assert_ok!(Pallet::<T>::claim_default_evm_address(
            RawOrigin::Signed(caller.clone()).into()
        ));

        #[block]
        {
            let _ = <Pallet<T> as UnifiedAddressMapper<T::AccountId>>::to_h160(&caller);
        }
    }

    #[benchmark]
    fn to_h160_or_default() {
        let caller: T::AccountId = whitelisted_caller();
        assert_ok!(T::Currency::mint_into(
            &caller,
            T::AccountMappingStorageFee::get()
        ));
        // claim mapping
        assert_ok!(Pallet::<T>::claim_default_evm_address(
            RawOrigin::Signed(caller.clone()).into()
        ));

        #[block]
        {
            let _ = <Pallet<T> as UnifiedAddressMapper<T::AccountId>>::to_h160_or_default(&caller);
        }
    }
}
