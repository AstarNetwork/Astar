// Copyright 2020-2021 Zenlink
// Licensed under GPL-3.0.

//! Test utilities
use frame_support::{parameter_types, PalletId};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use crate as pallet_zenlink;
pub use crate::{
    Config, MultiAssetsHandler, Pallet, ParaId, ZenlinkMultiAssets, LIQUIDITY, LOCAL, NATIVE,
    RESERVED,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 8,
        Zenlink: pallet_zenlink::{Pallet, Call, Storage, Event<T>} = 9,
    }
);

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;

    pub const BlockHashCount: u64 = 250;
    pub const ZenlinkPalletId: PalletId = PalletId(*b"/zenlink");
    pub const GetExchangeFee: (u32, u32) = (3, 1000);   // 0.3%
}

impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type Origin = Origin;
    type Index = u64;
    type Call = Call;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u128;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type PalletInfo = PalletInfo;
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
    type OnSetCode = ();
}

impl pallet_balances::Config for Test {
    type Balance = u128;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Test>;
    type WeightInfo = ();
    type MaxLocks = ();
}

impl Config for Test {
    type Event = Event;
    type GetExchangeFee = GetExchangeFee;
    type MultiAssetsHandler = ZenlinkMultiAssets<Zenlink, Balances>;
    type PalletId = ZenlinkPalletId;
    type TargetChains = ();
    type SelfParaId = ();
    type XcmExecutor = ();
    type Conversion = ();
}

pub type DexPallet = Pallet<Test>;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 34028236692093846346337460743176821145),
            (2, 10),
            (3, 10),
            (4, 10),
            (5, 10),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}
