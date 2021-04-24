//! The Substrate runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

use codec::{Decode, Encode};
use frame_support::{
    construct_runtime, debug, parameter_types,
    traits::{FindAuthor, KeyOwnerProofSystem, Randomness},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        DispatchClass, IdentityFee, Weight,
    },
};
use frame_system::limits::{BlockLength, BlockWeights};
use pallet_contracts::weights::WeightInfo;
use pallet_evm::{
    Account as EVMAccount, EnsureAddressRoot, EnsureAddressTruncated, FeeCalculator,
    HashedAddressMapping, Runner,
};
use pallet_grandpa::fg_primitives;
use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as pallet_session_historical;
use pallet_transaction_payment::{
    CurrencyAdapter, FeeDetails, Multiplier, RuntimeDispatchInfo, TargetedFeeAdjustment,
};
use plasm_primitives::{
    AccountId, AccountIndex, Balance, BlockNumber, Hash, Index, Moment, Signature,
};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160, H256, U256};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::traits::{
    BlakeTwo256, Block as BlockT, ConvertInto, Extrinsic, NumberFor, OpaqueKeys,
    SaturatedConversion, StaticLookup, Verify,
};
use sp_runtime::transaction_validity::{
    TransactionPriority, TransactionSource, TransactionValidity,
};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys, ApplyExtrinsicResult, FixedPointNumber, Perbill,
    Perquintill, RuntimeAppPublic,
};
use sp_std::convert::TryFrom;
use sp_std::prelude::*;
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

/// Constant values used within the runtime.
pub mod constants;
use constants::{currency::*, time::*};

mod precompiles;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("dusty4"),
    impl_name: create_runtime_str!("staketechnologies-plasm"),
    authoring_version: 4,
    // Per convention: if the runtime behavior changes, increment spec_version
    // and set impl_version to equal spec_version. If only runtime
    // implementation changes and behavior does not, then leave spec_version as
    // is and increment impl_version.
    spec_version: 2,
    impl_version: 2,
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

/// We assume that ~10% of the block weight is consumed by `on_initalize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = 2 * WEIGHT_PER_SECOND;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
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
    pub const SS58Prefix: u8 = 5;
}

impl frame_system::Config for Runtime {
    type BaseCallFilter = ();
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type DbWeight = RocksDbWeight;
    type Origin = Origin;
    type Call = Call;
    type Index = Index;
    type BlockNumber = BlockNumber;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = Indices;
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = Version;
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
    type SS58Prefix = SS58Prefix;
}

parameter_types! {
    pub const EpochDuration: u64 = EPOCH_DURATION_IN_BLOCKS as u64;
    pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
    pub const ReportLongevity: u64 =
        BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

impl pallet_babe::Config for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;
    type EpochChangeTrigger = pallet_babe::ExternalTrigger;

    type KeyOwnerProofSystem = Historical;

    type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        pallet_babe::AuthorityId,
    )>>::Proof;

    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        pallet_babe::AuthorityId,
    )>>::IdentificationTuple;

    type HandleEquivocation =
        pallet_babe::EquivocationHandler<Self::KeyOwnerIdentification, Offences, ReportLongevity>;

    type WeightInfo = ();
}

parameter_types! {
    pub const IndexDeposit: Balance = 1 * PLM;
}

impl pallet_indices::Config for Runtime {
    type AccountIndex = AccountIndex;
    type Event = Event;
    type Currency = Balances;
    type Deposit = IndexDeposit;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1 * MILLIPLM;
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type MaxLocks = MaxLocks;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Module<Runtime>;
    type WeightInfo = ();
}

parameter_types! {
    pub const TransactionByteFee: Balance = 10 * MILLIPLM;
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate =
        TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
}

parameter_types! {
    pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = Moment;
    type OnTimestampSet = Babe;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const UncleGenerations: BlockNumber = 5;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = ImOnline;
}

impl_opaque_keys! {
    pub struct SessionKeys {
        pub babe: Babe,
        pub grandpa: Grandpa,
        pub im_online: ImOnline,
    }
}

