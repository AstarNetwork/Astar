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

//! A mock runtime for XCM benchmarking.

use crate::{fungible, generic, *};
use astar_primitives::xcm::ReserveAssetFilter;
use frame_benchmarking::BenchmarkError;
use frame_support::{
    assert_ok, derive_impl, parameter_types,
    traits::{fungible::ItemOf, AsEnsureOriginWithArg, Everything, Nothing},
    weights::Weight,
};
use frame_system::{EnsureRoot, EnsureSigned};

use core::marker::PhantomData;
use sp_core::{ConstU64, Get};
use sp_runtime::traits::IdentityLookup;
use xcm::latest::prelude::*;
use xcm_builder::{AllowUnpaidExecutionFrom, FungiblesAdapter, MintLocation, NoChecking};

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u64;
type AccountId = u64;
type AssetId = u128;

// For testing the pallet, we construct a mock runtime.
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system = 10,
        Balances: pallet_balances,
        PalletAssets: pallet_assets,
        PolkadotXcmGenericBenchmarks: pallet_xcm_benchmarks::generic,
        PolkadotXcmFungibleBenchmarks: pallet_xcm_benchmarks::fungible,
        XcmAssetsBenchmark: fungible,
        XcmGenericBenchmarks: generic,
    }
);

pub struct AccountIdConverter;
impl xcm_executor::traits::ConvertLocation<u64> for AccountIdConverter {
    fn convert_location(ml: &Location) -> Option<u64> {
        match ml.unpack() {
            (0, [AccountId32 { id, .. }]) => {
                <u64 as parity_scale_codec::Decode>::decode(&mut &*id.to_vec()).ok()
            }
            _ => None,
        }
    }
}

// An xcm sender/receiver akin to > /dev/null
pub struct DevNull;
impl SendXcm for DevNull {
    type Ticket = ();

    fn validate(
        _destination: &mut Option<Location>,
        _message: &mut Option<opaque::Xcm>,
    ) -> SendResult<Self::Ticket> {
        Ok(((), Assets::new()))
    }

    fn deliver(_: Self::Ticket) -> Result<XcmHash, SendError> {
        Ok(XcmHash::default())
    }
}

impl xcm_executor::traits::OnResponse for DevNull {
    fn expecting_response(_: &Location, _: u64, _: Option<&Location>) -> bool {
        false
    }
    fn on_response(
        _: &Location,
        _: u64,
        _: Option<&Location>,
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
    pub UniversalLocation: InteriorLocation = Here;
}
#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type AccountData = pallet_balances::AccountData<u64>;
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 10;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type Balance = Balance;
    type AccountStore = System;
}

#[derive_impl(pallet_assets::config_preludes::TestDefaultConfig)]
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
    type Freezer = ();
}

pub struct MatchOnlyAsset<MatchAsset>(PhantomData<MatchAsset>);
impl<MatchAsset: Get<AssetId>> xcm_executor::traits::MatchesFungibles<AssetId, Balance>
    for MatchOnlyAsset<MatchAsset>
{
    fn matches_fungibles(
        a: &Asset,
    ) -> core::result::Result<(AssetId, Balance), xcm_executor::traits::prelude::Error> {
        use sp_runtime::traits::SaturatedConversion;
        match a {
            Asset {
                fun: Fungible(amount),
                ..
            } => Ok((MatchAsset::get(), (*amount).saturated_into::<u64>())),
            _ => Err(xcm_executor::traits::prelude::Error::AssetNotHandled),
        }
    }
}

parameter_types! {
    pub const DummyCheckingAccount: AccountId = 0;

    // AssetId used as a fungible for benchmarks
    pub const TransactAssetId: u128 = 1;
}

// Use ONLY assets as the asset transactor.
pub type AssetTransactor = FungiblesAdapter<
    PalletAssets,
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

    pub WeightPrice: (xcm::latest::AssetId, u128, u128) = (Parent.into(), 1_000_000, 1024);
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
    type Aliasers = Nothing;
    type TransactionalProcessor = ();
    type HrmpNewChannelOpenRequestHandler = ();
    type HrmpChannelAcceptedHandler = ();
    type HrmpChannelClosingHandler = ();
    type XcmRecorder = ();
}

