//! The Shiden Network runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

use frame_support::{
    construct_runtime, match_type, parameter_types,
    traits::{Contains, Currency, FindAuthor, Imbalance, OnUnbalanced},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, WEIGHT_PER_SECOND},
        DispatchClass, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients,
        WeightToFeePolynomial,
    },
    ConsensusEngineId, PalletId,
};
use frame_system::limits::{BlockLength, BlockWeights};
use pallet_evm::{FeeCalculator, Runner};
use pallet_transaction_payment::{
    FeeDetails, Multiplier, RuntimeDispatchInfo, TargetedFeeAdjustment,
};
use polkadot_runtime_common::{BlockHashCount, RocksDbWeight};
use sp_api::impl_runtime_apis;
use sp_core::{OpaqueMetadata, H160, H256, U256};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{
        AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, ConvertInto,
        OpaqueKeys, Verify,
    },
    transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, FixedPointNumber, Perbill, Perquintill,
};
use sp_std::prelude::*;

#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// XCM support
use xcm::v0::{Junction::*, MultiLocation, MultiLocation::*};
use xcm_builder::{
    AllowUnpaidExecutionFrom, FixedWeightBounds, LocationInverter, ParentAsSuperuser,
    ParentIsDefault, SovereignSignedViaLocation,
};
use xcm_executor::{Config, XcmExecutor};

pub use pallet_balances::Call as BalancesCall;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

mod precompiles;
pub use precompiles::ShibuyaNetworkPrecompiles;

/// Constant values used within the runtime.
pub const MILLISDN: Balance = 1_000_000_000_000_000;
pub const SDN: Balance = 1_000 * MILLISDN;

/// Charge fee for stored bytes and items.
pub const fn deposit(items: u32, bytes: u32) -> Balance {
    (items as Balance + bytes as Balance) * MILLISDN / 1_000_000
}

/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 12000;
// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

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
    spec_name: create_runtime_str!("shibuya"),
    impl_name: create_runtime_str!("shibuya"),
    authoring_version: 1,
    spec_version: 5,
    impl_version: 0,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
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
const MAXIMUM_BLOCK_WEIGHT: Weight = WEIGHT_PER_SECOND / 2;

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
impl Contains<Call> for BaseFilter {
    fn contains(call: &Call) -> bool {
        match call {
            // These modules are not allowed to be called by transactions:
            // To leave collator just shutdown it, next session funds will be released
            Call::CollatorSelection(pallet_collator_selection::Call::leave_intent(..)) => false,
            // Other modules should works:
            _ => true,
        }
    }
}

impl frame_system::Config for Runtime {
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type Call = Call;
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
    type Event = Event;
    /// The ubiquitous origin type.
    type Origin = Origin;
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
    type SystemWeightInfo = ();
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type SS58Prefix = SS58Prefix;
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
}

parameter_types! {
    pub const MinimumPeriod: u64 = MILLISECS_PER_BLOCK / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = BlockReward;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const BasicDeposit: Balance = 10 * SDN;       // 258 bytes on-chain
    pub const FieldDeposit: Balance = 25 * MILLISDN;  // 66 bytes on-chain
    pub const SubAccountDeposit: Balance = 2 * SDN;   // 53 bytes on-chain
    pub const MaxSubAccounts: u32 = 100;
    pub const MaxAdditionalFields: u32 = 100;
    pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type BasicDeposit = BasicDeposit;
    type FieldDeposit = FieldDeposit;
    type SubAccountDeposit = SubAccountDeposit;
    type MaxSubAccounts = MaxSubAccounts;
    type MaxAdditionalFields = MaxAdditionalFields;
    type MaxRegistrars = MaxRegistrars;
    type Slashed = ();
    type ForceOrigin = frame_system::EnsureRoot<<Self as frame_system::Config>::AccountId>;
    type RegistrarOrigin = frame_system::EnsureRoot<<Self as frame_system::Config>::AccountId>;
    type WeightInfo = ();
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = deposit(0, 32);
    pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = MaxSignatories;
    type WeightInfo = ();
}

parameter_types! {
    pub const EcdsaUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
    pub const CallFee: Balance = SDN / 10;
    pub const CallMagicNumber: u16 = 0xff51;
}

impl pallet_custom_signatures::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Signature = pallet_custom_signatures::ethereum::EthereumSignature;
    type Signer = <Signature as Verify>::Signer;
    type CallMagicNumber = CallMagicNumber;
    type Currency = Balances;
    type CallFee = CallFee;
    type OnChargeTransaction = ToStakingPot;
    type UnsignedPriority = EcdsaUnsignedPriority;
}

impl pallet_utility::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type WeightInfo = ();
}