parameter_types! {
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

impl pallet_session::Config for Runtime {
    type SessionManager = PlasmRewards;
    type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type ShouldEndSession = Babe;
    type NextSessionRotation = Babe;
    type Event = Event;
    type Keys = SessionKeys;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = ConvertInto;
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
    type WeightInfo = ();
}

impl pallet_session::historical::Config for Runtime {
    type FullIdentification = ();
    type FullIdentificationOf = ();
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
        RuntimeBlockWeights::get().max_block;
    pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
    type Event = Event;
    type Origin = Origin;
    type PalletsOrigin = OriginCaller;
    type Call = Call;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = frame_system::EnsureRoot<AccountId>;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = ();
}

parameter_types! {
    pub const SessionsPerEra: pallet_plasm_rewards::SessionIndex = 6;
    pub const BondingDuration: pallet_plasm_rewards::EraIndex = 24 * 28;
}

impl pallet_plasm_rewards::Config for Runtime {
    type Currency = Balances;
    type Time = Timestamp;
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type ComputeEraForDapps = pallet_plasm_rewards::DefaultForDappsStaking<Runtime>;
    type ComputeEraForSecurity = PlasmValidator;
    type ComputeTotalPayout = pallet_plasm_rewards::inflation::CommunityRewards<u32>;
    type MaybeValidators = PlasmValidator;
    type Event = Event;
}

impl pallet_plasm_validator::Config for Runtime {
    type Currency = Balances;
    type Time = Timestamp;
    type RewardRemainder = (); // Reward remainder is burned.
    type Reward = (); // Reward is minted.
    type EraFinder = PlasmRewards;
    type ForSecurityEraReward = PlasmRewards;
    type ComputeEraParam = u32;
    type ComputeEra = PlasmValidator;
    type Event = Event;
}

/*
impl pallet_dapps_staking::Config for Runtime {
    type Currency = Balances;
    type BondingDuration = BondingDuration;
    type ContractFinder = Operator;
    type RewardRemainder = (); // Reward remainder is burned.
    type Reward = (); // Reward is minted.
    type Time = Timestamp;
    type ComputeRewardsForDapps = pallet_dapps_staking::rewards::VoidableRewardsForDapps;
    type EraFinder = PlasmRewards;
    type ForDappsEraReward = PlasmRewards;
    type HistoryDepthFinder = PlasmRewards;
    type Event = Event;
}
*/

parameter_types! {
    pub const TombstoneDeposit: Balance = deposit(
        1,
        sp_std::mem::size_of::<pallet_contracts::ContractInfo<Runtime>>() as u32
    );
    pub const DepositPerContract: Balance = TombstoneDeposit::get();
    pub const DepositPerStorageByte: Balance = deposit(0, 1);
    pub const DepositPerStorageItem: Balance = deposit(1, 0);
    pub RentFraction: Perbill = Perbill::from_rational_approximation(1u32, 30 * DAYS);
    pub const SurchargeReward: Balance = 150 * MILLIPLM;
    pub const SignedClaimHandicap: u32 = 2;
    pub const MaxDepth: u32 = 32;
    pub const MaxValueSize: u32 = 16 * 1024;
    // The lazy deletion runs inside on_initialize.
    pub DeletionWeightLimit: Weight = AVERAGE_ON_INITIALIZE_RATIO *
        RuntimeBlockWeights::get().max_block;
    // The weight needed for decoding the queue should be less or equal than a fifth
    // of the overall weight dedicated to the lazy deletion.
    pub DeletionQueueDepth: u32 = ((DeletionWeightLimit::get() / (
            <Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(1) -
            <Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(0)
        )) / 5) as u32;
    pub MaxCodeSize: u32 = 128 * 1024;
}

impl pallet_contracts::Config for Runtime {
    type Time = Timestamp;
    type Randomness = RandomnessCollectiveFlip;
    type Currency = Balances;
    type Event = Event;
    type RentPayment = ();
    type SignedClaimHandicap = SignedClaimHandicap;
    type TombstoneDeposit = TombstoneDeposit;
    type DepositPerContract = DepositPerContract;
    type DepositPerStorageByte = DepositPerStorageByte;
    type DepositPerStorageItem = DepositPerStorageItem;
    type RentFraction = RentFraction;
    type SurchargeReward = SurchargeReward;
    type MaxDepth = MaxDepth;
    type MaxValueSize = MaxValueSize;
    type WeightPrice = pallet_transaction_payment::Module<Self>;
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type ChainExtension = ();
    type DeletionQueueDepth = DeletionQueueDepth;
    type DeletionWeightLimit = DeletionWeightLimit;
    type MaxCodeSize = MaxCodeSize;
}

/*
impl pallet_contract_operator::Config for Runtime {
    type Parameters = pallet_dapps_staking::parameters::StakingParameters;
    type Event = Event;
}

impl pallet_operator_trading::Config for Runtime {
    type Currency = Balances;
    type OperatorFinder = Operator;
    type TransferOperator = Operator;
    type Event = Event;
}
*/

impl pallet_utility::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
    type Event = Event;
    type Call = Call;
}

