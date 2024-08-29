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
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use frame_support::{
    construct_runtime,
    genesis_builder_helper::{build_state, get_preset},
    parameter_types,
    traits::{
        fungible::{Balanced, Credit, HoldConsideration},
        tokens::{PayFromAccount, UnityAssetBalanceConversion},
        AsEnsureOriginWithArg, ConstU128, ConstU32, ConstU64, EqualPrivilegeOnly, FindAuthor, Get,
        InstanceFilter, LinearStoragePrice, Nothing, OnFinalize, WithdrawReasons,
    },
    weights::{
        constants::{ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
        ConstantMultiplier, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients,
        WeightToFeePolynomial,
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
use pallet_grandpa::{fg_primitives, AuthorityList as GrandpaAuthorityList};
use pallet_transaction_payment::{FungibleAdapter, Multiplier, TargetedFeeAdjustment};
use parity_scale_codec::{Compact, Decode, Encode, MaxEncodedLen};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, ConstBool, OpaqueMetadata, H160, H256, U256};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{
        AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, ConvertInto,
        DispatchInfoOf, Dispatchable, IdentityLookup, NumberFor, PostDispatchInfoOf,
        UniqueSaturatedInto,
    },
    transaction_validity::{TransactionSource, TransactionValidity, TransactionValidityError},
    ApplyExtrinsicResult, FixedPointNumber, FixedU128, Perbill, Permill, Perquintill, RuntimeDebug,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use astar_primitives::{
    dapp_staking::{
        CycleConfiguration, DAppId, EraNumber, PeriodNumber, RankedTier, SmartContract,
        StandardTierSlots,
    },
    evm::{EvmRevertCodeHandler, HashedDefaultMappings},
    governance::{
        CommunityCouncilCollectiveInst, CommunityCouncilMembershipInst, CommunityTreasuryInst,
        EnsureRootOrAllMainCouncil, EnsureRootOrAllTechnicalCommittee,
        EnsureRootOrTwoThirdsCommunityCouncil, EnsureRootOrTwoThirdsMainCouncil,
        EnsureRootOrTwoThirdsTechnicalCommittee, MainCouncilCollectiveInst,
        MainCouncilMembershipInst, MainTreasuryInst, TechnicalCommitteeCollectiveInst,
        TechnicalCommitteeMembershipInst,
    },
    Address, AssetId, Balance, BlockNumber, Hash, Header, Nonce,
};

pub use astar_primitives::{AccountId, Signature};
pub use pallet_dapp_staking_v3::TierThreshold;

pub use crate::precompiles::WhitelistedCalls;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_grandpa::AuthorityId as GrandpaId;
pub use pallet_inflation::InflationParameters;
pub use pallet_timestamp::Call as TimestampCall;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
#[cfg(feature = "std")]
/// Wasm binary unwrapped. If built with `BUILD_DUMMY_WASM_BINARY`, the function panics.
pub fn wasm_binary_unwrap() -> &'static [u8] {
    WASM_BINARY.expect(
        "Development wasm binary is not available. This means the client is \
                        built with `BUILD_DUMMY_WASM_BINARY` flag and it is only usable for \
                        production chains. Please rebuild with the flag disabled.",
    )
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("local"),
    impl_name: create_runtime_str!("local"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    state_version: 1,
};

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
        pub grandpa: Grandpa,
    }
}

mod precompiles;
pub use precompiles::{LocalPrecompiles, ASSET_PRECOMPILE_ADDRESS_PREFIX};
pub type Precompiles = LocalPrecompiles<Runtime>;

mod chain_extensions;
pub use chain_extensions::LocalChainExtensions;

mod weights;

/// Constant values used within the runtime.
pub const MICROAST: Balance = 1_000_000_000_000;
pub const MILLIAST: Balance = 1_000 * MICROAST;
pub const AST: Balance = 1_000 * MILLIAST;

pub const STORAGE_BYTE_FEE: Balance = 100 * MICROAST;

