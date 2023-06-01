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

use super::{
    AccountId, AllPalletsWithSystem, AssetId, Assets, Balance, Balances, BurnFees, ParachainInfo,
    ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin,
    ShidenAssetLocationIdConverter, TreasuryAccountId, WeightToFee, XcAssetConfig, XcmpQueue,
    MAXIMUM_BLOCK_WEIGHT,
};
use frame_support::{
    match_types, parameter_types,
    traits::{ConstU32, Contains, Everything, Nothing},
    weights::Weight,
};
use frame_system::EnsureRoot;
use sp_runtime::traits::{Convert, Get};
use sp_std::marker::PhantomData;

// Polkadot imports
use xcm::latest::prelude::*;
use xcm_builder::{
    Account32Hash, AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
    AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, ConvertedConcreteId, CurrencyAdapter,
    EnsureXcmOrigin, FixedWeightBounds, FungiblesAdapter, IsConcrete, NoChecking,
    ParentAsSuperuser, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative,
    SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
    SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
};
use xcm_executor::{
    traits::{Convert as XcmConvert, JustTry, WithOriginFilter},
    XcmExecutor,
};

// ORML imports
use orml_traits::location::{RelativeReserveProvider, Reserve};
use orml_xcm_support::DisabledParachainFee;

// Astar imports
use xcm_primitives::{FixedRateOfForeignAsset, ReserveAssetFilter, XcmFungibleFeeHandler};

parameter_types! {
    pub RelayNetwork: Option<NetworkId> = Some(NetworkId::Kusama);
    pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
    pub UniversalLocation: InteriorMultiLocation =
        X2(GlobalConsensus(RelayNetwork::get().unwrap()), Parachain(ParachainInfo::parachain_id().into()));
    pub const ShidenLocation: MultiLocation = Here.into_location();
    pub DummyCheckingAccount: AccountId = PolkadotXcm::check_account();
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
    // The parent (Relay-chain) origin converts to the default `AccountId`.
    ParentIsPreset<AccountId>,
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<polkadot_parachain::primitives::Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<RelayNetwork, AccountId>,
    // Derives a private `Account32` by hashing `("multiloc", received multilocation)`
    Account32Hash<RelayNetwork, AccountId>,
);

/// Means for transacting the native currency on this chain.
pub type CurrencyTransactor = CurrencyAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    IsConcrete<ShidenLocation>,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports of `Balances`.
    (),
>;

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
    // Use this fungibles implementation:
    Assets,
    // Use this currency when it is a fungible asset matching the given location or name:
    ConvertedConcreteId<AssetId, Balance, ShidenAssetLocationIdConverter, JustTry>,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't support teleport so no need to check any assets.
    NoChecking,
    // We don't track any teleports of `Assets`.
    DummyCheckingAccount,
>;

/// Means for transacting assets on this chain.
pub type AssetTransactors = (CurrencyTransactor, FungiblesTransactor);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
    // Native converter for Relay-chain (Parent) location; will convert to a `Relay` origin when
    // recognised.
    RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
    // Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
    // recognised.
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
    // Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
    // transaction from the Root origin.
    ParentAsSuperuser<RuntimeOrigin>,
    // Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
    pallet_xcm::XcmPassthrough<RuntimeOrigin>,
    // Native signed account converter; this just converts an `AccountId32` origin into a normal
    // `Origin::Signed` origin of the same 32-byte value.
    SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
);

parameter_types! {
    // One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
    // The default POV size used by Polkadot/Kusama was 64 kB but that has been updated here: https://github.com/paritytech/polkadot/pull/7081
    // We should properly benchmark instructions and get rid of fixed weights.
    pub UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 1024);
    pub const MaxInstructions: u32 = 100;
}

match_types! {
    pub type ParentOrParentsPlurality: impl Contains<MultiLocation> = {
        MultiLocation { parents: 1, interior: Here } |
        MultiLocation { parents: 1, interior: X1(Plurality { .. }) }
    };
}

