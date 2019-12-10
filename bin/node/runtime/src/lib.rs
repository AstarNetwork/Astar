//! The Substrate runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit="256"]

use rstd::prelude::*;
use support::{construct_runtime, parameter_types, weights::Weight, traits::Randomness,};
use plasm_primitives::{AccountId, AccountIndex, Balance, BlockNumber, Hash, Index, Moment, Signature,};
use txpool_api::runtime_api as txpool_runtime_api;
use sp_api::impl_runtime_apis;
use sp_runtime::{Perbill, ApplyExtrinsicResult, impl_opaque_keys, generic, create_runtime_str};
use sp_runtime::transaction_validity::TransactionValidity;
use sp_runtime::traits::{
    BlakeTwo256, Block as BlockT, OpaqueKeys, Verify, Extrinsic,
    NumberFor, SaturatedConversion, StaticLookup, ConvertInto,
};
use version::RuntimeVersion;
#[cfg(any(feature = "std", test))]
use version::NativeVersion;
use primitives::OpaqueMetadata;
use grandpa::fg_primitives;
use grandpa::AuthorityList as GrandpaAuthorityList;
use transaction_payment_rpc_runtime_api::RuntimeDispatchInfo;
use contracts_rpc_runtime_api::ContractExecResult;
use inherents::{InherentData, CheckInherentsResult};

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use timestamp::Call as TimestampCall;
pub use balances::Call as BalancesCall;
pub use contracts::Gas;

/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;
use impls::{LinearWeightToFee, TargetedFeeAdjustment};

/// Constant values used within the runtime.
pub mod constants;
use constants::{time::*, currency::*};

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("plasm"),
    impl_name: create_runtime_str!("staketechnologies-plasm"),
    authoring_version: 2,
    // Per convention: if the runtime behavior changes, increment spec_version
    // and set impl_version to equal spec_version. If only runtime
    // implementation changes and behavior does not, then leave spec_version as
    // is and increment impl_version.
    spec_version: 22,
    impl_version: 22,
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

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const MaximumBlockWeight: Weight = 1_000_000_000;
    pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
    pub const Version: RuntimeVersion = VERSION;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

impl system::Trait for Runtime {
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
}

parameter_types! {
    pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
    pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}

impl babe::Trait for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;
    type EpochChangeTrigger = babe::ExternalTrigger;
}

impl indices::Trait for Runtime {
    type AccountIndex = AccountIndex;
    type IsDeadAccount = Balances;
    type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
    type Event = Event;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1 * MILLIPLM;
    pub const TransferFee: Balance = 1 * MILLIPLM;
    pub const CreationFee: Balance = 1 * MILLIPLM;
}

impl balances::Trait for Runtime {
    type Balance = Balance;
    type OnFreeBalanceZero = Contracts;
    type OnNewAccount = Indices;
    type Event = Event;
    type DustRemoval = ();
    type TransferPayment = ();
    type ExistentialDeposit = ExistentialDeposit;
    type TransferFee = TransferFee;
    type CreationFee = CreationFee;
}

parameter_types! {
    pub const TransactionBaseFee: Balance = 1 * MILLIPLM;
    pub const TransactionByteFee: Balance = 10 * MILLIPLM;
    // setting this to zero will disable the weight fee.
    pub const WeightFeeCoefficient: Balance = 1_000;
    // for a sane configuration, this should always be less than `AvailableBlockRatio`.
    pub const TargetBlockFullness: Perbill = Perbill::from_percent(25);
}

impl transaction_payment::Trait for Runtime {
    type Currency = Balances;
    type OnTransactionPayment = ();
    type TransactionBaseFee = TransactionBaseFee;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = LinearWeightToFee<WeightFeeCoefficient>;
    type FeeMultiplierUpdate = TargetedFeeAdjustment<TargetBlockFullness>;
}

