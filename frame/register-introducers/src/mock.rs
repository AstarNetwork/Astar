//! Test mockup for register-introducers module.

#![cfg(test)]

use super::*;

use frame_support::{impl_outer_dispatch, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

pub type AccountId = u64;
pub type Moment = u64;

impl_outer_origin! {
    pub enum Origin for Test {}
}

impl_outer_dispatch! {
    pub enum Call for Test where origin: Origin {
        pallet_reguster_introducers::RegisterIntroducers,
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Trait for Test {
    type Origin = Origin;
    type BaseCallFilter = ();
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
    type PalletInfo = ();
    type AccountData = pallet_balances::AccountData<()>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BlockExecutionWeight = ();
    type ExtrinsicBaseWeight = ();
    type MaximumExtrinsicWeight = ();
    type SystemWeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: Moment = 1;
}

impl pallet_timestamp::Trait for Test {
    type Moment = Moment;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const EndTimeOfRegistering: Moment = 100;
}

impl Trait for Test {
    type Time = Timestamp;
    type EndTimeOfRegistering = EndTimeOfRegistering;
    type Moment = Moment;
    type Event = ();
}

pub type Timestamp = pallet_timestamp::Module<Test>;
pub type RegisterIntroducers = Module<Test>;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let storage = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    storage.into()
}