/// Charge fee for stored bytes and items.
pub const fn deposit(items: u32, bytes: u32) -> Balance {
    items as Balance * 1 * AST + (bytes as Balance) * STORAGE_BYTE_FEE
}

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 2000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

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

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
    pub const Version: RuntimeVersion = VERSION;
    pub const BlockHashCount: BlockNumber = 2400;
    /// We allow for 1 seconds of compute with a 2 second average block time.
    pub RuntimeBlockWeights: BlockWeights = BlockWeights
        ::with_sensible_defaults(Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND, u64::MAX), NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockLength: BlockLength = BlockLength
        ::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub const SS58Prefix: u8 = 5;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
    /// The basic call filter to use in dispatchable.
    type BaseCallFilter = frame_support::traits::Everything;
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = RuntimeBlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = RuntimeBlockLength;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type RuntimeCall = RuntimeCall;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = (AccountIdLookup<AccountId, ()>, UnifiedAccounts);
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
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// Version of the runtime.
    type Version = Version;
    /// Converts a module to the index of the module in `construct_runtime!`.
    ///
    /// This type is being generated by `construct_runtime!`.
    type PalletInfo = PalletInfo;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = pallet_unified_accounts::KillAccountMapping<Self>;
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    /// The set code logic, just the default since we're not a parachain.
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<50>;
    type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Runtime>;
    type AllowMultipleBlocksPerSlot = ConstBool<false>;
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;

    type KeyOwnerProof = sp_core::Void;
    type EquivocationReportSystem = ();

    type WeightInfo = ();
    type MaxAuthorities = ConstU32<50>;
    type MaxSetIdSessionEntries = ConstU64<0>;
    type MaxNominators = ConstU32<0>;
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

parameter_types! {
    pub const ExistentialDeposit: u128 = 500;
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = MaxLocks;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = weights::pallet_balances::SubstrateWeight<Runtime>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = ConstU32<1>;
}

parameter_types! {
    pub const AssetDeposit: Balance = 1 * AST;
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

// These values are based on the Astar 2.0 Tokenomics Modeling report.
parameter_types! {
    pub const TransactionLengthFeeFactor: Balance = 23_500_000_000_000; // 0.000_023_500_000_000_000 SBY per byte
    pub const WeightFeeFactor: Balance = 30_855_000_000_000_000; // Around 0.03 SBY per unit of ref time.
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

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = FungibleAdapter<Balances, ()>;
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
    pub StepLimitRatio: Perquintill = Perquintill::from_rational(5_u128, 100_000);
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

parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const DappsStakingPalletId: PalletId = PalletId(*b"py/dpsst");
}

impl pallet_static_price_provider::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchmarkHelper<SC, ACC>(sp_std::marker::PhantomData<(SC, ACC)>);
#[cfg(feature = "runtime-benchmarks")]
impl pallet_dapp_staking_v3::BenchmarkHelper<SmartContract<AccountId>, AccountId>
    for BenchmarkHelper<SmartContract<AccountId>, AccountId>
{
    fn get_smart_contract(id: u32) -> SmartContract<AccountId> {
        SmartContract::Wasm(AccountId::from([id as u8; 32]))
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

impl pallet_dapp_staking_v3::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type Currency = Balances;
    type SmartContract = SmartContract<AccountId>;
    type ContractRegisterOrigin = EnsureRootOrTwoThirdsCommunityCouncil;
    type ContractUnregisterOrigin = EnsureRoot<AccountId>;
    type ManagerOrigin = EnsureRootOrTwoThirdsTechnicalCommittee;
    type NativePriceProvider = StaticPriceProvider;
    type StakingRewardHandler = Inflation;
    type CycleConfiguration = InflationCycleConfig;
    type Observers = Inflation;
    type AccountCheck = ();
    type TierSlots = StandardTierSlots;
    type BaseNativeCurrencyPrice = BaseNativeCurrencyPrice;
    type EraRewardSpanLength = ConstU32<8>;
    type RewardRetentionInPeriods = ConstU32<2>;
    type MaxNumberOfContracts = ConstU32<100>;
    type MaxUnlockingChunks = ConstU32<5>;
    type MinimumLockedAmount = ConstU128<AST>;
    type UnlockingPeriod = ConstU32<2>;
    type MaxNumberOfStakedContracts = ConstU32<3>;
    type MinimumStakeAmount = ConstU128<AST>;
    type NumberOfTiers = ConstU32<4>;
    type RankingEnabled = ConstBool<true>;
    type WeightInfo = pallet_dapp_staking_v3::weights::SubstrateWeight<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = BenchmarkHelper<SmartContract<AccountId>, AccountId>;
}

pub struct InflationPayoutPerBlock;
impl pallet_inflation::PayoutPerBlock<Credit<AccountId, Balances>> for InflationPayoutPerBlock {
    fn treasury(reward: Credit<AccountId, Balances>) {
        let _ = Balances::resolve(&TreasuryPalletId::get().into_account_truncating(), reward);
    }

    fn collators(_reward: Credit<AccountId, Balances>) {
        // no collators for local dev node
    }
}

pub struct InflationCycleConfig;
impl CycleConfiguration for InflationCycleConfig {
    fn periods_per_cycle() -> PeriodNumber {
        4
    }

    fn eras_per_voting_subperiod() -> EraNumber {
        2
    }

    fn eras_per_build_and_earn_subperiod() -> EraNumber {
        22
    }

    fn blocks_per_era() -> BlockNumber {
        30
    }
}

impl pallet_inflation::Config for Runtime {
    type Currency = Balances;
    type PayoutPerBlock = InflationPayoutPerBlock;
    type CycleConfiguration = InflationCycleConfig;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_inflation::weights::SubstrateWeight<Runtime>;
}

impl pallet_utility::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    // 2 storage items with value size 20 and 32
    pub const AccountMappingStorageFee: u128 = deposit(2, 32 + 20);
}

impl pallet_unified_accounts::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type DefaultMappings = HashedDefaultMappings<BlakeTwo256>;
    type ChainId = ChainId;
    type AccountMappingStorageFee = AccountMappingStorageFee;
    type WeightInfo = pallet_unified_accounts::weights::SubstrateWeight<Self>;
}

