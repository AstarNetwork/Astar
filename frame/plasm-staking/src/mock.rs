//! Test utilities

#![cfg(test)]

use super::*;
use crate::{Module, Trait};
use sp_runtime::{Perbill, KeyTypeId};
use sp_runtime::testing::{Header, UintAuthorityId};
use sp_runtime::traits::{IdentityLookup, BlakeTwo256, ConvertInto, OpaqueKeys};
use primitives::{H256, crypto::key_types};
use support::{impl_outer_origin, impl_outer_dispatch, parameter_types};

/// The AccountId alias in this test module.
pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Balance = u64;

impl_outer_origin! {
    pub enum Origin for Test {}
}

impl_outer_dispatch! {
    pub enum Call for Test where origin: Origin {
    	session::Session,
    	balances::Balances,
    	validator_manager::ValidatorManager,
    	plasm_staking::PlasmStaking,
    }
}

//mod plasm_staking {
//	// Re-export contents of the root. This basically
//	// needs to give a name for the current crate.
//	// This hack is required for `impl_outer_event!`.
//	pub use super::super::*;
//	use support::impl_outer_event;
//}
//
//impl_outer_event! {
//    pub enum () for Test {
//    	session,
//    	balances<T>,
//    	session_manager<T>,
//        plasm_staking<T>,
//    }
//}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();

	let validators = vec![1, 2];

	let _ = validator_manager::GenesisConfig::<Test> {
		validators: validators.clone(),
	}.assimilate_storage(&mut storage);

	let _ = session::GenesisConfig::<Test> {
		keys: validators.iter().map(|x| (*x, UintAuthorityId(*x))).collect(),
	}.assimilate_storage(&mut storage);

	let _ = balances::GenesisConfig::<Test> {
		balances: vec![
			(1, 10),
			(2, 20),
			(3, 300),
			(4, 400),
		],
		vesting: vec![],
	}.assimilate_storage(&mut storage);

	let _ = GenesisConfig {
		current_era: 0,
		force_era: Forcing::NotForcing,
		storage_version: 1,
	}.assimilate_storage(&mut storage);

	storage.into()
}


#[derive(Clone, PartialEq, Eq, Debug)]
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
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
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
    pub const Period: u64 = 1;
    pub const Offset: u64 = 0;
}

pub struct TestSessionHandler;

impl session::SessionHandler<u64> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [KeyTypeId] = &[key_types::DUMMY];
	fn on_genesis_session<T: OpaqueKeys>(_validators: &[(u64, T)]) {}
	fn on_new_session<T: OpaqueKeys>(
		_changed: bool,
		_validators: &[(u64, T)],
		_queued_validators: &[(u64, T)],
	) {}
	fn on_disabled(_validator_index: usize) {}
	fn on_before_session_ending() {}
}

impl session::Trait for Test {
	type ShouldEndSession = session::PeriodicSessions<Period, Offset>;
	type OnSessionEnding = PlasmStaking;
	type SelectInitialValidators = ValidatorManager;
	type SessionHandler = TestSessionHandler;
	type ValidatorId = u64;
	type ValidatorIdOf = ConvertInto;
	type Keys = UintAuthorityId;
	type Event = ();
	type DisabledValidatorsThreshold = ();
}

impl validator_manager::Trait for Test {
	type Event = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 10;
	pub const TransferFee: Balance = 0;
	pub const CreationFee: Balance = 0;
}

impl balances::Trait for Test {
	type Balance = Balance;
	type OnFreeBalanceZero = ();
	type OnNewAccount = ();
	type Event = ();
	type DustRemoval = ();
	type TransferPayment = ();
	type ExistentialDeposit = ExistentialDeposit;
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
}

parameter_types! {
	pub const SessionsPerEra: sp_staking::SessionIndex = 10;
}

impl Trait for Test {
	type Currency = Balances;
	type Time = Timestamp;
	type SessionsPerEra = SessionsPerEra;
	type OnEraEnding = ValidatorManager;
	type Event = ();
}

/// ValidatorManager module.
pub type System = system::Module<Test>;
pub type Balances = balances::Module<Test>;
pub type Session = session::Module<Test>;
pub type ValidatorManager = validator_manager::Module<Test>;
pub type Timestamp = timestamp::Module<Test>;
pub type PlasmStaking = Module<Test>;

pub fn advance_session() {
	// increase block numebr
	let now = System::block_number();
	System::set_block_number(now + 1);
	// increase timestamp + 10
	let now_time = Timestamp::get();
	Timestamp::set_timestamp(now_time + 10);
	Session::rotate_session();
	assert_eq!(Session::current_index(), (now / Period::get()) as u32);
}