parameter_types! {
    pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl timestamp::Trait for Runtime {
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

impl session::Trait for Runtime {
	type OnSessionEnding = SessionManager;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type ShouldEndSession = Babe;
	type Event = Event;
	type Keys = SessionKeys;
	type ValidatorId = <Self as system::Trait>::AccountId;
	type ValidatorIdOf = ConvertInto;
	type SelectInitialValidators = SessionManager;
	type DisabledValidatorsThreshold = ();
}

impl session_manager::Trait for Runtime {
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

impl contracts::Trait for Runtime {
    type Currency = Balances;
    type Time = Timestamp;
    type Randomness = RandomnessCollectiveFlip;
    type Call = Call;
    type Event = Event;
    type DetermineContractAddress = contracts::SimpleAddressDeterminator<Runtime>;
    type ComputeDispatchFee = contracts::DefaultDispatchFeeComputor<Runtime>;
    type TrieIdGenerator = contracts::TrieIdFromParentCounter<Runtime>;
    type GasPayment = ();
    type RentPayment = ();
    type SignedClaimHandicap = contracts::DefaultSignedClaimHandicap;
    type TombstoneDeposit = TombstoneDeposit;
    type StorageSizeOffset = contracts::DefaultStorageSizeOffset;
    type RentByteFee = RentByteFee;
    type RentDepositOffset = RentDepositOffset;
    type SurchargeReward = SurchargeReward;
    type TransferFee = ContractTransferFee;
    type CreationFee = ContractCreationFee;
    type TransactionBaseFee = ContractTransactionBaseFee;
    type TransactionByteFee = ContractTransactionByteFee;
    type ContractFee = ContractFee;
    type CallBaseFee = contracts::DefaultCallBaseFee;
    type InstantiateBaseFee = contracts::DefaultInstantiateBaseFee;
    type MaxDepth = contracts::DefaultMaxDepth;
    type MaxValueSize = contracts::DefaultMaxValueSize;
    type BlockGasLimit = contracts::DefaultBlockGasLimit;
}

impl operator::Trait for Runtime {
    type Parameters = operator::parameters::DefaultParameters;
    type Event = Event;
}

impl sudo::Trait for Runtime {
    type Event = Event;
    type Proposal = Call;
}

impl grandpa::Trait for Runtime {
    type Event = Event;
}

parameter_types! {
    pub const WindowSize: BlockNumber = 101;
    pub const ReportLatency: BlockNumber = 1000;
}

impl finality_tracker::Trait for Runtime {
    type OnFinalizationStalled = Grandpa;
    type WindowSize = WindowSize;
    type ReportLatency = ReportLatency;
}

impl system::offchain::CreateTransaction<Runtime, UncheckedExtrinsic> for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;

	fn create_transaction<TSigner: system::offchain::Signer<Self::Public, Self::Signature>>(
		call: Call,
		public: Self::Public,
		account: AccountId,
		index: Index,
	) -> Option<(Call, <UncheckedExtrinsic as Extrinsic>::SignaturePayload)> {
		let period = 1 << 8;
		let current_block = System::block_number().saturated_into::<u64>();
		let tip = 0;
		let extra: SignedExtra = (
			system::CheckVersion::<Runtime>::new(),
			system::CheckGenesis::<Runtime>::new(),
			system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
			system::CheckNonce::<Runtime>::from(index),
			system::CheckWeight::<Runtime>::new(),
			transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
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
        System: system::{Module, Call, Storage, Config, Event},
        Timestamp: timestamp::{Module, Call, Storage, Inherent},
        TransactionPayment: transaction_payment::{Module, Storage},
        Indices: indices,
        Balances: balances,
        Contracts: contracts,
        SessionManager: session_manager::{Module, Call, Storage, Event<T>, Config<T>},
        Session: session::{Module, Call, Storage, Event, Config<T>},
        Babe: babe::{Module, Call, Storage, Config, Inherent(Timestamp)},
        Grandpa: grandpa::{Module, Call, Storage, Config, Event},
        FinalityTracker: finality_tracker::{Module, Call, Inherent},
        Sudo: sudo,
        Operator: operator::{Module, Call, Storage, Event<T>},
        RandomnessCollectiveFlip: randomness_collective_flip::{Module, Call, Storage},
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
    system::CheckVersion<Runtime>,
    system::CheckGenesis<Runtime>,
    system::CheckEra<Runtime>,
    system::CheckNonce<Runtime>,
    system::CheckWeight<Runtime>,
    transaction_payment::ChargeTransactionPayment<Runtime>,
    contracts::CheckBlockGasLimit<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = executive::Executive<Runtime, Block, system::ChainContext<Runtime>, Runtime, AllModules>;

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

    impl block_builder_api::BlockBuilder<Block> for Runtime {
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

    impl txpool_runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(tx: <Block as BlockT>::Extrinsic) -> TransactionValidity {
            Executive::validate_transaction(tx)
        }
    }

    impl offchain_primitives::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(number: NumberFor<Block>) {
            Executive::offchain_worker(number)
        }
    }

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> GrandpaAuthorityList {
            Grandpa::grandpa_authorities()
        }
    }

    impl babe_primitives::BabeApi<Block> for Runtime {
        fn configuration() -> babe_primitives::BabeConfiguration {
            // The choice of `c` parameter (where `1 - c` represents the
            // probability of a slot being empty), is done in accordance to the
            // slot duration and expected target block time, for safely
            // resisting network delays of maximum two seconds.
            // <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
            babe_primitives::BabeConfiguration {
                slot_duration: Babe::slot_duration(),
                epoch_length: EpochDuration::get(),
                c: PRIMARY_PROBABILITY,
                genesis_authorities: Babe::authorities(),
                randomness: Babe::randomness(),
                secondary_slots: true,
            }
        }
    }

    impl system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl contracts_rpc_runtime_api::ContractsApi<Block, AccountId, Balance> for Runtime {
        fn call(
            origin: AccountId,
            dest: AccountId,
            value: Balance,
            gas_limit: u64,
            input_data: Vec<u8>,
        ) -> ContractExecResult {
            let exec_result = Contracts::bare_call(
                origin,
                dest.into(),
                value,
                gas_limit,
                input_data,
            );
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
        ) -> contracts_rpc_runtime_api::GetStorageResult {
            Contracts::get_storage(address, key).map_err(|rpc_err| {
                use contracts::GetStorageError;
                use contracts_rpc_runtime_api::{GetStorageError as RpcGetStorageError};
                /// Map the contract error into the RPC layer error.
                match rpc_err {
                    GetStorageError::ContractDoesntExist => RpcGetStorageError::ContractDoesntExist,
                    GetStorageError::IsTombstone => RpcGetStorageError::IsTombstone,
                }
            })
        }
    }

    impl transaction_payment_rpc_runtime_api::TransactionPaymentApi<
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
    }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn block_hooks_weight_should_not_exceed_limits() {
		use support::weights::WeighBlock;
		let check_for_block = |b| {
			let block_hooks_weight =
				<AllModules as WeighBlock<BlockNumber>>::on_initialize(b) +
				<AllModules as WeighBlock<BlockNumber>>::on_finalize(b);

			assert_eq!(
				block_hooks_weight,
				0,
				"This test might fail simply because the value being compared to has increased to a \
				module declaring a new weight for a hook or call. In this case update the test and \
				happily move on.",
			);

			// Invariant. Always must be like this to have a sane chain.
			assert!(block_hooks_weight < MaximumBlockWeight::get());

			// Warning.
			if block_hooks_weight > MaximumBlockWeight::get() / 2 {
				println!(
					"block hooks weight is consuming more than a block's capacity. You probably want \
					to re-think this. This test will fail now."
				);
				assert!(false);
			}
		};

		let _ = (0..100_000).for_each(check_for_block);
	}
}
