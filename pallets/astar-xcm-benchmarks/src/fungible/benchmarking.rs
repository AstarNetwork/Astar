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

use super::{Pallet as AstarBenchmarks, *};
use crate::WrappedBenchmark;
use frame_benchmarking::v2::*;
use frame_support::{
    dispatch::Weight,
    traits::{fungible::Inspect, Get},
};
use pallet_xcm_benchmarks::{
    account_and_location, fungible::Pallet as PalletXcmBenchmarks, new_executor, AssetTransactorOf,
};
use sp_std::vec;
use xcm::latest::prelude::*;
use xcm_executor::traits::{Convert, TransactAsset};

#[benchmarks(
    where
		<
			<
				T::TransactAsset
				as
				Inspect<T::AccountId>
			>::Balance
			as
			TryInto<u128>
        >::Error: sp_std::fmt::Debug,
)]
mod benchmarks {
    use super::*;

    /// Re-write for fungibles assets (like pallet_assets's assets) as
    /// upstream benchmark does not take ED (assets's min_balance) into consideration
    #[benchmark]
    fn transfer_asset() -> Result<(), BenchmarkError> {
        let (sender_account, sender_location) = account_and_location::<T>(1);
        let asset_to_deposit = T::get_multi_asset();
        // take out ED from given asset
        let (asset_to_send, min_balance) =
            take_minimum_balance::<T>(asset_to_deposit.clone()).unwrap();
        let assets: MultiAssets = vec![asset_to_send.clone()].into();
        // this xcm doesn't use holding

        let dest_location = T::valid_destination()?;
        let dest_account = T::AccountIdConverter::convert(dest_location.clone()).unwrap();

        <AssetTransactorOf<T>>::deposit_asset(
            &asset_to_deposit,
            &sender_location,
            &XcmContext {
                origin: Some(sender_location.clone()),
                message_hash: [0; 32],
                topic: None,
            },
        )
        .unwrap();

        let mut executor = new_executor::<T>(sender_location);
        let instruction = Instruction::TransferAsset {
            assets,
            beneficiary: dest_location,
        };
        let xcm = Xcm(vec![instruction]);

        #[block]
        {
            executor.bench_process(xcm)?;
        }

        assert_eq!(T::TransactAsset::balance(&sender_account), min_balance);
        assert!(!T::TransactAsset::balance(&dest_account).is_zero());
        Ok(())
    }

    /// Re-write for fungibles assets (like pallet_assets's assets) as
    /// upstream benchmark does not take ED (assets's min_balance) into consideration
    #[benchmark]
    fn transfer_reserve_asset() -> Result<(), BenchmarkError> {
        let (sender_account, sender_location) = account_and_location::<T>(1);
        let dest_location = T::valid_destination()?;
        let dest_account = T::AccountIdConverter::convert(dest_location.clone()).unwrap();

        let asset_to_deposit = T::get_multi_asset();
        // take out ED from given asset
        let (asset_to_send, min_balance) =
            take_minimum_balance::<T>(asset_to_deposit.clone()).unwrap();
        let assets: MultiAssets = vec![asset_to_send].into();

        <AssetTransactorOf<T>>::deposit_asset(
            &asset_to_deposit,
            &sender_location,
            &XcmContext {
                origin: Some(sender_location.clone()),
                message_hash: [0; 32],
                topic: None,
            },
        )
        .unwrap();
        assert!(T::TransactAsset::balance(&dest_account).is_zero());

        let mut executor = new_executor::<T>(sender_location);
        let instruction = Instruction::TransferReserveAsset {
            assets,
            dest: dest_location,
            xcm: Xcm::new(),
        };
        let xcm = Xcm(vec![instruction]);

        #[block]
        {
            executor.bench_process(xcm)?;
        }

        assert_eq!(T::TransactAsset::balance(&sender_account), min_balance);
        assert!(!T::TransactAsset::balance(&dest_account).is_zero());
        Ok(())
    }

    /// The benchmarks for `reserve_asset_deposited` was added in later versions of
    /// `pallet-xcm-benchmarks` (in v1.x.x versions).
    /// TODO: remove this once we uplift to new polkadot release
    #[benchmark]
    fn reserve_asset_deposited() -> Result<(), BenchmarkError> {
        let (trusted_reserve, transferable_reserve_asset) = T::TrustedReserve::get().ok_or(
            BenchmarkError::Override(BenchmarkResult::from_weight(Weight::MAX)),
        )?;

        let assets: MultiAssets = vec![transferable_reserve_asset].into();

        let mut executor = new_executor::<T>(trusted_reserve);
        let instruction = Instruction::ReserveAssetDeposited(assets.clone());
        let xcm = Xcm(vec![instruction]);

        #[block]
        {
            executor.bench_process(xcm)?;
        }

        assert!(executor.holding().ensure_contains(&assets).is_ok());
        Ok(())
    }

    #[benchmark]
    fn receive_teleported_asset() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    #[benchmark]
    fn initiate_teleport() -> Result<(), BenchmarkError> {
        #[block]
        {}
        Err(BenchmarkError::Override(BenchmarkResult::from_weight(
            Weight::MAX,
        )))
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}

// wrapper benchmarks
pub type XcmFungibleBenchmarks<T> = WrappedBenchmark<AstarBenchmarks<T>, PalletXcmBenchmarks<T>>;

/// Take out the ED from given MultiAsset (if fungible)
fn take_minimum_balance<T: Config>(
    mut asset: MultiAsset,
) -> Result<
    (
        MultiAsset,
        <T::TransactAsset as Inspect<T::AccountId>>::Balance,
    ),
    (),
>
where
    <<T::TransactAsset as Inspect<T::AccountId>>::Balance as TryInto<u128>>::Error:
        sp_std::fmt::Debug,
{
    let minimum_balance = T::TransactAsset::minimum_balance();

    if let Fungible(fun) = asset.fun {
        asset.fun = Fungible(fun.saturating_sub(minimum_balance.try_into().map_err(|_| ())?));
    }

    Ok((asset, minimum_balance))
}
