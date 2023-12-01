// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

use crate::{
    self as pallet_inflation, ActiveInflationConfig, CycleConfiguration, InflationConfiguration,
    InflationParameters, InflationParams, NegativeImbalanceOf, PayoutPerBlock,
};

use frame_support::{
    construct_runtime, parameter_types,
    sp_io::TestExternalities,
    traits::Currency,
    traits::{ConstU128, ConstU32, Hooks},
    weights::Weight,
    PalletId,
};

use sp_core::H256;
use sp_runtime::{
    traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
    Perquintill,
};

use astar_primitives::{testing::Header, Balance, BlockNumber};
pub(crate) type AccountId = u64;

/// Initial inflation params set by the mock.
pub const INIT_PARAMS: InflationParameters = InflationParameters {
    max_inflation_rate: Perquintill::from_percent(7),
    treasury_part: Perquintill::from_percent(5),
    collators_part: Perquintill::from_percent(3),
    dapps_part: Perquintill::from_percent(20),
    base_stakers_part: Perquintill::from_percent(25),
    adjustable_stakers_part: Perquintill::from_percent(35),
    bonus_part: Perquintill::from_percent(12),
    ideal_staking_rate: Perquintill::from_percent(50),
};

/// Initial inflation config set by the mock.
pub const INIT_CONFIG: InflationConfiguration = InflationConfiguration {
    recalculation_block: 100,
    issuance_safety_cap: 1_000_000,
    collator_reward_per_block: 1000,
    treasury_reward_per_block: 1500,
    dapp_reward_pool_per_era: 3000,
    base_staker_reward_pool_per_era: 5000,
    adjustable_staker_reward_pool_per_era: 7000,
    bonus_reward_pool_per_period: 4000,
    ideal_staking_rate: Perquintill::from_percent(50),
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Index = u64;
    type RuntimeCall = RuntimeCall;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
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
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<4>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type HoldIdentifier = ();
    type FreezeIdentifier = ();
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
}
// Dummy accounts used to simulate reward beneficiaries balances
pub(crate) const TREASURY_POT: PalletId = PalletId(*b"moktrsry");
pub(crate) const COLLATOR_POT: PalletId = PalletId(*b"mokcolat");

pub struct DummyPayoutPerBlock;
impl PayoutPerBlock<NegativeImbalanceOf<Test>> for DummyPayoutPerBlock {
    fn treasury(reward: NegativeImbalanceOf<Test>) {
        Balances::resolve_creating(&TREASURY_POT.into_account_truncating(), reward);
    }

    fn collators(reward: NegativeImbalanceOf<Test>) {
        Balances::resolve_creating(&COLLATOR_POT.into_account_truncating(), reward);
    }
}

pub struct DummyCycleConfiguration;
impl CycleConfiguration for DummyCycleConfiguration {
    fn periods_per_cycle() -> u32 {
        5
    }

    fn eras_per_voting_subperiod() -> u32 {
        2
    }

    fn eras_per_build_and_earn_subperiod() -> u32 {
        17
    }

    fn blocks_per_era() -> u32 {
        11
    }
}

impl pallet_inflation::Config for Test {
    type Currency = Balances;
    type PayoutPerBlock = DummyPayoutPerBlock;
    type CycleConfiguration = DummyCycleConfiguration;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

construct_runtime!(
    pub struct Test
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Balances: pallet_balances,
        Inflation: pallet_inflation,
    }
);

pub struct ExternalityBuilder;
impl ExternalityBuilder {
    pub fn build() -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        // This will cause some initial issuance
        pallet_balances::GenesisConfig::<Test> {
            balances: vec![(1, 9000), (2, 800), (3, 10000)],
        }
        .assimilate_storage(&mut storage)
        .ok();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| {
            // Set initial pallet inflation values
            ActiveInflationConfig::<Test>::put(INIT_CONFIG);
            InflationParams::<Test>::put(INIT_PARAMS);

            System::set_block_number(1);
            Inflation::on_initialize(1);
        });
        ext
    }
}

/// Advance to the specified block number.
/// Function assumes first block has been initialized.
pub(crate) fn advance_to_block(n: BlockNumber) {
    while System::block_number() < n {
        Inflation::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        Inflation::on_initialize(System::block_number());
    }
}
