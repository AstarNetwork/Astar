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

use crate::{
    self as pallet_price_aggregator, AverageBlockValue, BlockNumberFor, IntermediateValueAggregator,
};

use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{ConstU128, ConstU32, Hooks},
    weights::Weight,
};
use sp_io::TestExternalities;
use sp_runtime::BuildStorage;

use astar_primitives::{oracle::CurrencyId, Balance, BlockNumber};

type Block = frame_system::mocking::MockBlockU32<Test>;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountData = pallet_balances::AccountData<Balance>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type Balance = Balance;
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
}

parameter_types! {
    pub const NativeCurrencyId: CurrencyId = CurrencyId::ASTR;
    pub const AggregationDuration: BlockNumberFor<Test> = 16;
}

impl pallet_price_aggregator::Config for Test {
    // Should at least be 3 for tests to work properly
    type MaxValuesPerBlock = ConstU32<4>;
    type ProcessBlockValues = AverageBlockValue;
    type NativeCurrencyId = NativeCurrencyId;
    type CircularBufferLength = ConstU32<7>;
    type AggregationDuration = AggregationDuration;
    type WeightInfo = ();
}

construct_runtime!(
    pub struct Test {
        System: frame_system,
        Balances: pallet_balances,
        PriceAggregator: pallet_price_aggregator,
    }
);

pub struct ExtBuilder;
impl ExtBuilder {
    pub fn build() -> TestExternalities {
        let storage = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| {
            // 1. Set the initial limit block for the intermediate value aggregator
            IntermediateValueAggregator::<Test>::mutate(|v| {
                v.limit_block =
                    <Test as pallet_price_aggregator::Config>::AggregationDuration::get() + 1
            });

            // 2. Init block setting
            let init_block_number = 1;
            System::set_block_number(init_block_number);
            PriceAggregator::on_initialize(init_block_number);
        });

        ext
    }
}