parameter_types! {
    pub ReservedXcmpWeight: Weight = Weight::zero();
}

impl pallet_ethereum_checked::Config for Runtime {
    type ReservedXcmpWeight = ReservedXcmpWeight;
    type InvalidEvmTransactionError = pallet_ethereum::InvalidTransactionWrapper;
    type ValidatedTransaction = pallet_ethereum::ValidatedTransaction<Self>;
    type AddressMapper = UnifiedAccounts;
    type XcmTransactOrigin = pallet_ethereum_checked::EnsureXcmEthereumTx<AccountId>;
    type WeightInfo = pallet_ethereum_checked::weights::SubstrateWeight<Runtime>;
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
    /// * Local: 4369
    pub ChainId: u64 = 0x1111;
    /// EVM gas limit
    pub BlockGasLimit: U256 = U256::from(
        NORMAL_DISPATCH_RATIO * WEIGHT_REF_TIME_PER_SECOND / WEIGHT_PER_GAS
    );
    pub PrecompilesValue: Precompiles = LocalPrecompiles::<_>::new();
    pub WeightPerGas: Weight = Weight::from_parts(WEIGHT_PER_GAS, 0);
    /// The amount of gas per PoV size. Value is calculated as:
    ///
    /// max_gas_limit = max_tx_ref_time / WEIGHT_PER_GAS = max_pov_size * gas_limit_pov_size_ratio
    /// gas_limit_pov_size_ratio = ceil((max_tx_ref_time / WEIGHT_PER_GAS) / max_pov_size)
    ///
    /// Local runtime has no strict bounds as parachain, but we keep the value set to 4 for consistency.
    pub const GasLimitPovSizeRatio: u64 = 4;
}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = DynamicEvmBaseFee;
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Runtime>;
    type CallOrigin = pallet_evm::EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = pallet_evm::EnsureAddressTruncated;
    type AddressMapping = UnifiedAccounts;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = Precompiles;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = ChainId;
    type OnChargeTransaction = pallet_evm::EVMFungibleAdapter<Balances, ()>;
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

parameter_types! {
    pub MaximumSchedulerWeight: Weight = NORMAL_DISPATCH_RATIO * RuntimeBlockWeights::get().max_block;
}

