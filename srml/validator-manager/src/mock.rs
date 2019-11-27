//! Test utilities

#![cfg(test)]

use crate::{Module, Trait};
use sr_primitives::{Perbill, KeyTypeId};
use sr_primitives::testing::{Header, UintAuthorityId};
use sr_primitives::traits::{IdentityLookup, BlakeTwo256, ConvertInto, OpaqueKeys};
use primitives::{H256, crypto::key_types};
use support::{impl_outer_origin, impl_outer_dispatch, parameter_types};

impl_outer_origin!{
    pub enum Origin for Runtime {}
}

impl_outer_dispatch! {
    pub enum Call for Runtime where origin: Origin {
        system::System,
        session::Session,
        validatormanager::ValidatorManager,
    }
}

pub fn new_test_ext() -> sr_io::TestExternalities {
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
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
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
    ) { }
    fn on_disabled(_validator_index: usize) { }
    fn on_before_session_ending() { }
}

impl session::Trait for Runtime {
    type ShouldEndSession = session::PeriodicSessions<Period, Offset>;
    type OnSessionEnding = ValidatorManager;
    type SelectInitialValidators = ValidatorManager;
    type SessionHandler = TestSessionHandler;
    type ValidatorId = u64;
    type ValidatorIdOf = ConvertInto;
    type Keys = UintAuthorityId;
    type Event = ();
    type DisabledValidatorsThreshold = ();
}

impl Trait for Runtime {
    type Event = ();
}

/// ValidatorManager module.
pub type System = system::Module<Runtime>;
pub type Session = session::Module<Runtime>;
pub type ValidatorManager = Module<Runtime>;

pub fn advance_session() {
    let now = System::block_number();
    System::set_block_number(now + 1);
    Session::rotate_session();
    assert_eq!(Session::current_index(), (now / Period::get()) as u32);
}
