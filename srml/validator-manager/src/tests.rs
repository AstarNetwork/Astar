#![allow(unused)]

use super::*;
use codec::{Decode, Encode};

use sr_primitives::{
	testing::{Digest, DigestItem, Header, UintAuthorityId, H256},
	traits::{BlakeTwo256, Hash, IdentityLookup, SignedExtension},
	weights::{DispatchClass, DispatchInfo},
	BuildStorage, Perbill,
};

use support::{
	assert_err, assert_ok, impl_outer_dispatch, impl_outer_event, impl_outer_origin,
	parameter_types,
	storage::child,
	traits::{Currency, Get},
	StorageMap, StorageValue,
};

use system::{self, EventRecord, Phase};

mod validator_manager {
	// Re-export contents of the root. This basically
	// needs to give a name for the current crate.
	// This hack is required for `impl_outer_event!`.
	pub use super::super::*;
	use support::impl_outer_event;
}

impl_outer_event! {
    pub enum MetaEvent for Test {
        session, validator_manager<T>,
    }
}

impl_outer_origin! {
    pub enum Origin for Test { }
}

impl_outer_dispatch! {
    pub enum Call for Test where origin: Origin { }
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

parameter_types! {
    pub const MinimumPeriod: u64 = 1;
}

impl timestamp::Trait for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
}

impl_opaque_keys! {
    pub struct SessionKeys {
    }
}

impl session::Trait for Test {
	type OnSessionEnding = ValidatorManager;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type ShouldEndSession = ValidatorManager;
	type Event = MetaEvent;
	type Keys = ();
	type ValidatorId = <Self as system::Trait>::AccountId;
	type ValidatorIdOf = ();
	type SelectInitialValidators = ValidatorManager;
	type DisabledValidatorsThreshold = ();
}

impl Trait for Test {
	type Event = MetaEvent;
}

type Timestamp = timestamp::Module<Test>;
type System = system::Module<Test>;
type Session = session::Module<Test>;
type ValidatorManager = Module<Test>;

const ALICE: u64 = 1;
const BOB: u64 = 2;
const CHARLIE: u64 = 3;
const DJANGO: u64 = 4;

pub struct ExtBuilder { }

impl Default for ExtBuilder {
	fn default() -> Self {
        ExtBuilder { }
	}
}

impl ExtBuilder {
	pub fn build(self) -> sr_io::TestExternalities {
		let mut t = system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();
        sr_io::TestExternalities::new(t)
	}
}

#[test]
fn test_set_validators() {
	ExtBuilder::default()
		.build()
		.execute_with(|| {
			ValidatorManager::set_validators([ALICE]);

			assert_eq!(
				System::events(),
				vec![
					EventRecord {
						phase: Phase::ApplyExtrinsic(0),
						event: MetaEvent::validator_manager(validator_manager::RawEvent::NewValidators([ALICE])),
						topics: vec![],
					},
				]
			);
		});
}
