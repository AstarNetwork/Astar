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
    construct_runtime, derive_impl,
    dispatch::DispatchClass,
    parameter_types,
    traits::{
        AsEnsureOriginWithArg, ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, Contains,
        Disabled, Everything, InstanceFilter, Nothing,
    },
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, WEIGHT_REF_TIME_PER_SECOND},
        Weight,
    },
    PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot, EnsureSigned,
};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use sp_runtime::{
    traits::{AccountIdConversion, Convert, IdentityLookup, MaybeEquivalence},
    AccountId32, FixedU128, Perbill, RuntimeDebug,
};
use sp_std::prelude::*;

use super::msg_queue::*;
use xcm::latest::prelude::{AssetId as XcmAssetId, *};
use xcm_builder::{
    Account32Hash, AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
    AllowUnpaidExecutionFrom, ConvertedConcreteId, EnsureXcmOrigin, FixedRateOfFungible,
    FixedWeightBounds, FungibleAdapter, FungiblesAdapter, IsConcrete, NoChecking,
    ParentAsSuperuser, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative,
    SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
    SovereignSignedViaLocation, TakeWeightCredit, WithComputedOrigin,
};

use orml_xcm_support::DisabledParachainFee;
use sp_core::DecodeWithMemTracking;
use xcm_executor::{traits::JustTry, XcmExecutor};

use astar_primitives::{
    dapp_staking::{AccountCheck, CycleConfiguration, SmartContract, StakingRewardHandler},
    oracle::PriceProvider,
    xcm::{
        AbsoluteAndRelativeReserveProvider, AllowTopLevelPaidExecutionFrom,
        AssetLocationIdConverter, FixedRateOfForeignAsset, ReserveAssetFilter,
        XcmFungibleFeeHandler,
    },
};

pub type AccountId = AccountId32;
pub type Balance = u128;
pub type AssetId = u128;

pub type ShidenAssetLocationIdConverter = AssetLocationIdConverter<AssetId, XcAssetConfig>;

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
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = ConstU32<1>;
}

impl pallet_utility::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

#[derive_impl(pallet_assets::config_preludes::TestDefaultConfig)]
impl pallet_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = AssetId;
    type AssetIdParameter = AssetId;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = ConstU128<10>;
    type MetadataDepositBase = ConstU128<10>;
    type MetadataDepositPerByte = ConstU128<1>;
    type AssetAccountDeposit = ConstU128<10>;
    type ApprovalDeposit = ConstU128<10>;
    type Freezer = ();
    type RemoveItemsLimit = ConstU32<100>;
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
}

#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for Runtime {
    type MinimumPeriod = ConstU64<1>;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

/// Constant values used within the runtime.
pub const MICROSDN: Balance = 1_000_000_000_000;
pub const MILLISDN: Balance = 1_000 * MICROSDN;
/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 0.5 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
    WEIGHT_REF_TIME_PER_SECOND.saturating_div(2),
    polkadot_primitives::MAX_POV_SIZE as u64,
);

parameter_types! {
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub SS58Prefix: u8 = 5;
}

parameter_types! {
    pub const DepositPerItem: Balance = MILLISDN / 1_000_000;
    pub const DepositPerByte: Balance = MILLISDN / 1_000_000;
    pub const DefaultDepositLimit: Balance = 1000 * MILLISDN;
    pub const MaxDelegateDependencies: u32 = 32;
    pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(10);
    pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
}

impl Convert<Weight, Balance> for Runtime {
    fn convert(w: Weight) -> Balance {
        w.ref_time().into()
    }
}

pub struct CallFilter;
impl Contains<RuntimeCall> for CallFilter {
    fn contains(call: &RuntimeCall) -> bool {
        match call {
            // allow pallet_xcm::send()
            RuntimeCall::PolkadotXcm(pallet_xcm::Call::send { .. }) => true,
            // no other calls allowed
            _ => false,
        }
    }
}

#[derive_impl(pallet_contracts::config_preludes::TestDefaultConfig)]
impl pallet_contracts::Config for Runtime {
    type Time = Timestamp;
    type Randomness = Randomness;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type RuntimeHoldReason = RuntimeHoldReason;
    /// The safest default is to allow no calls at all.
    ///
    /// Runtimes should whitelist dispatchables that are allowed to be called from contracts
    /// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
    /// change because that would break already deployed contracts. The `Call` structure itself
    /// is not allowed to change the indices of existing pallets, too.
    type CallFilter = CallFilter;
    type CallStack = [pallet_contracts::Frame<Self>; 5];
    /// We are not using the pallet_transaction_payment for simplicity
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type Schedule = Schedule;
    type UploadOrigin = EnsureSigned<AccountId32>;
    type InstantiateOrigin = EnsureSigned<AccountId32>;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    MaxEncodedLen,
    scale_info::TypeInfo,
)]
pub enum ProxyType {
    CancelProxy,
    DappStaking,
    StakerRewardClaim,
}

impl Default for ProxyType {
    fn default() -> Self {
        Self::CancelProxy
    }
}

