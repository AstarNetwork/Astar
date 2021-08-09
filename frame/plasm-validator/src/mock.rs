//! Runtime utilities

#![cfg(test)]

use super::*;
use crate as plasm_validator;
use frame_support::{parameter_types, traits::OnFinalize};
use pallet_plasm_rewards::inflation::SimpleComputeTotalPayout;
use sp_core::{crypto::key_types, H256};
use sp_runtime::{
    testing::{Header, UintAuthorityId},
    traits::{BlakeTwo256, ConvertInto, IdentityLookup, OpaqueKeys},
    KeyTypeId,
};

pub type BlockNumber = u64;
pub type AccountId = u64;
pub type Balance = u64;

pub const VALIDATOR_A: u64 = 1;
pub const VALIDATOR_B: u64 = 2;
pub const VALIDATOR_C: u64 = 3;
pub const VALIDATOR_D: u64 = 4;
pub const VALIDATOR_E: u64 = 5;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    let _ = pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![
            (VALIDATOR_A, 1_000_000_000_000_000_000),
            (VALIDATOR_B, 1_000_000_000_000_000_000),
            (VALIDATOR_C, 1_000_000_000_000_000_000),
            (VALIDATOR_D, 1_000_000_000_000_000_000),
        ],
    }
    .assimilate_storage(&mut storage);

    let validators = vec![VALIDATOR_A, VALIDATOR_B, VALIDATOR_C, VALIDATOR_D];

    let _ = pallet_plasm_rewards::GenesisConfig {
        ..Default::default()
    }
    .assimilate_storage(&mut storage);

    let _ = plasm_validator::GenesisConfig::<Runtime> {
        validators: validators.clone(),
    }
    .assimilate_storage(&mut storage);

    let _ = pallet_session::GenesisConfig::<Runtime> {
        keys: validators
            .iter()
            .map(|x| (*x, *x, UintAuthorityId(*x)))
            .collect(),
    }
    .assimilate_storage(&mut storage);

    storage.into()
}

frame_support::construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Module, Storage},
        Session: pallet_session::{Module, Call, Storage, Event},
        Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
        PlasmRewards: pallet_plasm_rewards::{Module, Call, Storage, Config, Event<T>},
        PlasmValidator: plasm_validator::{Module, Call, Storage, Config<T>, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
    type Origin = Origin;
    type BaseCallFilter = ();
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Call = Call;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type SystemWeightInfo = ();
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 1;
}
impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const Period: u64 = 1;
    pub const Offset: u64 = 0;
}

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<u64> for TestSessionHandler {
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

impl pallet_session::Config for Runtime {
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = PlasmRewards;
    type SessionHandler = TestSessionHandler;
    type ValidatorId = u64;
    type ValidatorIdOf = ConvertInto;
    type Keys = UintAuthorityId;
    type Event = Event;
    type DisabledValidatorsThreshold = ();
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1_000_000_000_000;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Module<Runtime>;
    type WeightInfo = ();
    type MaxLocks = ();
}

parameter_types! {
    pub const SessionsPerEra: sp_staking::SessionIndex = 10;
    pub const BondingDuration: EraIndex = 3;
}

impl pallet_plasm_rewards::Config for Runtime {
    type Currency = Balances;
    type Time = Timestamp;
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type ComputeEraForDapps = PlasmValidator;
    type ComputeEraForSecurity = PlasmValidator;
    type ComputeTotalPayout = SimpleComputeTotalPayout;
    type MaybeValidators = PlasmValidator;
    type Event = Event;
}

impl Config for Runtime {
    type Currency = Balances;
    type Time = Timestamp;
    type RewardRemainder = (); // Reward remainder is burned.
    type Reward = (); // Reward is minted.
    type EraFinder = PlasmRewards;
    type ForSecurityEraReward = PlasmRewards;
    type ComputeEraParam = u32;
    type ComputeEra = PlasmValidator;
    type Event = Event;
}

pub const PER_SESSION: u64 = 60 * 1000;

pub fn advance_session() {
    let next = System::block_number() + 1;
    // increase block numebr
    System::set_block_number(next);
    // increase timestamp + 10
    let now_time = Timestamp::get();
    // on initialize
    Timestamp::set_timestamp(now_time + PER_SESSION);
    Session::rotate_session();
    assert_eq!(Session::current_index(), (next / Period::get()) as u32);

    // on finalize
    PlasmRewards::on_finalize(next);
}

pub fn advance_era() {
    let current_era = PlasmRewards::current_era().unwrap();
    while current_era == PlasmRewards::current_era().unwrap() {
        advance_session();
    }
}
