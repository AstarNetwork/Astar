//! The Substrate runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit="256"]

use rstd::prelude::*;
use support::{
	construct_runtime, parameter_types, traits::{SplitTwoWays, Currency, Randomness}
};
use primitives::u32_trait::{_1, _2, _3, _4};
use plasm_primitives::{
	AccountId, AccountIndex, Balance, BlockNumber, Hash, Index,
	Moment, Signature,
};
use aura_primitives::sr25519::AuthorityId as AuraId;
use grandpa::{fg_primitives, AuthorityList as GrandpaAuthorityList};
use client::{
	block_builder::api::{self as block_builder_api, InherentData, CheckInherentsResult},
	runtime_api as client_api, impl_runtime_apis
};
use sr_primitives::{ApplyResult, impl_opaque_keys, generic, create_runtime_str, key_types};
use sr_primitives::transaction_validity::TransactionValidity;
use sr_primitives::weights::Weight;
use sr_primitives::traits::{
	self, BlakeTwo256, Block as BlockT, DigestFor, NumberFor, StaticLookup, SaturatedConversion,
};
use version::RuntimeVersion;
use elections::VoteIndex;
#[cfg(any(feature = "std", test))]
use version::NativeVersion;
use primitives::OpaqueMetadata;
use grandpa::{AuthorityId as GrandpaId, AuthorityWeight as GrandpaWeight};
use transaction_payment_rpc_runtime_api::RuntimeDispatchInfo;
use contracts_rpc_runtime_api::ContractExecResult;
use system::offchain::TransactionSubmitter;

#[cfg(any(feature = "std", test))]
pub use sr_primitives::BuildStorage;
pub use timestamp::Call as TimestampCall;
pub use balances::Call as BalancesCall;
pub use contracts::Gas;
pub use sr_primitives::{Permill, Perbill};
pub use support::{StorageValue, traits::OnUnbalanced};
pub use staking::StakerStatus;

/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;
use impls::{CurrencyToVoteHandler, Author, LinearWeightToFee, TargetedFeeAdjustment};

/// Constant values used within the runtime.
pub mod constants;
use constants::{time::*, currency::*};

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("plasm"),
	impl_name: create_runtime_str!("plasm-node"),
	authoring_version: 10,
	// Per convention: if the runtime behavior changes, increment spec_version
	// and set impl_version to equal spec_version. If only runtime
	// implementation changes and behavior does not, then leave spec_version as
	// is and increment impl_version.
	spec_version: 154,
	impl_version: 159,
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

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct SlashBox;
impl OnUnbalanced<NegativeImbalance> for SlashBox {
	fn on_unbalanced(amount: NegativeImbalance) {}
}

pub type DealWithFees = SplitTwoWays<
	Balance,
	NegativeImbalance,
	_4, SlashBox,   // 4 parts (80%) goes to the slash box.
	_1, Author,     // 1 part (20%) goes to the block author.
>;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const MaximumBlockWeight: Weight = 1_000_000_000;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
	pub const Version: RuntimeVersion = VERSION;
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

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core datastructures.
pub mod opaque {
	use super::*;

	pub use sr_primitives::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
		}
	}
}

parameter_types! {
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}

impl aura::Trait for Runtime {
	type AuthorityId = AuraId;
}

impl indices::Trait for Runtime {
	type AccountIndex = AccountIndex;
	type IsDeadAccount = Balances;
	type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
	type Event = Event;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1 * DOLLARS;
	pub const TransferFee: Balance = 1 * CENTS;
	pub const CreationFee: Balance = 1 * CENTS;
}

impl balances::Trait for Runtime {
	type Balance = Balance;
	type OnFreeBalanceZero = (Contracts);
	type OnNewAccount = Indices;
	type Event = Event;
	type DustRemoval = ();
	type TransferPayment = ();
	type ExistentialDeposit = ExistentialDeposit;
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
}

parameter_types! {
	pub const TransactionBaseFee: Balance = 1 * CENTS;
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
	// setting this to zero will disable the weight fee.
	pub const WeightFeeCoefficient: Balance = 1_000;
	// for a sane configuration, this should always be less than `AvailableBlockRatio`.
	pub const TargetBlockFullness: Perbill = Perbill::from_percent(25);
}

impl transaction_payment::Trait for Runtime {
	type Currency = Balances;
	type OnTransactionPayment = DealWithFees;
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
	type OnTimestampSet = Aura;
	type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
	pub const ContractTransferFee: Balance = 1 * CENTS;
	pub const ContractCreationFee: Balance = 1 * CENTS;
	pub const ContractTransactionBaseFee: Balance = 1 * CENTS;
	pub const ContractTransactionByteFee: Balance = 10 * MILLICENTS;
	pub const ContractFee: Balance = 1 * CENTS;
	pub const TombstoneDeposit: Balance = 1 * DOLLARS;
	pub const RentByteFee: Balance = 1 * DOLLARS;
	pub const RentDepositOffset: Balance = 1000 * DOLLARS;
	pub const SurchargeReward: Balance = 150 * DOLLARS;
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
	type Public = <Signature as traits::Verify>::Signer;
	type Signature = Signature;

	fn create_transaction<F: system::offchain::Signer<Self::Public, Self::Signature>>(
		call: Call,
		public: Self::Public,
		account: AccountId,
		index: Index,
	) -> Option<(Call, <UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload)> {
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
		let signature = F::sign(public, &raw_payload)?;
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
		Aura: aura::{Module, Config<T>, Inherent(Timestamp)},
		Timestamp: timestamp::{Module, Call, Storage, Inherent},
		TransactionPayment: transaction_payment::{Module, Storage},
		Indices: indices,
		Balances: balances,
		Grandpa: grandpa::{Module, Call, Storage, Config, Event},
		Contracts: contracts,
		Operator: operator::{Module, Call, Storage, Event<T>},
		RandomnessCollectiveFlip: randomness_collective_flip::{Module, Call, Storage},
		Sudo: sudo,
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
	impl client_api::Core<Block> for Runtime {
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

	impl client_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl block_builder_api::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyResult {
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

	impl client_api::TaggedTransactionQueue<Block> for Runtime {
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

	impl aura_primitives::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> u64 {
			Aura::slot_duration()
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities()
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

	impl substrate_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			let seed = seed.as_ref().map(|s| rstd::str::from_utf8(&s).expect("Seed is an utf8 string"));
			opaque::SessionKeys::generate(seed)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sr_primitives::app_crypto::RuntimeAppPublic;
	use system::offchain::SubmitSignedTransaction;

	fn is_submit_signed_transaction<T, Signer>(_arg: T) where
		T: SubmitSignedTransaction<
			Runtime,
			Call,
			Extrinsic=UncheckedExtrinsic,
			CreateTransaction=Runtime,
			Signer=Signer,
		>,
		Signer: RuntimeAppPublic + From<AccountId>,
		Signer::Signature: Into<Signature>,
	{}

}
