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

//! The Astar Network runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

use cumulus_primitives_core::AggregateMessageOrigin;
use frame_support::{
    construct_runtime,
    dispatch::DispatchClass,
    genesis_builder_helper::{build_state, get_preset},
    parameter_types,
    traits::{
        fungible::{Balanced, Credit},
        AsEnsureOriginWithArg, ConstBool, ConstU32, ConstU64, Contains, FindAuthor, Get, Imbalance,
        InstanceFilter, Nothing, OnFinalize, OnUnbalanced, Randomness, WithdrawReasons,
    },
    weights::{
        constants::{
            BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
        },
        ConstantMultiplier, Weight, WeightToFee as WeightToFeeT, WeightToFeeCoefficient,
        WeightToFeeCoefficients, WeightToFeePolynomial,
    },
    ConsensusEngineId, PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot, EnsureSigned,
};
use pallet_ethereum::PostLogContent;
use pallet_evm::{FeeCalculator, GasWeightMapping, Runner};
use pallet_evm_precompile_assets_erc20::AddressToAssetId;
use pallet_identity::legacy::IdentityInfo;
use pallet_transaction_payment::{
    FeeDetails, Multiplier, RuntimeDispatchInfo, TargetedFeeAdjustment,
};
use parity_scale_codec::{Compact, Decode, Encode, MaxEncodedLen};
use polkadot_runtime_common::BlockHashCount;
use sp_api::impl_runtime_apis;
use sp_core::{OpaqueMetadata, H160, H256, U256};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{
        AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, ConvertInto,
        DispatchInfoOf, Dispatchable, OpaqueKeys, PostDispatchInfoOf, UniqueSaturatedInto, Zero,
    },
    transaction_validity::{TransactionSource, TransactionValidity, TransactionValidityError},
    ApplyExtrinsicResult, FixedPointNumber, FixedU128, Perbill, Permill, Perquintill, RuntimeDebug,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};
use xcm::{
    v4::{AssetId as XcmAssetId, Location as XcmLocation},
    IntoVersion, VersionedAssetId, VersionedAssets, VersionedLocation, VersionedXcm,
};
use xcm_fee_payment_runtime_api::Error as XcmPaymentApiError;

use astar_primitives::{
    dapp_staking::{
        AccountCheck as DappStakingAccountCheck, CycleConfiguration, DAppId, EraNumber,
        PeriodNumber, RankedTier, SmartContract, StandardTierSlots,
    },
    evm::EvmRevertCodeHandler,
    oracle::{CurrencyId, DummyCombineData, Price},
    xcm::AssetLocationIdConverter,
    Address, AssetId, BlockNumber, Hash, Header, Nonce, UnfreezeChainOnFailedMigration,
};
pub use astar_primitives::{governance::OracleMembershipInst, AccountId, Balance, Signature};

pub use pallet_dapp_staking_v3::TierThreshold;
pub use pallet_inflation::InflationParameters;

pub use crate::precompiles::WhitelistedCalls;

#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
use parachains_common::message_queue::NarrowOriginToSibling;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

mod chain_extensions;
mod precompiles;
mod weights;
mod xcm_config;

pub type AstarAssetLocationIdConverter = AssetLocationIdConverter<AssetId, XcAssetConfig>;

pub use precompiles::{AstarPrecompiles, ASSET_PRECOMPILE_ADDRESS_PREFIX};
pub type Precompiles = AstarPrecompiles<Runtime, AstarAssetLocationIdConverter>;

use chain_extensions::AstarChainExtensions;

/// Constant values used within the runtime.
pub const MICROASTR: Balance = 1_000_000_000_000;
pub const MILLIASTR: Balance = 1_000 * MICROASTR;
pub const ASTR: Balance = 1_000 * MILLIASTR;

pub const STORAGE_BYTE_FEE: Balance = 20 * MICROASTR;

/// Charge fee for stored bytes and items.
pub const fn deposit(items: u32, bytes: u32) -> Balance {
    items as Balance * 100 * MILLIASTR + (bytes as Balance) * STORAGE_BYTE_FEE
}

/// Charge fee for stored bytes and items as part of `pallet-contracts`.
///
/// The slight difference to general `deposit` function is because there is fixed bound on how large the DB
/// key can grow so it doesn't make sense to have as high deposit per item as in the general approach.
pub const fn contracts_deposit(items: u32, bytes: u32) -> Balance {
    items as Balance * 4 * MILLIASTR + (bytes as Balance) * STORAGE_BYTE_FEE
}

/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 12000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// Maximum number of blocks simultaneously accepted by the Runtime, not yet included into the
/// relay chain.
pub const UNINCLUDED_SEGMENT_CAPACITY: u32 = 1;
/// How many parachain blocks are processed by the relay chain per parent. Limits the number of
/// blocks authored per slot.
pub const BLOCK_PROCESSING_VELOCITY: u32 = 1;
/// Relay chain slot duration, in milliseconds.
pub const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

#[cfg(feature = "std")]
/// Wasm binary unwrapped. If built with `BUILD_DUMMY_WASM_BINARY`, the function panics.
pub fn wasm_binary_unwrap() -> &'static [u8] {
    WASM_BINARY.expect(
        "Development wasm binary is not available. This means the client is \
                        built with `BUILD_DUMMY_WASM_BINARY` flag and it is only usable for \
                        production chains. Please rebuild with the flag disabled.",
    )
}

/// Runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("astar"),
    impl_name: create_runtime_str!("astar"),
    authoring_version: 1,
    spec_version: 93,
    impl_version: 0,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 3,
    state_version: 1,
};

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
    }
}

/// We assume that ~10% of the block weight is consumed by `on_initalize` handlers.
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
    pub const Version: RuntimeVersion = VERSION;
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

pub struct BaseFilter;
impl Contains<RuntimeCall> for BaseFilter {
    fn contains(call: &RuntimeCall) -> bool {
        match call {
            // Filter permission-less assets creation/destroying.
            // Custom asset's `id` should fit in `u32` as not to mix with service assets.
            RuntimeCall::Assets(method) => match method {
                pallet_assets::Call::create { id, .. } => *id < (u32::MAX as AssetId).into(),

                _ => true,
            },
            // These modules are not allowed to be called by transactions:
            // To leave collator just shutdown it, next session funds will be released
            // Other modules should works:
            _ => true,
        }
    }
}

impl frame_system::Config for Runtime {
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type RuntimeCall = RuntimeCall;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = AccountIdLookup<AccountId, ()>;
    /// The nonce type for storing how many extrinsics an account has signed.
    type Nonce = Nonce;
    /// The type for blocks.
    type Block = Block;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    /// The ubiquitous origin type.
    type RuntimeOrigin = RuntimeOrigin;
    /// The aggregated RuntimeTask type.
    type RuntimeTask = RuntimeTask;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// Runtime version.
    type Version = Version;
    /// Converts a module to an index of this module in the runtime.
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = RocksDbWeight;
    type BaseCallFilter = BaseFilter;
    type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type SS58Prefix = SS58Prefix;
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = MultiBlockMigrations;
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = MILLISECS_PER_BLOCK / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const BasicDeposit: Balance = deposit(1, 258);  // 258 bytes on-chain
    pub const ByteDeposit: Balance = deposit(0, 1);
    pub const SubAccountDeposit: Balance = deposit(1, 53);  // 53 bytes on-chain
    pub const MaxSubAccounts: u32 = 100;
    pub const MaxAdditionalFields: u32 = 100;
    pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BasicDeposit = BasicDeposit;
    type ByteDeposit = ByteDeposit;
    type SubAccountDeposit = SubAccountDeposit;
    type MaxSubAccounts = MaxSubAccounts;
    type IdentityInformation = IdentityInfo<MaxAdditionalFields>;
    type MaxRegistrars = MaxRegistrars;
    type Slashed = ();
    type ForceOrigin = EnsureRoot<<Self as frame_system::Config>::AccountId>;
    type RegistrarOrigin = EnsureRoot<<Self as frame_system::Config>::AccountId>;
    type OffchainSignature = Signature;
    type SigningPublicKey = <Signature as sp_runtime::traits::Verify>::Signer;
    type UsernameAuthorityOrigin = EnsureRoot<<Self as frame_system::Config>::AccountId>;
    type PendingUsernameExpiration = ConstU32<{ 7 * DAYS }>;
    type MaxSuffixLength = ConstU32<7>;
    type MaxUsernameLength = ConstU32<32>;
    type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = deposit(0, 32);
}

