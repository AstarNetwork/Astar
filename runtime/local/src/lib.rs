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

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use frame_support::{
    construct_runtime, parameter_types,
    traits::{
        AsEnsureOriginWithArg, ConstU128, ConstU32, ConstU64, Currency, EitherOfDiverse,
        EqualPrivilegeOnly, FindAuthor, Get, InstanceFilter, Nothing, OnFinalize, WithdrawReasons,
    },
    weights::{
        constants::{RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
        ConstantMultiplier, IdentityFee, Weight,
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
use parity_scale_codec::{Compact, Decode, Encode, MaxEncodedLen};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, ConstBool, OpaqueMetadata, H160, H256, U256};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{
        AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, ConvertInto,
        DispatchInfoOf, Dispatchable, NumberFor, PostDispatchInfoOf, UniqueSaturatedInto, Verify,
    },
    transaction_validity::{
        TransactionPriority, TransactionSource, TransactionValidity, TransactionValidityError,
    },
    ApplyExtrinsicResult, RuntimeDebug,
};
use sp_std::prelude::*;

pub use astar_primitives::{
    AccountId, Address, AssetId, Balance, BlockNumber, Hash, Header, Index, Signature,
};

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_grandpa::AuthorityId as GrandpaId;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::CurrencyAdapter;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

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
pub use precompiles::{LocalNetworkPrecompiles, ASSET_PRECOMPILE_ADDRESS_PREFIX};
pub type Precompiles = LocalNetworkPrecompiles<Runtime>;

mod chain_extensions;
pub use chain_extensions::*;

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
    type Lookup = AccountIdLookup<AccountId, ()>;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    /// The index type for blocks.
    type BlockNumber = BlockNumber;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    /// The ubiquitous origin type.
    type RuntimeOrigin = RuntimeOrigin;
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
    type OnKilledAccount = ();
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    /// The set code logic, just the default since we're not a parachain.
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const MaxAuthorities: u32 = 50;
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<50>;
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;

    type KeyOwnerProof = sp_core::Void;
    type EquivocationReportSystem = ();

    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxSetIdSessionEntries = ConstU64<0>;
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = (Aura, BlockReward);
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
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type HoldIdentifier = ();
    type FreezeIdentifier = ();
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
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
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
    type RemoveItemsLimit = ConstU32<1000>;
    type AssetIdParameter = Compact<AssetId>;
    type CallbackHandle = ();
}

parameter_types! {
    pub const TransactionByteFee: Balance = 1;
    pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
    type WeightToFee = IdentityFee<Balance>;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
    type FeeMultiplierUpdate = ();
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
}

parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const DappsStakingPalletId: PalletId = PalletId(*b"py/dpsst");
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DappsStakingTvlProvider();
impl Get<Balance> for DappsStakingTvlProvider {
    fn get() -> Balance {
        DappsStaking::tvl()
    }
}

pub struct BeneficiaryPayout();
impl pallet_block_reward::BeneficiaryPayout<NegativeImbalance> for BeneficiaryPayout {
    fn treasury(reward: NegativeImbalance) {
        Balances::resolve_creating(&TreasuryPalletId::get().into_account_truncating(), reward);
    }

    fn collators(_reward: NegativeImbalance) {
        // no collators for local dev node
    }

    fn dapps_staking(stakers: NegativeImbalance, dapps: NegativeImbalance) {
        DappsStaking::rewards(stakers, dapps)
    }
}

parameter_types! {
    pub const RewardAmount: Balance = 2_664 * MILLIAST;
}

impl pallet_block_reward::Config for Runtime {
    type Currency = Balances;
    type DappsStakingTvlProvider = DappsStakingTvlProvider;
    type BeneficiaryPayout = BeneficiaryPayout;
    type RewardAmount = RewardAmount;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_block_reward::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const BlockPerEra: BlockNumber = 60;
    pub const RegisterDeposit: Balance = 100 * AST;
    pub const MaxNumberOfStakersPerContract: u32 = 512;
    pub const MinimumStakingAmount: Balance = 10 * AST;
    pub const MinimumRemainingAmount: Balance = AST;
    pub const MaxUnlockingChunks: u32 = 2;
    pub const UnbondingPeriod: u32 = 2;
    pub const MaxEraStakeValues: u32 = 5;
}