parameter_types! {
    // We do anything the parent chain tells us in this runtime.
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 2;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type Event = Event;
    type OnValidationData = ();
    type SelfParaId = parachain_info::Pallet<Runtime>;
    type OutboundXcmpMessageSource = ();
    type DmpMessageHandler = cumulus_pallet_xcm::UnlimitedDmpExecution<Runtime>;
    type ReservedDmpWeight = ReservedDmpWeight;
    type XcmpMessageHandler = ();
    type ReservedXcmpWeight = ();
}

impl parachain_info::Config for Runtime {}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
    pub const UncleGenerations: BlockNumber = 5;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = (CollatorSelection,);
}

parameter_types! {
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(33);
    pub const SessionPeriod: BlockNumber = 1 * HOURS;
    pub const SessionOffset: BlockNumber = 0;
}

impl pallet_session::Config for Runtime {
    type Event = Event;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ShouldEndSession = pallet_session::PeriodicSessions<SessionPeriod, SessionOffset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<SessionPeriod, SessionOffset>;
    type SessionManager = CollatorSelection;
    type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const PotId: PalletId = PalletId(*b"PotStake");
    pub const MaxCandidates: u32 = 200;
    pub const MinCandidates: u32 = 5;
    pub const MaxInvulnerables: u32 = 20;
}

impl pallet_collator_selection::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type UpdateOrigin = frame_system::EnsureRoot<AccountId>;
    type PotId = PotId;
    type MaxCandidates = MaxCandidates;
    type MinCandidates = MinCandidates;
    type MaxInvulnerables = MaxInvulnerables;
    // should be a multiple of session or things will get inconsistent
    type KickThreshold = SessionPeriod;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ValidatorRegistration = Session;
    type WeightInfo = ();
}

parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const DappsStakingPalletId: PalletId = PalletId(*b"py/dpsst");
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct ToStakingPot;
impl OnUnbalanced<NegativeImbalance> for ToStakingPot {
    fn on_nonzero_unbalanced(amount: NegativeImbalance) {
        let staking_pot = PotId::get().into_account();
        Balances::resolve_creating(&staking_pot, amount);
    }
}

pub struct OnBlockReward;
impl OnUnbalanced<NegativeImbalance> for OnBlockReward {
    fn on_nonzero_unbalanced(amount: NegativeImbalance) {
        let (dapps, maintain) = amount.ration(50, 50);
        // dapp staking block reward
        Balances::resolve_creating(&DappsStakingPalletId::get().into_account(), dapps);

        let (treasury, collators) = maintain.ration(40, 10);
        // treasury slice of block reward
        Balances::resolve_creating(&TreasuryPalletId::get().into_account(), treasury);
        // collators block reward
        ToStakingPot::on_unbalanced(collators);
    }
}

parameter_types! {
    pub const RewardAmount: Balance = 2_664 * MILLISDN;
}

impl pallet_block_reward::Config for Runtime {
    type Currency = Balances;
    type OnBlockReward = OnBlockReward;
    type RewardAmount = RewardAmount;
}

parameter_types! {
    pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// bias the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<ParentIsDefault<AccountId>, Origin>,
    // Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
    // transaction from the Root origin.
    ParentAsSuperuser<Origin>,
);

match_type! {
    pub type JustTheParent: impl Contains<MultiLocation> = { X1(Parent) };
}

parameter_types! {
    // One XCM operation is 1_000_000 weight - almost certainly a conservative estimate.
    pub UnitWeightCost: Weight = 1_000_000;
}

pub struct XcmConfig;
impl Config for XcmConfig {
    type Call = Call;
    type XcmSender = (); // sending XCM not supported
    type AssetTransactor = (); // balances not supported
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = (); // balances not supported
    type IsTeleporter = (); // balances not supported
    type LocationInverter = LocationInverter<Ancestry>;
    type Barrier = AllowUnpaidExecutionFrom<JustTheParent>;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call>; // balances not supported
    type Trader = (); // balances not supported
    type ResponseHandler = (); // Don't handle responses for now.
}

impl cumulus_pallet_xcm::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1_000_000;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Runtime>;
    type WeightInfo = ();
}

parameter_types! {
    pub const MinVestedTransfer: Balance = 1 * SDN;
}

impl pallet_vesting::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type BlockNumberToBalance = ConvertInto;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = ();
}

parameter_types! {
    pub const TransactionByteFee: Balance = MILLISDN / 100;
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
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
        // in Shiden, extrinsic base weight (smallest non-zero weight) is mapped to 1/10 mSDN:
        let p = MILLISDN;
        let q = 10 * Balance::from(ExtrinsicBaseWeight::get());
        smallvec::smallvec![WeightToFeeCoefficient {
            degree: 1,
            negative: false,
            coeff_frac: Perbill::from_rational(p % q, q),
            coeff_integer: p / q,
        }]
    }
}

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
        if let Some(mut fees) = fees_then_tips.next() {
            if let Some(tips) = fees_then_tips.next() {
                tips.merge_into(&mut fees);
            }
            // pay fees to collators
            <ToStakingPot as OnUnbalanced<_>>::on_unbalanced(fees);
        }
    }
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, DealWithFees>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = WeightToFee;
    type FeeMultiplierUpdate =
        TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
}