impl pallet_multisig::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = ConstU32<100>;
    type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const MinimumStakingAmount: Balance = 500 * ASTR;
    pub const BaseNativeCurrencyPrice: FixedU128 = FixedU128::from_rational(5, 100);
}

#[cfg(feature = "runtime-benchmarks")]
pub struct DAppStakingBenchmarkHelper<SC, ACC>(sp_std::marker::PhantomData<(SC, ACC)>);
#[cfg(feature = "runtime-benchmarks")]
impl pallet_dapp_staking_v3::BenchmarkHelper<SmartContract<AccountId>, AccountId>
    for DAppStakingBenchmarkHelper<SmartContract<AccountId>, AccountId>
{
    fn get_smart_contract(id: u32) -> SmartContract<AccountId> {
        let id_bytes = id.to_le_bytes();
        let mut account = [0u8; 32];
        account[..id_bytes.len()].copy_from_slice(&id_bytes);

        SmartContract::Wasm(AccountId::from(account))
    }

    fn set_balance(account: &AccountId, amount: Balance) {
        use frame_support::traits::fungible::Unbalanced as FunUnbalanced;
        Balances::write_balance(account, amount)
            .expect("Must succeed in test/benchmark environment.");
    }
}

pub struct AccountCheck;
impl DappStakingAccountCheck<AccountId> for AccountCheck {
    fn allowed_to_stake(account: &AccountId) -> bool {
        !CollatorSelection::is_account_candidate(account)
    }
}

impl pallet_dapp_staking_v3::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type Currency = Balances;
    type SmartContract = SmartContract<AccountId>;
    type ContractRegisterOrigin = frame_system::EnsureRoot<AccountId>;
    type ContractUnregisterOrigin = frame_system::EnsureRoot<AccountId>;
    type ManagerOrigin = frame_system::EnsureRoot<AccountId>;
    type NativePriceProvider = PriceAggregator;
    type StakingRewardHandler = Inflation;
    type CycleConfiguration = InflationCycleConfig;
    type Observers = Inflation;
    type AccountCheck = AccountCheck;
    type TierSlots = StandardTierSlots;
    type BaseNativeCurrencyPrice = BaseNativeCurrencyPrice;
    type EraRewardSpanLength = ConstU32<16>;
    type RewardRetentionInPeriods = ConstU32<4>;
    type MaxNumberOfContracts = ConstU32<500>;
    type MaxUnlockingChunks = ConstU32<8>;
    type MinimumLockedAmount = MinimumStakingAmount;
    type UnlockingPeriod = ConstU32<9>;
    type MaxNumberOfStakedContracts = ConstU32<16>;
    type MinimumStakeAmount = MinimumStakingAmount;
    type NumberOfTiers = ConstU32<4>;
    type RankingEnabled = ConstBool<true>;
    type WeightInfo = weights::pallet_dapp_staking_v3::SubstrateWeight<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = DAppStakingBenchmarkHelper<SmartContract<AccountId>, AccountId>;
}

pub struct InflationPayoutPerBlock;
impl pallet_inflation::PayoutPerBlock<Credit<AccountId, Balances>> for InflationPayoutPerBlock {
    fn treasury(reward: Credit<AccountId, Balances>) {
        let _ = Balances::resolve(&TreasuryPalletId::get().into_account_truncating(), reward);
    }

    fn collators(reward: Credit<AccountId, Balances>) {
        ToStakingPot::on_unbalanced(reward);
    }
}

pub struct InflationCycleConfig;
impl CycleConfiguration for InflationCycleConfig {
    fn periods_per_cycle() -> u32 {
        3
    }

    fn eras_per_voting_subperiod() -> u32 {
        11
    }

    fn eras_per_build_and_earn_subperiod() -> u32 {
        111
    }

    fn blocks_per_era() -> BlockNumber {
        24 * HOURS
    }
}

impl pallet_inflation::Config for Runtime {
    type Currency = Balances;
    type PayoutPerBlock = InflationPayoutPerBlock;
    type CycleConfiguration = InflationCycleConfig;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::pallet_inflation::SubstrateWeight<Runtime>;
}

impl pallet_utility::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnSystemEvent = ();
    type SelfParaId = parachain_info::Pallet<Runtime>;
    type OutboundXcmpMessageSource = XcmpQueue;
    type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
    type ReservedDmpWeight = ReservedDmpWeight;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
    type CheckAssociatedRelayNumber = cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases;
    type ConsensusHook = ConsensusHook;
    type WeightInfo = cumulus_pallet_parachain_system::weights::SubstrateWeight<Runtime>;
}

type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
    Runtime,
    RELAY_CHAIN_SLOT_DURATION_MILLIS,
    BLOCK_PROCESSING_VELOCITY,
    UNINCLUDED_SEGMENT_CAPACITY,
>;

impl parachain_info::Config for Runtime {}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<250>;
    type AllowMultipleBlocksPerSlot = ConstBool<false>;
    type SlotDuration = ConstU64<SLOT_DURATION>;
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type EventHandler = (CollatorSelection,);
}

parameter_types! {
    pub const SessionPeriod: BlockNumber = HOURS;
    pub const SessionOffset: BlockNumber = 0;
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ShouldEndSession = pallet_session::PeriodicSessions<SessionPeriod, SessionOffset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<SessionPeriod, SessionOffset>;
    type SessionManager = CollatorSelection;
    type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const PotId: PalletId = PalletId(*b"PotStake");
    pub const MaxCandidates: u32 = 148;
    pub const MinCandidates: u32 = 5;
    pub const MaxInvulnerables: u32 = 48;
    pub const SlashRatio: Perbill = Perbill::from_percent(1);
    pub const KickThreshold: BlockNumber = 2 * HOURS; // 2 SessionPeriod
}

pub struct CollatorSelectionAccountCheck;
impl pallet_collator_selection::AccountCheck<AccountId> for CollatorSelectionAccountCheck {
    fn allowed_candidacy(account: &AccountId) -> bool {
        !DappStaking::is_staker(account)
    }
}

impl pallet_collator_selection::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type UpdateOrigin = EnsureRoot<AccountId>;
    type PotId = PotId;
    type MaxCandidates = MaxCandidates;
    type MinCandidates = MinCandidates;
    type MaxInvulnerables = MaxInvulnerables;
    // should be a multiple of session or things will get inconsistent
    type KickThreshold = KickThreshold;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ValidatorRegistration = Session;
    type ValidatorSet = Session;
    type SlashRatio = SlashRatio;
    type AccountCheck = CollatorSelectionAccountCheck;
    type WeightInfo = pallet_collator_selection::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const DappsStakingPalletId: PalletId = PalletId(*b"py/dpsst");
    pub TreasuryAccountId: AccountId = TreasuryPalletId::get().into_account_truncating();
}

pub struct ToStakingPot;
impl OnUnbalanced<Credit<AccountId, Balances>> for ToStakingPot {
    fn on_nonzero_unbalanced(amount: Credit<AccountId, Balances>) {
        let staking_pot = PotId::get().into_account_truncating();
        let _ = Balances::resolve(&staking_pot, amount);
    }
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1_000_000;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Runtime>;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = ConstU32<1>;
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

parameter_types! {
    pub const AssetDeposit: Balance = 1000 * ASTR;
    pub const AssetsStringLimit: u32 = 50;
    /// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
    // https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
    pub const MetadataDepositBase: Balance = deposit(1, 68);
    pub const MetadataDepositPerByte: Balance = deposit(0, 1);
    pub const AssetAccountDeposit: Balance = deposit(1, 18);
}

impl pallet_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = AssetId;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type AssetAccountDeposit = AssetAccountDeposit;
    type ApprovalDeposit = ExistentialDeposit;
    type StringLimit = AssetsStringLimit;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = weights::pallet_assets::SubstrateWeight<Runtime>;
    type RemoveItemsLimit = ConstU32<1000>;
    type AssetIdParameter = Compact<AssetId>;
    type CallbackHandle = EvmRevertCodeHandler<Self, Self>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = astar_primitives::benchmarks::AssetsBenchmarkHelper;
}

parameter_types! {
    pub const MinVestedTransfer: Balance = 100 * ASTR;
    pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
        WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BlockNumberToBalance = ConvertInto;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
    type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
    type BlockNumberProvider = System;
    // `VestingInfo` encode length is 36bytes. 28 schedules gets encoded as 1009 bytes, which is the
    // highest number of schedules that encodes less than 2^10.
    const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
    pub const DepositPerItem: Balance = contracts_deposit(1, 0);
    pub const DepositPerByte: Balance = contracts_deposit(0, 1);
    // Fallback value if storage deposit limit not set by the user
    pub const DefaultDepositLimit: Balance = contracts_deposit(16, 16 * 1024);
    pub const MaxDelegateDependencies: u32 = 32;
    pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(10);
    pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
}