impl pallet_dapps_staking::Config for Runtime {
    type Currency = Balances;
    type BlockPerEra = BlockPerEra;
    type SmartContract = SmartContract<AccountId>;
    type RegisterDeposit = RegisterDeposit;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_dapps_staking::weights::SubstrateWeight<Runtime>;
    type MaxNumberOfStakersPerContract = MaxNumberOfStakersPerContract;
    type MinimumStakingAmount = MinimumStakingAmount;
    type PalletId = DappsStakingPalletId;
    type MaxUnlockingChunks = MaxUnlockingChunks;
    type UnbondingPeriod = UnbondingPeriod;
    type MinimumRemainingAmount = MinimumRemainingAmount;
    type MaxEraStakeValues = MaxEraStakeValues;
    type UnregisteredDappRewardRetention = ConstU32<3>;
}

/// Multi-VM pointer to smart contract instance.
#[derive(
    PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, scale_info::TypeInfo,
)]
pub enum SmartContract<AccountId> {
    /// EVM smart contract instance.
    Evm(sp_core::H160),
    /// Wasm smart contract instance.
    Wasm(AccountId),
}

impl<AccountId> Default for SmartContract<AccountId> {
    fn default() -> Self {
        SmartContract::Evm(H160::repeat_byte(0x00))
    }
}

impl<AccountId: From<[u8; 32]>> From<[u8; 32]> for SmartContract<AccountId> {
    fn from(input: [u8; 32]) -> Self {
        SmartContract::Wasm(input.into())
    }
}

impl pallet_utility::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

///TODO: Placeholder account mapping. This would be replaced once account abstraction is finished.
pub struct HashedAccountMapping;
impl astar_primitives::ethereum_checked::AccountMapping<AccountId> for HashedAccountMapping {
    fn into_h160(account_id: AccountId) -> H160 {
        let data = (b"evm:", account_id);
        return H160::from_slice(&data.using_encoded(sp_io::hashing::blake2_256)[0..20]);
    }
}

parameter_types! {
    /// Equal to normal class dispatch weight limit.
    pub XvmTxWeightLimit: Weight = NORMAL_DISPATCH_RATIO * Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND, u64::MAX);
    pub ReservedXcmpWeight: Weight = Weight::zero();
}

impl pallet_ethereum_checked::Config for Runtime {
    type ReservedXcmpWeight = ReservedXcmpWeight;
    type XvmTxWeightLimit = XvmTxWeightLimit;
    type InvalidEvmTransactionError = pallet_ethereum::InvalidTransactionWrapper;
    type ValidatedTransaction = pallet_ethereum::ValidatedTransaction<Self>;
    type AccountMapping = HashedAccountMapping;
    type XcmTransactOrigin = pallet_ethereum_checked::EnsureXcmEthereumTx<AccountId>;
    type WeightInfo = pallet_ethereum_checked::weights::SubstrateWeight<Runtime>;
}

impl pallet_xvm::Config for Runtime {
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type AccountMapping = HashedAccountMapping;
    type EthereumTransact = EthereumChecked;
    type WeightInfo = pallet_xvm::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    // Tells `pallet_base_fee` whether to calculate a new BaseFee `on_finalize` or not.
    pub DefaultBaseFeePerGas: U256 = (MILLIAST / 1_000_000).into();
    // At the moment, we don't use dynamic fee calculation for local chain by default
    pub DefaultElasticity: Permill = Permill::zero();
}

pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
    fn lower() -> Permill {
        Permill::zero()
    }
    fn ideal() -> Permill {
        Permill::from_parts(500_000)
    }
    fn upper() -> Permill {
        Permill::from_parts(1_000_000)
    }
}

