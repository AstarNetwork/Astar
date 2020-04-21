//! The Substrate runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

use frame_support::{construct_runtime, parameter_types, traits::Randomness, weights::Weight};
use pallet_contracts_rpc_runtime_api::ContractExecResult;
use pallet_grandpa::fg_primitives;
use pallet_grandpa::AuthorityList as GrandpaAuthorityList;
use pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo;
use plasm_primitives::{
    AccountId, AccountIndex, Balance, BlockNumber, Hash, Index, Moment, Signature,
};
use sp_api::impl_runtime_apis;
use sp_core::OpaqueMetadata;
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::traits::{
    BlakeTwo256, Block as BlockT, ConvertInto, Extrinsic, OpaqueKeys, SaturatedConversion,
    StaticLookup, Verify,
};
use sp_runtime::transaction_validity::{TransactionSource, TransactionValidity};
use sp_runtime::{create_runtime_str, generic, impl_opaque_keys, ApplyExtrinsicResult, Perbill};
use sp_std::prelude::*;
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use pallet_balances::Call as BalancesCall;
pub use pallet_contracts::Gas;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;

/// Constant values used within the runtime.
pub mod constants;
use constants::{currency::*, time::*};

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("dusty"),
    impl_name: create_runtime_str!("staketechnologies-plasm"),
    authoring_version: 4,
    // Per convention: if the runtime behavior changes, increment spec_version
    // and set impl_version to equal spec_version. If only runtime
    // implementation changes and behavior does not, then leave spec_version as
    // is and increment impl_version.
    spec_version: 40,
    impl_version: 40,
    apis: RUNTIME_API_VERSIONS,
};

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

/// A transaction submitter with the given key type.
pub type TransactionSubmitterOf<KeyType> =
    frame_system::offchain::TransactionSubmitter<KeyType, Runtime, UncheckedExtrinsic>;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const MaximumBlockWeight: Weight = 1_000_000_000;
    pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
    pub const Version: RuntimeVersion = VERSION;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

impl frame_system::Trait for Runtime {
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
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = Version;
    type ModuleToIndex = ModuleToIndex;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
}

parameter_types! {
    pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
    pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}

impl pallet_babe::Trait for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;
    type EpochChangeTrigger = pallet_babe::ExternalTrigger;
}

parameter_types! {
    pub const IndexDeposit: Balance = 1 * PLM;
}

impl pallet_indices::Trait for Runtime {
    type AccountIndex = AccountIndex;
    type Event = Event;
    type Currency = Balances;
    type Deposit = IndexDeposit;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1 * MILLIPLM;
}

impl pallet_balances::Trait for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Module<Runtime>;
}

parameter_types! {
    pub const TransactionBaseFee: Balance = 1 * MILLIPLM;
    pub const TransactionByteFee: Balance = 10 * MILLIPLM;
    // setting this to zero will disable the weight fee.
    pub const WeightFeeCoefficient: Balance = 1_000;
    // for a sane configuration, this should always be less than `AvailableBlockRatio`.
    pub const TargetBlockFullness: Perbill = Perbill::from_percent(25);
}

impl pallet_transaction_payment::Trait for Runtime {
    type Currency = Balances;
    type OnTransactionPayment = ();
    type TransactionBaseFee = TransactionBaseFee;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = impls::LinearWeightToFee<WeightFeeCoefficient>;
    type FeeMultiplierUpdate = impls::TargetedFeeAdjustment<TargetBlockFullness>;
}

