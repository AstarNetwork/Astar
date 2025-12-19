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

use super::*;
use crate as pallet_dynamic_evm_base_fee;

use frame_support::{
    construct_runtime, derive_impl, parameter_types, storage,
    traits::{ConstU128, ConstU64, Get},
};
use parity_scale_codec::Encode;
use sp_io::TestExternalities;
use sp_runtime::{traits::One, BuildStorage, FixedU128, Perquintill};

pub(crate) type Balance = u128;

parameter_types! {
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for TestRuntime {
    type Block = Block;
    type AccountData = pallet_balances::AccountData<Balance>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for TestRuntime {
    type Balance = Balance;
    type ExistentialDeposit = ConstU128<2>;
    type AccountStore = System;
}

#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for TestRuntime {
    type MinimumPeriod = ConstU64<3>;
}

parameter_types! {
    pub DefaultBaseFeePerGas: U256 = U256::from(1_500_000_000_000_u128);
    pub MinBaseFeePerGas: U256 = U256::from(800_000_000_000_u128);
    pub MaxBaseFeePerGas: U256 = U256::from(80_000_000_000_000_u128);
    pub StepLimitRation: Perquintill = Perquintill::from_rational(30_u128, 1_000_000);
}

impl pallet_dynamic_evm_base_fee::Config for TestRuntime {
    type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
    type MinBaseFeePerGas = MinBaseFeePerGas;
    type MaxBaseFeePerGas = MaxBaseFeePerGas;
    type AdjustmentFactor = GetAdjustmentFactor;
    type WeightFactor = ConstU128<30_000_000_000_000_000>;
    type StepLimitRatio = StepLimitRation;
    type WeightInfo = ();
}

type Block = frame_system::mocking::MockBlockU32<TestRuntime>;

construct_runtime!(
    pub struct TestRuntime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        DynamicEvmBaseFee: pallet_dynamic_evm_base_fee,
    }
);

const ADJUSTMENT_FACTOR: &[u8] = b":adj_factor_evm";

/// Helper method to set the adjustment factor used by the pallet.
pub fn set_adjustment_factor(factor: FixedU128) {
    storage::unhashed::put_raw(&ADJUSTMENT_FACTOR, &factor.encode());
}

pub struct GetAdjustmentFactor;
impl Get<FixedU128> for GetAdjustmentFactor {
    fn get() -> FixedU128 {
        storage::unhashed::get::<FixedU128>(&ADJUSTMENT_FACTOR).unwrap_or_default()
    }
}

pub struct ExtBuilder;
impl ExtBuilder {
    pub fn build() -> TestExternalities {
        let storage = frame_system::GenesisConfig::<TestRuntime>::default()
            .build_storage()
            .unwrap();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| {
            set_adjustment_factor(FixedU128::one());
            System::set_block_number(1);
        });
        ext
    }
}

/// Ideal `base fee per gas` value according to the fee alignment formula.
/// It changes dynamically based on `adjustment factor` and `weight factor` parameters.
pub fn get_ideal_bfpg() -> U256 {
    U256::from(
        <TestRuntime as pallet_dynamic_evm_base_fee::Config>::AdjustmentFactor::get()
            .saturating_mul_int::<u128>(
                <TestRuntime as pallet_dynamic_evm_base_fee::Config>::WeightFactor::get(),
            )
            .saturating_mul(25)
            .saturating_div(98974),
    )
}

/// Max step limit describes how much `base fee per gas` can move in any direction during one block.
pub fn get_max_step_limit() -> U256 {
    let bfpg: u128 = BaseFeePerGas::<TestRuntime>::get().unique_saturated_into();
    let max_allowed_step: u128 = <TestRuntime as pallet::Config>::StepLimitRatio::get() * bfpg;

    U256::from(max_allowed_step)
}