impl pallet_base_fee::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Threshold = BaseFeeThreshold;
    type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
    type DefaultElasticity = DefaultElasticity;
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
            let authority_id = Aura::authorities()[author_index as usize].clone();
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
    pub PrecompilesValue: Precompiles = LocalNetworkPrecompiles::<_>::new();
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
    type FeeCalculator = BaseFee;
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
    type OnChargeTransaction = pallet_evm::EVMCurrencyAdapter<Balances, ()>;
    type BlockGasLimit = BlockGasLimit;
    type Timestamp = Timestamp;
    type OnCreate = ();
    type FindAuthor = FindAuthorTruncated<Aura>;
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
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
    pub const EcdsaUnsignedPriority: TransactionPriority = TransactionPriority::MAX / 2;
    pub const CallFee: Balance = AST / 10;
    pub const CallMagicNumber: u16 = 0xff51;
}

impl pallet_custom_signatures::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Signature = pallet_custom_signatures::ethereum::EthereumSignature;
    type Signer = <Signature as Verify>::Signer;
    type CallMagicNumber = CallMagicNumber;
    type Currency = Balances;
    type CallFee = CallFee;
    type OnChargeTransaction = ();
    type UnsignedPriority = EcdsaUnsignedPriority;
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
}

impl pallet_preimage::Config for Runtime {
    type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type ManagerOrigin = EnsureRoot<AccountId>;
    type BaseDeposit = PreimageBaseDeposit;
    type ByteDeposit = PreimageByteDeposit;
}

parameter_types! {
    pub CouncilMotionDuration: BlockNumber = 3 * MINUTES;
    pub MaxProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeEvent = RuntimeEvent;
    type Proposal = RuntimeCall;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = ConstU32<100>;
    type MaxMembers = ConstU32<3>;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
    type SetMembersOrigin = EnsureRoot<Self::AccountId>;
    type MaxProposalWeight = MaxProposalWeight;
}

parameter_types! {
    pub const TechnicalCommitteeMotionDuration: BlockNumber = 3 * MINUTES;
}

type TechnicalCommitteeCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCommitteeCollective> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeEvent = RuntimeEvent;
    type Proposal = RuntimeCall;
    type MotionDuration = TechnicalCommitteeMotionDuration;
    type MaxProposals = ConstU32<100>;
    type MaxMembers = ConstU32<3>;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
    type SetMembersOrigin = EnsureRoot<Self::AccountId>;
    type MaxProposalWeight = MaxProposalWeight;
}

parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub const ProposalBondMinimum: Balance = 100 * AST;
    pub const ProposalBondMaximum: Balance = 500 * AST;
    pub const SpendPeriod: BlockNumber = 1 * MINUTES;
    pub const Burn: Permill = Permill::from_percent(1);
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    type ApproveOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>,
    >;
    type RejectOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
    >;
    type RuntimeEvent = RuntimeEvent;
    type OnSlash = Treasury;
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type ProposalBondMaximum = ProposalBondMaximum;
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
    type BurnDestination = ();
    type SpendFunds = ();
    type MaxApprovals = ConstU32<100>;
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
    type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
}

parameter_types! {
    pub LaunchPeriod: BlockNumber = 1 * MINUTES;
    pub VotingPeriod: BlockNumber = 3 * MINUTES;
    pub FastTrackVotingPeriod: BlockNumber = 1 * MINUTES;
    pub const MinimumDeposit: Balance = 7000 * AST;
    pub EnactmentPeriod: BlockNumber = 5 * MINUTES;
    pub VoteLockingPeriod: BlockNumber = 10 * MINUTES;
    pub CooloffPeriod: BlockNumber = 10 * MINUTES;
    pub const InstantAllowed: bool = true;
}

