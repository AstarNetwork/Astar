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

use crate::self as pallet_xc_asset_config;

use frame_support::{construct_runtime, derive_impl, parameter_types, weights::Weight};

use sp_io::TestExternalities;
use sp_runtime::BuildStorage;

type Balance = u128;
type AccountId = u64;

type Block = frame_system::mocking::MockBlock<Test>;

const EXISTENTIAL_DEPOSIT: Balance = 2;

construct_runtime!(
    pub struct Test {
        System: frame_system,
        Balances: pallet_balances,
        XcAssetConfig: pallet_xc_asset_config,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountData = pallet_balances::AccountData<Balance>;
}

parameter_types! {
    pub const MaxLocks: u32 = 4;
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type Balance = Balance;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

type AssetId = u128;

impl pallet_xc_asset_config::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type ManagerOrigin = frame_system::EnsureRoot<AccountId>;
    type AssetHubMigrationUpdater = frame_system::EnsureRoot<AccountId>;
    type WeightInfo = ();
}

pub struct ExternalityBuilder;

impl ExternalityBuilder {
    pub fn build() -> TestExternalities {
        let storage = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