impl pallet_scheduler::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    type MaxScheduledPerBlock = ConstU32<50>;
    type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type Preimages = Preimage;
}

parameter_types! {
    pub const PreimageBaseDeposit: Balance = deposit(1, 0);
    pub const PreimageByteDeposit: Balance = deposit(0, 1);
    pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
    type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type ManagerOrigin = EnsureRoot<AccountId>;
    type Consideration = HoldConsideration<
        AccountId,
        Balances,
        PreimageHoldReason,
        LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
    >;
}

parameter_types! {
    pub const MinVestedTransfer: Balance = 1 * AST;
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
    pub const DepositPerItem: Balance = deposit(1, 0);
    pub const DepositPerByte: Balance = deposit(0, 1);
    // Fallback value if storage deposit limit not set by the user
    pub const DefaultDepositLimit: Balance = deposit(16, 16 * 1024);
    pub const MaxDelegateDependencies: u32 = 32;
    pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(10);
    pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
}

impl pallet_contracts::Config for Runtime {
    type Time = Timestamp;
    type Randomness = RandomnessCollectiveFlip;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
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
    type ChainExtension = LocalChainExtensions<Self, UnifiedAccounts>;
    type Schedule = Schedule;
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
    type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
    type MaxStorageKeyLen = ConstU32<128>;
    type UnsafeUnstableInterface = ConstBool<true>;
    type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
    type MaxDelegateDependencies = MaxDelegateDependencies;
    type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Debug = ();
    type Environment = ();
    type Migrations = ();
    type Xcm = ();
    type UploadOrigin = EnsureSigned<<Self as frame_system::Config>::AccountId>;
    type InstantiateOrigin = EnsureSigned<<Self as frame_system::Config>::AccountId>;
    type ApiVersion = ();
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
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
    /// To know exact calls check InstanceFilter implementation for ProxyTypes
    NonTransfer,
    /// All Runtime calls from Pallet Balances allowed for proxy account
    Balances,
    /// All Runtime calls from Pallet Assets allowed for proxy account
    Assets,
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
            // Allows all runtime calls for proxy account
            ProxyType::Any => true,
            // Allows only NonTransfer runtime calls for proxy account
            ProxyType::NonTransfer => {
                matches!(
                    c,
                    RuntimeCall::System(..)
                        | RuntimeCall::Timestamp(..)
                        | RuntimeCall::Scheduler(..)
                        | RuntimeCall::Proxy(..)
                        | RuntimeCall::Grandpa(..)
                        // Skip entire Balances pallet
                        | RuntimeCall::Vesting(pallet_vesting::Call::vest{..})
				        | RuntimeCall::Vesting(pallet_vesting::Call::vest_other{..})
				        // Specifically omitting Vesting `vested_transfer`, and `force_vested_transfer`
                        | RuntimeCall::DappStaking(..)
                        // Skip entire EVM pallet
                        // Skip entire Ethereum pallet
                        | RuntimeCall::DynamicEvmBaseFee(..) // Skip entire Contracts pallet
                )
            }
            // All Runtime calls from Pallet Balances allowed for proxy account
            ProxyType::Balances => {
                matches!(c, RuntimeCall::Balances(..))
            }
            // All Runtime calls from Pallet Assets allowed for proxy account
            ProxyType::Assets => {
                matches!(c, RuntimeCall::Assets(..))
            }
            // Only reject_announcement call from pallet proxy allowed for proxy account
            ProxyType::CancelProxy => {
                matches!(
                    c,
                    RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. })
                )
            }
            // All runtime calls from pallet DappStaking allowed for proxy account
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
            (ProxyType::NonTransfer, _) => true,
            (ProxyType::DappStaking, ProxyType::StakerRewardClaim) => true,
            _ => false,
        }
    }
}

impl pallet_proxy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type ProxyType = ProxyType;
    // One storage item; key size 32, value size 8; .
    type ProxyDepositBase = ConstU128<{ AST * 10 }>;
    // Additional storage item size of 33 bytes.
    type ProxyDepositFactor = ConstU128<{ MILLIAST * 330 }>;
    type MaxProxies = ConstU32<32>;
    type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
    type MaxPending = ConstU32<32>;
    type CallHasher = BlakeTwo256;
    // Key size 32 + 1 item
    type AnnouncementDepositBase = ConstU128<{ AST * 10 }>;
    // Acc Id + Hash + block number
    type AnnouncementDepositFactor = ConstU128<{ MILLIAST * 660 }>;
}

