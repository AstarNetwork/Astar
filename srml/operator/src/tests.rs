// TODO: #1417 Add more integration tests
// also remove the #![allow(unused)] below.

#![allow(unused)]

use super::*;
use contract::{BalanceOf, ComputeDispatchFee, ContractAddressFor, ContractInfo, ContractInfoOf, RawAliveContractInfo, TrieId, TrieIdFromParentCounter, Schedule, TrieIdGenerator};
use hex_literal::*;
use codec::{Decode, Encode, KeyedVec};
use sr_primitives::{
	Perbill, BuildStorage,
	traits::{BlakeTwo256, Hash, IdentityLookup, SignedExtension},
	weights::{DispatchInfo, DispatchClass},
	testing::{Digest, DigestItem, Header, UintAuthorityId, H256},
};
use support::{
	assert_ok, assert_err, impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types,
	storage::child, StorageMap, StorageValue, traits::{Currency, Get},
};
use std::{cell::RefCell, sync::atomic::{AtomicUsize, Ordering}};
use primitives::storage::well_known_keys;
use system::{self, EventRecord, Phase};
use serde::{Serialize, Deserialize, de::DeserializeOwned};

mod operator {
	// Re-export contents of the root. This basically
	// needs to give a name for the current crate.
	// This hack is required for `impl_outer_event!`.
	pub use super::super::*;
	use support::impl_outer_event;
}
impl_outer_event! {
	pub enum MetaEvent for Test {
		balances<T>, contract<T>, operator<T>,
	}
}
impl_outer_origin! {
	pub enum Origin for Test { }
}
impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		balances::Balances,
		contract::Contract,
	}
}

thread_local! {
	static EXISTENTIAL_DEPOSIT: RefCell<u64> = RefCell::new(0);
	static TRANSFER_FEE: RefCell<u64> = RefCell::new(0);
	static INSTANTIATION_FEE: RefCell<u64> = RefCell::new(0);
	static BLOCK_GAS_LIMIT: RefCell<u64> = RefCell::new(0);
}

pub struct ExistentialDeposit;
impl Get<u64> for ExistentialDeposit {
	fn get() -> u64 { EXISTENTIAL_DEPOSIT.with(|v| *v.borrow()) }
}

pub struct TransferFee;
impl Get<u64> for TransferFee {
	fn get() -> u64 { TRANSFER_FEE.with(|v| *v.borrow()) }
}

pub struct CreationFee;
impl Get<u64> for CreationFee {
	fn get() -> u64 { INSTANTIATION_FEE.with(|v| *v.borrow()) }
}

pub struct BlockGasLimit;
impl Get<u64> for BlockGasLimit {
	fn get() -> u64 { BLOCK_GAS_LIMIT.with(|v| *v.borrow()) }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl system::Trait for Test {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Call = ();
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = MetaEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type AvailableBlockRatio = AvailableBlockRatio;
	type MaximumBlockLength = MaximumBlockLength;
	type Version = ();
}
impl balances::Trait for Test {
	type Balance = u64;
	type OnFreeBalanceZero = Contract;
	type OnNewAccount = ();
	type Event = MetaEvent;
	type DustRemoval = ();
	type TransferPayment = ();
	type ExistentialDeposit = ExistentialDeposit;
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
}
parameter_types! {
	pub const MinimumPeriod: u64 = 1;
}
impl timestamp::Trait for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
}
parameter_types! {
	pub const SignedClaimHandicap: u64 = 2;
	pub const TombstoneDeposit: u64 = 16;
	pub const StorageSizeOffset: u32 = 8;
	pub const RentByteFee: u64 = 4;
	pub const RentDepositOffset: u64 = 10_000;
	pub const SurchargeReward: u64 = 150;
	pub const TransactionBaseFee: u64 = 2;
	pub const TransactionByteFee: u64 = 6;
	pub const ContractFee: u64 = 21;
	pub const CallBaseFee: u64 = 135;
	pub const InstantiateBaseFee: u64 = 175;
	pub const MaxDepth: u32 = 100;
	pub const MaxValueSize: u32 = 16_384;
}
impl contract::Trait for Test {
	type Currency = Balances;
	type Time = Timestamp;
	type Randomness = Randomness;
	type Call = Call;
	type DetermineContractAddress = DummyContractAddressFor;
	type Event = MetaEvent;
	type ComputeDispatchFee = DummyComputeDispatchFee;
	type TrieIdGenerator = DummyTrieIdGenerator;
	type GasPayment = ();
	type RentPayment = ();
	type SignedClaimHandicap = SignedClaimHandicap;
	type TombstoneDeposit = TombstoneDeposit;
	type StorageSizeOffset = StorageSizeOffset;
	type RentByteFee = RentByteFee;
	type RentDepositOffset = RentDepositOffset;
	type SurchargeReward = SurchargeReward;
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
	type TransactionBaseFee = TransactionBaseFee;
	type TransactionByteFee = TransactionByteFee;
	type ContractFee = ContractFee;
	type CallBaseFee = CallBaseFee;
	type InstantiateBaseFee = InstantiateBaseFee;
	type MaxDepth = MaxDepth;
	type MaxValueSize = MaxValueSize;
	type BlockGasLimit = BlockGasLimit;
}