/// Current approximation of the gas/s consumption considering
/// EVM execution over compiled WASM (on 4.4Ghz CPU).
/// Given the 500ms Weight, from which 75% only are used for transactions,
/// the total EVM execution gas limit is: GAS_PER_SECOND * 0.500 * 0.75 ~= 15_000_000.
pub const GAS_PER_SECOND: u64 = 40_000_000;

/// Approximate ratio of the amount of Weight per Gas.
/// u64 works for approximations because Weight is a very small unit compared to gas.
pub const WEIGHT_PER_GAS: u64 = WEIGHT_PER_SECOND / GAS_PER_SECOND;

pub struct ShidenGasWeightMapping;
impl pallet_evm::GasWeightMapping for ShidenGasWeightMapping {
    fn gas_to_weight(gas: u64) -> Weight {
        gas.saturating_mul(WEIGHT_PER_GAS)
    }

    fn weight_to_gas(weight: Weight) -> u64 {
        use core::convert::TryFrom;

        u64::try_from(weight.wrapping_div(WEIGHT_PER_GAS)).unwrap_or(u32::MAX as u64)
    }
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
    fn min_gas_price() -> U256 {
        (MILLISDN / 1_000_000).into()
    }
}

pub struct FindAuthorTruncated<F>(sp_std::marker::PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for FindAuthorTruncated<F> {
    fn find_author<'a, I>(digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        use sp_core::crypto::Public;

        if let Some(author_index) = F::find_author(digests) {
            let authority_id = Aura::authorities()[author_index as usize].clone();
            return Some(H160::from_slice(&authority_id.to_raw_vec()[4..24]));
        }

        None
    }
}

parameter_types! {
    /// Ethereum-compatible chain_id:
    /// * Dusty:   80
    /// * Shibuya: 81
    /// * Shiden: 336
    pub ChainId: u64 = 0x51;
    /// EVM gas limit
    pub BlockGasLimit: U256 = U256::from(
        NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT / WEIGHT_PER_GAS
    );
}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = FixedGasPrice;
    type GasWeightMapping = ShidenGasWeightMapping;
    type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Runtime>;
    type CallOrigin = pallet_evm::EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = pallet_evm::EnsureAddressTruncated;
    type AddressMapping = pallet_evm::HashedAddressMapping<BlakeTwo256>;
    type Currency = Balances;
    type Event = Event;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type Precompiles = ShibuyaNetworkPrecompiles<Self>;
    type ChainId = ChainId;
    type OnChargeTransaction = pallet_evm::EVMCurrencyAdapter<Balances, ToStakingPot>;
    type BlockGasLimit = BlockGasLimit;
    type FindAuthor = FindAuthorTruncated<Aura>;
}

pub struct TransactionConverter;

impl fp_rpc::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
    fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> UncheckedExtrinsic {
        UncheckedExtrinsic::new_unsigned(
            pallet_ethereum::Call::<Runtime>::transact(transaction).into(),
        )
    }
}

impl fp_rpc::ConvertTransaction<sp_runtime::OpaqueExtrinsic> for TransactionConverter {
    fn convert_transaction(
        &self,
        transaction: pallet_ethereum::Transaction,
    ) -> sp_runtime::OpaqueExtrinsic {
        use codec::{Decode, Encode};

        let extrinsic = UncheckedExtrinsic::new_unsigned(
            pallet_ethereum::Call::<Runtime>::transact(transaction).into(),
        );
        let encoded = extrinsic.encode();
        sp_runtime::OpaqueExtrinsic::decode(&mut &encoded[..])
            .expect("Encoded extrinsic is always valid")
    }
}

impl pallet_ethereum::Config for Runtime {
    type Event = Event;
    type StateRoot = pallet_ethereum::IntermediateStateRoot;
}