parameter_types! {
    pub const CouncilMaxMembers: u32 = 5;
    pub const TechnicalCommitteeMaxMembers: u32 = 3;
    pub const CommunityCouncilMaxMembers: u32 = 10;
}

impl pallet_membership::Config<MainCouncilMembershipInst> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AddOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type RemoveOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type SwapOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type ResetOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type PrimeOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type MembershipInitialized = Council;
    type MembershipChanged = Council;
    type MaxMembers = CouncilMaxMembers;
    type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

impl pallet_membership::Config<TechnicalCommitteeMembershipInst> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AddOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type RemoveOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type SwapOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type ResetOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type PrimeOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type MembershipInitialized = TechnicalCommittee;
    type MembershipChanged = TechnicalCommittee;
    type MaxMembers = TechnicalCommitteeMaxMembers;
    type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

impl pallet_membership::Config<CommunityCouncilMembershipInst> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AddOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type RemoveOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type SwapOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type ResetOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type PrimeOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type MembershipInitialized = CommunityCouncil;
    type MembershipChanged = CommunityCouncil;
    type MaxMembers = CommunityCouncilMaxMembers;
    type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub MaxProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

impl pallet_collective::Config<MainCouncilCollectiveInst> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = ConstU32<{ 5 * MINUTES }>;
    type MaxProposals = ConstU32<16>;
    type MaxMembers = CouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type SetMembersOrigin = EnsureRoot<AccountId>;
    type MaxProposalWeight = MaxProposalWeight;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

impl pallet_collective::Config<TechnicalCommitteeCollectiveInst> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = ConstU32<{ 5 * MINUTES }>;
    type MaxProposals = ConstU32<16>;
    type MaxMembers = TechnicalCommitteeMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type SetMembersOrigin = EnsureRoot<AccountId>;
    type MaxProposalWeight = MaxProposalWeight;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

impl pallet_collective::Config<CommunityCouncilCollectiveInst> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = ConstU32<{ 5 * MINUTES }>;
    type MaxProposals = ConstU32<16>;
    type MaxMembers = CommunityCouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type SetMembersOrigin = EnsureRoot<AccountId>;
    type MaxProposalWeight = MaxProposalWeight;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

impl pallet_democracy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EnactmentPeriod = ConstU32<{ 5 * MINUTES }>;
    type LaunchPeriod = ConstU32<{ 5 * MINUTES }>;
    type VotingPeriod = ConstU32<{ 5 * MINUTES }>;
    type VoteLockingPeriod = ConstU32<{ 2 * MINUTES }>;
    type MinimumDeposit = ConstU128<{ 10 * AST }>;
    type FastTrackVotingPeriod = ConstU32<{ MINUTES / 2 }>;
    type CooloffPeriod = ConstU32<{ 2 * MINUTES }>;

    type MaxVotes = ConstU32<128>;
    type MaxProposals = ConstU32<128>;
    type MaxDeposits = ConstU32<128>;
    type MaxBlacklisted = ConstU32<128>;

    /// A two third majority of the Council can choose the next external "super majority approve" proposal.
    type ExternalOrigin = EnsureRootOrTwoThirdsMainCouncil;
    /// A two third majority of the Council can choose the next external "majority approve" proposal. Also bypasses blacklist filter.
    type ExternalMajorityOrigin = EnsureRootOrTwoThirdsMainCouncil;
    /// Unanimous approval of the Council can choose the next external "super majority against" proposal.
    type ExternalDefaultOrigin = EnsureRootOrAllMainCouncil;
    /// A two third majority of the Technical Committee can have an external proposal tabled immediately
    /// for a _fast track_ vote, and a custom enactment period.
    type FastTrackOrigin = EnsureRootOrTwoThirdsTechnicalCommittee;
    /// Unanimous approval of the Technical Committee can have an external proposal tabled immediately
    /// for a completely custom _voting period length_ vote, and a custom enactment period.
    type InstantOrigin = EnsureRootOrAllTechnicalCommittee;
    type InstantAllowed = ConstBool<true>;

    /// A two third majority of the Council can cancel a passed proposal. Can happen only once per unique proposal.
    type CancellationOrigin = EnsureRootOrTwoThirdsMainCouncil;
    /// Only a passed public referendum can permanently blacklist a proposal.
    type BlacklistOrigin = EnsureRoot<AccountId>;
    /// An unanimous Technical Committee can cancel a public proposal, slashing the deposit(s).
    type CancelProposalOrigin = EnsureRootOrAllTechnicalCommittee;
    /// Any member of the Technical Committee can veto Council's proposal. This can be only done once per proposal, and _veto_ lasts for a _cooloff_ period.
    type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCommitteeCollectiveInst>;

    type SubmitOrigin = EnsureSigned<AccountId>;
    type PalletsOrigin = OriginCaller;
    type Preimages = Preimage;
    type Scheduler = Scheduler;
    type Slash = ();
    type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub MainTreasuryAccount: AccountId = Treasury::account_id();
}