#[derive(Clone, Eq, PartialEq, Default, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize, derive_more::Display))]
pub struct TestParameters {
	pub a: u128,
}

impl Trait for Test {
	type Parameters = TestParameters;
	type Event = MetaEvent;
}

type Balances = balances::Module<Test>;
type Timestamp = timestamp::Module<Test>;
type Contract = contract::Module<Test>;
type System = system::Module<Test>;
type Randomness = randomness_collective_flip::Module<Test>;
type Operator = Module<Test>;

pub struct DummyContractAddressFor;
impl ContractAddressFor<H256, u64> for DummyContractAddressFor {
	fn contract_address_for(_code_hash: &H256, _data: &[u8], origin: &u64) -> u64 {
		*origin + 1
	}
}

pub struct DummyTrieIdGenerator;
impl TrieIdGenerator<u64> for DummyTrieIdGenerator {
	fn trie_id(account_id: &u64) -> TrieId {
		use primitives::storage::well_known_keys;

		let new_seed = contract::AccountCounter::mutate(|v| {
			*v = v.wrapping_add(1);
			*v
		});

		// TODO: see https://github.com/paritytech/substrate/issues/2325
		let mut res = vec![];
		res.extend_from_slice(well_known_keys::CHILD_STORAGE_KEY_PREFIX);
		res.extend_from_slice(b"default:");
		res.extend_from_slice(&new_seed.to_le_bytes());
		res.extend_from_slice(&account_id.to_le_bytes());
		res
	}
}

pub struct DummyComputeDispatchFee;
impl ComputeDispatchFee<Call, u64> for DummyComputeDispatchFee {
	fn compute_dispatch_fee(call: &Call) -> u64 {
		69
	}
}

const ALICE: u64 = 1;
const BOB: u64 = 2;
const CHARLIE: u64 = 3;
const DJANGO: u64 = 4;

pub struct ExtBuilder {
	existential_deposit: u64,
	gas_price: u64,
	block_gas_limit: u64,
	transfer_fee: u64,
	instantiation_fee: u64,
}
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			existential_deposit: 0,
			gas_price: 2,
			block_gas_limit: 100_000_000,
			transfer_fee: 0,
			instantiation_fee: 0,
		}
	}
}
impl ExtBuilder {
	pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
		self.existential_deposit = existential_deposit;
		self
	}
	pub fn gas_price(mut self, gas_price: u64) -> Self {
		self.gas_price = gas_price;
		self
	}
	pub fn block_gas_limit(mut self, block_gas_limit: u64) -> Self {
		self.block_gas_limit = block_gas_limit;
		self
	}
	pub fn transfer_fee(mut self, transfer_fee: u64) -> Self {
		self.transfer_fee = transfer_fee;
		self
	}
	pub fn instantiation_fee(mut self, instantiation_fee: u64) -> Self {
		self.instantiation_fee = instantiation_fee;
		self
	}
	pub fn set_associated_consts(&self) {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
		TRANSFER_FEE.with(|v| *v.borrow_mut() = self.transfer_fee);
		INSTANTIATION_FEE.with(|v| *v.borrow_mut() = self.instantiation_fee);
		BLOCK_GAS_LIMIT.with(|v| *v.borrow_mut() = self.block_gas_limit);
	}
	pub fn build(self) -> sr_io::TestExternalities {
		self.set_associated_consts();
		let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap();
		balances::GenesisConfig::<Test> {
			balances: vec![],
			vesting: vec![],
		}.assimilate_storage(&mut t).unwrap();
		contract::GenesisConfig::<Test> {
			current_schedule: Schedule {
				enable_println: true,
				..Default::default()
			},
			gas_price: self.gas_price,
		}.assimilate_storage(&mut t).unwrap();
		sr_io::TestExternalities::new(t)
	}
}

/// Generate Wasm binary and code hash from wabt source.
fn compile_module<T>(wabt_module: &str)
					 -> std::result::Result<(Vec<u8>, <T::Hashing as Hash>::Output), wabt::Error>
	where T: system::Trait
{
	let wasm = wabt::wat2wasm(wabt_module)?;
	let code_hash = T::Hashing::hash(&wasm);
	Ok((wasm, code_hash))
}

