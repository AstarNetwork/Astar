// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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

#![cfg(test)]

use frame_support::{
    construct_runtime, derive_impl,
    migrations::MultiStepMigrator,
    pallet_prelude::*,
    parameter_types,
    traits::{OnFinalize, OnInitialize, WithdrawReasons},
};
use sp_runtime::{traits::ConvertInto, BuildStorage};

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
    pub struct Runtime {
        System: frame_system,
        Balances: pallet_balances,
        Vesting: pallet_vesting,
        MultiBlockMigrations: pallet_migrations,
        Pallet: crate,
    }
);

impl crate::Config for Runtime {}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type AccountData = pallet_balances::AccountData<u64>;
    type Block = Block;
    type MultiBlockMigrator = MultiBlockMigrations;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Runtime {
    type AccountStore = System;
    type ExistentialDeposit = ExistentialDeposit;
}

parameter_types! {
    pub const MaxServiceWeight: Weight = Weight::MAX.div(10);
}

#[derive_impl(pallet_migrations::config_preludes::TestDefaultConfig)]
impl pallet_migrations::Config for Runtime {
    // #[cfg(feature = "runtime-benchmarks")]
    // type Migrations = pallet_migrations::mock_helpers::MockedMigrations;
    // #[cfg(not(feature = "runtime-benchmarks"))]
    type Migrations = (crate::LazyMigration<Runtime, crate::weights::SubstrateWeight<Runtime>>,);
    type MigrationStatusHandler = ();
    type MaxServiceWeight = MaxServiceWeight;
}

parameter_types! {
    pub const MinVestedTransfer: u64 = 256 * 2;
    pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
        WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
    pub static ExistentialDeposit: u64 = 1;
}

impl pallet_vesting::Config for Runtime {
    type BlockNumberToBalance = ConvertInto;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    const MAX_VESTING_SCHEDULES: u32 = 3;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = ();
    type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
    type BlockNumberProvider = System;
}

#[derive(Default)]
pub struct ExtBuilder;

pub(crate) const ALICE: u64 = 1;
pub(crate) const BOB: u64 = 2;
pub(crate) const CHARLIE: u64 = 3;
pub(crate) const DAVE: u64 = 4;

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .unwrap();
        pallet_balances::GenesisConfig::<Runtime> {
            balances: vec![(ALICE, 100), (BOB, 100), (CHARLIE, 1000), (DAVE, 800)],
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let vesting = vec![
            // who, start_at, length, liquid
            (ALICE, 0, 10, 0),
            (BOB, 10, 10, 0),
            (CHARLIE, 20, 10, 0),
            (DAVE, 5, 20, 400),
        ];

        pallet_vesting::GenesisConfig::<Runtime> { vesting }
            .assimilate_storage(&mut t)
            .unwrap();
        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    ExtBuilder::default().build()
}

#[allow(dead_code)]
pub fn run_to_block(n: u64) {
    assert!(System::block_number() < n);
    while System::block_number() < n {
        let b = System::block_number();
        AllPalletsWithSystem::on_finalize(b);
        // Done by Executive:
        <Runtime as frame_system::Config>::MultiBlockMigrator::step();
        System::set_block_number(b + 1);
        AllPalletsWithSystem::on_initialize(b + 1);
    }
}
