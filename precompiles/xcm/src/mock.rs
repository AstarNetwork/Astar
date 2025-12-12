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

//! Testing utilities.

use super::*;

use fp_evm::{IsPrecompileResult, Precompile};
use frame_support::traits::Disabled;
use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{AsEnsureOriginWithArg, ConstU64, Everything, Nothing},
    weights::Weight,
};
use once_cell::unsync::Lazy;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

use pallet_evm::{
    AddressMapping, EnsureAddressNever, EnsureAddressRoot, PrecompileResult, PrecompileSet,
};
use pallet_evm_precompile_assets_erc20::AddressToAssetId;
use sp_core::{ConstU32, DecodeWithMemTracking, H160};
use sp_runtime::{traits::IdentityLookup, BuildStorage};
use sp_std::cell::RefCell;

use astar_primitives::xcm::AllowTopLevelPaidExecutionFrom;
use xcm::prelude::XcmVersion;
use xcm_builder::{
    test_utils::TransactAsset, AllowKnownQueryResponses, AllowSubscriptionsFrom, FixedWeightBounds,
    SignedToAccountId32, TakeWeightCredit,
};
use xcm_executor::XcmExecutor;
// orml imports
use orml_traits::location::{RelativeReserveProvider, Reserve};
use orml_xcm_support::DisabledParachainFee;

pub type AccountId = TestAccount;
pub type AssetId = u128;
pub type Balance = u128;
pub type Block = frame_system::mocking::MockBlock<Runtime>;
pub type CurrencyId = u128;

/// Multilocations for assetId
const PARENT: Location = Location::parent();
const PARACHAIN: Lazy<Location> = Lazy::new(|| Location {
    parents: 1,
    interior: [Parachain(10)].into(),
});
const GENERAL_INDEX: Lazy<Location> = Lazy::new(|| Location {
    parents: 1,
    interior: [Parachain(10), GeneralIndex(20)].into(),
});
const LOCAL_ASSET: Lazy<Location> = Lazy::new(|| Location {
    parents: 0,
    interior: [GeneralIndex(20)].into(),
});

pub const PRECOMPILE_ADDRESS: H160 = H160::repeat_byte(0x7B);
pub const ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8; 4];

#[derive(
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Clone,
    Encode,
    Decode,
    DecodeWithMemTracking,
    Debug,
    MaxEncodedLen,
    Serialize,
    Deserialize,
    derive_more::Display,
    TypeInfo,
)]
pub enum TestAccount {
    Alice,
    Bob,
    Charlie,
    Bogus,
    Precompile,
}

impl Default for TestAccount {
    fn default() -> Self {
        Self::Alice
    }
}

impl AddressMapping<TestAccount> for TestAccount {
    fn into_account_id(h160_account: H160) -> TestAccount {
        match h160_account {
            a if a == H160::repeat_byte(0xAA) => Self::Alice,
            a if a == H160::repeat_byte(0xBB) => Self::Bob,
            a if a == H160::repeat_byte(0xCC) => Self::Charlie,
            a if a == PRECOMPILE_ADDRESS => Self::Precompile,
            _ => Self::Bogus,
        }
    }
}

impl From<H160> for TestAccount {
    fn from(x: H160) -> TestAccount {
        TestAccount::into_account_id(x)
    }
}

impl From<TestAccount> for H160 {
    fn from(value: TestAccount) -> H160 {
        match value {
            TestAccount::Alice => H160::repeat_byte(0xAA),
            TestAccount::Bob => H160::repeat_byte(0xBB),
            TestAccount::Charlie => H160::repeat_byte(0xCC),
            TestAccount::Precompile => PRECOMPILE_ADDRESS,
            TestAccount::Bogus => Default::default(),
        }
    }
}

impl From<TestAccount> for [u8; 32] {
    fn from(value: TestAccount) -> [u8; 32] {
        match value {
            TestAccount::Alice => [0xAA; 32],
            TestAccount::Bob => [0xBB; 32],
            TestAccount::Charlie => [0xCC; 32],
            _ => Default::default(),
        }
    }
}

impl AddressToAssetId<AssetId> for Runtime {
    fn address_to_asset_id(address: H160) -> Option<AssetId> {
        let mut data = [0u8; 16];
        let address_bytes: [u8; 20] = address.into();
        if ASSET_PRECOMPILE_ADDRESS_PREFIX.eq(&address_bytes[0..4]) {
            data.copy_from_slice(&address_bytes[4..20]);
            Some(u128::from_be_bytes(data))
        } else {
            None
        }
    }

    fn asset_id_to_address(asset_id: AssetId) -> H160 {
        let mut data = [0u8; 20];
        data[0..4].copy_from_slice(ASSET_PRECOMPILE_ADDRESS_PREFIX);
        data[4..20].copy_from_slice(&asset_id.to_be_bytes());
        H160::from(data)
    }
}

pub struct CurrencyIdToMultiLocation;

