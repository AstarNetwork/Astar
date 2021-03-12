//! The Substrate runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

use codec::{Decode, Encode};
use frame_support::{
    construct_runtime,
    pallet_prelude::PhantomData,
    parameter_types,
    traits::{FindAuthor, Randomness},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        DispatchClass, IdentityFee, Weight,
    },
    ConsensusEngineId,
};
use frame_system::limits::{BlockLength, BlockWeights};
use orml_xcm_support::{
    CurrencyIdConverter, IsConcreteWithGeneralKey, MultiCurrencyAdapter, NativePalletAssetOr,
};
use pallet_contracts::weights::WeightInfo;
use pallet_evm::{
    Account as EVMAccount, EnsureAddressRoot, EnsureAddressTruncated, FeeCalculator,
    HashedAddressMapping, Runner,
};
use pallet_transaction_payment::{
    FeeDetails, Multiplier, RuntimeDispatchInfo, TargetedFeeAdjustment,
};
use plasm_primitives::{
    AccountId, Amount, Balance, BlockNumber, CurrencyId, Hash, Index, Moment, Signature,
    TokenSymbol,
};
use sp_api::impl_runtime_apis;
use sp_core::{OpaqueMetadata, H160, H256, U256};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::traits::{
    AccountIdConversion, BlakeTwo256, Block as BlockT, Convert, IdentityLookup,
};
use sp_runtime::transaction_validity::{TransactionSource, TransactionValidity};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys, ApplyExtrinsicResult, FixedPointNumber,
    ModuleId, Perbill, Perquintill,
};
use sp_std::{collections::btree_set::BTreeSet, convert::TryFrom, prelude::*};
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use cumulus_primitives_core::relay_chain::Balance as RelayChainBalance;
use polkadot_parachain::primitives::Sibling;
use xcm::v0::{Junction, MultiLocation, NetworkId};
use xcm_builder::{
    AccountId32Aliases, LocationInverter, ParentIsDefault, RelayChainAsNative,
    SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
    SovereignSignedViaLocation,
};
use xcm_executor::{Config, XcmExecutor};

pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

/// Constant values used within the runtime.
pub mod constants;
use constants::{currency::*, time::*};
mod currency_adapter;
use currency_adapter::NativeCurrencyAdapter;

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
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("plasm-parachain"),
    impl_name: create_runtime_str!("staketechnologies-plasm"),
    authoring_version: 1,
    // Per convention: if the runtime behavior changes, increment spec_version
    // and set impl_version to equal spec_version. If only runtime
    // implementation changes and behavior does not, then leave spec_version as
    // is and increment impl_version.
    spec_version: 6,
    impl_version: 6,
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
    pub const BlockHashCount: BlockNumber = 250;
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

impl frame_system::Config for Runtime {
    type BaseCallFilter = ();
    type Origin = Origin;
    type Call = Call;
    type Index = Index;
    type BlockNumber = BlockNumber;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<AccountId>;
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    type Event = Event;
    type DbWeight = RocksDbWeight;
    type BlockHashCount = BlockHashCount;
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type Version = Version;
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
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
    pub DustAccount: AccountId = ModuleId(*b"orml/dst").into_account();
}

orml_traits::parameter_type_with_key! {
    pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
        Default::default()
    };
}

impl orml_tokens::Config for Runtime {
    type Balance = Balance;
    type Amount = Amount;
    type OnDust = orml_tokens::TransferDust<Runtime, DustAccount>;
    type CurrencyId = CurrencyId;
    type ExistentialDeposits = ExistentialDeposits;
    type WeightInfo = ();
    type Event = Event;
}

parameter_types! {
    pub const GetNativeCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::PLM);
}

impl orml_currencies::Config for Runtime {
    type Event = Event;
    type MultiCurrency = Tokens;
    type NativeCurrency = NativeCurrencyAdapter<Runtime, Balances, Amount, Balance>;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type WeightInfo = ();
}

parameter_types! {
    pub const TransactionByteFee: Balance = 10 * MILLIPLM;
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate =
        TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
}

