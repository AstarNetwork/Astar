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

use super::*;

use frame_benchmarking::v2::*;
use frame_system::{Pallet as System, RawOrigin};

/// Assert that the last event equals the provided one.
fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
    System::<T>::assert_last_event(generic_event.into());
}

// Set up initial config in the database, so it's not empty.
fn initial_config<T: Config>() {
    // Some dummy inflation params
    let params = InflationParameters {
        max_inflation_rate: Perquintill::from_percent(7),
        treasury_part: Perquintill::from_percent(5),
        collators_part: Perquintill::from_percent(3),
        dapps_part: Perquintill::from_percent(20),
        base_stakers_part: Perquintill::from_percent(25),
        adjustable_stakers_part: Perquintill::from_percent(35),
        bonus_part: Perquintill::from_percent(12),
        ideal_staking_rate: Perquintill::from_percent(50),
    };
    assert!(params.is_valid());

    // Some dummy inflation config
    let total_issuance = T::Currency::total_issuance();
    let issuance_safety_cap =
        total_issuance.saturating_add(params.max_inflation_rate * total_issuance);
    let config = InflationConfiguration {
        recalculation_block: 123,
        issuance_safety_cap,
        collator_reward_per_block: 11111,
        treasury_reward_per_block: 33333,
        dapp_reward_pool_per_era: 55555,
        base_staker_reward_pool_per_era: 77777,
        adjustable_staker_reward_pool_per_era: 99999,
        bonus_reward_pool_per_period: 123987,
        ideal_staking_rate: Perquintill::from_percent(50),
    };

    InflationParams::<T>::put(params);
    ActiveInflationConfig::<T>::put(config);

    // Create some issuance so it's not zero
    let dummy_account = whitelisted_caller();
    T::Currency::make_free_balance_be(&dummy_account, 1_000_000_000_000_000_000_000);
}

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn force_set_inflation_params() {
        initial_config::<T>();

        let params = InflationParameters::default();
        assert!(params.is_valid());

        #[extrinsic_call]
        _(RawOrigin::Root, params);

        assert_last_event::<T>(Event::<T>::InflationParametersForceChanged.into());
    }

    #[benchmark]
    fn force_set_inflation_config() {
        initial_config::<T>();
        let config = InflationConfiguration::default();

        #[extrinsic_call]
        _(RawOrigin::Root, config.clone());

        assert_last_event::<T>(Event::<T>::InflationConfigurationForceChanged { config }.into());
    }

    #[benchmark]
    fn force_inflation_recalculation() {
        initial_config::<T>();

        #[extrinsic_call]
        _(RawOrigin::Root);

        let config = ActiveInflationConfig::<T>::get();
        assert_last_event::<T>(Event::<T>::ForcedInflationRecalculation { config }.into());
    }

    #[benchmark]
    fn hook_with_recalculation() {
        initial_config::<T>();

        ActiveInflationConfig::<T>::mutate(|config| {
            config.recalculation_block = 0;
        });
        let init_issuance = T::Currency::total_issuance();

        let block = 1;
        #[block]
        {
            Pallet::<T>::on_initialize(block);
            Pallet::<T>::on_finalize(block);
        }

        assert!(ActiveInflationConfig::<T>::get().recalculation_block > 0);

        // The 'sane' assumption is that at least something will be issued for treasury & collators
        assert!(T::Currency::total_issuance() > init_issuance);
    }

    #[benchmark]
    fn hook_without_recalculation() {
        initial_config::<T>();

        ActiveInflationConfig::<T>::mutate(|config| {
            config.recalculation_block = 2;
        });
        let init_config = ActiveInflationConfig::<T>::get();
        let init_issuance = T::Currency::total_issuance();

        // Has to be at least 2 blocks less than the recalculation block.
        let block = 0;
        #[block]
        {
            Pallet::<T>::on_initialize(block);
            Pallet::<T>::on_finalize(block);
        }

        assert_eq!(ActiveInflationConfig::<T>::get(), init_config);

        // The 'sane' assumption is that at least something will be issued for treasury & collators
        assert!(T::Currency::total_issuance() > init_issuance);
    }

    impl_benchmark_test_suite!(
        Pallet,
        crate::benchmarking::tests::new_test_ext(),
        crate::mock::Test,
    );
}

#[cfg(test)]
mod tests {
    use crate::mock;
    use frame_support::sp_io::TestExternalities;

    pub fn new_test_ext() -> TestExternalities {
        mock::ExternalityBuilder::build()
    }
}