parameter_types! {
    pub const SessionDuration: BlockNumber = EPOCH_DURATION_IN_SLOTS as _;
    pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}

impl pallet_im_online::Config for Runtime {
    type AuthorityId = ImOnlineId;
    type Event = Event;
    type ValidatorSet = Historical;
    type SessionDuration = SessionDuration;
    type ReportUnresponsiveness = Offences;
    type UnsignedPriority = ImOnlineUnsignedPriority;
    type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub OffencesWeightSoftLimit: Weight = Perbill::from_percent(60) *
        RuntimeBlockWeights::get().max_block;
}

impl pallet_offences::Config for Runtime {
    type Event = Event;
    type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
    type OnOffenceHandler = ();
    type WeightSoftLimit = OffencesWeightSoftLimit;
}

impl pallet_grandpa::Config for Runtime {
    type Event = Event;
    type Call = Call;

    type KeyOwnerProofSystem = Historical;

    type KeyOwnerProof =
        <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        GrandpaId,
    )>>::IdentificationTuple;

    type HandleEquivocation = pallet_grandpa::EquivocationHandler<
        Self::KeyOwnerIdentification,
        Offences,
        ReportLongevity,
    >;

    type WeightInfo = ();
}

parameter_types! {
    pub const EcdsaUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
}

impl pallet_custom_signatures::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Signature = pallet_custom_signatures::ethereum::EthereumSignature;
    type Signer = <Signature as Verify>::Signer;
    type UnsignedPriority = EcdsaUnsignedPriority;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
    Call: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: Call,
        public: <Signature as Verify>::Signer,
        account: AccountId,
        nonce: Index,
    ) -> Option<(Call, <UncheckedExtrinsic as Extrinsic>::SignaturePayload)> {
        // take the biggest period possible.
        let period = BlockHashCount::get()
            .checked_next_power_of_two()
            .map(|c| c / 2)
            .unwrap_or(2) as u64;
        let current_block = System::block_number()
            .saturated_into::<u64>()
            // The `System::block_number` is initialized with `n+1`,
            // so the actual block number is `n`.
            .saturating_sub(1);
        let tip = 0;
        let extra: SignedExtra = (
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
            frame_system::CheckNonce::<Runtime>::from(nonce),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
        );
        let raw_payload = SignedPayload::new(call, extra)
            .map_err(|e| {
                debug::warn!("Unable to create signed payload: {:?}", e);
            })
            .ok()?;
        let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
        let (call, extra, _) = raw_payload.deconstruct();
        let address = Indices::unlookup(account);
        Some((call, (address, signature.into(), extra)))
    }
}

