//! Test utilities

#![cfg(test)]

use super::*;
use crate::{Module, Trait};
use sp_runtime::{Perbill, KeyTypeId};
use sp_runtime::testing::{Header, UintAuthorityId};
use sp_runtime::traits::{IdentityLookup, BlakeTwo256, ConvertInto, OpaqueKeys};
use primitives::{H256, crypto::key_types};
use support::{impl_outer_origin, impl_outer_dispatch, impl_outer_event, parameter_types};

impl_outer_origin! {
    pub enum Origin for Runtime {}
}

impl_outer_dispatch! {
    pub enum Call for Runtime where origin: Origin {
    	balances::Balances,
    	plasm_session::PlasmSession,
        session_manager::SessionManager,
    }
}

impl_outer_event! {
    pub enum MetaEvent for Test {
        session<T>, sessionManager<T>, plasmSession<T>,
    }
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap();

	let validators = vec![1, 2];

	let _ = crate::GenesisConfig::<Runtime> {
		validators: validators.clone(),
	}.assimilate_storage(&mut storage);

	let _ = session::GenesisConfig::<Runtime> {
		keys: validators.iter().map(|x| (*x, UintAuthorityId(*x))).collect(),
	}.assimilate_storage(&mut storage);

	storage.into()
}


#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: u32 = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl system::Trait for Runtime {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = MetaEvent;
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

impl session::Trait for Runtime {
	type ShouldEndSession = session::PeriodicSessions<Period, Offset>;
	type OnSessionEnding = SessionManager;
	type SelectInitialValidators = SessionManager;
	type SessionHandler = TestSessionHandler;
	type ValidatorId = u64;
	type ValidatorIdOf = ConvertInto;
	type Keys = UintAuthorityId;
	type Event = ();
	type DisabledValidatorsThreshold = ();
}

impl session_manager::Trait for Runtime {
	type Event = MetaEvent;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 500;
	pub const TransferFee: Balance = 0;
	pub const CreationFee: Balance = 0;
}

impl balances::Trait for Runtime {
	type Balance = Balance;
	type OnFreeBalanceZero = ();
	type OnNewAccount = Indices;
	type Event = MetaEvent;
	type DustRemoval = ();
	type TransferPayment = ();
	type ExistentialDeposit = ExistentialDeposit;
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
}

parameter_types! {
	pub const SessionsPerEra: sp_staking::SessionIndex = 6;
	pub const BondingDuration: staking::EraIndex = 24 * 28;
	pub const SlashDeferDuration: staking::EraIndex = 24 * 7; // 1/4 the bonding duration.
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
}

impl Trait for Runtime {
	type Currency = Balances;
	type Time = Timestamp;
	type SessionsPerEra = SessionsPerEra;
	type OnSessionEnding = SessionManager;
	type Event = MetaEvent;
}


/// SessionManager module.
pub type System = system::Module<Runtime>;
pub type Session = session::Module<Runtime>;
pub type SessionManager = Module<Runtime>;
pub type PlasmSession = Module<Runtime>;

pub fn advance_session() {
	let now = System::block_number();
	System::set_block_number(now + 1);
	Session::rotate_session();
	assert_eq!(Session::current_index(), (now / Period::get()) as u32);
}
