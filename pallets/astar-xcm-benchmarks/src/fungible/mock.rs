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

//! A mock runtime for XCM benchmarking.

use core::marker::PhantomData;

use crate::{fungible as xcm_fungible_benchmark, mock::*, *};
use astar_primitives::xcm::ReserveAssetFilter;
use frame_benchmarking::BenchmarkError;
use frame_support::{
    assert_ok, parameter_types,
    traits::{fungible::ItemOf, AsEnsureOriginWithArg, ConstU32, Everything, Nothing},
    weights::Weight,
};
use frame_system::{EnsureRoot, EnsureSigned};
use sp_core::{ConstU64, Get, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use xcm::latest::prelude::*;
use xcm_builder::{AllowUnpaidExecutionFrom, FungiblesAdapter, MintLocation, NoChecking};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u64;
type AccountId = u64;
type AssetId = u128;

// For testing the pallet, we construct a mock runtime.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
        XcmBalancesBenchmark: xcm_fungible_benchmark::{Pallet},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, u64::MAX));
    pub UniversalLocation: InteriorMultiLocation = Here;
}
impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type RuntimeCall = RuntimeCall;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 10;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type HoldIdentifier = ();
    type FreezeIdentifier = ();
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
}

impl pallet_assets::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = AssetId;
    type AssetIdParameter = AssetId;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = ConstU64<10>;
    type MetadataDepositBase = ConstU64<10>;
    type MetadataDepositPerByte = ConstU64<1>;
    type AssetAccountDeposit = ConstU64<10>;
    type ApprovalDeposit = ConstU64<10>;
    type StringLimit = ConstU32<50>;
    type Freezer = ();
    type Extra = ();
    type RemoveItemsLimit = ConstU32<100>;
    type CallbackHandle = ();
    type WeightInfo = ();
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

parameter_types! {
    pub const AssetDeposit: u64 = 100 * ExistentialDeposit::get();
    pub const ApprovalDeposit: u64 = 1 * ExistentialDeposit::get();
    pub const StringLimit: u32 = 50;
    pub const MetadataDepositBase: u64 = 10 * ExistentialDeposit::get();
    pub const MetadataDepositPerByte: u64 = 1 * ExistentialDeposit::get();
}

pub struct MatchOnlyAsset<Asset>(PhantomData<Asset>);
impl<Asset: Get<AssetId>> xcm_executor::traits::MatchesFungibles<AssetId, Balance>
    for MatchOnlyAsset<Asset>
{
    fn matches_fungibles(
        a: &MultiAsset,
    ) -> core::result::Result<(AssetId, Balance), xcm_executor::traits::prelude::Error> {
        use sp_runtime::traits::SaturatedConversion;
        match a {
            MultiAsset {
                fun: Fungible(amount),
                ..
            } => Ok((Asset::get(), (*amount).saturated_into::<u64>())),
            _ => Err(xcm_executor::traits::prelude::Error::AssetNotHandled),
        }
    }
}

// Use ONLY assets as the asset transactor.
pub type AssetTransactor = FungiblesAdapter<
    Assets,
    MatchOnlyAsset<TransactAssetId>,
    AccountIdConverter,
    AccountId,
    NoChecking,
    DummyCheckingAccount,
>;

parameter_types! {
    /// Maximum number of instructions in a single XCM fragment. A sanity check against weight
    /// calculations getting too crazy.
    pub const MaxInstructions: u32 = 100;
    pub const MaxAssetsIntoHolding: u32 = 64;
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = DevNull;
    type AssetTransactor = AssetTransactor;
    type OriginConverter = ();
    type IsReserve = ReserveAssetFilter;
    type IsTeleporter = ();
    type UniversalLocation = UniversalLocation;
    type Barrier = AllowUnpaidExecutionFrom<Everything>;
    type Weigher = xcm_builder::FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type Trader = xcm_builder::FixedRateOfFungible<WeightPrice, ()>;
    type ResponseHandler = DevNull;
    type AssetTrap = ();
    type AssetLocker = ();
    type AssetExchanger = ();
    type AssetClaims = ();
    type SubscriptionService = ();
    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = RuntimeCall;
    type SafeCallFilter = Everything;
}

impl pallet_xcm_benchmarks::Config for Test {
    type XcmConfig = XcmConfig;
    type AccountIdConverter = AccountIdConverter;
    fn valid_destination() -> Result<MultiLocation, BenchmarkError> {
        let valid_destination: MultiLocation = X1(AccountId32 {
            network: None,
            id: [0u8; 32],
        })
        .into();

        Ok(valid_destination)
    }
    fn worst_case_holding(_depositable_count: u32) -> MultiAssets {
        crate::mock::mock_worst_case_holding()
    }
}

parameter_types! {
    pub DummyCheckingAccount: AccountId = 0;
    pub const CheckingAccount: Option<(u64, MintLocation)> = Some((100, MintLocation::Local));
    pub const ChildTeleporter: MultiLocation = Parachain(1000).into_location();
    pub const TrustedTeleporter: Option<(MultiLocation, MultiAsset)> = Some((
        ChildTeleporter::get(),
        MultiAsset { id: Concrete(Here.into_location()), fun: Fungible(100) },
    ));
    pub const TeleportConcreteFungible: (MultiAssetFilter, MultiLocation) =
        (Wild(AllOf { fun: WildFungible, id: Concrete(Here.into_location()) }), ChildTeleporter::get());
    pub const ReserveConcreteFungible: (MultiAssetFilter, MultiLocation) =
        (Wild(AllOf { fun: WildFungible, id: Concrete(Here.into_location()) }), ChildTeleporter::get());
    pub const TransactAssetId: u128 = 1;
    pub const TransactAssetLocation: MultiLocation = MultiLocation { parents: 0, interior: X1(GeneralIndex(TransactAssetId::get())) };
    pub const WeightPrice: (xcm::latest::AssetId, u128, u128) = (Concrete(TransactAssetLocation::get()), 1_000_000, 1024);
}

impl pallet_xcm_benchmarks::fungible::Config for Test {
    type TransactAsset = ItemOf<Assets, TransactAssetId, AccountId>;
    type CheckedAccount = CheckingAccount;
    type TrustedTeleporter = TrustedTeleporter;

    fn get_multi_asset() -> MultiAsset {
        let min_balance = 100u64;
        let asset_location: MultiLocation = GeneralIndex(TransactAssetId::get()).into();

        assert_ok!(Assets::force_create(
            RuntimeOrigin::root(),
            TransactAssetId::get(),
            0u64,
            true,
            min_balance,
        ));

        MultiAsset {
            id: Concrete(asset_location),
            fun: Fungible((min_balance * 100).into()),
        }
    }
}

impl fungible::Config for Test {}
impl Config for Test {}

#[cfg(feature = "runtime-benchmarks")]
pub fn new_test_ext() -> sp_io::TestExternalities {
    use sp_runtime::BuildStorage;
    let t = GenesisConfig {
        ..Default::default()
    }
    .build_storage()
    .unwrap();

    let mut ext = sp_io::TestExternalities::from(t);
    ext.execute_with(|| {
        System::set_block_number(1);
        // assert_ok!(Assets::force_create(
        //     RuntimeOrigin::root(),
        //     TransactAssetId::get(),
        //     0u64,
        //     true,
        //     100,
        // ));
        // register the transact asset
    });
    ext
}