parameter_types! {
    pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Trait for Runtime {
    type Moment = Moment;
    type OnTimestampSet = Babe;
    type MinimumPeriod = MinimumPeriod;
}

impl_opaque_keys! {
    pub struct SessionKeys {
        pub babe: Babe,
        pub grandpa: Grandpa,
    }
}

impl pallet_session::Trait for Runtime {
    type SessionManager = PlasmRewards;
    type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type ShouldEndSession = Babe;
    type NextSessionRotation = Babe;
    type Event = Event;
    type Keys = SessionKeys;
    type ValidatorId = <Self as frame_system::Trait>::AccountId;
    type ValidatorIdOf = ConvertInto;
    type DisabledValidatorsThreshold = ();
}

parameter_types! {
    pub const SessionsPerEra: pallet_plasm_rewards::SessionIndex = 6;
    pub const BondingDuration: pallet_plasm_rewards::EraIndex = 24 * 28;
}

impl pallet_plasm_rewards::Trait for Runtime {
    type Currency = Balances;
    type Time = Timestamp;
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type GetForDappsStaking = DappsStaking;
    type GetForSecurityStaking = PlasmValidator;
    type ComputeTotalPayout = pallet_plasm_rewards::inflation::SimpleComputeTotalPayout;
    type MaybeValidators = PlasmValidator;
    type Event = Event;
}

impl pallet_plasm_validator::Trait for Runtime {
    type Currency = Balances;
    type Time = Timestamp;
    type RewardRemainder = (); // Reward remainder is burned.
    type Reward = (); // Reward is minted.
    type EraFinder = PlasmRewards;
    type ForSecurityEraReward = PlasmRewards;
    type Event = Event;
}

impl pallet_dapps_staking::Trait for Runtime {
    type Currency = Balances;
    type BondingDuration = BondingDuration;
    type ContractFinder = Operator;
    type RewardRemainder = (); // Reward remainder is burned.
    type Reward = (); // Reward is minted.
    type Time = Timestamp;
    type ComputeRewardsForDapps = pallet_dapps_staking::rewards::BasedComputeRewardsForDapps;
    type EraFinder = PlasmRewards;
    type ForDappsEraReward = PlasmRewards;
    type Event = Event;
}

parameter_types! {
    pub const ContractTransferFee: Balance = 1 * MILLIPLM;
    pub const ContractCreationFee: Balance = 1 * MILLIPLM;
    pub const ContractTransactionBaseFee: Balance = 1 * MILLIPLM;
    pub const ContractTransactionByteFee: Balance = 10 * MILLIPLM;
    pub const ContractFee: Balance = 1 * MILLIPLM;
    pub const TombstoneDeposit: Balance = 1 * PLM;
    pub const RentByteFee: Balance = 1 * PLM;
    pub const RentDepositOffset: Balance = 1000 * PLM;
    pub const SurchargeReward: Balance = 150 * PLM;
}

impl pallet_contracts::Trait for Runtime {
    type Currency = Balances;
    type Time = Timestamp;
    type Randomness = RandomnessCollectiveFlip;
    type Call = Call;
    type Event = Event;
    type DetermineContractAddress = pallet_contracts::SimpleAddressDeterminer<Runtime>;
    type ComputeDispatchFee = pallet_contracts::DefaultDispatchFeeComputor<Runtime>;
    type TrieIdGenerator = pallet_contracts::TrieIdFromParentCounter<Runtime>;
    type GasPayment = ();
    type RentPayment = ();
    type SignedClaimHandicap = pallet_contracts::DefaultSignedClaimHandicap;
    type TombstoneDeposit = TombstoneDeposit;
    type StorageSizeOffset = pallet_contracts::DefaultStorageSizeOffset;
    type RentByteFee = RentByteFee;
    type RentDepositOffset = RentDepositOffset;
    type SurchargeReward = SurchargeReward;
    type TransactionBaseFee = ContractTransactionBaseFee;
    type TransactionByteFee = ContractTransactionByteFee;
    type ContractFee = ContractFee;
    type CallBaseFee = pallet_contracts::DefaultCallBaseFee;
    type InstantiateBaseFee = pallet_contracts::DefaultInstantiateBaseFee;
    type MaxDepth = pallet_contracts::DefaultMaxDepth;
    type MaxValueSize = pallet_contracts::DefaultMaxValueSize;
    type BlockGasLimit = pallet_contracts::DefaultBlockGasLimit;
}

impl pallet_contract_operator::Trait for Runtime {
    type Parameters = pallet_dapps_staking::parameters::StakingParameters;
    type Event = Event;
}

impl pallet_operator_trading::Trait for Runtime {
    type Currency = Balances;
    type OperatorFinder = Operator;
    type TransferOperator = Operator;
    type Event = Event;
}

impl pallet_sudo::Trait for Runtime {
    type Event = Event;
    type Call = Call;
}

impl pallet_grandpa::Trait for Runtime {
    type Event = Event;
}

parameter_types! {
    pub const WindowSize: BlockNumber = 101;
    pub const ReportLatency: BlockNumber = 1000;
}

impl pallet_finality_tracker::Trait for Runtime {
    type OnFinalizationStalled = Grandpa;
    type WindowSize = WindowSize;
    type ReportLatency = ReportLatency;
}

parameter_types! {
    pub const BitcoinTickerUri: &'static str = "http://api.coingecko.com/api/v3/coins/bitcoin";
    pub const EthereumTickerUri: &'static str = "http://api.coingecko.com/api/v3/coins/ethereum";
    pub const BitcoinApiUri: &'static str = "http://api.blockcypher.com/v1/btc/test3/txs";
    pub const EthereumApiUri: &'static str = "http://api.blockcypher.com/v1/eth/test/txs";
    pub const MedianFilterExpire: Moment = 300; // 10 blocks is one minute, 300 - half hour
}

impl frame_system::offchain::CreateTransaction<Runtime, UncheckedExtrinsic> for Runtime {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;

    fn create_transaction<
        TSigner: frame_system::offchain::Signer<Self::Public, Self::Signature>,
    >(
        call: Call,
        public: Self::Public,
        account: AccountId,
        index: Index,
    ) -> Option<(Call, <UncheckedExtrinsic as Extrinsic>::SignaturePayload)> {
        let period = 1 << 8;
        let current_block = System::block_number().saturated_into::<u64>();
        let tip = 0;
        let extra: SignedExtra = (
            frame_system::CheckVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
            frame_system::CheckNonce::<Runtime>::from(index),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
            Default::default(),
        );
        let raw_payload = SignedPayload::new(call, extra).ok()?;
        let signature = TSigner::sign(public, &raw_payload)?;
        let address = Indices::unlookup(account);
        let (call, extra, _) = raw_payload.deconstruct();
        Some((call, (address, signature, extra)))
    }
}

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = plasm_primitives::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Module, Call, Storage, Config, Event<T>},
        Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
        TransactionPayment: pallet_transaction_payment::{Module, Storage},
        Indices: pallet_indices::{Module, Call, Storage, Event<T>, Config<T>},
        Balances: pallet_balances::{Module, Call, Storage, Event<T>, Config<T>},
        Contracts: pallet_contracts::{Module, Call, Storage, Event<T>, Config<T>},
        PlasmValidator: pallet_plasm_validator::{Module, Call, Storage, Event<T>, Config<T>},
        PlasmRewards: pallet_plasm_rewards::{Module, Call, Storage, Event<T>, Config},
        Session: pallet_session::{Module, Call, Storage, Event, Config<T>},
        Babe: pallet_babe::{Module, Call, Storage, Config, Inherent(Timestamp)},
        Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event},
        FinalityTracker: pallet_finality_tracker::{Module, Call, Inherent},
        Operator: pallet_contract_operator::{Module, Call, Storage, Event<T>},
        Trading: pallet_operator_trading::{Module, Call, Storage, Event<T>},
        DappsStaking: pallet_dapps_staking::{Module, Call, Storage, Event<T>, Config},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
        Sudo: pallet_sudo::{Module, Call, Storage, Event<T>, Config<T>},
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
    frame_system::CheckVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
    pallet_contracts::CheckBlockGasLimit<Runtime>,
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
    }

    impl sp_consensus_babe::BabeApi<Block> for Runtime {
        fn configuration() -> sp_consensus_babe::BabeConfiguration {
            // The choice of `c` parameter (where `1 - c` represents the
            // probability of a slot being empty), is done in accordance to the
            // slot duration and expected target block time, for safely
            // resisting network delays of maximum two seconds.
            // <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
            sp_consensus_babe::BabeConfiguration {
                slot_duration: Babe::slot_duration(),
                epoch_length: EpochDuration::get(),
                c: PRIMARY_PROBABILITY,
                genesis_authorities: Babe::authorities(),
                randomness: Babe::randomness(),
                secondary_slots: true,
            }
        }

        fn current_epoch_start() -> sp_consensus_babe::SlotNumber {
            Babe::current_epoch_start()
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
        ) -> ContractExecResult {
            let exec_result =
                Contracts::bare_call(origin, dest.into(), value, gas_limit, input_data);
            match exec_result {
                Ok(v) => ContractExecResult::Success {
                    status: v.status,
                    data: v.data,
                },
                Err(_) => ContractExecResult::Error,
            }
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
        UncheckedExtrinsic,
    > for Runtime {
        fn query_info(uxt: UncheckedExtrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
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
}
