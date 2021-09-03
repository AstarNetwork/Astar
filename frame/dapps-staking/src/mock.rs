use crate::{
    self as pallet_dapps_staking, Config, ContractFinder, EraPayout, NegativeImbalanceOf,
    PositiveImbalanceOf,
};

use frame_support::{
    assert_noop, assert_ok, construct_runtime, parameter_types,
    storage::{StorageDoubleMap, StorageMap},
    traits::OnUnbalanced,
};
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

/// The AccountId alias in this test module.
pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;
pub(crate) type EraIndex = u32;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

construct_runtime!(
    pub enum TestRuntime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>}, // TODO: should this be mocked 'properly'? This doesn't seem like I should be doing it.
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent}, // TODO: should I do it like this or create a special mock?
        DappStaking: pallet_dapps_staking::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
}

impl frame_system::Config for TestRuntime {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Index = u64;
    type Call = Call;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
}

parameter_types! {
    pub const MaxLocks: u32 = 4;
    pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for TestRuntime {
    type MaxLocks = MaxLocks;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 3;
}

impl pallet_timestamp::Config for TestRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaxStakings: u32 = 32; // TODO: should this be renamed/changed? I haven't see const declared under config yet.
    pub const BlockPerEra: BlockNumber = 100; // TODO: check this number later
    pub const BondingDuration: EraIndex = 5;
}

/// Mocked implementation for EraPayout. Might need to be changed later when used.
pub struct EraPayoutMock;

impl<Balance: Default> EraPayout<Balance> for EraPayoutMock {
    fn era_payout(
        _total_staked: Balance,
        _total_issuance: Balance,
        _era_duration_millis: u64,
    ) -> (Balance, Balance) {
        (Default::default(), Default::default())
    }
}

/// Mocked implementation for ContractFinder. Might need to be changed later when used.
pub struct ContractFinderMock;

impl ContractFinder<AccountId> for ContractFinderMock {
    fn is_exists_contract(contract_id: &AccountId) -> bool {
        true
    }
}

/// Mocked implementation for RewardRemainder. Might need to be changed later when used.
pub struct RewardRemainderMock;

impl OnUnbalanced<NegativeImbalanceOf<TestRuntime>> for RewardRemainderMock {}

/// Mocked implementation for Reward. Might need to be changed later when used.
pub struct RewardMock;

impl OnUnbalanced<PositiveImbalanceOf<TestRuntime>> for RewardMock {}

impl pallet_dapps_staking::Config for TestRuntime {
    type Event = Event;
    type Currency = Balances;
    type MaxStakings = MaxStakings;
    type BlockPerEra = BlockPerEra;
    type BondingDuration = BondingDuration;
    type EraPayout = EraPayoutMock;
    type ContractFinder = ContractFinderMock;
    type WeightInfo = ();
    type UnixTime = Timestamp; // TODO see of this can be maybe simplified
    type RewardRemainder = RewardRemainderMock;
    type Reward = RewardMock;
}

pub struct ExternalityBuilder;

impl ExternalityBuilder {
    pub fn build() -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::default() // TODO: add some balance to accounts
            .build_storage::<TestRuntime>()
            .unwrap();

        pallet_balances::GenesisConfig::<TestRuntime> {
            balances: vec![
                (1, 9000),
                (2, 800),
                (3, 650),
                (4, 490),
                (1337, 1_000_000_000_000),
            ],
        }
        .assimilate_storage(&mut storage);

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

pub(crate) fn get_dapp_staking_events() -> Vec<pallet_dapps_staking::Event<TestRuntime>> {
    System::events()
        .into_iter()
        .map(|dapp_event| dapp_event.event)
        .filter_map(|e| {
            if let Event::DappStaking(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .collect()
}