/// Codes using the randomness functionality cannot be uploaded. Neither can contracts
/// be instantiated from existing codes that use this deprecated functionality.
///
/// But since some `Randomness` config type is still required for `pallet-contracts`, we provide this dummy type.
pub struct DummyDeprecatedRandomness;
impl Randomness<Hash, BlockNumber> for DummyDeprecatedRandomness {
    fn random(_: &[u8]) -> (Hash, BlockNumber) {
        (Default::default(), Zero::zero())
    }
}

impl pallet_contracts::Config for Runtime {
    type Time = Timestamp;
    type Randomness = DummyDeprecatedRandomness;
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
    type CallFilter = Nothing;
    type DepositPerItem = DepositPerItem;
    type DepositPerByte = DepositPerByte;
    type DefaultDepositLimit = DefaultDepositLimit;
    type CallStack = [pallet_contracts::Frame<Self>; 5];
    type WeightPrice = pallet_transaction_payment::Pallet<Self>;
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type ChainExtension = AstarChainExtensions<Self>;
    type Schedule = Schedule;
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
    type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
    type MaxStorageKeyLen = ConstU32<128>;
    type UnsafeUnstableInterface = ConstBool<false>;
    type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
    type MaxDelegateDependencies = MaxDelegateDependencies;
    type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
    type Debug = ();
    type Environment = ();
    type Migrations = (pallet_contracts::migration::v16::Migration<Runtime>,);
    type Xcm = ();
    type UploadOrigin = EnsureSigned<<Self as frame_system::Config>::AccountId>;
    type InstantiateOrigin = EnsureSigned<<Self as frame_system::Config>::AccountId>;
    type ApiVersion = ();
}

// These values are based on the Astar 2.0 Tokenomics Modeling report.
parameter_types! {
    pub const TransactionLengthFeeFactor: Balance = 23_500_000_000_000; // 0.0000235 ASTR per byte
    pub const WeightFeeFactor: Balance = 30_855_000_000_000_000; // Around 0.03 ASTR per unit of ref time.
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub const OperationalFeeMultiplier: u8 = 5;
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(000_015, 1_000_000); // 0.000_015
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 10); // 0.1
    pub MaximumMultiplier: Multiplier = Multiplier::saturating_from_integer(10); // 10
}

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - [0, MAXIMUM_BLOCK_WEIGHT]
///   - [Balance::min, Balance::max]
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
    type Balance = Balance;
    fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
        let p = WeightFeeFactor::get();
        let q = Balance::from(ExtrinsicBaseWeight::get().ref_time());
        smallvec::smallvec![WeightToFeeCoefficient {
            degree: 1,
            negative: false,
            coeff_frac: Perbill::from_rational(p % q, q),
            coeff_integer: p / q,
        }]
    }
}

/// Handles coverting weight consumed by XCM into native currency fee.
///
/// Similar to standard `WeightToFee` handler, but force uses the minimum multiplier.
pub struct XcmWeightToFee;
impl WeightToFeeT for XcmWeightToFee {
    type Balance = Balance;

    fn weight_to_fee(n: &Weight) -> Self::Balance {
        MinimumMultiplier::get().saturating_mul_int(WeightToFee::weight_to_fee(&n))
    }
}

pub struct DealWithFees;
impl OnUnbalanced<Credit<AccountId, Balances>> for DealWithFees {
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = Credit<AccountId, Balances>>) {
        if let Some(fees) = fees_then_tips.next() {
            // Burn 80% of fees, rest goes to collator, including 100% of the tips.
            let (to_burn, mut collator) = fees.ration(80, 20);
            if let Some(tips) = fees_then_tips.next() {
                tips.merge_into(&mut collator);
            }

            // burn part of the fees
            drop(to_burn);

            // pay fees to collator
            <ToStakingPot as OnUnbalanced<_>>::on_unbalanced(collator);
        }
    }
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, DealWithFees>;
    type WeightToFee = WeightToFee;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
    type FeeMultiplierUpdate = TargetedFeeAdjustment<
        Self,
        TargetBlockFullness,
        AdjustmentVariable,
        MinimumMultiplier,
        MaximumMultiplier,
    >;
    type LengthToFee = ConstantMultiplier<Balance, TransactionLengthFeeFactor>;
}

parameter_types! {
    pub DefaultBaseFeePerGas: U256 = U256::from(1_470_000_000_000_u128);
    pub MinBaseFeePerGas: U256 = U256::from(800_000_000_000_u128);
    pub MaxBaseFeePerGas: U256 = U256::from(80_000_000_000_000_u128);
    pub StepLimitRatio: Perquintill = Perquintill::from_rational(93_u128, 1_000_000);
}

/// Simple wrapper for fetching current native transaction fee weight fee multiplier.
pub struct AdjustmentFactorGetter;
impl Get<Multiplier> for AdjustmentFactorGetter {
    fn get() -> Multiplier {
        pallet_transaction_payment::NextFeeMultiplier::<Runtime>::get()
    }
}

impl pallet_dynamic_evm_base_fee::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
    type MinBaseFeePerGas = MinBaseFeePerGas;
    type MaxBaseFeePerGas = MaxBaseFeePerGas;
    type AdjustmentFactor = AdjustmentFactorGetter;
    type WeightFactor = WeightFeeFactor;
    type StepLimitRatio = StepLimitRatio;
    type WeightInfo = pallet_dynamic_evm_base_fee::weights::SubstrateWeight<Runtime>;
}

/// Current approximation of the gas/s consumption considering
/// EVM execution over compiled WASM (on 4.4Ghz CPU).
/// Given the 500ms Weight, from which 75% only are used for transactions,
/// the total EVM execution gas limit is: GAS_PER_SECOND * 0.500 * 0.75 ~= 15_000_000.
pub const GAS_PER_SECOND: u64 = 40_000_000;

/// Approximate ratio of the amount of Weight per Gas.
/// u64 works for approximations because Weight is a very small unit compared to gas.
pub const WEIGHT_PER_GAS: u64 = WEIGHT_REF_TIME_PER_SECOND.saturating_div(GAS_PER_SECOND);

pub struct FindAuthorTruncated<F>(sp_std::marker::PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for FindAuthorTruncated<F> {
    fn find_author<'a, I>(digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        if let Some(author_index) = F::find_author(digests) {
            let authority_id =
                pallet_aura::Authorities::<Runtime>::get()[author_index as usize].clone();
            return Some(H160::from_slice(&authority_id.encode()[4..24]));
        }

        None
    }
}

parameter_types! {
    /// Ethereum-compatible chain_id:
    /// * Dusty:   80
    /// * Shibuya: 81
    /// * Shiden: 336
    /// * Astar:  592
    pub ChainId: u64 = 0x250;
    /// EVM gas limit
    pub BlockGasLimit: U256 = U256::from(
        NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT.ref_time() / WEIGHT_PER_GAS
    );
    pub PrecompilesValue: Precompiles = AstarPrecompiles::<_, _>::new();
    pub WeightPerGas: Weight = Weight::from_parts(WEIGHT_PER_GAS, 0);
    /// The amount of gas per PoV size. Value is calculated as:
    ///
    /// max_gas_limit = max_tx_ref_time / WEIGHT_PER_GAS = max_pov_size * gas_limit_pov_size_ratio
    /// gas_limit_pov_size_ratio = ceil((max_tx_ref_time / WEIGHT_PER_GAS) / max_pov_size)
    ///
    /// Equals 4 for values used by Astar runtime.
    pub const GasLimitPovSizeRatio: u64 = 4;
}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = DynamicEvmBaseFee;
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Runtime>;
    type CallOrigin = pallet_evm::EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = pallet_evm::EnsureAddressTruncated;
    type AddressMapping = pallet_evm::HashedAddressMapping<BlakeTwo256>;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = Precompiles;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = ChainId;
    type OnChargeTransaction = pallet_evm::EVMFungibleAdapter<Balances, ToStakingPot>;
    type BlockGasLimit = BlockGasLimit;
    type Timestamp = Timestamp;
    type OnCreate = ();
    type FindAuthor = FindAuthorTruncated<Aura>;
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
    type SuicideQuickClearLimit = ConstU32<0>;
    type WeightInfo = pallet_evm::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
    type PostLogContent = PostBlockAndTxnHashes;
    // Maximum length (in bytes) of revert message to include in Executed event
    type ExtraDataLength = ConstU32<30>;
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

