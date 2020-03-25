//! Test utilities

#![cfg(test)]

use super::*;
use primitives::{crypto::key_types, H256};
use sp_runtime::testing::{Header, UintAuthorityId};
use sp_runtime::traits::{BlakeTwo256, ConvertInto, IdentityLookup, OnFinalize, OpaqueKeys};
use sp_runtime::{traits::Hash, KeyTypeId, Perbill};
use sp_std::marker::PhantomData;
use support::{assert_ok, impl_outer_dispatch, impl_outer_origin, parameter_types};
use traits::{ComputeTotalPayout, GetEraStakingAmount, MaybeValidators};

pub type BlockNumber = u64;
pub type AccountId = u64;
pub type Balance = u64;

pub const ALICE_STASH: u64 = 1;

impl_outer_origin! {
    pub enum Origin for Test {}
}

impl_outer_dispatch! {
    pub enum Call for Test where origin: Origin {
        session::Session,
        balances::Balances,
        plasm_rewards::PlasmRewards,
    }
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let _ = balances::GenesisConfig::<Test> {
        balances: vec![
            (ALICE_STASH, 1_000_000_000_000_000_000),
        ],
    }
    .assimilate_storage(&mut storage);

    let validators = vec![1, 2];

    let _ = GenesisConfig {
        ..Default::default()
    }
    .assimilate_storage(&mut storage);

    let _ = session::GenesisConfig::<Test> {
        keys: validators
            .iter()
            .map(|x| (*x, *x, UintAuthorityId(*x)))
            .collect(),
    }
    .assimilate_storage(&mut storage);

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
    type ModuleToIndex = ();
    type AccountData = balances::AccountData<u64>;
    type MigrateAccount = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
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
    ) {
    }
    fn on_disabled(_validator_index: usize) {}
    fn on_before_session_ending() {}
}

impl session::Trait for Test {
    type ShouldEndSession = session::PeriodicSessions<Period, Offset>;
    type SessionManager = PlasmRewards;
    type SessionHandler = TestSessionHandler;
    type ValidatorId = u64;
    type ValidatorIdOf = ConvertInto;
    type Keys = UintAuthorityId;
    type Event = ();
    type DisabledValidatorsThreshold = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 10;
}

impl balances::Trait for Test {
    type Balance = Balance;
    type Event = ();
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = system::Module<Test>;
}

pub struct DummyForSecurityStaking;
impl GetEraStakingAmount<EraIndex, Balance> for DummyForSecurityStaking {
    fn get_era_staking_amount(era: EraIndex) -> Balance {
        (era * 1_000_000).into()
    }
}

pub struct DummyForDappsStaking;
impl GetEraStakingAmount<EraIndex, Balance> for DummyForDappsStaking {
    fn get_era_staking_amount(era: EraIndex) -> Balance {
        (era * 200_000).into()
    }
}

pub struct DummyMaybeValidators;
impl MaybeValidators<EraIndex, AccountId> for DummyMaybeValidators {
    fn maybe_validators(current_era: EraIndex) -> Option<Vec<AccountId>> {
        Some(vec![1, 2, 3, (current_era + 100).into()])
    }
}

parameter_types! {
    pub const SessionsPerEra: sp_staking::SessionIndex = 10;
    pub const BondingDuration: EraIndex = 3;
}

impl Trait for Test {
    type Currency = Balances;
    type Time = Timestamp;
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type GetForDappsStaking = DummyForDappsStaking;
    type GetForSecurityStaking = DummyForSecurityStaking;
    type ComputeTotalPayout = inflation::MaintainRatioComputeTotalPayout;
    type MaybeValidators = DummyMaybeValidators;
    type Event = ();
}

/// ValidatorManager module.
pub type System = system::Module<Test>;
pub type Session = session::Module<Test>;
pub type Balances = balances::Module<Test>;
pub type Timestamp = timestamp::Module<Test>;
pub type PlasmRewards = Module<Test>;

pub const PER_SESSION: u64 = 60 * 1000;

pub fn advance_session() {
    let now = System::block_number();
    // increase block numebr
    System::set_block_number(now + 1);
    // increase timestamp + 10
    let now_time = Timestamp::get();
    // on initialize
    Timestamp::set_timestamp(now_time + PER_SESSION);
    Session::rotate_session();
    assert_eq!(Session::current_index(), (now / Period::get()) as u32);

    // on finalize
    PlasmRewards::on_finalize(now);
}

pub fn advance_era() {
    let current_era = PlasmRewards::current_era().unwrap();
    assert_ok!(PlasmRewards::force_new_era(Origin::ROOT));
    assert_eq!(PlasmRewards::force_era(), Forcing::ForceNew);
    advance_session();
    assert_eq!(PlasmRewards::current_era().unwrap(), current_era + 1);
}