impl InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            ProxyType::CancelProxy => {
                matches!(
                    c,
                    RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. })
                )
            }
            ProxyType::DappStaking => {
                matches!(c, RuntimeCall::DappStaking(..) | RuntimeCall::Utility(..))
            }
            ProxyType::StakerRewardClaim => {
                matches!(
                    c,
                    RuntimeCall::DappStaking(
                        pallet_dapp_staking::Call::claim_staker_rewards { .. }
                    ) | RuntimeCall::Utility(..)
                )
            }
        }
    }

    fn is_superset(&self, o: &Self) -> bool {
        match (self, o) {
            (Self::StakerRewardClaim, Self::DappStaking) => true,
            (x, y) if x == y => true,
            _ => false,
        }
    }
}

impl pallet_proxy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type ProxyType = ProxyType;
    type ProxyDepositBase = ConstU128<100>;
    type ProxyDepositFactor = ConstU128<200>;
    type MaxProxies = ConstU32<16>;
    type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
    type MaxPending = ConstU32<16>;
    type CallHasher = sp_runtime::traits::BlakeTwo256;
    type AnnouncementDepositBase = ConstU128<100>;
    type AnnouncementDepositFactor = ConstU128<400>;
    type BlockNumberProvider = System;
}

parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub TreasuryAccountId: AccountId = TreasuryPalletId::get().into_account_truncating();
}

impl pallet_xc_asset_config::Config for Runtime {
    type AssetId = AssetId;
    type ManagerOrigin = EnsureRoot<AccountId>;
    type WeightInfo = pallet_xc_asset_config::weights::SubstrateWeight<Runtime>;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}
parameter_types! {
    pub const ReservedXcmpWeight: Weight = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_div(4), 0);
    pub const ReservedDmpWeight: Weight = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_div(4), 0);
}

parameter_types! {
    pub RelayNetwork: Option<NetworkId> = Some(NetworkId::Kusama);
    pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
    pub UniversalLocation: InteriorLocation =
        [GlobalConsensus(RelayNetwork::get().unwrap()), Parachain(MsgQueue::parachain_id().into())].into();
    pub const ShidenLocation: Location = Here.into_location();
    pub DummyCheckingAccount: AccountId = PolkadotXcm::check_account();
}

/// Type for specifying how a `Location` can be converted into an `AccountId`. This is used
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
pub type CurrencyTransactor = FungibleAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    IsConcrete<ShidenLocation>,
    // Convert an XCM Location into a local account id:
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
    // Convert an XCM Location into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports of `Assets`.
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
    pub const UnitWeightCost: Weight = Weight::from_parts(10, 0);
    pub const MaxInstructions: u32 = 100;
    pub NativePerSecond: (XcmAssetId, u128, u128) = (AssetId(ShidenLocation::get()), 1_000_000_000_000, 1024 * 1024);
}

pub type XcmRouter = super::ParachainXcmRouter<MsgQueue>;

pub struct ParentOrParentsPlurality;
impl Contains<Location> for ParentOrParentsPlurality {
    fn contains(location: &Location) -> bool {
        matches!(location.unpack(), (1, []) | (1, [Plurality { .. }]))
    }
}

pub type XcmBarrier = (
    TakeWeightCredit,
    // This will first calculate the derived origin, before checking it against the barrier implementation
    WithComputedOrigin<AllowTopLevelPaidExecutionFrom<Everything>, UniversalLocation, ConstU32<8>>,
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

pub type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;

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
        FixedRateOfFungible<NativePerSecond, ()>,
        FixedRateOfForeignAsset<XcAssetConfig, ShidenXcmFungibleFeeHandler>,
    );
    type ResponseHandler = ();
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
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

impl mock_msg_queue::Config for Runtime {
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Everything;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Nothing;
    type XcmReserveTransferFilter = Everything;
    type Weigher = Weigher;
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

/// Convert `AccountId` to `Location`.
pub struct AccountIdToLocation;
impl Convert<AccountId, Location> for AccountIdToLocation {
    fn convert(account: AccountId) -> Location {
        Junction::AccountId32 {
            network: None,
            id: account.into(),
        }
        .into()
    }
}

parameter_types! {
    /// The absolute location in perspective of the whole network.
    pub ShidenLocationAbsolute: Location = Location {
        parents: 1,
        interior: Parachain(MsgQueue::parachain_id().into()).into()
    };
    /// Max asset types for one cross-chain transfer. `2` covers all current use cases.
    /// Can be updated with extra test cases in the future if needed.
    pub const MaxAssetsForTransfer: usize = 2;
}

/// Convert `AssetId` to optional `Location`. The impl is a wrapper
/// on `ShidenAssetLocationIdConverter`.
pub struct AssetIdConvert;
impl Convert<AssetId, Option<Location>> for AssetIdConvert {
    fn convert(asset_id: AssetId) -> Option<Location> {
        ShidenAssetLocationIdConverter::convert_back(&asset_id)
    }
}

impl orml_xtokens::Config for Runtime {
    type Balance = Balance;
    type CurrencyId = AssetId;
    type CurrencyIdConvert = AssetIdConvert;
    type AccountIdToLocation = AccountIdToLocation;
    type SelfLocation = ShidenLocation;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type Weigher = Weigher;
    type BaseXcmWeight = UnitWeightCost;
    type UniversalLocation = UniversalLocation;
    type MaxAssetsForTransfer = MaxAssetsForTransfer;
    // Default impl. Refer to `orml-xtokens` docs for more details.
    type MinXcmFee = DisabledParachainFee;
    type LocationsFilter = Everything;
    type ReserveProvider = AbsoluteAndRelativeReserveProvider<ShidenLocationAbsolute>;
    type RateLimiter = ();
    type RateLimiterId = ();
}

pub struct DummyCycleConfiguration;
impl CycleConfiguration for DummyCycleConfiguration {
    fn periods_per_cycle() -> u32 {
        4
    }