impl pallet_xc_asset_config::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type ManagerOrigin = EnsureRoot<AccountId>;
    type WeightInfo = pallet_xc_asset_config::weights::SubstrateWeight<Self>;
}

parameter_types! {
    pub MessageQueueServiceWeight: Weight =
        Perbill::from_percent(25) * RuntimeBlockWeights::get().max_block;
}

impl pallet_message_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_message_queue::weights::SubstrateWeight<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
        cumulus_primitives_core::AggregateMessageOrigin,
    >;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type MessageProcessor = xcm_builder::ProcessXcmMessage<
        AggregateMessageOrigin,
        xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
        RuntimeCall,
    >;
    type Size = u32;
    type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
    type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
    type HeapSize = ConstU32<{ 128 * 1048 }>;
    type MaxStale = ConstU32<8>;
    type ServiceWeight = MessageQueueServiceWeight;
    type IdleMaxServiceWeight = MessageQueueServiceWeight;
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
    RuntimeDebug,
    MaxEncodedLen,
    scale_info::TypeInfo,
)]
pub enum ProxyType {
    /// Allows all runtime calls for proxy account
    Any,
    /// Allows only NonTransfer runtime calls for proxy account
    /// To know exact calls check InstanceFilter inmplementation for ProxyTypes
    NonTransfer,
    /// All Runtime calls from Pallet Balances allowed for proxy account
    Balances,
    /// All Runtime calls from Pallet Assets allowed for proxy account
    Assets,
    /// Only provide_judgement call from pallet identity allowed for proxy account
    IdentityJudgement,
    /// Only reject_announcement call from pallet proxy allowed for proxy account
    CancelProxy,
    /// All runtime calls from pallet DappStaking allowed for proxy account
    DappStaking,
    /// Only claim_staker call from pallet DappStaking allowed for proxy account
    StakerRewardClaim,
}

impl Default for ProxyType {
    fn default() -> Self {
        Self::Any
    }
}

impl InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            // Always allowed RuntimeCall::Utility no matter type.
            // Only transactions allowed by Proxy.filter can be executed
            _ if matches!(c, RuntimeCall::Utility(..)) => true,
            ProxyType::Any => true,
            ProxyType::NonTransfer => {
                matches!(
                    c,
                    RuntimeCall::System(..)
                        | RuntimeCall::Identity(..)
                        | RuntimeCall::Timestamp(..)
                        | RuntimeCall::Multisig(..)
                        | RuntimeCall::Proxy(..)
                        | RuntimeCall::ParachainSystem(..)
                        | RuntimeCall::ParachainInfo(..)
                        // Skip entire Balances pallet
                        | RuntimeCall::Vesting(pallet_vesting::Call::vest{..})
				        | RuntimeCall::Vesting(pallet_vesting::Call::vest_other{..})
				        // Specifically omitting Vesting `vested_transfer`, and `force_vested_transfer`
                        | RuntimeCall::DappStaking(..)
                        // Skip entire Assets pallet
                        | RuntimeCall::CollatorSelection(..)
                        | RuntimeCall::Session(..)
                        | RuntimeCall::XcmpQueue(..)
                        | RuntimeCall::PolkadotXcm(..)
                        | RuntimeCall::CumulusXcm(..)
                        | RuntimeCall::XcAssetConfig(..)
                        // Skip entire EVM pallet
                        // Skip entire Ethereum pallet
                        | RuntimeCall::DynamicEvmBaseFee(..) // Skip entire Contracts pallet
                )
            }
            ProxyType::Balances => {
                matches!(c, RuntimeCall::Balances(..))
            }
            ProxyType::Assets => {
                matches!(c, RuntimeCall::Assets(..))
            }
            ProxyType::IdentityJudgement => {
                matches!(
                    c,
                    RuntimeCall::Identity(pallet_identity::Call::provide_judgement { .. })
                )
            }
            ProxyType::CancelProxy => {
                matches!(
                    c,
                    RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. })
                )
            }
            ProxyType::DappStaking => {
                matches!(c, RuntimeCall::DappStaking(..))
            }
            ProxyType::StakerRewardClaim => {
                matches!(
                    c,
                    RuntimeCall::DappStaking(
                        pallet_dapp_staking_v3::Call::claim_staker_rewards { .. }
                    )
                )
            }
        }
    }

    fn is_superset(&self, o: &Self) -> bool {
        match (self, o) {
            (x, y) if x == y => true,
            (ProxyType::Any, _) => true,
            (_, ProxyType::Any) => false,
            (ProxyType::DappStaking, ProxyType::StakerRewardClaim) => true,
            _ => false,
        }
    }
}

parameter_types! {
    // One storage item; key size 32, value size 8; .
    pub const ProxyDepositBase: Balance = deposit(1, 8);
    // Additional storage item size of 33 bytes.
    pub const ProxyDepositFactor: Balance = deposit(0, 33);
    pub const MaxProxies: u16 = 32;
    pub const MaxPending: u16 = 32;
    pub const AnnouncementDepositBase: Balance = deposit(1, 8);
    pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
}

impl pallet_proxy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type ProxyType = ProxyType;
    type ProxyDepositBase = ProxyDepositBase;
    type ProxyDepositFactor = ProxyDepositFactor;
    type MaxProxies = MaxProxies;
    type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
    type MaxPending = MaxPending;
    type CallHasher = BlakeTwo256;
    type AnnouncementDepositBase = AnnouncementDepositBase;
    type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

parameter_types! {
    pub const NativeCurrencyId: CurrencyId = CurrencyId::ASTR;
    // Aggregate values for one day.
    pub const AggregationDuration: BlockNumber = 7200;
}

impl pallet_price_aggregator::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxValuesPerBlock = ConstU32<8>;
    type ProcessBlockValues = pallet_price_aggregator::MedianBlockValue;
    type NativeCurrencyId = NativeCurrencyId;
    // 7 days
    type CircularBufferLength = ConstU32<7>;
    type AggregationDuration = AggregationDuration;
    type WeightInfo = pallet_price_aggregator::weights::SubstrateWeight<Runtime>;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct OracleBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl orml_oracle::BenchmarkHelper<CurrencyId, Price, ConstU32<2>> for OracleBenchmarkHelper {
    fn get_currency_id_value_pairs() -> sp_runtime::BoundedVec<(CurrencyId, Price), ConstU32<2>> {
        sp_runtime::BoundedVec::try_from(vec![
            (CurrencyId::ASTR, Price::from_rational(15, 100)),
            (CurrencyId::ASTR, Price::from_rational(15, 100)),
        ])
        .expect("out of bounds")
    }
}

parameter_types! {
    // Cannot specify `Root` so need to do it like this, unfortunately.
    pub RootOperatorAccountId: AccountId = AccountId::from([0xffu8; 32]);
}

impl orml_oracle::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnNewData = PriceAggregator;
    type CombineData = DummyCombineData<Runtime>;
    type Time = Timestamp;
    type OracleKey = CurrencyId;
    type OracleValue = Price;
    type RootOperatorAccountId = RootOperatorAccountId;
    #[cfg(feature = "runtime-benchmarks")]
    type Members = OracleMembershipWrapper;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type Members = OracleMembership;
    type MaxHasDispatchedSize = ConstU32<8>;
    type WeightInfo = weights::orml_oracle::SubstrateWeight<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type MaxFeedValues = ConstU32<2>;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type MaxFeedValues = ConstU32<1>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = OracleBenchmarkHelper;
}

impl pallet_membership::Config<OracleMembershipInst> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AddOrigin = EnsureRoot<AccountId>;
    type RemoveOrigin = EnsureRoot<AccountId>;
    type SwapOrigin = EnsureRoot<AccountId>;
    type ResetOrigin = EnsureRoot<AccountId>;
    type PrimeOrigin = EnsureRoot<AccountId>;

    type MembershipInitialized = ();
    type MembershipChanged = ();
    type MaxMembers = ConstU32<16>;
    type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

/// OracleMembership wrapper used by benchmarks
#[cfg(feature = "runtime-benchmarks")]
pub struct OracleMembershipWrapper;

#[cfg(feature = "runtime-benchmarks")]
impl frame_support::traits::SortedMembers<AccountId> for OracleMembershipWrapper {
    fn sorted_members() -> Vec<AccountId> {
        OracleMembership::sorted_members()
    }

    fn add(account: &AccountId) {
        frame_support::assert_ok!(OracleMembership::add_member(
            frame_system::RawOrigin::Root.into(),
            account.to_owned().into()
        ));
    }
}