impl pallet_sudo::Config for Runtime {
    type Event = Event;
    type Call = Call;
}

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = generic::Block<Header, sp_runtime::OpaqueExtrinsic>,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>} = 10,
        Utility: pallet_utility::{Pallet, Call, Event} = 11,
        Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 12,
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 13,
        Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 14,
        EthCall: pallet_custom_signatures::{Pallet, Call, Event<T>, ValidateUnsigned} = 15,

        ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event<T>} = 20,
        ParachainInfo: parachain_info::{Pallet, Storage, Config} = 21,

        TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 30,
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 31,
        Vesting: pallet_vesting::{Pallet, Call, Storage, Config<T>, Event<T>} = 32,
        BlockReward: pallet_block_reward::{Pallet} = 33,

        Authorship: pallet_authorship::{Pallet, Call, Storage, Inherent} = 40,
        CollatorSelection: pallet_collator_selection::{Pallet, Call, Storage, Event<T>, Config<T>} = 41,
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 42,
        Aura: pallet_aura::{Pallet, Storage, Config<T>} = 43,
        AuraExt: cumulus_pallet_aura_ext::{Pallet, Storage, Config} = 44,

        CumulusXcm: cumulus_pallet_xcm::{Pallet, Call, Event<T>, Origin} = 50,

        EVM: pallet_evm::{Pallet, Config, Call, Storage, Event<T>} = 60,
        Ethereum: pallet_ethereum::{Pallet, Call, Storage, Event, Config, ValidateUnsigned} = 61,

        Sudo: pallet_sudo::{Pallet, Call, Storage, Event<T>, Config<T>} = 99,
    }
);

/// Balance of an account.
pub type Balance = u128;
/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = sp_runtime::MultiSignature;
/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as sp_runtime::traits::Verify>::Signer as sp_runtime::traits::IdentifyAccount>::AccountId;
/// Index of a transaction in the chain.
pub type Index = u32;
/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;
/// An index to a block.
pub type BlockNumber = u32;
/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
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
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPallets,
>;

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            Runtime::metadata().into()
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities()
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

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
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
        fn collect_collation_info() -> cumulus_primitives_core::CollationInfo {
            ParachainSystem::collect_collation_info()
        }
    }

    impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
        fn chain_id() -> u64 {
            ChainId::get()
        }

        fn account_basic(address: H160) -> pallet_evm::Account {
            EVM::account_basic(&address)
        }

        fn gas_price() -> U256 {
            <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price()
        }

        fn account_code_at(address: H160) -> Vec<u8> {
            EVM::account_codes(address)
        }

        fn author() -> H160 {
            <pallet_evm::Pallet<Runtime>>::find_author()
        }

        fn storage_at(address: H160, index: U256) -> H256 {
            let mut tmp = [0u8; 32];
            index.to_big_endian(&mut tmp);
            EVM::account_storages(address, H256::from_slice(&tmp[..]))
        }

        fn call(
            from: H160,
            to: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            gas_price: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
        ) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };

            <Runtime as pallet_evm::Config>::Runner::call(
                from,
                to,
                data,
                value,
                gas_limit.low_u64(),
                gas_price,
                nonce,
                config
                    .as_ref()
                    .unwrap_or_else(|| <Runtime as pallet_evm::Config>::config()),
            )
            .map_err(|err| err.into())
        }

        fn create(
            from: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            gas_price: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
        ) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };

            #[allow(clippy::or_fun_call)] // suggestion not helpful here
            <Runtime as pallet_evm::Config>::Runner::create(
                from,
                data,
                value,
                gas_limit.low_u64(),
                gas_price,
                nonce,
                config
                    .as_ref()
                    .unwrap_or(<Runtime as pallet_evm::Config>::config()),
                )
                .map_err(|err| err.into())
        }

        fn current_transaction_statuses() -> Option<Vec<fp_rpc::TransactionStatus>> {
            Ethereum::current_transaction_statuses()
        }

        fn current_block() -> Option<pallet_ethereum::Block> {
            Ethereum::current_block()
        }

        fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
            Ethereum::current_receipts()
        }

        fn current_all() -> (
            Option<pallet_ethereum::Block>,
            Option<Vec<pallet_ethereum::Receipt>>,
            Option<Vec<fp_rpc::TransactionStatus>>,
        ) {
            (
                Ethereum::current_block(),
                Ethereum::current_receipts(),
                Ethereum::current_transaction_statuses(),
            )
        }

        fn extrinsic_filter(
            xts: Vec<<Block as BlockT>::Extrinsic>,
        ) -> Vec<pallet_ethereum::Transaction> {
            xts.into_iter().filter_map(|xt| match xt.function {
                Call::Ethereum(pallet_ethereum::Call::transact(t)) => Some(t),
                _ => None
            }).collect::<Vec<pallet_ethereum::Transaction>>()
        }
    }
}

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
    fn check_inherents(
        block: &Block,
        relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
    ) -> sp_inherents::CheckInherentsResult {
        let relay_chain_slot = relay_state_proof
            .read_slot()
            .expect("Could not read the relay chain slot from the proof");
        let inherent_data =
            cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
                relay_chain_slot,
                sp_std::time::Duration::from_secs(6),
            )
            .create_inherent_data()
            .expect("Could not create the timestamp inherent data");
        inherent_data.check_extrinsics(&block)
    }
}

cumulus_pallet_parachain_system::register_validate_block! {
    Runtime = Runtime,
    BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
    CheckInherents = CheckInherents,
}