parameter_types! {
    pub const MinimumPeriod: Moment = MILLISECS_PER_BLOCK / 2;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = Moment;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl_opaque_keys! {
    pub struct SessionKeys {}
}

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

/// Current approximation of the gas/s consumption considering
/// EVM execution over compiled WASM (on 4.4Ghz CPU).
/// Given the 500ms Weight, from which 75% only are used for transactions,
/// the total EVM execution gas limit is: GAS_PER_SECOND * 0.500 * 0.75 => 6_000_000.
pub const GAS_PER_SECOND: u64 = 40_000_000;

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
    type Precompiles = (
        pallet_evm_precompile_simple::ECRecover,
        pallet_evm_precompile_simple::Sha256,
        pallet_evm_precompile_simple::Ripemd160,
        pallet_evm_precompile_simple::Identity,
        pallet_evm_precompile_dispatch::Dispatch<Runtime>,
    );
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

pub struct EthereumFindAuthor<F>(PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for EthereumFindAuthor<F> {
    fn find_author<'a, I>(_digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        // TODO: collator as block author
        None
    }
}

parameter_types! {
    pub BlockGasLimit: U256
        = U256::from(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT / WEIGHT_PER_GAS);
}

impl pallet_ethereum::Config for Runtime {
    type Event = Event;
    type FindAuthor = EthereumFindAuthor<()>;
    type StateRoot = pallet_ethereum::IntermediateStateRoot;
    type BlockGasLimit = BlockGasLimit;
}

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

impl parachain_info::Config for Runtime {}

parameter_types! {
    pub const RococoLocation: MultiLocation = MultiLocation::X1(Junction::Parent);
    pub PlasmNetwork: NetworkId = NetworkId::Named("plasm".into());
    pub PolkadotNetwork: NetworkId = NetworkId::Polkadot;
    pub RelayChainOrigin: Origin = cumulus_pallet_xcm_handler::Origin::Relay.into();
    pub Ancestry: MultiLocation = Junction::Parachain {
        id: ParachainInfo::parachain_id().into()
    }.into();
    pub const RelayChainCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
}

type LocationConverter = (
    ParentIsDefault<AccountId>,
    SiblingParachainConvertsVia<Sibling, AccountId>,
    AccountId32Aliases<PlasmNetwork, AccountId>,
);

type LocalOriginConverter = (
    SovereignSignedViaLocation<LocationConverter, Origin>,
    RelayChainAsNative<RelayChainOrigin, Origin>,
    SiblingParachainAsNative<cumulus_pallet_xcm_handler::Origin, Origin>,
    SignedAccountId32AsNative<PlasmNetwork, Origin>,
);

pub type LocalAssetTransactor = MultiCurrencyAdapter<
    Currencies,
    IsConcreteWithGeneralKey<CurrencyId, RelayToNative>,
    LocationConverter,
    AccountId,
    CurrencyIdConverter<CurrencyId, RelayChainCurrencyId>,
    CurrencyId,
>;

parameter_types! {
    pub NativeOrmlTokens: BTreeSet<(Vec<u8>, MultiLocation)> = {
        let mut t = BTreeSet::new();
        // Acala tokens
        t.insert(("ACA".into(), (Junction::Parent, Junction::Parachain { id: 666 }).into()));
        t.insert(("AUSD".into(), (Junction::Parent, Junction::Parachain { id: 666 }).into()));
        t.insert(("XBTC".into(), (Junction::Parent, Junction::Parachain { id: 666 }).into()));
        t.insert(("LDOT".into(), (Junction::Parent, Junction::Parachain { id: 666 }).into()));
        t
    };
}

pub struct XcmConfig;
impl Config for XcmConfig {
    type Call = Call;
    type XcmSender = XcmHandler;
    // How to withdraw and deposit an asset.
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = LocalOriginConverter;
    type IsReserve = NativePalletAssetOr<NativeOrmlTokens>;
    type IsTeleporter = ();
    type LocationInverter = LocationInverter<Ancestry>;
}

impl cumulus_pallet_xcm_handler::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type UpwardMessageSender = ParachainSystem;
    type HrmpMessageSender = ParachainSystem;
    type SendXcmOrigin = frame_system::EnsureRoot<AccountId>;
    type AccountIdConverter = LocationConverter;
}

pub struct RelayToNative;
impl Convert<RelayChainBalance, Balance> for RelayToNative {
    fn convert(val: u128) -> Balance {
        // native is 15
        // relay is 12
        val * 1_000
    }
}

pub struct NativeToRelay;
impl Convert<Balance, RelayChainBalance> for NativeToRelay {
    fn convert(val: u128) -> Balance {
        // native is 15
        // relay is 12
        val / 1_000
    }
}

pub struct AccountId32Convert;
impl Convert<AccountId, [u8; 32]> for AccountId32Convert {
    fn convert(account_id: AccountId) -> [u8; 32] {
        account_id.into()
    }
}

impl orml_xtokens::Config for Runtime {
    type Event = Event;
    type Balance = Balance;
    type ToRelayChainBalance = NativeToRelay;
    type AccountId32Convert = AccountId32Convert;
    type RelayChainNetworkId = PolkadotNetwork;
    type AccountIdConverter = LocationConverter;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ParaId = ParachainInfo;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type Event = Event;
    type OnValidationData = ();
    type SelfParaId = parachain_info::Module<Runtime>;
    type DownwardMessageHandlers = XcmHandler;
    type HrmpMessageHandlers = XcmHandler;
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
        TransactionPayment: pallet_transaction_payment::{Module, Storage},
        Balances: pallet_balances::{Module, Call, Storage, Event<T>, Config<T>},
        Currencies: orml_currencies::{Module, Call, Event<T>},
        Tokens: orml_tokens::{Module, Storage, Event<T>, Config<T>},
        Contracts: pallet_contracts::{Module, Call, Storage, Event<T>, Config<T>},
        Ethereum: pallet_ethereum::{Module, Call, Storage, Event, Config, ValidateUnsigned},
        EVM: pallet_evm::{Module, Call, Storage, Config, Event<T>},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
        Sudo: pallet_sudo::{Module, Call, Storage, Event<T>, Config<T>},
        Nicks: pallet_nicks::{Module, Call, Storage, Event<T>},
        ParachainSystem: cumulus_pallet_parachain_system::{Module, Call, Storage, Inherent, Event},
        ParachainInfo: parachain_info::{Module, Storage, Config},
        XcmHandler: cumulus_pallet_xcm_handler::{Module, Event<T>, Origin, Call},
        XTokens: orml_xtokens::{Module, Storage, Call, Event<T>},
    }
);

/// The address format for describing accounts.
pub type Address = AccountId;
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

cumulus_pallet_parachain_system::register_validate_block!(Runtime, Executive);
