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

use super::{Pallet as DappStaking, *};

use astar_primitives::Balance;

use frame_system::Pallet as System;

/// Run to the specified block number.
/// Function assumes first block has been initialized.
pub(super) fn run_to_block<T: Config>(n: BlockNumberFor<T>) {
    while System::<T>::block_number() < n {
        DappStaking::<T>::on_finalize(System::<T>::block_number());
        System::<T>::set_block_number(System::<T>::block_number() + 1);
        // This is performed outside of dapps staking but we expect it before on_initialize
        DappStaking::<T>::on_initialize(System::<T>::block_number());
    }
}

/// Run for the specified number of blocks.
/// Function assumes first block has been initialized.
pub(super) fn run_for_blocks<T: Config>(n: BlockNumberFor<T>) {
    run_to_block::<T>(System::<T>::block_number() + n);
}

/// Advance blocks until the specified era has been reached.
///
/// Function has no effect if era is already passed.
pub(super) fn advance_to_era<T: Config>(era: EraNumber) {
    assert!(era >= ActiveProtocolState::<T>::get().era);
    while ActiveProtocolState::<T>::get().era < era {
        run_for_blocks::<T>(One::one());
    }
}

/// Advance blocks until the specified era has been reached.
///
/// Relies on the `force` approach to advance one era per block.
pub(super) fn force_advance_to_era<T: Config>(era: EraNumber) {
    assert!(era >= ActiveProtocolState::<T>::get().era);
    while ActiveProtocolState::<T>::get().era < era {
        assert_ok!(DappStaking::<T>::force(
            RawOrigin::Root.into(),
            ForcingType::Era
        ));
        run_for_blocks::<T>(One::one());
    }
}

/// Advance blocks until next era has been reached.
pub(super) fn _advance_to_next_era<T: Config>() {
    advance_to_era::<T>(ActiveProtocolState::<T>::get().era + 1);
}

/// Advance to next era, in the next block using the `force` approach.
pub(crate) fn force_advance_to_next_era<T: Config>() {
    assert_ok!(DappStaking::<T>::force(
        RawOrigin::Root.into(),
        ForcingType::Era
    ));
    run_for_blocks::<T>(One::one());
}

/// Advance blocks until the specified period has been reached.
///
/// Function has no effect if period is already passed.
pub(super) fn _advance_to_period<T: Config>(period: PeriodNumber) {
    assert!(period >= ActiveProtocolState::<T>::get().period_number());
    while ActiveProtocolState::<T>::get().period_number() < period {
        run_for_blocks::<T>(One::one());
    }
}

/// Advance to the specified period, using the `force` approach.
pub(super) fn force_advance_to_period<T: Config>(period: PeriodNumber) {
    assert!(period >= ActiveProtocolState::<T>::get().period_number());
    while ActiveProtocolState::<T>::get().period_number() < period {
        force_advance_to_next_subperiod::<T>();
    }
}

/// Advance blocks until next period has been reached.
pub(super) fn _advance_to_next_period<T: Config>() {
    _advance_to_period::<T>(ActiveProtocolState::<T>::get().period_number() + 1);
}

/// Advance blocks until next period has been reached.
///
/// Relies on the `force` approach to advance one subperiod per block.
pub(super) fn force_advance_to_next_period<T: Config>() {
    let init_period_number = ActiveProtocolState::<T>::get().period_number();
    while ActiveProtocolState::<T>::get().period_number() == init_period_number {
        assert_ok!(DappStaking::<T>::force(
            RawOrigin::Root.into(),
            ForcingType::Subperiod
        ));
        run_for_blocks::<T>(One::one());
    }
}

/// Advance blocks until next period type has been reached.
pub(super) fn _advance_to_next_subperiod<T: Config>() {
    let subperiod = ActiveProtocolState::<T>::get().subperiod();
    while ActiveProtocolState::<T>::get().subperiod() == subperiod {
        run_for_blocks::<T>(One::one());
    }
}

/// Use the `force` approach to advance to the next subperiod immediately in the next block.
pub(super) fn force_advance_to_next_subperiod<T: Config>() {
    assert_ok!(DappStaking::<T>::force(
        RawOrigin::Root.into(),
        ForcingType::Subperiod
    ));
    run_for_blocks::<T>(One::one());
}

/// All our networks use 18 decimals for native currency so this should be fine.
pub(super) const UNIT: Balance = 1_000_000_000_000_000_000;

/// Minimum amount that must be staked on a dApp to enter any tier
pub(super) const MIN_TIER_THRESHOLD: Balance = 10 * UNIT;

/// Number of slots in the tier system.
pub(super) const NUMBER_OF_SLOTS: u32 = 100;

/// Random seed.
pub(super) const SEED: u32 = 9000;

/// Assert that the last event equals the provided one.
pub(super) fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
    frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