impl pallet_treasury::Config<MainTreasuryInst> for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;

    // Two origins which can either approve or reject the spending proposal
    type ApproveOrigin = EnsureRootOrTwoThirdsMainCouncil;
    type RejectOrigin = EnsureRootOrTwoThirdsMainCouncil;

    type OnSlash = Treasury;
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ConstU128<{ 10 * AST }>;
    type ProposalBondMaximum = ConstU128<{ 100 * AST }>;
    type SpendPeriod = ConstU32<{ 1 * MINUTES }>;

    // We don't do periodic burns of the treasury
    type Burn = ();
    type BurnDestination = ();
    type SpendFunds = ();

    type MaxApprovals = ConstU32<64>;
    type AssetKind = (); // Only native asset is supported
    type Beneficiary = AccountId;
    type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
    type Paymaster = PayFromAccount<Balances, MainTreasuryAccount>;
    type BalanceConverter = UnityAssetBalanceConversion;

    // New approach to using treasury, useful for OpenGov but not necessarily for us.
    type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
    // Only used by 'spend' approach which is disabled
    type PayoutPeriod = ConstU32<0>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const CommunityTreasuryPalletId: PalletId = PalletId(*b"py/comtr");
}

impl pallet_treasury::Config<CommunityTreasuryInst> for Runtime {
    type PalletId = CommunityTreasuryPalletId;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;

    // Two origins which can either approve or reject the spending proposal
    type ApproveOrigin = EnsureRootOrTwoThirdsCommunityCouncil;
    type RejectOrigin = EnsureRootOrTwoThirdsCommunityCouncil;

    type OnSlash = CommunityTreasury;
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ConstU128<{ 10 * AST }>;
    type ProposalBondMaximum = ConstU128<{ 100 * AST }>;
    type SpendPeriod = ConstU32<{ 1 * MINUTES }>;

    // We don't do periodic burns of the community treasury
    type Burn = ();
    type BurnDestination = ();
    type SpendFunds = ();

    type MaxApprovals = ConstU32<64>;
    type AssetKind = (); // Only native asset is supported
    type Beneficiary = AccountId;
    type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
    type Paymaster = PayFromAccount<Balances, MainTreasuryAccount>;
    type BalanceConverter = UnityAssetBalanceConversion;

    // New approach to using treasury, useful for OpenGov but not necessarily for us.
    type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
    // Only used by 'spend' approach which is disabled
    type PayoutPeriod = ConstU32<0>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub CommunityTreasuryAccountId: AccountId = CommunityTreasuryPalletId::get().into_account_truncating();
}

#[derive(Default)]
pub struct CommunityCouncilCallFilter;
impl InstanceFilter<RuntimeCall> for CommunityCouncilCallFilter {
    fn filter(&self, c: &RuntimeCall) -> bool {
        matches!(
            c,
            RuntimeCall::DappStaking(..)
                | RuntimeCall::System(frame_system::Call::remark { .. })
                | RuntimeCall::Utility(..)
        )
    }
}

