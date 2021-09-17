use crate::{
    self as pallet_dapps_staking, pallet::pallet::Config, EraPayout, NegativeImbalanceOf,
    PositiveImbalanceOf,
};

use frame_support::{
    assert_noop, assert_ok, construct_runtime, parameter_types,
    storage::{StorageDoubleMap, StorageMap},
    traits::OnUnbalanced,
};
use sp_core::{H160, H256};

use sp_io::TestExternalities;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;
pub(crate) type EraIndex = u32;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

/// Value shouldn't be less than 2 for testing purposes, otherwise we cannot test certain corner cases.
pub(crate) const EXISTENTIAL_DEPOSIT: Balance = 2;
pub(crate) const REGISTER_DEPOSIT: Balance = 200;
pub(crate) const UNBONDING_DURATION: EraIndex = 5;
pub(crate) const MAX_NUMBER_OF_STAKERS: u32 = 4;
pub(crate) const MINUMUM_STAKING_AMOUNT: Balance = 10;

construct_runtime!(
    pub enum TestRuntime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        DappsStaking: pallet_dapps_staking::{Pallet, Call, Config, Storage, Event<T>},
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
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
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
    pub const MaxStakings: u32 = 32;
    pub const BlockPerEra: BlockNumber = 100;
    pub const UnbondingDuration: EraIndex = UNBONDING_DURATION;
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

pub struct RewardRemainderMock;

impl OnUnbalanced<NegativeImbalanceOf<TestRuntime>> for RewardRemainderMock {}

/// Mocked implementation for Reward. Might need to be changed later when used.
pub struct RewardMock;

impl OnUnbalanced<PositiveImbalanceOf<TestRuntime>> for RewardMock {}

parameter_types! {
    pub const RegisterDeposit: u32 = 100;
    pub const MockBlockPerEra: BlockNumber = 10;
    pub const MaxNumberOfStakersPerContract: u32 = MAX_NUMBER_OF_STAKERS;
    pub const MinimumStakingAmount: Balance = MINUMUM_STAKING_AMOUNT;
}
impl pallet_dapps_staking::Config for TestRuntime {
    type Event = Event;
    type Currency = Balances;
    type MaxStakings = MaxStakings;
    type BlockPerEra = MockBlockPerEra;
    type UnbondingDuration = UnbondingDuration;
    type EraPayout = EraPayoutMock;
    type RegisterDeposit = RegisterDeposit;
    type WeightInfo = ();
    type UnixTime = Timestamp;
    type RewardRemainder = RewardRemainderMock;
    type Reward = RewardMock;
    type MaxNumberOfStakersPerContract = MaxNumberOfStakersPerContract;
    type MinimumStakingAmount = MinimumStakingAmount;
}

pub struct ExternalityBuilder;

impl ExternalityBuilder {
    pub fn build() -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<TestRuntime>()
            .unwrap();

        pallet_balances::GenesisConfig::<TestRuntime> {
            balances: vec![
                (1, 9000),
                (2, 800),
                (3, 650),
                (4, 490),
                (5, 380),
                (10, 300),
                (540, EXISTENTIAL_DEPOSIT),
                (1337, 1_000_000_000_000),
            ],
        }
        .assimilate_storage(&mut storage);

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