// Return all dApp staking events from the event buffer.
pub(super) fn dapp_staking_events<T: Config>() -> Vec<crate::Event<T>> {
    System::<T>::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| <T as Config>::RuntimeEvent::from(e).try_into().ok())
        .collect::<Vec<_>>()
}

/// Initialize dApp staking pallet with initial config.
///
/// **NOTE:** This assumes similar tier configuration for all runtimes.
/// If we decide to change this, we'll need to provide a more generic init function.
pub(super) fn initial_config<T: Config>() {
    let era_length = T::CycleConfiguration::blocks_per_era();
    let voting_period_length_in_eras = T::CycleConfiguration::eras_per_voting_subperiod();

    // Init protocol state
    ActiveProtocolState::<T>::put(ProtocolState {
        era: 1,
        next_era_start: era_length.saturating_mul(voting_period_length_in_eras.into()) + 1,
        period_info: PeriodInfo {
            number: 1,
            subperiod: Subperiod::Voting,
            next_subperiod_start_era: 2,
        },
        maintenance: false,
    });

    // Init tier params
    let tier_params = TierParameters::<T::NumberOfTiers> {
        reward_portion: BoundedVec::try_from(vec![
            Permill::from_percent(40),
            Permill::from_percent(30),
            Permill::from_percent(20),
            Permill::from_percent(10),
        ])
        .unwrap(),
        slot_distribution: BoundedVec::try_from(vec![
            Permill::from_percent(10),
            Permill::from_percent(20),
            Permill::from_percent(30),
            Permill::from_percent(40),
        ])
        .unwrap(),
        tier_thresholds: BoundedVec::try_from(vec![
            TierThreshold::DynamicTvlAmount {
                amount: 100 * UNIT,
                minimum_amount: 80 * UNIT,
            },
            TierThreshold::DynamicTvlAmount {
                amount: 50 * UNIT,
                minimum_amount: 40 * UNIT,
            },
            TierThreshold::DynamicTvlAmount {
                amount: 20 * UNIT,
                minimum_amount: 20 * UNIT,
            },
            TierThreshold::FixedTvlAmount {
                amount: MIN_TIER_THRESHOLD,
            },
        ])
        .unwrap(),
    };

    // Init tier config, based on the initial params
    let init_tier_config = TiersConfiguration::<T::NumberOfTiers> {
        number_of_slots: NUMBER_OF_SLOTS.try_into().unwrap(),
        slots_per_tier: BoundedVec::try_from(vec![10, 20, 30, 40]).unwrap(),
        reward_portion: tier_params.reward_portion.clone(),
        tier_thresholds: tier_params.tier_thresholds.clone(),
    };

    assert!(tier_params.is_valid());
    assert!(init_tier_config.is_valid());

    StaticTierParams::<T>::put(tier_params);
    TierConfig::<T>::put(init_tier_config.clone());
}

/// Maximum number of contracts that 'makes sense' - considers both contract number limit & number of slots.
pub(super) fn max_number_of_contracts<T: Config>() -> u32 {
    T::MaxNumberOfContracts::get().min(NUMBER_OF_SLOTS).into()
}

/// Registers & staked on the specified number of smart contracts
///
/// Stake amounts are decided in such a way to maximize tier filling rate.
/// This means that all of the contracts should end up in some tier.
pub(super) fn prepare_contracts_for_tier_assignment<T: Config>(x: u32) {
    let developer: T::AccountId = whitelisted_caller();
    for id in 0..x {
        let smart_contract = T::BenchmarkHelper::get_smart_contract(id as u32);
        assert_ok!(DappStaking::<T>::register(
            RawOrigin::Root.into(),
            developer.clone().into(),
            smart_contract,
        ));
    }

    let anchor_amount = 1000 * MIN_TIER_THRESHOLD;
    let mut amounts: Vec<_> = (0..x)
        .map(|i| anchor_amount - UNIT * i as Balance)
        .collect();
    trivial_fisher_yates_shuffle(&mut amounts, SEED.into());

    for id in 0..x {
        let amount = amounts[id as usize];
        let staker = account("staker", id.into(), 1337);
        T::BenchmarkHelper::set_balance(&staker, amount);
        assert_ok!(DappStaking::<T>::lock(
            RawOrigin::Signed(staker.clone()).into(),
            amount,
        ));

        let smart_contract = T::BenchmarkHelper::get_smart_contract(id as u32);
        assert_ok!(DappStaking::<T>::stake(
            RawOrigin::Signed(staker.clone()).into(),
            smart_contract,
            amount,
        ));
    }
}

/// Reuse from `sassafras` pallet tests.
///
/// Just a trivial, insecure shuffle for the benchmarks.
fn trivial_fisher_yates_shuffle<T>(vector: &mut Vec<T>, random_seed: u64) {
    let mut rng = random_seed as usize;
    for i in (1..vector.len()).rev() {
        let j = rng % (i + 1);
        vector.swap(i, j);
        rng = (rng.wrapping_mul(8427637) + 1) as usize; // Some random number generation
    }
}