impl pallet_collective_proxy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type CollectiveProxy = EnsureRootOrTwoThirdsCommunityCouncil;
    type ProxyAccountId = CommunityTreasuryAccountId;
    type CallFilter = CommunityCouncilCallFilter;
    type WeightInfo = pallet_collective_proxy::weights::SubstrateWeight<Runtime>;
}

construct_runtime!(
    pub struct Runtime {
        System: frame_system = 10,
        Utility: pallet_utility = 11,
        Timestamp: pallet_timestamp = 13,
        RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip = 16,
        Scheduler: pallet_scheduler = 17,
        Proxy: pallet_proxy = 18,

        TransactionPayment: pallet_transaction_payment = 30,
        Balances: pallet_balances = 31,
        Vesting: pallet_vesting = 32,
        DappStaking: pallet_dapp_staking_v3 = 34,
        Inflation: pallet_inflation = 35,
        Assets: pallet_assets = 36,
        StaticPriceProvider: pallet_static_price_provider = 37,

        Aura: pallet_aura = 43,
        Grandpa: pallet_grandpa = 44,

        EVM: pallet_evm = 60,
        Ethereum: pallet_ethereum = 61,
        DynamicEvmBaseFee: pallet_dynamic_evm_base_fee = 62,
        EthereumChecked: pallet_ethereum_checked = 64,
        UnifiedAccounts: pallet_unified_accounts = 65,

        Contracts: pallet_contracts = 70,

        Preimage: pallet_preimage = 84,

        // skip 90 - for runtimes consistency (pallet_xvm previously)

        // Governance
        Sudo: pallet_sudo = 99,
        CouncilMembership: pallet_membership::<Instance2> = 100,
        TechnicalCommitteeMembership: pallet_membership::<Instance3> = 101,
        CommunityCouncilMembership: pallet_membership::<Instance4> = 102,
        Council: pallet_collective::<Instance2> = 103,
        TechnicalCommittee: pallet_collective::<Instance3> = 104,
        CommunityCouncil: pallet_collective::<Instance4> = 105,
        Democracy: pallet_democracy = 106,
        Treasury: pallet_treasury::<Instance1> = 107,
        CommunityTreasury: pallet_treasury::<Instance2> = 108,
        CollectiveProxy: pallet_collective_proxy = 109,
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
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic =
    fp_self_contained::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra, H160>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    Migrations,
>;

pub type Migrations = ();

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
        [pallet_assets, pallet_assets::Pallet::<Runtime>]
        [frame_system, SystemBench::<Runtime>]
        [pallet_balances, Balances]
        [pallet_timestamp, Timestamp]
        [pallet_ethereum_checked, EthereumChecked]
        [pallet_dapp_staking_v3, DappStaking]
        [pallet_inflation, Inflation]
        [pallet_dynamic_evm_base_fee, DynamicEvmBaseFee]
    );
}

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block);
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

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
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

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
        }

        fn authorities() -> Vec<AuraId> {
            pallet_aura::Authorities::<Runtime>::get().into_inner()
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> GrandpaAuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn current_set_id() -> fg_primitives::SetId {
            Grandpa::current_set_id()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            _equivocation_proof: fg_primitives::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            _key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            None
        }

        fn generate_key_ownership_proof(
            _set_id: fg_primitives::SetId,
            _authority_id: GrandpaId,
        ) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
            // NOTE: this is the only implementation possible since we've
            // defined our key owner proof type as a bottom type (i.e. a type
            // with no values).
            None
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
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
            use frame_system_benchmarking::Pallet as SystemBench;
            use baseline::Pallet as BaselineBench;

            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);

            let storage_info = AllPalletsWithSystem::storage_info();

            (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch};
            use frame_system_benchmarking::Pallet as SystemBench;
            use baseline::Pallet as BaselineBench;

            impl frame_system_benchmarking::Config for Runtime {}
            impl baseline::Config for Runtime {}

            use frame_support::traits::{WhitelistedStorageKeys, TrackedStorageKey};
            let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);
            add_benchmarks!(params, batches);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }

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

            let mut config = <Runtime as pallet_evm::Config>::config().clone();
            config.estimate = true;

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
