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

use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{ConstU32, Disabled, Everything, Nothing, ProcessMessage, ProcessMessageError},
    weights::{Weight, WeightMeter},
};
use frame_system::EnsureRoot;
use sp_runtime::{traits::IdentityLookup, AccountId32};

use polkadot_parachain::primitives::Id as ParaId;
use polkadot_runtime_parachains::{
    configuration,
    inclusion::{AggregateMessageOrigin, UmpQueueId},
    origin, shared,
};
use xcm::latest::prelude::*;
use xcm_builder::{
    AccountId32Aliases, AllowUnpaidExecutionFrom, ChildParachainAsNative,
    ChildParachainConvertsVia, ChildSystemParachainAsSuperuser, FixedRateOfFungible,
    FixedWeightBounds, FungibleAdapter, IsConcrete, SignedAccountId32AsNative, SignedToAccountId32,
    SovereignSignedViaLocation,
};
use xcm_executor::XcmExecutor;

pub type AccountId = AccountId32;
pub type Balance = u128;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = Block;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type AccountData = pallet_balances::AccountData<Balance>;
}

parameter_types! {
    pub ExistentialDeposit: Balance = 1;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

impl shared::Config for Runtime {
    type DisabledValidators = ();
}

impl configuration::Config for Runtime {
    type WeightInfo = configuration::TestWeightInfo;
}

parameter_types! {
    pub const KsmLocation: Location = Here.into_location();
    pub const KusamaNetwork: NetworkId = NetworkId::Kusama;
    pub UniversalLocation: InteriorLocation = Here;
}

pub type SovereignAccountOf = (
    ChildParachainConvertsVia<ParaId, AccountId>,
    AccountId32Aliases<KusamaNetwork, AccountId>,
);

pub type LocalAssetTransactor =
    FungibleAdapter<Balances, IsConcrete<KsmLocation>, SovereignAccountOf, AccountId, ()>;

type LocalOriginConverter = (
    SovereignSignedViaLocation<SovereignAccountOf, RuntimeOrigin>,
    ChildParachainAsNative<origin::Origin, RuntimeOrigin>,
    SignedAccountId32AsNative<KusamaNetwork, RuntimeOrigin>,
    ChildSystemParachainAsSuperuser<ParaId, RuntimeOrigin>,
);

parameter_types! {
    pub const BaseXcmWeight: Weight = Weight::from_parts(1_000, 0);
    pub KsmPerSecond: (AssetId, u128, u128) = (AssetId(KsmLocation::get()), 1, 1024 * 1024);
    pub const MaxInstructions: u32 = 100;
}

pub type XcmRouter = super::RelayChainXcmRouter;
pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = LocalOriginConverter;
    type IsReserve = ();
    type IsTeleporter = ();
    type UniversalLocation = UniversalLocation;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
    type Trader = FixedRateOfFungible<KsmPerSecond, ()>;
    type ResponseHandler = ();
    type AssetTrap = ();
    type AssetClaims = ();
    type SubscriptionService = ();

    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = ConstU32<64>;
    type AssetLocker = ();
    type AssetExchanger = ();
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

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, KusamaNetwork>;

pub type LocationToAccountId = (ChildParachainConvertsVia<ParaId, AccountId>,);

impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    // Anyone can execute XCM messages locally...
    type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Nothing;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Everything;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;

    type Currency = Balances;
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = LocationToAccountId;
    type MaxLockers = ConstU32<0>;
    type WeightInfo = pallet_xcm::TestWeightInfo;
    type MaxRemoteLockConsumers = ConstU32<0>;
    type RemoteLockConsumerIdentifier = ();
    type AdminOrigin = EnsureRoot<AccountId>;
    type AuthorizedAliasConsideration = Disabled;
}

parameter_types! {
    pub const FirstMessageFactorPercent: u64 = 100;
}

impl origin::Config for Runtime {}

type Block = frame_system::mocking::MockBlock<Runtime>;

parameter_types! {
    /// Amount of weight that can be spent per block to service messages.
    pub MessageQueueServiceWeight: Weight = Weight::from_parts(1_000_000_000, 1_000_000);
    pub const MessageQueueHeapSize: u32 = 65_536;
    pub const MessageQueueMaxStale: u32 = 16;
}

/// Message processor to handle any messages that were enqueued into the `MessageQueue` pallet.
pub struct MessageProcessor;
impl ProcessMessage for MessageProcessor {
    type Origin = AggregateMessageOrigin;

    fn process_message(
        message: &[u8],
        origin: Self::Origin,
        meter: &mut WeightMeter,
        id: &mut [u8; 32],
    ) -> Result<bool, ProcessMessageError> {
        let para = match origin {
            AggregateMessageOrigin::Ump(UmpQueueId::Para(para)) => para,
        };
        xcm_builder::ProcessXcmMessage::<
			Junction,
			xcm_executor::XcmExecutor<XcmConfig>,
			RuntimeCall,
		>::process_message(message, Junction::Parachain(para.into()), meter, id)
    }
}

impl pallet_message_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Size = u32;
    type HeapSize = MessageQueueHeapSize;
    type MaxStale = MessageQueueMaxStale;
    type MessageProcessor = MessageProcessor;
    type QueueChangeHandler = ();
    type WeightInfo = ();
    type QueuePausedQuery = ();
    type ServiceWeight = MessageQueueServiceWeight;
    type IdleMaxServiceWeight = MessageQueueServiceWeight;
}

construct_runtime!(
    pub struct Runtime {
        System: frame_system,
        Balances: pallet_balances,
        ParasOrigin: origin,
        XcmPallet: pallet_xcm,
        MessageQueue: pallet_message_queue,
    }
);
