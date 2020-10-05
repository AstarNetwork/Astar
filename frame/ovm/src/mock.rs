//! Test utilities

#![cfg(test)]

use super::*;
use crate::predicate::CallContext;
use frame_support::{impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types};
pub use hex_literal::hex;
use pallet_balances as balances;
use sp_core::{crypto::AccountId32, Pair, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentifyAccount, IdentityLookup},
    Perbill,
};
pub type BlockNumber = u64;
pub type AccountId = AccountId32;
pub type Balance = u64;

lazy_static::lazy_static! {
    pub static ref ALICE_STASH: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000005553"
    ]);
}

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
        balances: vec![((*ALICE_STASH).clone(), 1_000_000_000_000_000_000)],
    }
    .assimilate_storage(&mut storage);

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
    type PalletInfo = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
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
    type WeightInfo = ();
    type MaxLocks = ();
}

parameter_types! {
    pub const MaxDepth: u32 = 32;
    pub const DisputePeriod: BlockNumber = 7;
}

lazy_static::lazy_static! {
    pub static ref NOT_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000003"
    ]);
    pub static ref AND_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000004"
    ]);
    pub static ref OR_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000005"
    ]);
    pub static ref FOR_ALL_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000006"
    ]);
    pub static ref THERE_EXISTS_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000007"
    ]);
    pub static ref EQUAL_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000008"
    ]);
    pub static ref IS_CONTAINED_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000009"
    ]);
    pub static ref IS_LESS_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000010"
    ]);
    pub static ref IS_STORED_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000011"
    ]);
    pub static ref IS_VALID_SIGNATURE_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000012"
    ]);
    pub static ref VERIFY_INCLUSION_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000013"
    ]);
    pub static ref SECP_256_K1: H256 = H256::from(&hex![
        "d4fa99b1e08c4e5e6deb461846aa629344d95ff03ed04754c2053d54c756f439"
    ]);
}

impl Get<AtomicPredicateIdConfig<AccountId, H256>> for AtomicPredicateIdConfig<AccountId, H256> {
    fn get() -> AtomicPredicateIdConfig<AccountId, H256> {
        AtomicPredicateIdConfig {
            not_address: (*NOT_ADDRESS).clone(),
            and_address: (*AND_ADDRESS).clone(),
            or_address: (*OR_ADDRESS).clone(),
            for_all_address: (*FOR_ALL_ADDRESS).clone(),
            there_exists_address: (*THERE_EXISTS_ADDRESS).clone(),
            equal_address: (*EQUAL_ADDRESS).clone(),
            is_contained_address: (*IS_CONTAINED_ADDRESS).clone(),
            is_less_address: (*IS_LESS_ADDRESS).clone(),
            is_stored_address: (*IS_STORED_ADDRESS).clone(),
            is_valid_signature_address: (*IS_VALID_SIGNATURE_ADDRESS).clone(),
            verify_inclusion_address: (*VERIFY_INCLUSION_ADDRESS).clone(),
            secp256k1: (*SECP_256_K1).clone(),
        }
    }
}

impl Trait for Test {
    type MaxDepth = MaxDepth;
    type DisputePeriod = DisputePeriod;
    type DeterminePredicateAddress = SimpleAddressDeterminer<Test>;
    type HashingL2 = BlakeTwo256;
    type ExternalCall = CallContext<Test>;
    type AtomicPredicateIdConfig = AtomicPredicateIdConfig<AccountId, H256>;
    type Event = MetaEvent;
}

pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Ovm = Module<Test>;

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

pub fn to_account_from_seed(seed: &[u8; 32]) -> AccountId {
    to_account(sp_core::ecdsa::Pair::from_seed(&seed).public().as_ref())
}

pub fn to_account(full_public: &[u8]) -> AccountId {
    let public = sp_core::ecdsa::Public::from_full(full_public).unwrap();
    sp_runtime::MultiSigner::from(public).into_account()
}