/// A call filter for the XCM Transact instruction. This is a temporary measure until we properly
/// account for proof size weights.
///
/// Calls that are allowed through this filter must:
/// 1. Have a fixed weight;
/// 2. Cannot lead to another call being made (Astar: we slightly loosen this requirement)
/// 3. Have a defined proof size weight, e.g. no unbounded vecs in call parameters. - TODO: shouldn't max XCM weight handle this?
pub struct SafeCallFilter;

impl SafeCallFilter {
    // 1. RuntimeCall::Multisig(..) - contains `Vec` in argument so we should avoid this
    // 2. RuntimeCall::EVM(..) & RuntimeCall::Ethereum(..) have to be prohibited since we cannot measure PoV size properly
    // 3. RuntimeCall::Contracts(..) it should be safe to allow for such calls but perhaps it's better to do more delibrate testing on Shibuya/RocStar.

    /// Checks whether the base (non-composite) call is allowed to be executed via `Transact` XCM instruction.
    pub fn allow_base_call(call: &RuntimeCall) -> bool {
        match call {
            RuntimeCall::System(..)
            | RuntimeCall::Identity(..)
            | RuntimeCall::Balances(..)
            | RuntimeCall::Vesting(..)
            | RuntimeCall::DappsStaking(..)
            | RuntimeCall::Assets(..)
            | RuntimeCall::PolkadotXcm(..)
            | RuntimeCall::Session(..)
            | RuntimeCall::Proxy(
                pallet_proxy::Call::add_proxy { .. }
                | pallet_proxy::Call::remove_proxy { .. }
                | pallet_proxy::Call::remove_proxies { .. }
                | pallet_proxy::Call::create_pure { .. }
                | pallet_proxy::Call::kill_pure { .. }
                | pallet_proxy::Call::announce { .. }
                | pallet_proxy::Call::remove_announcement { .. }
                | pallet_proxy::Call::reject_announcement { .. },
            ) => true,
            _ => false,
        }
    }
    /// Checks whether composite call is allowed to be executed via `Transact` XCM instruction.
    ///
    /// Each composite call's subcalls are checked against base call filter. No nesting of composite calls is allowed.
    pub fn allow_composite_call(call: &RuntimeCall) -> bool {
        match call {
            RuntimeCall::Proxy(pallet_proxy::Call::proxy { call, .. }) => {
                Self::allow_base_call(call)
            }
            RuntimeCall::Proxy(pallet_proxy::Call::proxy_announced { call, .. }) => {
                Self::allow_base_call(call)
            }
            RuntimeCall::Utility(pallet_utility::Call::batch { calls, .. }) => {
                calls.iter().all(|call| Self::allow_base_call(call))
            }
            RuntimeCall::Utility(pallet_utility::Call::batch_all { calls, .. }) => {
                calls.iter().all(|call| Self::allow_base_call(call))
            }
            RuntimeCall::Utility(pallet_utility::Call::as_derivative { call, .. }) => {
                Self::allow_base_call(call)
            }
            _ => false,
        }
    }
}

impl Contains<RuntimeCall> for SafeCallFilter {
    fn contains(call: &RuntimeCall) -> bool {
        #[cfg(feature = "runtime-benchmarks")]
        {
            if matches!(
                call,
                RuntimeCall::System(frame_system::Call::remark_with_event { .. })
            ) {
                return true;
            }
        }

        Self::allow_base_call(call) || Self::allow_composite_call(call)
    }
}

pub type XcmBarrier = (
    TakeWeightCredit,
    AllowTopLevelPaidExecutionFrom<Everything>,
    // Parent and its plurality get free execution
    AllowUnpaidExecutionFrom<ParentOrParentsPlurality>,
    // Expected responses are OK.
    AllowKnownQueryResponses<PolkadotXcm>,
    // Subscriptions for version tracking are OK.
    AllowSubscriptionsFrom<Everything>,
);

// Used to handle XCM fee deposit into treasury account
pub type ShidenXcmFungibleFeeHandler = XcmFungibleFeeHandler<
    AccountId,
    ConvertedConcreteId<AssetId, Balance, ShidenAssetLocationIdConverter, JustTry>,
    Assets,
    TreasuryAccountId,
