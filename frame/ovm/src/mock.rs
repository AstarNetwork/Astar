//! Test utilities

#![cfg(test)]

use super::*;
use frame_support::{
    impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types, traits::OnFinalize,
};
use pallet_balances as balances;
use sp_core::{crypto::key_types, H256};
use sp_runtime::testing::{Header, UintAuthorityId};
use sp_runtime::traits::{BlakeTwo256, ConvertInto, IdentityLookup};
use sp_runtime::{KeyTypeId, Perbill};

pub type BlockNumber = u64;
pub type AccountId = u64;
pub type Balance = u64;

pub const ALICE_STASH: u64 = 1;

impl_outer_origin! {
    pub enum Origin for Test  where system = frame_system {}
}

impl_outer_dispatch! {
    pub enum Call for Test where origin: Origin {
        pallet_balances::Balances,
    }
}

mod ovm {
    // Re-export contents of the root. This basically
    // needs to give a name for the current crate.
    // This hack is required for `impl_outer_event!`.
    pub use super::super::*;
    use frame_support::impl_outer_event;
}

impl_outer_event! {
    pub enum MetaEvent for Test {
        system<T>,
        balances<T>,
        ovm<T>,
    }
}
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let _ = pallet_balances::GenesisConfig::<Test> {
        balances: vec![(ALICE_STASH, 1_000_000_000_000_000_000)],
    }
    .assimilate_storage(&mut storage);

    let validators = vec![1, 2];

    let _ = GenesisConfig {
        current_schedule: Default::default(),
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

impl frame_system::Trait for Test {
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
    type Event = MetaEvent;
    type DbWeight = ();
    type BlockHashCount = BlockHashCount;
    type BlockExecutionWeight = ();
    type ExtrinsicBaseWeight = ();
    type MaximumExtrinsicWeight = ();
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type ModuleToIndex = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 10;
}

impl pallet_balances::Trait for Test {
    type Balance = Balance;
    type Event = MetaEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Module<Test>;
}

parameter_types! {
    pub const DisputePeriod: BlockNumber = 7;
}

pub struct DummyPredicateAddressFor;
impl PredicateAddressFor<H256, u64> for DummyPredicateAddressFor {
    fn predicate_address_for(_code_hash: &H256, _data: &[u8], origin: &u64) -> u64 {
        *origin + 1
    }
}

parameter_types! {
    pub const MaxDepth: u32 = 32;
}

impl Trait for Test {
    type MaxDepth = MaxDepth;
    type DisputePeriod = DisputePeriod;
    type DeterminePredicateAddress = DummyPredicateAddressFor;
    type Event = MetaEvent;
}

pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Ovm = Module<Test>;

const PER_BLOCK: u64 = 1000;

pub fn advance_block() {
    let next = System::block_number() + 1;
    // increase block numebr
    System::set_block_number(next);
}

/// Generate compiled predicate binary and code hash from predicate source.
pub fn compile_predicate<T>(predicate_module: &str) -> (Vec<u8>, <T::Hashing as Hash>::Output)
where
    T: frame_system::Trait,
{
    // TODO actually predicate to compiled predicate.
    let compiled_predicate = predicate_module.as_bytes().to_vec();
    let code_hash = T::Hashing::hash_of(&compiled_predicate);
    (compiled_predicate.to_vec(), code_hash)
}