impl sp_runtime::traits::Convert<CurrencyId, Option<Location>> for CurrencyIdToMultiLocation {
    fn convert(currency: CurrencyId) -> Option<Location> {
        match currency {
            1u128 => Some(PARENT),
            2u128 => Some((*PARACHAIN).clone()),
            3u128 => Some((*GENERAL_INDEX).clone()),
            4u128 => Some((*LOCAL_ASSET).clone()),
            _ => None,
        }
    }
}

/// Convert `AccountId` to `Location`.
pub struct AccountIdToLocation;
impl sp_runtime::traits::Convert<AccountId, Location> for AccountIdToLocation {
    fn convert(account: AccountId) -> Location {
        AccountId32 {
            network: None,
            id: account.into(),
        }
        .into()
    }
}

/// `Asset` reserve location provider. It's based on `RelativeReserveProvider` and in
/// addition will convert self absolute location to relative location.
pub struct AbsoluteAndRelativeReserveProvider<AbsoluteLocation>(PhantomData<AbsoluteLocation>);
impl<AbsoluteLocation: Get<Location>> Reserve
    for AbsoluteAndRelativeReserveProvider<AbsoluteLocation>
{
    fn reserve(asset: &Asset) -> Option<Location> {
        RelativeReserveProvider::reserve(asset).map(|reserve_location| {
            if reserve_location == AbsoluteLocation::get() {
                Location::here()
            } else {
                reserve_location
            }
        })
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = Block;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type AccountData = pallet_balances::AccountData<Balance>;
}

#[derive(Debug, Clone, Copy)]
pub struct TestPrecompileSet<R>(PhantomData<R>);

impl<Runtime> PrecompileSet for TestPrecompileSet<Runtime>
where
    Runtime: pallet_evm::Config
        + pallet_xcm::Config
        + pallet_assets::Config
        + AddressToAssetId<<Runtime as pallet_assets::Config>::AssetId>,
    XcmPrecompile<Runtime, AssetIdConverter<AssetId>>: Precompile,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            a if a == PRECOMPILE_ADDRESS => {
                Some(XcmPrecompile::<Runtime, AssetIdConverter<AssetId>>::execute(handle))
            }
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160, _remaining_gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: address == PRECOMPILE_ADDRESS,
            extra_cost: 0,
        }
    }
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for Runtime {
    type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

// These parameters dont matter much as this will only be called by root with the forced arguments
// No deposit is substracted with those methods
parameter_types! {
    pub const AssetDeposit: Balance = 0;
    pub const AssetAccountDeposit: Balance = 0;
    pub const ApprovalDeposit: Balance = 0;
    pub const AssetsStringLimit: u32 = 50;
    pub const MetadataDepositBase: Balance = 0;
    pub const MetadataDepositPerByte: Balance = 0;
}

#[derive_impl(pallet_assets::config_preludes::TestDefaultConfig)]
impl pallet_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = AssetId;
    type Currency = Balances;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type AssetAccountDeposit = AssetAccountDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = AssetsStringLimit;
    type Freezer = ();
    type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
    type RemoveItemsLimit = ConstU32<0>;
    type AssetIdParameter = AssetId;
}

pub struct AssetIdConverter<AssetId>(PhantomData<AssetId>);
impl<AssetId> sp_runtime::traits::MaybeEquivalence<Location, AssetId> for AssetIdConverter<AssetId>
where
    AssetId: Clone + Eq + From<u8>,
{
    fn convert(a: &Location) -> Option<AssetId> {
        if a.eq(&Location::parent()) {
            Some(AssetId::from(1u8))
        } else {
            None
        }
    }

    fn convert_back(b: &AssetId) -> Option<Location> {
        if b.eq(&AssetId::from(1u8)) {
            Some(Location::parent())
        } else {
            None
        }
    }
}

parameter_types! {
    pub const PrecompilesValue: TestPrecompileSet<Runtime> =
        TestPrecompileSet(PhantomData);
    pub WeightPerGas: Weight = Weight::from_parts(1,0);
}

pub type PrecompileCall = XcmPrecompileCall<Runtime, AssetIdConverter<AssetId>>;

impl pallet_evm::Config for Runtime {
    type FeeCalculator = ();
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type CallOrigin = EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = EnsureAddressNever<AccountId>;
    type AddressMapping = AccountId;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = TestPrecompileSet<Self>;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = ();
    type OnChargeTransaction = ();
    type BlockGasLimit = ();
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type FindAuthor = ();
    type OnCreate = ();
    type WeightInfo = ();
    type GasLimitPovSizeRatio = ConstU64<4>;
    type AccountProvider = pallet_evm::FrameSystemAccountProvider<Self>;
    type GasLimitStorageGrowthRatio = ConstU64<0>;
    type Timestamp = Timestamp;
    type CreateOriginFilter = ();
    type CreateInnerOriginFilter = ();
}

parameter_types! {
    pub RelayNetwork: Option<NetworkId> = Some(NetworkId::Polkadot);
    pub const AnyNetwork: Option<NetworkId> = None;
    pub UniversalLocation: InteriorLocation = [GlobalConsensus(RelayNetwork::get().unwrap()), Parachain(123)].into();
    pub Ancestry: Location = Here.into();
    pub UnitWeightCost: u64 = 1_000;
    pub const MaxAssetsIntoHolding: u32 = 64;
}