impl frame_system::offchain::SigningTypes for Runtime {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    Call: From<C>,
{
    type OverarchingCall = Call;
    type Extrinsic = UncheckedExtrinsic;
}

/*
parameter_types! {
    pub const MaxDepth: u32 = 32;
    pub const DisputePeriod: BlockNumber = 7;
}

pub struct GetAtomicPredicateIdConfig;
impl Get<pallet_ovm::AtomicPredicateIdConfig<AccountId, Hash>> for GetAtomicPredicateIdConfig {
    fn get() -> pallet_ovm::AtomicPredicateIdConfig<AccountId, Hash> {
        pallet_ovm::AtomicPredicateIdConfig {
            not_address: ([1; 32]).into(),
            and_address: ([2; 32]).into(),
            or_address: ([3; 32]).into(),
            for_all_address: ([4; 32]).into(),
            there_exists_address: ([5; 32]).into(),
            equal_address: ([6; 32]).into(),
            is_contained_address: ([7; 32]).into(),
            is_less_address: ([8; 32]).into(),
            is_stored_address: ([9; 32]).into(),
            is_valid_signature_address: ([10; 32]).into(),
            verify_inclusion_address: ([11; 32]).into(),
            // Keccak256::hash("seck256k1") = d4fa99b1e08c4e5e6deb461846aa629344d95ff03ed04754c2053d54c756f439;
            secp256k1: ([
                212, 250, 153, 177, 224, 140, 78, 94, 109, 235, 70, 24, 70, 170, 98, 147, 68, 217,
                95, 240, 62, 208, 71, 84, 194, 5, 61, 84, 199, 86, 244, 57,
            ])
            .into(),
        }
    }
}

impl pallet_ovm::Config for Runtime {
    type MaxDepth = MaxDepth;
    type DisputePeriod = DisputePeriod;
    type DeterminePredicateAddress = pallet_ovm::SimpleAddressDeterminer<Runtime>;
    type HashingL2 = Keccak256;
    type ExternalCall = pallet_ovm::predicate::CallContext<Runtime>;
    type AtomicPredicateIdConfig = GetAtomicPredicateIdConfig;
    type Event = Event;
}

pub struct MaximumTokenAddress;
impl Get<AccountId> for MaximumTokenAddress {
    fn get() -> AccountId {
        ([255; 32]).into()
    }
}

impl pallet_plasma::Config for Runtime {
    type Currency = Balances;
    type DeterminePlappsAddress = pallet_plasma::SimpleAddressDeterminer<Runtime>;
    type MaximumTokenAddress = MaximumTokenAddress;
    type PlasmaHashing = Keccak256;
    type Event = Event;
}
*/

parameter_types! {
    pub const NickReservationFee: u128 = 0;
    pub const MinNickLength: usize = 4;
    pub const MaxNickLength: usize = 32;
}

impl pallet_nicks::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type ReservationFee = NickReservationFee;
    type Slashed = ();
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type MinLength = MinNickLength;
    type MaxLength = MaxNickLength;
}

/// Current approximation of the gas/s consumption considering
/// EVM execution over compiled WASM (on 4.4Ghz CPU).
/// Given the 500ms Weight, from which 75% only are used for transactions,
/// the total EVM execution gas limit is: GAS_PER_SECOND * 0.500 * 0.75 => 6_000_000.
pub const GAS_PER_SECOND: u64 = 16_000_000;

/// Approximate ratio of the amount of Weight per Gas.
/// u64 works for approximations because Weight is a very small unit compared to gas.
pub const WEIGHT_PER_GAS: u64 = WEIGHT_PER_SECOND / GAS_PER_SECOND;

pub struct GasWeightMapping;
impl pallet_evm::GasWeightMapping for GasWeightMapping {
    fn gas_to_weight(gas: u64) -> Weight {
        Weight::try_from(gas.saturating_mul(WEIGHT_PER_GAS)).unwrap_or(Weight::MAX)
    }
    fn weight_to_gas(weight: Weight) -> u64 {
        u64::try_from(weight.wrapping_div(WEIGHT_PER_GAS)).unwrap_or(u64::MAX)
    }
}