    fn eras_per_voting_subperiod() -> u32 {
        8
    }

    fn eras_per_build_and_earn_subperiod() -> u32 {
        16
    }

    fn blocks_per_era() -> u32 {
        10
    }
}

pub struct DummyAccountCheck;
impl AccountCheck<AccountId> for DummyAccountCheck {
    fn allowed_to_stake(_: &AccountId) -> bool {
        true
    }
}

pub struct DummyStakingRewardHandler;
impl StakingRewardHandler<AccountId> for DummyStakingRewardHandler {
    fn staker_and_dapp_reward_pools(_total_staked_value: Balance) -> (Balance, Balance) {
        (
            Balance::from(1_000_000_000_000_u128),
            Balance::from(1_000_000_000_u128),
        )
    }

    fn bonus_reward_pool() -> Balance {
        Balance::from(3_000_000_u128)
    }

    fn payout_reward(_: &AccountId, _: Balance) -> Result<(), ()> {
        Ok(())
    }
}

pub struct DummyPriceProvider;
impl PriceProvider for DummyPriceProvider {
    fn average_price() -> FixedU128 {
        FixedU128::from_rational(1, 10)
    }
}

pub(crate) type MockSmartContract = SmartContract<AccountId>;

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchmarkHelper<SC, ACC>(sp_std::marker::PhantomData<(SC, ACC)>);
#[cfg(feature = "runtime-benchmarks")]
impl pallet_dapp_staking::BenchmarkHelper<MockSmartContract, AccountId>
    for BenchmarkHelper<MockSmartContract, AccountId>
{
    fn get_smart_contract(id: u32) -> MockSmartContract {
        MockSmartContract::Wasm(AccountId::from([id as u8; 32]))
    }

    fn set_balance(account: &AccountId, amount: Balance) {
        use frame_support::traits::fungible::Unbalanced as FunUnbalanced;
        Balances::write_balance(account, amount)
            .expect("Must succeed in test/benchmark environment.");
    }
}

parameter_types! {
    pub const BaseNativeCurrencyPrice: FixedU128 = FixedU128::from_rational(5, 100);
}

impl pallet_dapp_staking::Config for Runtime {
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type Currency = Balances;
    type SmartContract = MockSmartContract;
    type ContractRegisterOrigin = frame_system::EnsureRoot<AccountId>;
    type ContractUnregisterOrigin = frame_system::EnsureRoot<AccountId>;
    type ManagerOrigin = frame_system::EnsureRoot<AccountId>;
    type NativePriceProvider = DummyPriceProvider;
    type StakingRewardHandler = DummyStakingRewardHandler;
    type CycleConfiguration = DummyCycleConfiguration;
    type Observers = ();
    type AccountCheck = DummyAccountCheck;
    type TierSlots = astar_primitives::dapp_staking::StandardTierSlots;
    type BaseNativeCurrencyPrice = BaseNativeCurrencyPrice;
    type EraRewardSpanLength = ConstU32<1>;
    type RewardRetentionInPeriods = ConstU32<2>;
    type MaxNumberOfContracts = ConstU32<10>;
    type MaxUnlockingChunks = ConstU32<5>;
    type MinimumLockedAmount = ConstU128<3>;
    type UnlockingPeriod = ConstU32<2>;
    type MaxNumberOfStakedContracts = ConstU32<5>;
    type MinimumStakeAmount = ConstU128<3>;
    type NumberOfTiers = ConstU32<4>;
    type RankingEnabled = ConstBool<true>;
    type MaxBonusSafeMovesPerPeriod = ConstU8<0>;
    type WeightInfo = ();
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = BenchmarkHelper<MockSmartContract, AccountId>;
}

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
    pub struct Runtime {
        System: frame_system,
        Balances: pallet_balances,
        MsgQueue: mock_msg_queue,
        PolkadotXcm: pallet_xcm,
        Assets: pallet_assets,
        XcAssetConfig: pallet_xc_asset_config,
        CumulusXcm: cumulus_pallet_xcm,
        DappStaking: pallet_dapp_staking,
        Proxy: pallet_proxy,
        Utility: pallet_utility,
        Randomness: pallet_insecure_randomness_collective_flip,
        Timestamp: pallet_timestamp,
        Contracts: pallet_contracts,
        Xtokens: orml_xtokens,
    }
);