impl pallet_democracy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type VoteLockingPeriod = VoteLockingPeriod;
    type MinimumDeposit = MinimumDeposit;
    /// A straight majority of the council can decide what their next motion is.
    type ExternalOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
        EnsureRoot<AccountId>,
    >;
    /// A 60% super-majority can have the next scheduled referendum be a straight majority-carries vote.
    type ExternalMajorityOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
        EnsureRoot<AccountId>,
    >;
    /// A unanimous council can have the next scheduled referendum be a straight default-carries
    /// (NTB) vote.
    type ExternalDefaultOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>,
        EnsureRoot<AccountId>,
    >;
    type SubmitOrigin = EnsureSigned<AccountId>;
    /// Two thirds of the technical committee can have an `ExternalMajority/ExternalDefault` vote
    /// be tabled immediately and with a shorter voting/enactment period.
    type FastTrackOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCommitteeCollective, 2, 3>,
        EnsureRoot<AccountId>,
    >;
    type InstantOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCommitteeCollective, 1, 1>,
        EnsureRoot<AccountId>,
    >;
    type InstantAllowed = InstantAllowed;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    // To cancel a proposal which has been passed, 2/3 of the council must agree to it.
    type CancellationOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>,
        EnsureRoot<AccountId>,
    >;
    // To cancel a proposal before it has been passed, the technical committee must be unanimous or
    // Root must agree.
    type CancelProposalOrigin = EitherOfDiverse<
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCommitteeCollective, 1, 1>,
        EnsureRoot<AccountId>,
    >;
    type BlacklistOrigin = EnsureRoot<AccountId>;
    // Any single technical committee member may veto a coming council proposal, however they can
    // only do it once and it lasts only for the cooloff period.
    type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCommitteeCollective>;
    type CooloffPeriod = CooloffPeriod;
    type Slash = Treasury;
    type Scheduler = Scheduler;
    type MaxVotes = ConstU32<100>;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
    type MaxProposals = ConstU32<100>;
    type Preimages = Preimage;
    type MaxDeposits = ConstU32<100>;
    type MaxBlacklisted = ConstU32<100>;
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
    // `VestingInfo` encode length is 36bytes. 28 schedules gets encoded as 1009 bytes, which is the
    // highest number of schedules that encodes less than 2^10.
    const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
    pub const DepositPerItem: Balance = deposit(1, 0);
    pub const DepositPerByte: Balance = deposit(0, 1);
    // Fallback value if storage deposit limit not set by the user
    pub const DefaultDepositLimit: Balance = deposit(16, 16 * 1024);
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
    type ChainExtension = (
        DappsStakingExtension<Self>,
        XvmExtension<Self, Xvm>,
        AssetsExtension<Self, pallet_chain_extension_assets::weights::SubstrateWeight<Self>>,
    );
    type Schedule = Schedule;
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
    type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
    type MaxStorageKeyLen = ConstU32<128>;
    type UnsafeUnstableInterface = ConstBool<false>;
    type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
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
    /// Only Runtime Calls related to goverance for proxy account
    /// To know exact calls check InstanceFilter implementation for ProxyTypes
    Governance,
    /// Only reject_announcement call from pallet proxy allowed for proxy account
    CancelProxy,
    /// All runtime calls from pallet DappStaking allowed for proxy account
    DappsStaking,
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
                        | RuntimeCall::DappsStaking(..)
                        // Skip entire EVM pallet
                        // Skip entire Ethereum pallet
                        // Skip entire EthCall pallet
                        | RuntimeCall::BaseFee(..)
                        // Skip entire Contracts pallet
                        | RuntimeCall::Democracy(..)
                        | RuntimeCall::Council(..)
                        | RuntimeCall::TechnicalCommittee(..)
                        | RuntimeCall::Treasury(..)
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
            ProxyType::Governance => {
                matches!(
                    c,
                    RuntimeCall::Democracy(..)
                        | RuntimeCall::Council(..)
                        | RuntimeCall::TechnicalCommittee(..)
                        | RuntimeCall::Treasury(..)
                )
            }
            // Only reject_announcement call from pallet proxy allowed for proxy account
            ProxyType::CancelProxy => {
                matches!(
                    c,
                    RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. })
                )
            }
            // All runtime calls from pallet DappStaking allowed for proxy account
            ProxyType::DappsStaking => {
                matches!(c, RuntimeCall::DappsStaking(..))
            }
            ProxyType::StakerRewardClaim => {
                matches!(
                    c,
                    RuntimeCall::DappsStaking(pallet_dapps_staking::Call::claim_staker { .. })
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
            (ProxyType::DappsStaking, ProxyType::StakerRewardClaim) => true,
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

// TODO: remove this once https://github.com/paritytech/substrate/issues/12161 is resolved
#[rustfmt::skip]
construct_runtime!(
    pub struct Runtime
    where
        Block = Block,
        NodeBlock = generic::Block<Header, sp_runtime::OpaqueExtrinsic>,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Utility: pallet_utility,
        Timestamp: pallet_timestamp,
        RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
        Aura: pallet_aura,
        Grandpa: pallet_grandpa,
        Balances: pallet_balances,
        Vesting: pallet_vesting,
        DappsStaking: pallet_dapps_staking,
        BlockReward: pallet_block_reward,
        TransactionPayment: pallet_transaction_payment,
        EVM: pallet_evm,
        Ethereum: pallet_ethereum,
        EthCall: pallet_custom_signatures,
        BaseFee: pallet_base_fee,
        Contracts: pallet_contracts,
        Sudo: pallet_sudo,
        Assets: pallet_assets,
        Scheduler: pallet_scheduler,
        Democracy: pallet_democracy,
        Council: pallet_collective::<Instance1>,
        TechnicalCommittee: pallet_collective::<Instance2>,
        Treasury: pallet_treasury,
        Xvm: pallet_xvm,
        Proxy: pallet_proxy,
        Preimage: pallet_preimage,
        EthereumChecked: pallet_ethereum_checked,
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
>;

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
        [pallet_balances, Balances]
        [pallet_timestamp, Timestamp]
        [pallet_dapps_staking, DappsStaking]
        [pallet_block_reward, BlockReward]
        [pallet_ethereum_checked, EthereumChecked]
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

        fn initialize_block(header: &<Block as BlockT>::Header) {
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
            sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities().into_inner()
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

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
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
            Some(pallet_base_fee::Elasticity::<Runtime>::get())
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
        ) -> pallet_contracts_primitives::ContractExecResult<Balance, EventRecord> {
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
            code: pallet_contracts_primitives::Code<Hash>,
            data: Vec<u8>,
            salt: Vec<u8>,
        ) -> pallet_contracts_primitives::ContractInstantiateResult<AccountId, Balance, EventRecord> {
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
        ) -> pallet_contracts_primitives::CodeUploadResult<Hash, Balance>
        {
            Contracts::bare_upload_code(origin, code, storage_deposit_limit, determinism)
        }

        fn get_storage(
            address: AccountId,
            key: Vec<u8>,
        ) -> pallet_contracts_primitives::GetStorageResult {
            Contracts::get_storage(address, key)
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
            use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, TrackedStorageKey};
            use frame_system_benchmarking::Pallet as SystemBench;
            use baseline::Pallet as BaselineBench;

            impl frame_system_benchmarking::Config for Runtime {}
            impl baseline::Config for Runtime {}

            use frame_support::traits::WhitelistedStorageKeys;
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
        ) -> Result<
            (),
            sp_runtime::DispatchError,
        > {
            use moonbeam_evm_tracer::tracer::EvmTracer;

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
        ) -> Result<
            (),
            sp_runtime::DispatchError,
        > {
            use moonbeam_evm_tracer::tracer::EvmTracer;

            let mut config = <Runtime as pallet_evm::Config>::config().clone();
            config.estimate = true;

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
