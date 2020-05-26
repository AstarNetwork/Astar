//! Test utilities

#![cfg(test)]

use super::*;
pub use frame_support::{
    impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types, traits::OnFinalize,
};
pub use pallet_balances as balances;
pub use pallet_ovm as ovm;
pub use sp_core::{crypto::key_types, H256};
pub use sp_runtime::testing::{Header, UintAuthorityId};
pub use sp_runtime::traits::{BlakeTwo256, ConvertInto, IdentityLookup};
pub use sp_runtime::{KeyTypeId, Perbill};

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

mod plasma {
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
        plasma<T>,
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

    let _ = ovm::GenesisConfig {
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
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Call = Call;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = MetaEvent;
    type BlockHashCount = BlockHashCount;
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
impl ovm::PredicateAddressFor<H256, u64> for DummyPredicateAddressFor {
    fn predicate_address_for(_code_hash: &H256, _data: &[u8], origin: &u64) -> u64 {
        *origin + 1
    }
}

parameter_types! {
    pub const MaxDepth: u32 = 32;
}

impl pallet_ovm::Trait for Test {
    type MaxDepth = MaxDepth;
    type DisputePeriod = DisputePeriod;
    type DeterminePredicateAddress = DummyPredicateAddressFor;
    type Event = MetaEvent;
}

pub struct DummyPlappsAddressFor;
impl PlappsAddressFor<H256, u64> for DummyPlappsAddressFor {
    fn plapps_address_for(_hash: &H256, origin: &u64) -> u64 {
        *origin + 10000
    }
}

parameter_types! {
    pub const MaximumTokenAddress: AccountId = AccountId::max_value();
}

impl Trait for Test {
    type Currency = Balances;
    type DeterminePlappsAddress = DummyPlappsAddressFor;
    type MaximumTokenAddress = MaximumTokenAddress;
    // TODO: should be Keccak;
    type PlasmaHashing = BlakeTwo256;
    type Event = MetaEvent;
}

pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
// pub type Ovm = pallet_ovm::Module<Test>;
pub type Plasma = Module<Test>;

pub fn advance_block() {
    System::finalize();
    let next = System::block_number() + 1;
    // increase block numebr
    System::set_block_number(next);
    System::initialize(
        &next,
        &[0u8; 32].into(),
        &[0u8; 32].into(),
        &Default::default(),
        system::InitKind::Full,
    );
    System::note_finished_initialize();
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
