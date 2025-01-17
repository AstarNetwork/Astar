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

use crate::{self as pallet_static_price_provider};

use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{ConstU128, ConstU32},
    weights::Weight,
};
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

use astar_primitives::{Balance, BlockNumber};
type AccountId = u64;

type Block = frame_system::mocking::MockBlock<Test>;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = u64;
    type RuntimeCall = RuntimeCall;
    type Block = Block;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type RuntimeTask = RuntimeTask;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<4>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type RuntimeHoldReason = RuntimeHoldReason;
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
}

impl pallet_static_price_provider::Config for Test {
    type RuntimeEvent = RuntimeEvent;
}

construct_runtime!(
    pub struct Test {
        System: frame_system,
        Balances: pallet_balances,
        StaticPriceProvider: pallet_static_price_provider,
    }
);

pub struct ExternalityBuilder;
impl ExternalityBuilder {
    pub fn build() -> TestExternalities {
        let storage = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| {
            System::set_block_number(1);
        });

        ext
    }
}
