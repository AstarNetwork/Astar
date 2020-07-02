//! Runtime mockup for plasm-lockdrop module.

#![cfg(test)]

use std::cell::RefCell;

use super::*;
use plasm_primitives::{AccountId, Balance, Moment};

use frame_support::{impl_outer_dispatch, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::Pair;
use sp_keyring::sr25519::Keyring as AccountKeyring;
use sp_runtime::{
    impl_opaque_keys,
    testing::{Header, TestXt},
    traits::{ConvertInto, IdentityLookup},
    MultiSigner, Perbill,
};
use sp_staking::SessionIndex;

impl_outer_origin! {
    pub enum Origin for Runtime {}
}

impl_outer_dispatch! {
    pub enum Call for Runtime where origin: Origin {
        pallet_balances::Balances,
        pallet_plasm_lockdrop::PlasmLockdrop,
    }
}

thread_local! {
    pub static VALIDATORS: RefCell<Option<Vec<AccountId>>> = RefCell::new(Some(vec![
        AccountKeyring::Alice.into(),
        AccountKeyring::Bob.into(),
        AccountKeyring::Charlie.into(),
    ]));
}

pub struct TestSessionManager;
impl pallet_session::SessionManager<AccountId> for TestSessionManager {
    fn new_session(_new_index: SessionIndex) -> Option<Vec<AccountId>> {
        VALIDATORS.with(|l| l.borrow_mut().take())
    }
    fn end_session(_: SessionIndex) {}
    fn start_session(_: SessionIndex) {}
}

impl pallet_session::historical::SessionManager<AccountId, AccountId> for TestSessionManager {
    fn new_session(_new_index: SessionIndex) -> Option<Vec<(AccountId, AccountId)>> {
        VALIDATORS.with(|l| {
            l.borrow_mut()
                .take()
                .map(|validators| validators.iter().map(|v| (v.clone(), v.clone())).collect())
        })
    }
    fn end_session(_: SessionIndex) {}
    fn start_session(_: SessionIndex) {}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Trait for Runtime {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
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
    type ModuleToIndex = ();
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BlockExecutionWeight = ();
    type ExtrinsicBaseWeight = ();
    type MaximumExtrinsicWeight = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Trait for Runtime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
    pub const Period: u64 = 1;
    pub const Offset: u64 = 0;
}

parameter_types! {
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(33);
}

impl_opaque_keys! {
    pub struct SessionKeys {
        pub lockdrop: PlasmLockdrop,
    }
}

impl pallet_session::Trait for Runtime {
    type SessionManager =
        pallet_session::historical::NoteHistoricalRoot<Runtime, TestSessionManager>;
    type SessionHandler = (PlasmLockdrop,);
    type ValidatorId = AccountId;
    type ValidatorIdOf = ConvertInto;
    type Keys = SessionKeys;
    type Event = ();
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
}

impl pallet_session::historical::Trait for Runtime {
    type FullIdentification = AccountId;
    type FullIdentificationOf = ConvertInto;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 10;
}

impl pallet_balances::Trait for Runtime {
    type Balance = Balance;
    type Event = ();
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Module<Runtime>;
}

parameter_types! {
    pub const MedianFilterExpire: Moment = 2;
}

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Runtime
where
    Call: From<LocalCall>,
{
    type OverarchingCall = Call;
    type Extrinsic = Extrinsic;
}

impl Trait for Runtime {
    type Currency = Balances;
    type MedianFilterExpire = MedianFilterExpire;
    type MedianFilterWidth = generic_array::typenum::U3;
    type AuthorityId = sr25519::AuthorityId;
    type Account = MultiSigner;
    type Time = Timestamp;
    type Moment = Moment;
    type DollarRate = Balance;
    type BalanceConvert = Balance;
    type Event = ();
    type UnsignedPriority = ();
}

pub type System = frame_system::Module<Runtime>;
pub type Session = pallet_session::Module<Runtime>;
pub type Balances = pallet_balances::Module<Runtime>;
pub type Timestamp = pallet_timestamp::Module<Runtime>;
pub type PlasmLockdrop = Module<Runtime>;

fn session_keys(account: &AccountId) -> SessionKeys {
    SessionKeys {
        lockdrop: sr25519::AuthorityPair::from_string(&format!("//{}", account), None)
            .unwrap()
            .public(),
    }
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    let _ = pallet_session::GenesisConfig::<Runtime> {
        keys: VALIDATORS
            .with(|l| {
                l.borrow_mut().take().map(|x| {
                    x.iter()
                        .map(|v| (v.clone(), v.clone(), session_keys(v)))
                        .collect()
                })
            })
            .unwrap(),
    }
    .assimilate_storage(&mut storage);

    let _ = GenesisConfig::<Runtime> {
        // Alpha2: 0.44698108660714747
        alpha: Perbill::from_parts(446_981_087),
        // Price in cents: BTC $9000, ETH $200
        dollar_rate: (9_000, 200),
        vote_threshold: 3,
        positive_votes: 2,
        lockdrop_end: 0,
    }
    .assimilate_storage(&mut storage);

    storage.into()
}

pub fn advance_session() {
    let next = System::block_number() + 1;
    System::set_block_number(next);
    Session::rotate_session();
    assert_eq!(Session::current_index(), (next / Period::get()) as u32);
}
