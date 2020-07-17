//! Runtime mockup for plasm-lockdrop module.

#![cfg(test)]

use super::*;
use plasm_primitives::{AccountId, Balance, Moment};

use frame_support::{impl_outer_dispatch, impl_outer_origin, parameter_types, weights::Weight};
use hex_literal::hex;
use sp_core::crypto::UncheckedInto;
use sp_runtime::{
    testing::{Header, TestXt},
    traits::IdentityLookup,
    MultiSigner, Perbill,
};

impl_outer_origin! {
    pub enum Origin for Runtime {}
}

impl_outer_dispatch! {
    pub enum Call for Runtime where origin: Origin {
        pallet_balances::Balances,
        pallet_plasm_lockdrop::PlasmLockdrop,
    }
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
    type DurationBonus = DustyDurationBonus;
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

pub type Balances = pallet_balances::Module<Runtime>;
pub type Timestamp = pallet_timestamp::Module<Runtime>;
pub type PlasmLockdrop = Module<Runtime>;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let alice: <Runtime as Trait>::AuthorityId =
        hex!["c83f0a4067f1b166132ed45995eee17ba7aeafeea27fe17550728ee34f998c4e"].unchecked_into();
    let bob: <Runtime as Trait>::AuthorityId =
        hex!["fa1b7e37aa3e463c81215f63f65a7c2b36ced251dd6f1511d357047672afa422"].unchecked_into();
    let charlie: <Runtime as Trait>::AuthorityId =
        hex!["88da12401449623ab60f20ed4302ab6e5db53de1e7b5271f35c858ab8b5ab37f"].unchecked_into();

    let mut storage = system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    let _ = GenesisConfig::<Runtime> {
        keys: vec![alice, bob, charlie],
        // Alpha2: 0.44698108660714747
        alpha: Perbill::from_parts(446_981_087),
        // Price in cents: BTC $9000, ETH $200
        dollar_rate: (9_000, 200),
        vote_threshold: 3,
        positive_votes: 2,
        lockdrop_bounds: (0, 1_000_000),
    }
    .assimilate_storage(&mut storage);

    storage.into()
}