parameter_types! {
    pub const BaseXcmWeight: u64 = 1_000;
    pub const MaxInstructions: u32 = 100;
}

pub type Barrier = (
    TakeWeightCredit,
    AllowTopLevelPaidExecutionFrom<Everything>,
    AllowKnownQueryResponses<XcmPallet>,
    AllowSubscriptionsFrom<Everything>,
);

pub struct LocalAssetTransactor;
impl TransactAsset for LocalAssetTransactor {
    fn deposit_asset(_what: &Asset, _who: &Location, _context: Option<&XcmContext>) -> XcmResult {
        Ok(())
    }

    fn withdraw_asset(
        _what: &Asset,
        _who: &Location,
        _maybe_context: Option<&XcmContext>,
    ) -> Result<xcm_executor::AssetsInHolding, XcmError> {
        Ok(Assets::new().into())
    }
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = StoringRouter;
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = ();
    type IsReserve = Everything;
    type IsTeleporter = ();
    type UniversalLocation = UniversalLocation;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
    type Trader = ();
    type ResponseHandler = XcmPallet;
    type AssetTrap = XcmPallet;
    type AssetLocker = ();
    type AssetExchanger = ();
    type AssetClaims = XcmPallet;
    type SubscriptionService = XcmPallet;
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
    type XcmEventEmitter = ();
}

parameter_types! {
    pub static AdvertisedXcmVersion: XcmVersion = 3;
    pub const MaxAssetsForTransfer: usize = 2;
    pub const SelfLocation: Location = Here.into_location();
    pub SelfLocationAbsolute: Location = Location {
        parents: 1,
        interior: Parachain(123).into()
    };
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, AnyNetwork>;

thread_local! {
    pub static SENT_XCM: RefCell<Vec<(Location, Xcm<()>)>> = RefCell::new(Vec::new());
}

pub(crate) fn _sent_xcm() -> Vec<(Location, Xcm<()>)> {
    SENT_XCM.with(|q| (*q.borrow()).clone())
}

pub(crate) fn take_sent_xcm() -> Vec<(Location, Xcm<()>)> {
    SENT_XCM.with(|q| {
        let mut r = Vec::new();
        std::mem::swap(&mut r, &mut *q.borrow_mut());
        r
    })
}

pub struct StoringRouter;
impl SendXcm for StoringRouter {
    type Ticket = (Location, Xcm<()>);

    fn validate(
        destination: &mut Option<Location>,
        message: &mut Option<Xcm<()>>,
    ) -> SendResult<(Location, Xcm<()>)> {
        Ok((
            (destination.take().unwrap(), message.take().unwrap()),
            Assets::new().into(),
        ))
    }

    fn deliver(pair: Self::Ticket) -> Result<XcmHash, SendError> {
        let (dest, msg) = (pair.0, pair.1);
        SENT_XCM.with(|q| q.borrow_mut().push((dest.into(), msg)));
        Ok(XcmHash::default())
    }
}

impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = StoringRouter;
    type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Everything;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Everything;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;

    type AdvertisedXcmVersion = AdvertisedXcmVersion;
    type TrustedLockers = ();
    type SovereignAccountOf = ();
    type Currency = Balances;
    type CurrencyMatcher = ();
    type MaxLockers = frame_support::traits::ConstU32<8>;
    type MaxRemoteLockConsumers = ConstU32<0>;
    type RemoteLockConsumerIdentifier = ();
    type WeightInfo = pallet_xcm::TestWeightInfo;
    type AdminOrigin = frame_system::EnsureRoot<AccountId>;
    type AuthorizedAliasConsideration = Disabled;
}

impl orml_xtokens::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type CurrencyId = AssetId;
    type CurrencyIdConvert = CurrencyIdToMultiLocation;
    type AccountIdToLocation = AccountIdToLocation;
    type SelfLocation = SelfLocation;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
    type BaseXcmWeight = UnitWeightCost;
    type UniversalLocation = UniversalLocation;
    type MaxAssetsForTransfer = MaxAssetsForTransfer;
    // Default impl. Refer to `orml-xtokens` docs for more details.
    type MinXcmFee = DisabledParachainFee;
    type LocationsFilter = Everything;
    type ReserveProvider = AbsoluteAndRelativeReserveProvider<SelfLocationAbsolute>;
    type RateLimiter = ();
    type RateLimiterId = ();
}

// Configure a mock runtime to test the pallet.
construct_runtime!(
    pub enum Runtime
    {
        System: frame_system,
        Balances: pallet_balances,
        PalletAssets: pallet_assets,
        Evm: pallet_evm,
        Timestamp: pallet_timestamp,
        XcmPallet: pallet_xcm,
        Xtokens: orml_xtokens,
    }
);

#[derive(Default)]
pub(crate) struct ExtBuilder;

impl ExtBuilder {
    pub(crate) fn build(self) -> sp_io::TestExternalities {
        let t = frame_system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .expect("Frame system builds valid default genesis config");

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

pub(crate) fn events() -> Vec<RuntimeEvent> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .collect::<Vec<_>>()
}
