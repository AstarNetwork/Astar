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

use crate::{fungible, generic, *};
use astar_primitives::xcm::ReserveAssetFilter;
use frame_benchmarking::BenchmarkError;
use frame_support::{
    assert_ok, parameter_types,
    traits::{fungible::ItemOf, AsEnsureOriginWithArg, Everything, Nothing},
    weights::Weight,
};
use frame_system::{EnsureRoot, EnsureSigned};

use core::marker::PhantomData;
use sp_core::{ConstU32, ConstU64, Get, H256};
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
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 10,
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
        PolkadotXcmGenericBenchmarks: pallet_xcm_benchmarks::generic::{Pallet},
        PolkadotXcmFungibleBenchmarks: pallet_xcm_benchmarks::fungible::{Pallet},
        XcmAssetsBenchmark: fungible::{Pallet},
        XcmGenericBenchmarks: generic::{Pallet},
    }
);

pub struct AccountIdConverter;
impl xcm_executor::traits::Convert<MultiLocation, u64> for AccountIdConverter {
    fn convert(ml: MultiLocation) -> Result<u64, MultiLocation> {
        match ml {
            MultiLocation {
                parents: 0,
                interior: X1(Junction::AccountId32 { id, .. }),
            } => Ok(<u64 as parity_scale_codec::Decode>::decode(&mut &*id.to_vec()).unwrap()),
            _ => Err(ml),
        }
    }

    fn reverse(acc: u64) -> Result<MultiLocation, u64> {
        Err(acc)
    }
}

// An xcm sender/receiver akin to > /dev/null
pub struct DevNull;
impl SendXcm for DevNull {
    type Ticket = ();

    fn validate(
        _destination: &mut Option<MultiLocation>,
        _message: &mut Option<opaque::Xcm>,
    ) -> SendResult<Self::Ticket> {
        Ok(((), MultiAssets::new()))
    }

    fn deliver(_: Self::Ticket) -> Result<XcmHash, SendError> {
        Ok(XcmHash::default())
    }
}

impl xcm_executor::traits::OnResponse for DevNull {
    fn expecting_response(_: &MultiLocation, _: u64, _: Option<&MultiLocation>) -> bool {
        false
    }
    fn on_response(
        _: &MultiLocation,
        _: u64,
        _: Option<&MultiLocation>,
        _: Response,
        _: Weight,
        _: &XcmContext,
    ) -> Weight {
        Weight::zero()
    }
}

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

parameter_types! {
    pub const DummyCheckingAccount: AccountId = 0;

    // AssetId used as a fungible for benchmarks
    pub const TransactAssetId: u128 = 1;
    pub const TransactAssetLocation: MultiLocation = MultiLocation { parents: 0, interior: X1(GeneralIndex(TransactAssetId::get())) };
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

    pub WeightPrice: (xcm::latest::AssetId, u128, u128) = (Concrete(Parent.into()), 1_000_000, 1024);
    pub const UnitWeightCost: u64 = 10;
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
        let assets: Vec<MultiAsset> = vec![MultiAsset {
            id: Concrete(MultiLocation::parent()),
            fun: Fungible(u128::MAX),
        }];
        assets.into()
    }
}

impl pallet_xcm_benchmarks::generic::Config for Test {
    type RuntimeCall = RuntimeCall;

    fn worst_case_response() -> (u64, Response) {
        let assets: MultiAssets = (Concrete(Here.into()), 100).into();
        (0, Response::Assets(assets))
    }

    fn worst_case_asset_exchange() -> Result<(MultiAssets, MultiAssets), BenchmarkError> {
        Err(BenchmarkError::Skip)
    }

    fn universal_alias() -> Result<(MultiLocation, Junction), BenchmarkError> {
        Err(BenchmarkError::Skip)
    }

    fn export_message_origin_and_destination(
    ) -> Result<(MultiLocation, NetworkId, Junctions), BenchmarkError> {
        Err(BenchmarkError::Skip)
    }

    fn transact_origin_and_runtime_call() -> Result<(MultiLocation, RuntimeCall), BenchmarkError> {
        Ok((
            Default::default(),
            frame_system::Call::remark_with_event { remark: vec![] }.into(),
        ))
    }

    fn subscribe_origin() -> Result<MultiLocation, BenchmarkError> {
        Ok(Default::default())
    }

    fn claimable_asset() -> Result<(MultiLocation, MultiLocation, MultiAssets), BenchmarkError> {
        let assets: MultiAssets = (Concrete(Here.into()), 100).into();
        let ticket = MultiLocation {
            parents: 0,
            interior: X1(GeneralIndex(0)),
        };
        Ok((Default::default(), ticket, assets))
    }

    fn unlockable_asset() -> Result<(MultiLocation, MultiLocation, MultiAsset), BenchmarkError> {
        Err(BenchmarkError::Skip)
    }
}

parameter_types! {
    pub const CheckingAccount: Option<(u64, MintLocation)> = None;
    pub const TrustedTeleporter: Option<(MultiLocation, MultiAsset)> = None;
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

parameter_types! {
    pub TrustedReserveLocation: MultiLocation = Parent.into();
    pub TrustedReserveAsset: MultiAsset = MultiAsset { id: Concrete(TrustedReserveLocation::get()), fun: Fungible(1_000_000) };
    pub TrustedReserve: Option<(MultiLocation, MultiAsset)> = Some((TrustedReserveLocation::get(), TrustedReserveAsset::get()));
}

impl fungible::Config for Test {
    type TrustedReserve = TrustedReserve;
}

impl generic::Config for Test {}
impl Config for Test {}

#[cfg(feature = "runtime-benchmarks")]
pub fn new_test_ext() -> sp_io::TestExternalities {
    use sp_runtime::BuildStorage;
    let t = GenesisConfig {
        ..Default::default()
    }
    .build_storage()
    .unwrap();
    t.into()
}