parameter_types! {
    pub const ChainId: u64 = 0x50;
}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = ();
    type GasWeightMapping = GasWeightMapping;
    type CallOrigin = EnsureAddressRoot<Self::AccountId>;
    type WithdrawOrigin = EnsureAddressTruncated;
    type AddressMapping = HashedAddressMapping<BlakeTwo256>;
    type Currency = Balances;
    type Event = Event;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type Precompiles = precompiles::PlasmPrecompiles<Self>;
    type ChainId = ChainId;
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
        let extrinsic = UncheckedExtrinsic::new_unsigned(
            pallet_ethereum::Call::<Runtime>::transact(transaction).into(),
        );
        let encoded = extrinsic.encode();
        sp_runtime::OpaqueExtrinsic::decode(&mut &encoded[..])
            .expect("Encoded extrinsic is always valid")
    }
}

// TODO consensus not supported
pub struct EthereumFindAuthor<F>(sp_std::marker::PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for EthereumFindAuthor<F> {
    fn find_author<'a, I>(digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
    {
        if let Some(author_index) = F::find_author(digests) {
            if (author_index as usize) < Babe::authorities().len() {
                let (authority_id, _) = Babe::authorities()[author_index as usize].clone();
                return Some(H160::from_slice(&authority_id.to_raw_vec()[4..24]));
            }
        }
        None
    }
}

parameter_types! {
    pub BlockGasLimit: U256 = U256::from(9_000_000);
}

impl pallet_ethereum::Config for Runtime {
    type Event = Event;
    type FindAuthor = EthereumFindAuthor<Babe>;
    type StateRoot = pallet_ethereum::IntermediateStateRoot;
    type BlockGasLimit = BlockGasLimit;
}

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = plasm_primitives::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Module, Call, Storage, Config, Event<T>},
        Utility: pallet_utility::{Module, Call, Event},
        Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
        Authorship: pallet_authorship::{Module, Call, Storage, Inherent},
        TransactionPayment: pallet_transaction_payment::{Module, Storage},
        Scheduler: pallet_scheduler::{Module, Call, Storage, Event<T>},
        Indices: pallet_indices::{Module, Call, Storage, Event<T>, Config<T>},
        Balances: pallet_balances::{Module, Call, Storage, Event<T>, Config<T>},
        Contracts: pallet_contracts::{Module, Call, Storage, Event<T>, Config<T>},
        //DappsStaking: pallet_dapps_staking::{Module, Call, Storage, Event<T>},
        PlasmValidator: pallet_plasm_validator::{Module, Call, Storage, Event<T>, Config<T>},
        PlasmRewards: pallet_plasm_rewards::{Module, Call, Storage, Event<T>, Config},
        Session: pallet_session::{Module, Call, Storage, Event, Config<T>},
        Historical: pallet_session_historical::{Module},
        Babe: pallet_babe::{Module, Call, Storage, Config, Inherent, ValidateUnsigned},
        Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event, ValidateUnsigned},
        ImOnline: pallet_im_online::{Module, Call, Storage, Event<T>, ValidateUnsigned, Config<T>},
        Offences: pallet_offences::{Module, Call, Storage, Event},
        //Operator: pallet_contract_operator::{Module, Call, Storage, Event<T>},
        //Trading: pallet_operator_trading::{Module, Call, Storage, Event<T>},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
        Sudo: pallet_sudo::{Module, Call, Storage, Event<T>, Config<T>},
        //OVM: pallet_ovm::{Module, Call, Storage, Event<T>},
        //Plasma: pallet_plasma::{Module, Call, Storage, Event<T>},
        Nicks: pallet_nicks::{Module, Call, Storage, Event<T>},
        Ethereum: pallet_ethereum::{Module, Call, Storage, Event, Config, ValidateUnsigned},
        EVM: pallet_evm::{Module, Call, Storage, Config, Event<T>},
        EthCall: pallet_custom_signatures::{Module, Call, Event<T>, ValidateUnsigned},
    }
);

