//! Runtime utilities

#![cfg(test)]

use super::*;
use primitives::{crypto::key_types, H256};
use sp_runtime::testing::{Header, UintAuthorityId};
use sp_runtime::traits::{BlakeTwo256, ConvertInto, IdentityLookup, OpaqueKeys};
use sp_runtime::{traits::Hash, KeyTypeId, Perbill};
use support::{assert_ok, impl_outer_dispatch, impl_outer_origin, parameter_types};

impl_outer_origin! {
    pub enum Origin for Runtime {}
}

impl_outer_dispatch! {
    pub enum Call for Runtime where origin: Origin {
        pallet_balances::Balances,
        pallet_plasm_lockdrop::PlasmLockdrop,
    }
}

pub fn new_test_ext() -> sp_io::RuntimeExternalities {
    let mut storage = system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

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
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnReapAccount = Balances;
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
    pub const ExistentialDeposit: Balance = 10;
}

impl pallet_balances::Trait for Runtime {
    type Balance = Balance;
    type Event = ();
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = system::Module<Runtime>;
}

parameter_types! {
    pub const VoteThreshold: AuthorityVote = 3;
    pub const PositiveVotes: AuthorityVote = 2;
    pub const BitcoinApiUri: &'static str = "";
    pub const EthereumApiUri: &'static str = "";
    pub const EthereumContractAddress: &'static str = "";
    pub const LockdropEnd: Moment = 0;
    pub const MedianFilterExpire: Moment = 100;
    pub const MedianFilterWidth: 5;
    pub const 

}

impl Trait for Runtime {
}

pub type System = system::Module<Runtime>;
pub type Balances = balances::Module<Runtime>;
pub type Timestamp = timestamp::Module<Runtime>;
pub type PlasmLockdrop = Module<Runtime>;