>;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    type AssetTransactor = AssetTransactors;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = ReserveAssetFilter;
    type IsTeleporter = ();
    type UniversalLocation = UniversalLocation;
    type Barrier = XcmBarrier;
    type Weigher = Weigher;
    type Trader = (
        UsingComponents<WeightToFee, ShidenLocation, AccountId, Balances, BurnFees>,
        FixedRateOfForeignAsset<XcAssetConfig, ShidenXcmFungibleFeeHandler>,
    );
    type ResponseHandler = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;

    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = ConstU32<64>;
    type AssetLocker = ();
    type AssetExchanger = ();
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = WithOriginFilter<SafeCallFilter>;
    type SafeCallFilter = SafeCallFilter;
}

parameter_types! {
    pub const MaxDownwardMessageWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(10);
}

/// Local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
);

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
    pub ReachableDest: Option<MultiLocation> = Some(Parent.into());
}

pub type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;

impl pallet_xcm::Config for Runtime {
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;

    type RuntimeEvent = RuntimeEvent;
    type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Nothing;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Nothing;
    type XcmReserveTransferFilter = Everything;
    type Weigher = Weigher;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;

    type Currency = Balances;
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = LocationToAccountId;
    type MaxLockers = ConstU32<0>;
    type WeightInfo = pallet_xcm::weights::SubstrateWeight<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type ReachableDest = ReachableDest;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = PolkadotXcm;
    type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
    type ControllerOrigin = EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type PriceForSiblingDelivery = ();
    type WeightInfo = cumulus_pallet_xcmp_queue::weights::SubstrateWeight<Runtime>;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

/// Convert `AccountId` to `MultiLocation`.
pub struct AccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
    fn convert(account: AccountId) -> MultiLocation {
        X1(AccountId32 {
            network: None,
            id: account.into(),
        })
        .into()
    }
}

parameter_types! {
    /// The absolute location in perspective of the whole network.
    pub ShidenLocationAbsolute: MultiLocation = MultiLocation {
        parents: 1,
        interior: X1(
            Parachain(ParachainInfo::parachain_id().into())
        )
    };
    /// Max asset types for one cross-chain transfer. `2` covers all current use cases.
    /// Can be updated with extra test cases in the future if needed.
    pub const MaxAssetsForTransfer: usize = 2;
}

/// Convert `AssetId` to optional `MultiLocation`. The impl is a wrapper
/// on `ShidenAssetLocationIdConverter`.
pub struct AssetIdConvert;
impl Convert<AssetId, Option<MultiLocation>> for AssetIdConvert {
    fn convert(asset_id: AssetId) -> Option<MultiLocation> {
        ShidenAssetLocationIdConverter::reverse_ref(&asset_id).ok()
    }
}

/// `MultiAsset` reserve location provider. It's based on `RelativeReserveProvider` and in
/// addition will convert self absolute location to relative location.
pub struct AbsoluteAndRelativeReserveProvider<AbsoluteLocation>(PhantomData<AbsoluteLocation>);
impl<AbsoluteLocation: Get<MultiLocation>> Reserve
    for AbsoluteAndRelativeReserveProvider<AbsoluteLocation>
{
    fn reserve(asset: &MultiAsset) -> Option<MultiLocation> {
        RelativeReserveProvider::reserve(asset).map(|reserve_location| {
            if reserve_location == AbsoluteLocation::get() {
                MultiLocation::here()
            } else {
                reserve_location
            }
        })
    }
}

impl orml_xtokens::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type CurrencyId = AssetId;
    type CurrencyIdConvert = AssetIdConvert;
    type AccountIdToMultiLocation = AccountIdToMultiLocation;
    type SelfLocation = ShidenLocation;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type Weigher = Weigher;
    type BaseXcmWeight = UnitWeightCost;
    type UniversalLocation = UniversalLocation;
    type MaxAssetsForTransfer = MaxAssetsForTransfer;
    // Default impl. Refer to `orml-xtokens` docs for more details.
    type MinXcmFee = DisabledParachainFee;
    type MultiLocationsFilter = Everything;
    type ReserveProvider = AbsoluteAndRelativeReserveProvider<ShidenLocationAbsolute>;
}
