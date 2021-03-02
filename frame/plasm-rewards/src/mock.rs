//! Runtime utilities

#![cfg(test)]

use super::*;
use crate as plasm_rewards;
use frame_support::{parameter_types, traits::OnFinalize};
use sp_core::{crypto::key_types, H256};
use sp_runtime::{
    testing::{Header, UintAuthorityId},
    traits::{BlakeTwo256, ConvertInto, IdentityLookup, OpaqueKeys},
    BuildStorage, KeyTypeId,
};
use traits::{ComputeEraWithParam, MaybeValidators};

pub type BlockNumber = u64;
pub type AccountId = u64;
pub type Balance = u64;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    let validators = vec![1, 2];
    let balances = validators
        .iter()
        .map(|x| (*x, 1_000_000_000_000_000_000))
        .collect();
    let _ = pallet_balances::GenesisConfig::<Runtime> { balances }.assimilate_storage(&mut storage);

    let _ = GenesisConfig {
        ..Default::default()
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
        PlasmRewards: plasm_rewards::{Module, Call, Storage, Config, Event<T>},
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
    type AccountData = pallet_balances::AccountData<Self::AccountId>;
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

pub struct RuntimeSessionHandler;

impl pallet_session::SessionHandler<u64> for RuntimeSessionHandler {
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
    type SessionHandler = RuntimeSessionHandler;
    type ValidatorId = u64;
    type ValidatorIdOf = ConvertInto;
    type Keys = UintAuthorityId;
    type Event = Event;
    type DisabledValidatorsThreshold = ();
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 10;
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

pub struct DummyForSecurityStaking;
impl ComputeEraWithParam<EraIndex> for DummyForSecurityStaking {
    type Param = Balance;
    fn compute(era: &EraIndex) -> Balance {
        (era * 1_000_000).into()
    }
}

pub struct DummyForDappsStaking;
impl ComputeEraWithParam<EraIndex> for DummyForDappsStaking {
    type Param = Balance;
    fn compute(era: &EraIndex) -> Balance {
        (era * 200_000).into()
    }
}

pub struct DummyMaybeValidators;
impl MaybeValidators<EraIndex, AccountId> for DummyMaybeValidators {
    fn compute(current_era: EraIndex) -> Option<Vec<AccountId>> {
        Some(vec![1, 2, 3, (current_era + 100).into()])
    }
}

parameter_types! {
    pub const SessionsPerEra: sp_staking::SessionIndex = 10;
    pub const BondingDuration: EraIndex = 3;
}

impl Config for Runtime {
    type Currency = Balances;
    type Time = Timestamp;
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type ComputeEraForDapps = DummyForDappsStaking;
    type ComputeEraForSecurity = DummyForSecurityStaking;
    type ComputeTotalPayout = inflation::MaintainRatioComputeTotalPayout<Balance>;
    type MaybeValidators = DummyMaybeValidators;
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