const CODE_RETURN_FROM_START_FN: &str = r#"
(module
	(import "env" "ext_return" (func $ext_return (param i32 i32)))
	(import "env" "ext_deposit_event" (func $ext_deposit_event (param i32 i32 i32 i32)))
	(import "env" "memory" (memory 1 1))

	(start $start)
	(func $start
		(call $ext_deposit_event
			(i32.const 0) ;; The topics buffer
			(i32.const 0) ;; The topics buffer's length
			(i32.const 8) ;; The data buffer
			(i32.const 4) ;; The data buffer's length
		)
		(call $ext_return
			(i32.const 8)
			(i32.const 4)
		)
		(unreachable)
	)

	(func (export "call")
		(unreachable)
	)
	(func (export "deploy"))

	(data (i32.const 8) "\01\02\03\04")
)
"#;

#[test]
fn instantiate_and_call_and_deposit_event() {
	let (wasm, code_hash) = compile_module::<Test>(CODE_RETURN_FROM_START_FN).unwrap();

	ExtBuilder::default().existential_deposit(100).build().execute_with(|| {
		Balances::deposit_creating(&ALICE, 1_000_000);

		assert_ok!(Contract::put_code(Origin::signed(ALICE), 100_000, wasm));

		// Check at the end to get hash on error easily
		let creation = Contract::instantiate(
			Origin::signed(ALICE),
			100,
			100_000,
			code_hash.into(),
			vec![],
		);

		assert_eq!(System::events(), vec![
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: MetaEvent::balances(balances::RawEvent::NewAccount(1, 1_000_000)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: MetaEvent::contract(contract::RawEvent::CodeStored(code_hash.into())),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: MetaEvent::balances(
					balances::RawEvent::NewAccount(BOB, 100)
				),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: MetaEvent::contract(contract::RawEvent::Transfer(ALICE, BOB, 100)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: MetaEvent::contract(contract::RawEvent::Contract(BOB, vec![1, 2, 3, 4])),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: MetaEvent::contract(contract::RawEvent::Instantiated(ALICE, BOB)),
				topics: vec![],
			}
		]);

		assert_ok!(creation);
		assert!(ContractInfoOf::<Test>::exists(BOB));
	});
}


#[test]
fn instantiate_and_relate_operator() {
	let (wasm, code_hash) = compile_module::<Test>(CODE_RETURN_FROM_START_FN).unwrap();

	ExtBuilder::default().existential_deposit(100).build().execute_with(|| {
		// prepare
		Balances::deposit_creating(&ALICE, 1_000_000);
		assert_ok!(Contract::put_code(Origin::signed(ALICE), 100_000, wasm));

		let test_params = TestParameters{a: 5_000_000};

		// instantiate
		// Check at the end to get hash on error easily
		let creation = Operator::instantiate(
			Origin::signed(ALICE),
			100,
			100_000,
			code_hash.into(),
			vec![],
			test_params.clone(),
		);
		// checks eventRecord
		assert_eq!(System::events(), vec![
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: MetaEvent::balances(balances::RawEvent::NewAccount(1, 1_000_000)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: MetaEvent::contract(contract::RawEvent::CodeStored(code_hash.into())),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: MetaEvent::balances(
					balances::RawEvent::NewAccount(BOB, 100)
				),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: MetaEvent::contract(contract::RawEvent::Transfer(ALICE, BOB, 100)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: MetaEvent::contract(contract::RawEvent::Contract(BOB, vec![1, 2, 3, 4])),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: MetaEvent::contract(contract::RawEvent::Instantiated(ALICE, BOB)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: MetaEvent::operator(RawEvent::SetOperator(ALICE, BOB)),
				topics: vec![],
			}
		]);

		// checks deployed contract
		assert!(ContractInfoOf::<Test>::exists(BOB));

		// checks mapping operator and contract
		// ALICE operate a only BOB contract.
		assert!(OperatorHasContracts::<Test>::exists(ALICE));
		let tree = OperatorHasContracts::<Test>::get(&ALICE);
		assert_eq!(tree.len(), 1);
		assert!(tree.contains(&BOB));

		// BOB contract is operated a ALICE.
		assert!(ContractHasOperator::<Test>::exists(BOB));
		assert_eq!(ContractHasOperator::<Test>::get(&BOB), Some(ALICE));

		// BOB's contract Parameters is same test_params.
		assert!(ContractParameters::<Test>::exists(BOB));
		assert_eq!(ContractParameters::<Test>::get(&BOB), Some(test_params));
	});
}