parameter_types! {
    pub MbmServiceWeight: Weight = Perbill::from_percent(80) * RuntimeBlockWeights::get().max_block;
}

impl pallet_migrations::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type Migrations = ();
    // Benchmarks need mocked migrations to guarantee that they succeed.
    #[cfg(feature = "runtime-benchmarks")]
    type Migrations = pallet_migrations::mock_helpers::MockedMigrations;
    type CursorMaxLen = ConstU32<65_536>;
    type IdentifierMaxLen = ConstU32<256>;
    type MigrationStatusHandler = ();
    type FailedMigrationHandler = UnfreezeChainOnFailedMigration;
    type MaxServiceWeight = MbmServiceWeight;
    type WeightInfo = pallet_migrations::weights::SubstrateWeight<Runtime>;
}

construct_runtime!(
    pub struct Runtime
    {
        System: frame_system = 10,
        Utility: pallet_utility = 11,
        Identity: pallet_identity = 12,
        Timestamp: pallet_timestamp = 13,
        Multisig: pallet_multisig = 14,
        Proxy: pallet_proxy = 15,

        ParachainSystem: cumulus_pallet_parachain_system = 20,
        ParachainInfo: parachain_info = 21,

        TransactionPayment: pallet_transaction_payment = 30,
        Balances: pallet_balances = 31,
        Vesting: pallet_vesting = 32,
// Inflation needs to execute `on_initialize` as soon as possible, and `on_finalize` as late as possible.
        // However, we need to execute Balance genesis before Inflation genesis, otherwise we'll have zero issuance when Inflation
        // logic is executed.
        // TODO: Address this later. It would be best if Inflation was first pallet.
        Inflation: pallet_inflation = 33,
        DappStaking: pallet_dapp_staking_v3 = 34,
        Assets: pallet_assets = 36,
        PriceAggregator: pallet_price_aggregator = 37,
        Oracle: orml_oracle = 38,
        OracleMembership: pallet_membership::<Instance1> = 39,

        Authorship: pallet_authorship = 40,
        CollatorSelection: pallet_collator_selection = 41,
        Session: pallet_session = 42,
        Aura: pallet_aura = 43,
        AuraExt: cumulus_pallet_aura_ext = 44,

        XcmpQueue: cumulus_pallet_xcmp_queue = 50,
        PolkadotXcm: pallet_xcm = 51,
        CumulusXcm: cumulus_pallet_xcm = 52,
        // skip 53 - cumulus_pallet_dmp_queue previously
        XcAssetConfig: pallet_xc_asset_config = 54,
        XTokens: orml_xtokens = 55,
        MessageQueue: pallet_message_queue = 56,

        EVM: pallet_evm = 60,
        Ethereum: pallet_ethereum = 61,
        DynamicEvmBaseFee: pallet_dynamic_evm_base_fee = 63,

        Contracts: pallet_contracts = 70,

        Sudo: pallet_sudo = 99,

        MultiBlockMigrations: pallet_migrations = 120,
    }
);

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
    frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic =
    fp_self_contained::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra, H160>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    Migrations,
>;

parameter_types! {
    // Threshold amount variation allowed for this migration - 10%
    pub const ThresholdVariationPercentage: u32 = 10;
    // percentages below are calculated based on total issuance at the time when dApp staking v3 was launched (8.4B)
    pub const TierThresholds: [TierThreshold; 4] = [
        TierThreshold::DynamicPercentage {
            percentage: Perbill::from_parts(35_700_000), // 3.57%
            minimum_required_percentage: Perbill::from_parts(23_800_000), // 2.38%
        },
        TierThreshold::DynamicPercentage {
            percentage: Perbill::from_parts(8_900_000), // 0.89%
            minimum_required_percentage: Perbill::from_parts(6_000_000), // 0.6%
        },
        TierThreshold::DynamicPercentage {
            percentage: Perbill::from_parts(2_380_000), // 0.238%
            minimum_required_percentage: Perbill::from_parts(1_790_000), // 0.179%
        },
        TierThreshold::FixedPercentage {
            required_percentage: Perbill::from_parts(200_000), // 0.02%
        },
    ];
}

parameter_types! {
    pub const DmpQueuePalletName: &'static str = "DmpQueue";
}

/// All migrations that will run on the next runtime upgrade.
///
/// __NOTE:__ THE ORDER IS IMPORTANT.
pub type Migrations = (Unreleased, Permanent);

/// Unreleased migrations. Add new ones here:
pub type Unreleased = (
    // dApp-staking dyn tier threshold migrations
    pallet_dapp_staking_v3::migration::versioned_migrations::V7ToV8<
        Runtime,
        TierThresholds,
        ThresholdVariationPercentage,
    >,
    frame_support::migrations::RemovePallet<
        DmpQueuePalletName,
        <Runtime as frame_system::Config>::DbWeight,
    >,
    pallet_contracts::Migration<Runtime>,
);

/// Migrations/checks that do not need to be versioned and can run on every upgrade.
pub type Permanent = (pallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,);

type EventRecord = frame_system::EventRecord<
    <Runtime as frame_system::Config>::RuntimeEvent,
    <Runtime as frame_system::Config>::Hash,
>;

impl fp_self_contained::SelfContainedCall for RuntimeCall {
    type SignedInfo = H160;

    fn is_self_contained(&self) -> bool {
        match self {
            RuntimeCall::Ethereum(call) => call.is_self_contained(),
            _ => false,
        }
    }

    fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => call.check_self_contained(),
            _ => None,
        }
    }

    fn validate_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<RuntimeCall>,
        len: usize,
    ) -> Option<TransactionValidity> {
        match self {
            RuntimeCall::Ethereum(call) => call.validate_self_contained(info, dispatch_info, len),
            _ => None,
        }
    }

    fn pre_dispatch_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<RuntimeCall>,
        len: usize,
    ) -> Option<Result<(), TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => {
                call.pre_dispatch_self_contained(info, dispatch_info, len)
            }
            _ => None,
        }
    }

    fn apply_self_contained(
        self,
        info: Self::SignedInfo,
    ) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
        match self {
            call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => {
                Some(call.dispatch(RuntimeOrigin::from(
                    pallet_ethereum::RawOrigin::EthereumTransaction(info),
                )))
            }
            _ => None,
        }
    }
}

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
    define_benchmarks!(
        [frame_benchmarking, BaselineBench::<Runtime>]
        [frame_system, SystemBench::<Runtime>]
        [pallet_assets, pallet_assets::Pallet::<Runtime>]
        [pallet_balances, Balances]
        [pallet_timestamp, Timestamp]
        [pallet_dapp_staking_v3, DappStaking]
        [pallet_inflation, Inflation]
        [pallet_migrations, MultiBlockMigrations]
        [pallet_xc_asset_config, XcAssetConfig]
        [pallet_collator_selection, CollatorSelection]
        [pallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
        [pallet_dynamic_evm_base_fee, DynamicEvmBaseFee]
        [xcm_benchmarks_generic, XcmGeneric]
        [xcm_benchmarks_fungible, XcmFungible]
        [orml_oracle, Oracle]
    );
}

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> sp_std::vec::Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
        }

        fn authorities() -> Vec<AuraId> {
            pallet_aura::Authorities::<Runtime>::get().into_inner()
        }
    }

    impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
        fn can_build_upon(
            included_hash: <Block as BlockT>::Hash,
            slot: cumulus_primitives_aura::Slot,
        ) -> bool {
            ConsensusHook::can_build_upon(included_hash, slot)
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(block: Block, data: InherentData) -> CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
        Block,
        Balance,
    > for Runtime {
        fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
        for Runtime
    {
        fn query_call_info(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_call_info(call, len)
        }
        fn query_call_fee_details(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_call_fee_details(call, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }

        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
        fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
            ParachainSystem::collect_collation_info(header)
        }
    }

    impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
        fn chain_id() -> u64 {
            ChainId::get()
        }

        fn account_basic(address: H160) -> pallet_evm::Account {
            let (account, _) = EVM::account_basic(&address);
            account
        }

        fn gas_price() -> U256 {
            let (gas_price, _) = <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price();
            gas_price
        }

        fn account_code_at(address: H160) -> Vec<u8> {
            pallet_evm::AccountCodes::<Runtime>::get(address)
        }

        fn author() -> H160 {
            <pallet_evm::Pallet<Runtime>>::find_author()
        }

        fn storage_at(address: H160, index: U256) -> H256 {
            let mut tmp = [0u8; 32];
            index.to_big_endian(&mut tmp);
            pallet_evm::AccountStorages::<Runtime>::get(address, H256::from_slice(&tmp[..]))
        }

        fn call(
            from: H160,
            to: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            max_fee_per_gas: Option<U256>,
            max_priority_fee_per_gas: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };

            let is_transactional = false;
            let validate = true;

            // Reused approach from Moonbeam since Frontier implementation doesn't support this
            let mut estimated_transaction_len = data.len() +
                // to: 20
                // from: 20
                // value: 32
                // gas_limit: 32
                // nonce: 32
                // 1 byte transaction action variant
                // chain id 8 bytes
                // 65 bytes signature
                210;
            if max_fee_per_gas.is_some() {
                estimated_transaction_len += 32;
            }
            if max_priority_fee_per_gas.is_some() {
                estimated_transaction_len += 32;
            }
            if access_list.is_some() {
                estimated_transaction_len += access_list.encoded_size();
            }

            let gas_limit = gas_limit.min(u64::MAX.into()).low_u64();
            let without_base_extrinsic_weight = true;

            let (weight_limit, proof_size_base_cost) =
                match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
                    gas_limit,
                    without_base_extrinsic_weight
                ) {
                    weight_limit if weight_limit.proof_size() > 0 => {
                        (Some(weight_limit), Some(estimated_transaction_len as u64))
                    }
                    _ => (None, None),
                };

            <Runtime as pallet_evm::Config>::Runner::call(
                from,
                to,
                data,
                value,
                gas_limit.unique_saturated_into(),
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                Vec::new(),
                is_transactional,
                validate,
                weight_limit,
                proof_size_base_cost,
                config
                    .as_ref()
                    .unwrap_or_else(|| <Runtime as pallet_evm::Config>::config()),
            )
            .map_err(|err| err.error.into())
        }

        fn create(
            from: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            max_fee_per_gas: Option<U256>,
            max_priority_fee_per_gas: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };

            let is_transactional = false;
            let validate = true;

            // Reused approach from Moonbeam since Frontier implementation doesn't support this
            let mut estimated_transaction_len = data.len() +
                // to: 20
                // from: 20
                // value: 32
                // gas_limit: 32
                // nonce: 32
                // 1 byte transaction action variant
                // chain id 8 bytes
                // 65 bytes signature
                210;
            if max_fee_per_gas.is_some() {
                estimated_transaction_len += 32;
            }
            if max_priority_fee_per_gas.is_some() {
                estimated_transaction_len += 32;
            }
            if access_list.is_some() {
                estimated_transaction_len += access_list.encoded_size();
            }

            let gas_limit = gas_limit.min(u64::MAX.into()).low_u64();
            let without_base_extrinsic_weight = true;

            let (weight_limit, proof_size_base_cost) =
                match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
                    gas_limit,
                    without_base_extrinsic_weight
                ) {
                    weight_limit if weight_limit.proof_size() > 0 => {
                        (Some(weight_limit), Some(estimated_transaction_len as u64))
                    }
                    _ => (None, None),
                };

            #[allow(clippy::or_fun_call)] // suggestion not helpful here
            <Runtime as pallet_evm::Config>::Runner::create(
                from,
                data,
                value,
                gas_limit.unique_saturated_into(),
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                Vec::new(),
                is_transactional,
                validate,
                weight_limit,
                proof_size_base_cost,
                config
                    .as_ref()
                    .unwrap_or(<Runtime as pallet_evm::Config>::config()),
                )
                .map_err(|err| err.error.into())
        }

        fn current_transaction_statuses() -> Option<Vec<fp_rpc::TransactionStatus>> {
            pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
        }

        fn current_block() -> Option<pallet_ethereum::Block> {
            pallet_ethereum::CurrentBlock::<Runtime>::get()
        }

        fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
            pallet_ethereum::CurrentReceipts::<Runtime>::get()
        }

        fn current_all() -> (
            Option<pallet_ethereum::Block>,
            Option<Vec<pallet_ethereum::Receipt>>,
            Option<Vec<fp_rpc::TransactionStatus>>,
        ) {
            (
                pallet_ethereum::CurrentBlock::<Runtime>::get(),
                pallet_ethereum::CurrentReceipts::<Runtime>::get(),
                pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
            )
        }

        fn extrinsic_filter(
            xts: Vec<<Block as BlockT>::Extrinsic>,
        ) -> Vec<pallet_ethereum::Transaction> {
            xts.into_iter().filter_map(|xt| match xt.0.function {
                RuntimeCall::Ethereum(pallet_ethereum::Call::transact { transaction }) => Some(transaction),
                _ => None
            }).collect::<Vec<pallet_ethereum::Transaction>>()
        }

        fn elasticity() -> Option<Permill> {
            Some(Permill::zero())
        }

        fn gas_limit_multiplier_support() {}

        fn pending_block(
            xts: Vec<<Block as BlockT>::Extrinsic>,
        ) -> (Option<pallet_ethereum::Block>, Option<Vec<fp_rpc::TransactionStatus>>) {
            for ext in xts.into_iter() {
                let _ = Executive::apply_extrinsic(ext);
            }

            Ethereum::on_finalize(System::block_number() + 1);

            (
                pallet_ethereum::CurrentBlock::<Runtime>::get(),
                pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
            )
        }
    }

    impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
        fn convert_transaction(
            transaction: pallet_ethereum::Transaction
        ) -> <Block as BlockT>::Extrinsic {
            UncheckedExtrinsic::new_unsigned(
                pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
            )
        }
    }

    impl pallet_contracts::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash, EventRecord> for Runtime {
        fn call(
            origin: AccountId,
            dest: AccountId,
            value: Balance,
            gas_limit: Option<Weight>,
            storage_deposit_limit: Option<Balance>,
            input_data: Vec<u8>,
        ) -> pallet_contracts::ContractExecResult<Balance, EventRecord> {
            let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
            Contracts::bare_call(
                origin,
                dest,
                value,
                gas_limit,
                storage_deposit_limit,
                input_data,
                pallet_contracts::DebugInfo::UnsafeDebug,
                pallet_contracts::CollectEvents::UnsafeCollect,
                pallet_contracts::Determinism::Enforced,
            )
        }

        fn instantiate(
            origin: AccountId,
            value: Balance,
            gas_limit: Option<Weight>,
            storage_deposit_limit: Option<Balance>,
            code: pallet_contracts::Code<Hash>,
            data: Vec<u8>,
            salt: Vec<u8>,
        ) -> pallet_contracts::ContractInstantiateResult<AccountId, Balance, EventRecord> {
            let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
            Contracts::bare_instantiate(
                origin,
                value,
                gas_limit,
                storage_deposit_limit,
                code,
                data,
                salt,
                pallet_contracts::DebugInfo::UnsafeDebug,
                pallet_contracts::CollectEvents::UnsafeCollect,
            )
        }

        fn upload_code(
            origin: AccountId,
            code: Vec<u8>,
            storage_deposit_limit: Option<Balance>,
            determinism: pallet_contracts::Determinism,
        ) -> pallet_contracts::CodeUploadResult<Hash, Balance>
        {
            Contracts::bare_upload_code(origin, code, storage_deposit_limit, determinism)
        }

        fn get_storage(
            address: AccountId,
            key: Vec<u8>,
        ) -> pallet_contracts::GetStorageResult {
            Contracts::get_storage(address, key)
        }
    }

    impl dapp_staking_v3_runtime_api::DappStakingApi<Block> for Runtime {
        fn periods_per_cycle() -> PeriodNumber {
            InflationCycleConfig::periods_per_cycle()
        }

        fn eras_per_voting_subperiod() -> EraNumber {
            InflationCycleConfig::eras_per_voting_subperiod()
        }

        fn eras_per_build_and_earn_subperiod() -> EraNumber {
            InflationCycleConfig::eras_per_build_and_earn_subperiod()
        }

        fn blocks_per_era() -> BlockNumber {
            InflationCycleConfig::blocks_per_era()
        }

        fn get_dapp_tier_assignment() -> BTreeMap<DAppId, RankedTier> {
            DappStaking::get_dapp_tier_assignment()
        }
    }

    impl xcm_fee_payment_runtime_api::XcmPaymentApi<Block> for Runtime {
        fn query_acceptable_payment_assets(xcm_version: xcm::Version) -> Result<Vec<VersionedAssetId>, XcmPaymentApiError> {
            if !matches!(xcm_version, xcm::v3::VERSION | xcm::v4::VERSION) {
                return Err(XcmPaymentApiError::UnhandledXcmVersion);
            }

            // Native asset is always supported
            let native_asset_location: XcmLocation = XcmLocation::try_from(xcm_config::AstarLocation::get())
            .map_err(|_| XcmPaymentApiError::VersionedConversionFailed)?;

            Ok([VersionedAssetId::V4(native_asset_location.into())]
                .into_iter()
                // Acquire foreign assets which have 'units per second' configured
                .chain(
                    pallet_xc_asset_config::AssetLocationUnitsPerSecond::<Runtime>::iter_keys().filter_map(|asset_location| {

                        match XcmLocation::try_from(asset_location) {
                            Ok(asset) => Some(VersionedAssetId::V4(asset.into())),
                            Err(_) => None,
                        }
                    })
            ).filter_map(|asset| asset.into_version(xcm_version).ok()).collect())
        }

        fn query_weight_to_asset_fee(weight: Weight, asset: VersionedAssetId) -> Result<u128, XcmPaymentApiError> {
            let native_asset_location = XcmLocation::try_from(xcm_config::AstarLocation::get())
                .map_err(|_| XcmPaymentApiError::VersionedConversionFailed)?;
            let native_asset = VersionedAssetId::V4(native_asset_location.into());

            let asset = asset
                .into_version(xcm::v4::VERSION)
                .map_err(|_| XcmPaymentApiError::VersionedConversionFailed)?;

            if native_asset == asset {
                Ok(XcmWeightToFee::weight_to_fee(&weight))
            } else {
                let asset_id: XcmAssetId = asset.try_into().map_err(|_| XcmPaymentApiError::VersionedConversionFailed)?;
                let versioned_location = VersionedLocation::V4(asset_id.0);

                match pallet_xc_asset_config::AssetLocationUnitsPerSecond::<Runtime>::get(versioned_location) {
                    Some(units_per_sec) => {
                        Ok(units_per_sec.saturating_mul(weight.ref_time() as u128)
                            / (WEIGHT_REF_TIME_PER_SECOND as u128))
                    }
                    None => Err(XcmPaymentApiError::AssetNotFound),
                }
            }
        }

        fn query_xcm_weight(message: VersionedXcm<()>) -> Result<Weight, XcmPaymentApiError> {
            PolkadotXcm::query_xcm_weight(message)
        }

        fn query_delivery_fees(destination: VersionedLocation, message: VersionedXcm<()>) -> Result<VersionedAssets, XcmPaymentApiError> {
            PolkadotXcm::query_delivery_fees(destination, message)
        }
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {

        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            get_preset::<RuntimeGenesisConfig>(id, |_| None)
        }

        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            vec![]
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;
            use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsicsBenchmark;
            use frame_system_benchmarking::Pallet as SystemBench;
            use baseline::Pallet as BaselineBench;

            // This is defined once again in dispatch_benchmark, because list_benchmarks!
            // and add_benchmarks! are macros exported by define_benchmarks! macros and those types
            // are referenced in that call.
            type XcmFungible = astar_xcm_benchmarks::fungible::benchmarking::XcmFungibleBenchmarks::<Runtime>;
            type XcmGeneric = astar_xcm_benchmarks::generic::benchmarking::XcmGenericBenchmarks::<Runtime>;

            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);

            let storage_info = AllPalletsWithSystem::storage_info();

            (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, BenchmarkError};
            use frame_system_benchmarking::Pallet as SystemBench;
            use frame_support::{traits::{WhitelistedStorageKeys, TrackedStorageKey, tokens::fungible::{ItemOf}}, assert_ok};
            use baseline::Pallet as BaselineBench;
            use xcm::latest::prelude::*;
            use xcm_builder::MintLocation;
            use astar_primitives::benchmarks::XcmBenchmarkHelper;
            use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsicsBenchmark;

            pub struct TestDeliveryHelper;
            impl xcm_builder::EnsureDelivery for TestDeliveryHelper {
                fn ensure_successful_delivery(
                    origin_ref: &Location,
                    _dest: &Location,
                    _fee_reason: xcm_executor::traits::FeeReason,
                ) -> (Option<xcm_executor::FeesMode>, Option<Assets>) {
                    use xcm_executor::traits::ConvertLocation;
                    let account = xcm_config::LocationToAccountId::convert_location(origin_ref)
                        .expect("Invalid location");
                    // Give the existential deposit at least
                    let balance = ExistentialDeposit::get();
                    let _ = <Balances as frame_support::traits::Currency<_>>::
                        make_free_balance_be(&account.into(), balance);

                    (None, None)
                }
            }

            impl pallet_xcm::benchmarking::Config for Runtime {
                type DeliveryHelper = TestDeliveryHelper;

                fn reachable_dest() -> Option<Location> {
                    Some(Parent.into())
                }

                fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
                    None
                }

                fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
                    let random_para_id = 43211234;
                    ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(
                        random_para_id.into()
                    );
                    Some((
                        Asset {
                            fun: Fungible(ExistentialDeposit::get()),
                            id: Here.into()
                        },
                        ParentThen(Parachain(random_para_id).into()).into(),
                    ))
                }

                fn get_asset() -> Asset {
                    Asset {
                        id: AssetId(Here.into()),
                        fun: Fungible(ExistentialDeposit::get()),
                    }
                }
            }

            impl frame_system_benchmarking::Config for Runtime {}
            impl baseline::Config for Runtime {}

            // XCM Benchmarks
            impl astar_xcm_benchmarks::Config for Runtime {}
            impl astar_xcm_benchmarks::generic::Config for Runtime {}
            impl astar_xcm_benchmarks::fungible::Config for Runtime {}

            impl pallet_xcm_benchmarks::Config for Runtime {
                type XcmConfig = xcm_config::XcmConfig;
                type AccountIdConverter = xcm_config::LocationToAccountId;
                type DeliveryHelper = ();

                // destination location to be used in benchmarks
                fn valid_destination() -> Result<Location, BenchmarkError> {
                    assert_ok!(PolkadotXcm::force_xcm_version(RuntimeOrigin::root(), Box::new(Location::parent()), xcm::v4::VERSION));
                    Ok(Location::parent())
                }
                fn worst_case_holding(_depositable_count: u32) -> Assets {
                   XcmBenchmarkHelper::<Runtime>::worst_case_holding()
                }
            }

            impl pallet_xcm_benchmarks::generic::Config for Runtime {
                type RuntimeCall = RuntimeCall;
                type TransactAsset = Balances;

                fn worst_case_response() -> (u64, Response) {
                    (0u64, Response::Version(Default::default()))
                }
                fn worst_case_asset_exchange()
                    -> Result<(Assets, Assets), BenchmarkError> {
                    Err(BenchmarkError::Skip)
                }

                fn universal_alias() -> Result<(Location, Junction), BenchmarkError> {
                    Err(BenchmarkError::Skip)
                }
                fn transact_origin_and_runtime_call()
                    -> Result<(Location, RuntimeCall), BenchmarkError> {
                    assert_ok!(PolkadotXcm::force_xcm_version(RuntimeOrigin::root(), Box::new(Location::parent()), xcm::v4::VERSION));
                    Ok((Location::parent(), frame_system::Call::remark_with_event {
                        remark: vec![]
                    }.into()))
                }
                fn subscribe_origin() -> Result<Location, BenchmarkError> {
                    assert_ok!(PolkadotXcm::force_xcm_version(RuntimeOrigin::root(), Box::new(Location::parent()), xcm::v4::VERSION));
                    Ok(Location::parent())
                }
                fn claimable_asset()
                    -> Result<(Location, Location, Assets), BenchmarkError> {
                    let origin = Location::parent();
                    let assets: Assets = (AssetId(Location::parent()), 1_000u128)
                        .into();
                    let ticket = Location { parents: 0, interior: Here };
                    Ok((origin, ticket, assets))
                }
                fn unlockable_asset()
                    -> Result<(Location, Location, Asset), BenchmarkError> {
                    Err(BenchmarkError::Skip)
                }
                fn export_message_origin_and_destination(
                ) -> Result<(Location, NetworkId, InteriorLocation), BenchmarkError> {
                    Err(BenchmarkError::Skip)
                }
                fn alias_origin() -> Result<(Location, Location), BenchmarkError> {
                    Err(BenchmarkError::Skip)
                }
                fn fee_asset() -> Result<Asset, BenchmarkError> {
                    Ok((AssetId(Here.into()), 1_000_000_000_000_000_000u128).into())
                }
            }

            parameter_types! {
                pub const NoCheckingAccount: Option<(AccountId, MintLocation)> = None;
                pub const NoTeleporter: Option<(Location, Asset)> = None;
                pub const TransactAssetId: u128 = 1001;
                pub TransactAssetLocation: Location = Location { parents: 0, interior: [GeneralIndex(TransactAssetId::get())].into() };

                pub TrustedReserveLocation: Location = Parent.into();
                pub TrustedReserveAsset: Asset = Asset { id: AssetId(TrustedReserveLocation::get()), fun: Fungible(1_000_000) };
                pub TrustedReserve: Option<(Location, Asset)> = Some((TrustedReserveLocation::get(), TrustedReserveAsset::get()));
            }

            impl pallet_xcm_benchmarks::fungible::Config for Runtime {
                type TransactAsset = ItemOf<pallet_assets::Pallet<Runtime>, TransactAssetId, AccountId>;
                type CheckedAccount = NoCheckingAccount;
                type TrustedTeleporter = NoTeleporter;
                type TrustedReserve = TrustedReserve;

                fn get_asset() -> Asset {
                    let min_balance = 100u128;
                    // create the transact asset and make it sufficient
                    assert_ok!(pallet_assets::Pallet::<Runtime>::force_create(
                        RuntimeOrigin::root(),
                        TransactAssetId::get().into(),
                        Address::Id([0u8; 32].into()),
                        true,
                        // min balance
                        min_balance
                    ));

                    // convert mapping for asset id
                    assert_ok!(
                        XcAssetConfig::register_asset_location(
                            RuntimeOrigin::root(),
                            Box::new(TransactAssetLocation::get().into_versioned()),
                            TransactAssetId::get(),
                        )
                    );

                    Asset {
                        id: AssetId(TransactAssetLocation::get()),
                        fun: Fungible(min_balance * 100),
                    }
                }
            }

            type XcmFungible = astar_xcm_benchmarks::fungible::benchmarking::XcmFungibleBenchmarks::<Runtime>;
            type XcmGeneric = astar_xcm_benchmarks::generic::benchmarking::XcmGenericBenchmarks::<Runtime>;

            let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);
            add_benchmarks!(params, batches);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }

    #[cfg(feature = "evm-tracing")]
    impl moonbeam_rpc_primitives_debug::DebugRuntimeApi<Block> for Runtime {
        fn trace_transaction(
            extrinsics: Vec<<Block as BlockT>::Extrinsic>,
            traced_transaction: &pallet_ethereum::Transaction,
            header: &<Block as BlockT>::Header,
        ) -> Result<
            (),
            sp_runtime::DispatchError,
        > {
            use moonbeam_evm_tracer::tracer::EvmTracer;

            // We need to follow the order when replaying the transactions.
            // Block initialize happens first then apply_extrinsic.
            Executive::initialize_block(header);

            // Apply the a subset of extrinsics: all the substrate-specific or ethereum
            // transactions that preceded the requested transaction.
            for ext in extrinsics.into_iter() {
                let _ = match &ext.0.function {
                    RuntimeCall::Ethereum(pallet_ethereum::Call::transact { transaction }) => {
                        if transaction == traced_transaction {
                            EvmTracer::new().trace(|| Executive::apply_extrinsic(ext));
                            return Ok(());
                        } else {
                            Executive::apply_extrinsic(ext)
                        }
                    }
                    _ => Executive::apply_extrinsic(ext),
                };
            }
            Err(sp_runtime::DispatchError::Other(
                "Failed to find Ethereum transaction among the extrinsics.",
            ))
        }

        fn trace_block(
            extrinsics: Vec<<Block as BlockT>::Extrinsic>,
            known_transactions: Vec<H256>,
            header: &<Block as BlockT>::Header,
        ) -> Result<
            (),
            sp_runtime::DispatchError,
        > {
            use moonbeam_evm_tracer::tracer::EvmTracer;

            // We need to follow the order when replaying the transactions.
            // Block initialize happens first then apply_extrinsic.
            Executive::initialize_block(header);

            // Apply all extrinsics. Ethereum extrinsics are traced.
            for ext in extrinsics.into_iter() {
                match &ext.0.function {
                    RuntimeCall::Ethereum(pallet_ethereum::Call::transact { transaction }) => {
                        if known_transactions.contains(&transaction.hash()) {
                            // Each known extrinsic is a new call stack.
                            EvmTracer::emit_new();
                            EvmTracer::new().trace(|| Executive::apply_extrinsic(ext));
                        } else {
                            let _ = Executive::apply_extrinsic(ext);
                        }
                    }
                    _ => {
                        let _ = Executive::apply_extrinsic(ext);
                    }
                };
            }

            Ok(())
        }

        fn trace_call(
            header: &<Block as BlockT>::Header,
            from: H160,
            to: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            max_fee_per_gas: Option<U256>,
            max_priority_fee_per_gas: Option<U256>,
            nonce: Option<U256>,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<(), sp_runtime::DispatchError> {
            use moonbeam_evm_tracer::tracer::EvmTracer;

            // Initialize block: calls the "on_initialize" hook on every pallet
            // in AllPalletsWithSystem.
            Executive::initialize_block(header);

            EvmTracer::new().trace(|| {
                let is_transactional = false;
                let validate = true;
                let without_base_extrinsic_weight = true;


                // Estimated encoded transaction size must be based on the heaviest transaction
                // type (EIP1559Transaction) to be compatible with all transaction types.
                let mut estimated_transaction_len = data.len() +
                // pallet ethereum index: 1
                // transact call index: 1
                // Transaction enum variant: 1
                // chain_id 8 bytes
                // nonce: 32
                // max_priority_fee_per_gas: 32
                // max_fee_per_gas: 32
                // gas_limit: 32
                // action: 21 (enum varianrt + call address)
                // value: 32
                // access_list: 1 (empty vec size)
                // 65 bytes signature
                258;

                if access_list.is_some() {
                    estimated_transaction_len += access_list.encoded_size();
                }

                let gas_limit = gas_limit.min(u64::MAX.into()).low_u64();

                let (weight_limit, proof_size_base_cost) =
                    match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
                        gas_limit,
                        without_base_extrinsic_weight
                    ) {
                        weight_limit if weight_limit.proof_size() > 0 => {
                            (Some(weight_limit), Some(estimated_transaction_len as u64))
                        }
                        _ => (None, None),
                    };

                let _ = <Runtime as pallet_evm::Config>::Runner::call(
                    from,
                    to,
                    data,
                    value,
                    gas_limit,
                    max_fee_per_gas,
                    max_priority_fee_per_gas,
                    nonce,
                    access_list.unwrap_or_default(),
                    is_transactional,
                    validate,
                    weight_limit,
                    proof_size_base_cost,
                    <Runtime as pallet_evm::Config>::config(),
                );
            });
            Ok(())
        }
    }

    #[cfg(feature = "evm-tracing")]
    impl moonbeam_rpc_primitives_txpool::TxPoolRuntimeApi<Block> for Runtime {
        fn extrinsic_filter(
            xts_ready: Vec<<Block as BlockT>::Extrinsic>,
            xts_future: Vec<<Block as BlockT>::Extrinsic>,
        ) -> moonbeam_rpc_primitives_txpool::TxPoolResponse {
            moonbeam_rpc_primitives_txpool::TxPoolResponse {
                ready: xts_ready
                    .into_iter()
                    .filter_map(|xt| match xt.0.function {
                        RuntimeCall::Ethereum(pallet_ethereum::Call::transact { transaction }) => Some(transaction),
                        _ => None,
                    })
                    .collect(),
                future: xts_future
                    .into_iter()
                    .filter_map(|xt| match xt.0.function {
                        RuntimeCall::Ethereum(pallet_ethereum::Call::transact { transaction }) => Some(transaction),
                        _ => None,
                    })
                    .collect(),
            }
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
            log::info!("try-runtime::on_runtime_upgrade");
            let weight = Executive::try_runtime_upgrade(checks).unwrap();
            (weight, RuntimeBlockWeights::get().max_block)
        }

        fn execute_block(
            block: Block,
            state_root_check: bool,
            signature_check: bool,
            select: frame_try_runtime::TryStateSelect
        ) -> Weight {
            log::info!(
                "try-runtime: executing block #{} ({:?}) / root checks: {:?} / sanity-checks: {:?}",
                block.header.number,
                block.header.hash(),
                state_root_check,
                select,
            );
            Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
        }
    }
}

cumulus_pallet_parachain_system::register_validate_block! {
    Runtime = Runtime,
    BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