/// The address format for describing accounts.
pub type Address = <Indices as StaticLookup>::Source;
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
    AllModules,
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

        fn random_seed() -> <Block as BlockT>::Hash {
            RandomnessCollectiveFlip::random_seed()
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> GrandpaAuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: fg_primitives::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Grandpa::submit_unsigned_equivocation_report(
                equivocation_proof,
                key_owner_proof,
            )
        }

        fn generate_key_ownership_proof(
            _set_id: fg_primitives::SetId,
            authority_id: GrandpaId,
        ) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
            use codec::Encode;

            Historical::prove((fg_primitives::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(fg_primitives::OpaqueKeyOwnershipProof::new)
        }
    }

    impl sp_consensus_babe::BabeApi<Block> for Runtime {
        fn configuration() -> sp_consensus_babe::BabeGenesisConfiguration {
            // The choice of `c` parameter (where `1 - c` represents the
            // probability of a slot being empty), is done in accordance to the
            // slot duration and expected target block time, for safely
            // resisting network delays of maximum two seconds.
            // <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
            sp_consensus_babe::BabeGenesisConfiguration {
                slot_duration: Babe::slot_duration(),
                epoch_length: EpochDuration::get(),
                c: PRIMARY_PROBABILITY,
                genesis_authorities: Babe::authorities(),
                randomness: Babe::randomness(),
                allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
            }
        }

        fn current_epoch_start() -> sp_consensus_babe::Slot {
            Babe::current_epoch_start()
        }

        fn current_epoch() -> sp_consensus_babe::Epoch {
            Babe::current_epoch()
        }

        fn next_epoch() -> sp_consensus_babe::Epoch {
            Babe::next_epoch()
        }

        fn generate_key_ownership_proof(
            _slot_number: sp_consensus_babe::Slot,
            authority_id: sp_consensus_babe::AuthorityId,
        ) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
            use codec::Encode;

            Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
            key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Babe::submit_unsigned_equivocation_report(
                equivocation_proof,
                key_owner_proof,
            )
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl pallet_contracts_rpc_runtime_api::ContractsApi<Block, AccountId, Balance, BlockNumber> for Runtime {
        fn call(
            origin: AccountId,
            dest: AccountId,
            value: Balance,
            gas_limit: u64,
            input_data: Vec<u8>,
        ) -> pallet_contracts_primitives::ContractExecResult {
            Contracts::bare_call(origin, dest, value, gas_limit, input_data)
        }

        fn get_storage(
            address: AccountId,
            key: [u8; 32],
        ) -> pallet_contracts_primitives::GetStorageResult {
            Contracts::get_storage(address, key)
        }

        fn rent_projection(
            address: AccountId,
        ) -> pallet_contracts_primitives::RentProjectionResult<BlockNumber> {
            Contracts::rent_projection(address)
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

    impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
        fn chain_id() -> u64 {
            <Runtime as pallet_evm::Config>::ChainId::get()
        }

        fn account_basic(address: H160) -> EVMAccount {
            EVM::account_basic(&address)
        }

        fn gas_price() -> U256 {
            <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price()
        }

        fn account_code_at(address: H160) -> Vec<u8> {
            EVM::account_codes(address)
        }

        fn author() -> H160 {
            <pallet_ethereum::Module<Runtime>>::find_author()
        }

        fn storage_at(address: H160, index: U256) -> H256 {
            let mut tmp = [0u8; 32];
            index.to_big_endian(&mut tmp);
            EVM::account_storages(address, H256::from_slice(&tmp[..]))
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
            Option<Vec<fp_rpc::TransactionStatus>>
        ) {
            (
                Ethereum::current_block(),
                Ethereum::current_receipts(),
                Ethereum::current_transaction_statuses()
            )
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

            #[allow(clippy::or_fun_call)] // suggestion not helpful here
            <Runtime as pallet_evm::Config>::Runner::call(
                from,
                to,
                data,
                value,
                gas_limit.low_u64(),
                gas_price,
                nonce,
                config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
            ).map_err(|err| err.into())
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
                config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
            ).map_err(|err| err.into())
        }
    }
}
