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
    self as pallet_inflation, ActiveInflationConfig, CreditOf, CycleConfiguration,
    InflationParameters, InflationParams, PayoutPerBlock,
};

use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{fungible::Balanced, ConstU128, Hooks},
    weights::Weight,
    PalletId,
};
use sp_io::TestExternalities;
use sp_runtime::{traits::AccountIdConversion, BuildStorage, Perquintill};

use astar_primitives::{Balance, BlockNumber};

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
// Dummy accounts used to simulate reward beneficiaries balances
pub(crate) const TREASURY_POT: PalletId = PalletId(*b"moktrsry");
pub(crate) const COLLATOR_POT: PalletId = PalletId(*b"mokcolat");

pub struct DummyPayoutPerBlock;
impl PayoutPerBlock<CreditOf<Test>> for DummyPayoutPerBlock {
    fn treasury(reward: CreditOf<Test>) {
        Balances::resolve(&TREASURY_POT.into_account_truncating(), reward)
            .expect("Must succeed for test.");
    }

    fn collators(reward: CreditOf<Test>) {
        Balances::resolve(&COLLATOR_POT.into_account_truncating(), reward)
            .expect("Must succeed for test.");
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
    pub struct Test {
        System: frame_system,
        Balances: pallet_balances,
        Inflation: pallet_inflation,
    }
);

pub struct ExternalityBuilder;
impl ExternalityBuilder {
    pub fn build() -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();

        let unit = 1_000_000_000_000_000_000;
        // This will cause some initial issuance
        pallet_balances::GenesisConfig::<Test> {
            balances: vec![(1, 9 * unit), (2, 7 * unit), (3, 5 * unit)],
        }
        .assimilate_storage(&mut storage)
        .ok();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| {
            // Set initial pallet inflation values
            InflationParams::<Test>::put(INIT_PARAMS);
            let config = Inflation::recalculate_inflation(1);
            ActiveInflationConfig::<Test>::put(config);

            System::set_block_number(1);
            Inflation::on_initialize(1);
        });
        ext
    }
}

/// Assert the equality between two balances, with some leniency factor.
#[macro_export]
macro_rules! lenient_balance_assert_eq {
    ($x:expr, $y:expr) => {{
        use sp_runtime::Permill;

        let ratio = if $x > $y {
            Permill::from_rational($y, $x)
        } else {
            Permill::from_rational($x, $y)
        };

        assert!(
            ratio >= Permill::from_rational(999_u32, 1000),
            "Ratio between old and new balance is too small: {:?}",
            ratio,
        );
    }};
}
