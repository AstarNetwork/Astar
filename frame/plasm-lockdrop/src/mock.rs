//! Runtime mockup for plasm-lockdrop module.

#![cfg(test)]

use super::*;
use frame_support::{
    impl_outer_dispatch, impl_outer_origin, parameter_types,
    weights::Weight,
};
use plasm_primitives::{AccountId, Balance, Moment};
use sp_runtime::{
    testing::{TestXt, Header},
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
    type MigrateAccount = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
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
    type AccountStore = frame_system::Module<Runtime>;
}

parameter_types! {
    pub const VoteThreshold: AuthorityVote = 3;
    pub const PositiveVotes: AuthorityVote = 2;
    pub const BitcoinTickerUri: &'static str = "";
    pub const EthereumTickerUri: &'static str = "";
    pub const BitcoinApiUri: &'static str = "";
    pub const EthereumApiUri: &'static str = "";
    pub const EthereumContractAddress: &'static str = "";
    pub const LockdropEnd: Moment = 0;
    pub const MedianFilterExpire: Moment = 3600;
    pub const MedianFilterWidth: usize = 5;
}

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;
type SubmitTransaction = frame_system::offchain::TransactionSubmitter<(), Call, Extrinsic>;

impl Trait for Runtime {
    type Currency = Balances;
    type VoteThreshold = VoteThreshold;
    type PositiveVotes = PositiveVotes;
    type BitcoinTickerUri = BitcoinTickerUri;
    type EthereumTickerUri = EthereumTickerUri;
    type BitcoinApiUri = BitcoinApiUri;
    type EthereumApiUri = EthereumApiUri;
    type EthereumContractAddress = EthereumContractAddress;
    type LockdropEnd = LockdropEnd;
    type MedianFilterExpire = MedianFilterExpire;
    type MedianFilterWidth = MedianFilterWidth;
    type Call = Call;
    type SubmitTransaction = SubmitTransaction;
    type AuthorityId = sr25519::AuthorityId;
    type Account = MultiSigner;
    type Time = Timestamp;
    type Moment = Moment;
    type DollarRate = Balance;
    type BalanceConvert = Balance;
    type Event = ();
}

pub type System = frame_system::Module<Runtime>;
pub type Balances = pallet_balances::Module<Runtime>;
pub type Timestamp = pallet_timestamp::Module<Runtime>;
pub type PlasmLockdrop = Module<Runtime>;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    storage.into()
}