impl pallet_xcm_benchmarks::Config for Test {
    type XcmConfig = XcmConfig;
    type AccountIdConverter = AccountIdConverter;
    type DeliveryHelper = ();

    fn valid_destination() -> Result<Location, BenchmarkError> {
        let valid_destination: Location = AccountId32 {
            network: None,
            id: [0u8; 32],
        }
        .into();

        Ok(valid_destination)
    }
    fn worst_case_holding(_depositable_count: u32) -> Assets {
        let assets: Vec<Asset> = vec![Asset {
            id: AssetId(Location::parent()),
            fun: Fungible(u128::MAX),
        }];
        assets.into()
    }
}

impl pallet_xcm_benchmarks::generic::Config for Test {
    type RuntimeCall = RuntimeCall;
    type TransactAsset = Balances;

    fn worst_case_response() -> (u64, Response) {
        let assets: Assets = (AssetId(Here.into()), 100).into();
        (0, Response::Assets(assets))
    }

    fn worst_case_asset_exchange() -> Result<(Assets, Assets), BenchmarkError> {
        Err(BenchmarkError::Skip)
    }

    fn universal_alias() -> Result<(Location, Junction), BenchmarkError> {
        Err(BenchmarkError::Skip)
    }

    fn export_message_origin_and_destination(
    ) -> Result<(Location, NetworkId, Junctions), BenchmarkError> {
        Err(BenchmarkError::Skip)
    }

    fn transact_origin_and_runtime_call() -> Result<(Location, RuntimeCall), BenchmarkError> {
        Ok((
            Default::default(),
            frame_system::Call::remark_with_event { remark: vec![] }.into(),
        ))
    }

    fn subscribe_origin() -> Result<Location, BenchmarkError> {
        Ok(Default::default())
    }

    fn claimable_asset() -> Result<(Location, Location, Assets), BenchmarkError> {
        let assets: Assets = (AssetId(Here.into()), 100).into();
        let ticket = Location {
            parents: 0,
            interior: [GeneralIndex(0)].into(),
        };
        Ok((Default::default(), ticket, assets))
    }

    fn unlockable_asset() -> Result<(Location, Location, Asset), BenchmarkError> {
        Err(BenchmarkError::Skip)
    }

    fn alias_origin() -> Result<(Location, Location), BenchmarkError> {
        Err(BenchmarkError::Skip)
    }

    fn fee_asset() -> Result<Asset, BenchmarkError> {
        Ok((AssetId(Here.into()), 100).into())
    }
}

parameter_types! {
    pub const CheckingAccount: Option<(u64, MintLocation)> = None;
    pub const TrustedTeleporter: Option<(Location, Asset)> = None;
}

impl pallet_xcm_benchmarks::fungible::Config for Test {
    type TransactAsset = ItemOf<PalletAssets, TransactAssetId, AccountId>;
    type CheckedAccount = CheckingAccount;
    type TrustedTeleporter = TrustedTeleporter;
    type TrustedReserve = TrustedReserve;

    fn get_asset() -> Asset {
        let min_balance = 100u64;
        let asset_location: Location = GeneralIndex(TransactAssetId::get()).into();

        assert_ok!(PalletAssets::force_create(
            RuntimeOrigin::root(),
            TransactAssetId::get(),
            0u64,
            true,
            min_balance,
        ));

        Asset {
            id: AssetId(asset_location),
            fun: Fungible((min_balance * 100).into()),
        }
    }
}

parameter_types! {
    pub TrustedReserveLocation: Location = Parent.into();
    pub TrustedReserveAsset: Asset = Asset { id: AssetId(TrustedReserveLocation::get()), fun: Fungible(1_000_000) };
    pub TrustedReserve: Option<(Location, Asset)> = Some((TrustedReserveLocation::get(), TrustedReserveAsset::get()));
}

impl fungible::Config for Test {}
impl generic::Config for Test {}
impl Config for Test {}

#[cfg(feature = "runtime-benchmarks")]
pub fn new_test_ext() -> sp_io::TestExternalities {
    use sp_runtime::BuildStorage;
    let t = RuntimeGenesisConfig {
        ..Default::default()
    }
    .build_storage()
    .unwrap();
    t.into()
}
