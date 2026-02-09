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

use super::*;
use astar_primitives::dapp_staking::FIXED_TIER_SLOTS_ARGS;
use core::marker::PhantomData;
use frame_support::traits::UncheckedOnRuntimeUpgrade;

#[cfg(feature = "try-runtime")]
mod try_runtime_imports {
    pub use sp_runtime::TryRuntimeError;
}

#[cfg(feature = "try-runtime")]
use try_runtime_imports::*;

/// Exports for versioned migration `type`s for this pallet.
pub mod versioned_migrations {
    use super::*;

    /// Migration V10 to V11 wrapped in a [`frame_support::migrations::VersionedMigration`], ensuring
    /// the migration is only performed when on-chain version is 10.
    pub type V10ToV11<T, TierParamsConfig, OldErasBnE> =
        frame_support::migrations::VersionedMigration<
            10,
            11,
            v11::VersionMigrateV10ToV11<T, TierParamsConfig, OldErasBnE>,
            Pallet<T>,
            <T as frame_system::Config>::DbWeight,
        >;
}

/// Configuration for V11 tier parameters
pub trait TierParamsV11Config {
    fn reward_portion() -> [Permill; 4];
    fn slot_distribution() -> [Permill; 4];
    fn tier_thresholds() -> [TierThreshold; 4];
    fn slot_number_args() -> (u64, u64);
    fn tier_rank_multipliers() -> [u32; 4];
}

pub struct DefaultTierParamsV11;
impl TierParamsV11Config for DefaultTierParamsV11 {
    fn reward_portion() -> [Permill; 4] {
        [
            Permill::from_percent(0),
            Permill::from_percent(70),
            Permill::from_percent(30),
            Permill::from_percent(0),
        ]
    }

    fn slot_distribution() -> [Permill; 4] {
        [
            Permill::from_percent(0),
            Permill::from_parts(375_000), // 37.5%
            Permill::from_parts(625_000), // 62.5%
            Permill::from_percent(0),
        ]
    }

    fn tier_thresholds() -> [TierThreshold; 4] {
        [
            TierThreshold::FixedPercentage {
                required_percentage: Perbill::from_parts(23_200_000), // 2.32%
            },
            TierThreshold::FixedPercentage {
                required_percentage: Perbill::from_parts(9_300_000), // 0.93%
            },
            TierThreshold::FixedPercentage {
                required_percentage: Perbill::from_parts(3_500_000), // 0.35%
            },
            // Tier 3: unreachable dummy
            TierThreshold::FixedPercentage {
                required_percentage: Perbill::from_parts(0), // 0%
            },
        ]
    }

    fn slot_number_args() -> (u64, u64) {
        FIXED_TIER_SLOTS_ARGS
    }

    fn tier_rank_multipliers() -> [u32; 4] {
        [0, 24_000, 46_700, 0]
    }
}

mod v11 {
    use super::*;

    pub struct VersionMigrateV10ToV11<T, P, OldErasBnE>(PhantomData<(T, P, OldErasBnE)>);

    impl<T: Config, P: TierParamsV11Config, OldErasBnE: Get<u32>> UncheckedOnRuntimeUpgrade
        for VersionMigrateV10ToV11<T, P, OldErasBnE>
    {
        fn on_runtime_upgrade() -> Weight {
            let old_eras_bne = OldErasBnE::get();

            let mut reads: u64 = 0;
            let mut writes: u64 = 0;

            // 1. Migrate StaticTierParams
            let new_params = TierParameters::<T::NumberOfTiers> {
                reward_portion: BoundedVec::try_from(P::reward_portion().to_vec())
                    .expect("4 tiers configured"),
                slot_distribution: BoundedVec::try_from(P::slot_distribution().to_vec())
                    .expect("4 tiers configured"),
                tier_thresholds: BoundedVec::try_from(P::tier_thresholds().to_vec())
                    .expect("4 tiers configured"),
                slot_number_args: P::slot_number_args(),
                tier_rank_multipliers: BoundedVec::try_from(P::tier_rank_multipliers().to_vec())
                    .expect("4 tiers configured"),
            };

            if !new_params.is_valid() {
                log::error!(
                    target: LOG_TARGET,
                    "New TierParameters validation failed. Enabling maintenance mode."
                );

                // ActiveProtocolState::mutate => 1 read + 1 write
                ActiveProtocolState::<T>::mutate(|state| {
                    state.maintenance = true;
                });
                reads += 1;
                writes += 1;

                return T::DbWeight::get().reads_writes(reads, writes);
            }

            // StaticTierParams::put => 1 write
            StaticTierParams::<T>::put(new_params);
            writes += 1;
            log::info!(target: LOG_TARGET, "StaticTierParams updated successfully");

            // 2. Update ActiveProtocolState in a SINGLE mutate (avoid extra .get() read)
            // ActiveProtocolState::mutate => 1 read + 1 write
            ActiveProtocolState::<T>::mutate(|state| {
                if state.period_info.subperiod == Subperiod::Voting {
                    // Recalculate next_era_start block
                    let current_block: u32 =
                        frame_system::Pallet::<T>::block_number().saturated_into();
                    let new_voting_length: u32 = Pallet::<T>::blocks_per_voting_period();

                    state.next_era_start = current_block.saturating_add(new_voting_length);

                    log::info!(target: LOG_TARGET, "ActiveProtocolState updated: next_era_start");
                } else {
                    // Build&Earn: adjust remainder for next_subperiod_start_era
                    let new_eras_total: EraNumber =
                        T::CycleConfiguration::eras_per_build_and_earn_subperiod();

                    // "only the remainder" logic
                    let current_era: EraNumber = state.era;
                    let old_end: EraNumber = state.period_info.next_subperiod_start_era;

                    let remaining_old: EraNumber = old_end.saturating_sub(current_era);
                    let elapsed: EraNumber = old_eras_bne.saturating_sub(remaining_old);

                    let remaining_new: EraNumber = new_eras_total.saturating_sub(elapsed);

                    state.period_info.next_subperiod_start_era =
                        current_era.saturating_add(remaining_new);

                    log::info!(
                        target: LOG_TARGET,
                        "ActiveProtocolState updated: next_subperiod_start_era (remainder-adjusted)"
                    );
                }
            });
            reads += 1;
            writes += 1;

            T::DbWeight::get().reads_writes(reads, writes)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
            let protocol_state = ActiveProtocolState::<T>::get();
            let subperiod = protocol_state.period_info.subperiod;
            let current_era = protocol_state.era;
            let next_era_start = protocol_state.next_era_start;
            let old_next_subperiod_era = protocol_state.period_info.next_subperiod_start_era;
            let current_block: u32 = frame_system::Pallet::<T>::block_number().saturated_into();

            log::info!(
                target: LOG_TARGET,
                "Pre-upgrade: era={}, subperiod={:?}, next_era_start={}, next_subperiod_era={}, block={}",
                current_era,
                subperiod,
                next_era_start,
                old_next_subperiod_era,
                current_block
            );

            // Verify current StaticTierParams can be read
            let old_params = StaticTierParams::<T>::get();
            ensure!(old_params.is_valid(), "Old tier params invalid");

            Ok((
                subperiod,
                current_era,
                next_era_start,
                old_next_subperiod_era,
                current_block,
            )
                .encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(data: Vec<u8>) -> Result<(), TryRuntimeError> {
            let (old_subperiod, old_era, old_next_era_start, old_next_subperiod_era, pre_block): (
                Subperiod,
                EraNumber,
                u32,
                EraNumber,
                u32,
            ) = Decode::decode(&mut &data[..])
                .map_err(|_| TryRuntimeError::Other("Failed to decode pre-upgrade data"))?;
            let old_eras_bne = OldErasBnE::get();

            // Verify storage version
            ensure!(
                Pallet::<T>::on_chain_storage_version() == StorageVersion::new(11),
                "Version should be 11"
            );

            // Verify new params are valid
            let new_params = StaticTierParams::<T>::get();
            ensure!(new_params.is_valid(), "New tier params invalid");
            ensure!(
                new_params.tier_rank_multipliers.len() == 4,
                "Should have 4 tier_rank_multipliers entries"
            );

            // Verify ActiveProtocolState update
            let protocol_state = ActiveProtocolState::<T>::get();

            if old_subperiod == Subperiod::Voting {
                // expected_end = pre_block + new_voting_length
                let new_voting_length: u32 = Pallet::<T>::blocks_per_voting_period();
                let expected_end = pre_block.saturating_add(new_voting_length);

                ensure!(
                    protocol_state.next_era_start == expected_end,
                    "next_era_start should be pre_block + new_voting_length"
                );

                // Optional sanity: should have changed (unless it already matched)
                ensure!(
                    protocol_state.next_era_start != old_next_era_start
                        || old_next_era_start == expected_end,
                    "next_era_start did not update as expected"
                );

                // We did NOT change next_subperiod_start_era in voting branch (in this migration)
                ensure!(
                    protocol_state.period_info.next_subperiod_start_era == old_next_subperiod_era,
                    "next_subperiod_start_era should be unchanged in Voting branch"
                );

                log::info!(target: LOG_TARGET, "Post-upgrade: Voting branch OK");
            } else {
                // Build&Earn branch: remainder-adjusted next_subperiod_start_era
                let new_total: EraNumber =
                    T::CycleConfiguration::eras_per_build_and_earn_subperiod();

                let current_era: EraNumber = old_era;
                let old_end: EraNumber = old_next_subperiod_era;

                let remaining_old: EraNumber = old_end.saturating_sub(current_era);
                let elapsed: EraNumber = old_eras_bne.saturating_sub(remaining_old);
                let remaining_new: EraNumber = new_total.saturating_sub(elapsed);

                let expected_new_end: EraNumber = current_era.saturating_add(remaining_new);

                ensure!(
                    protocol_state.period_info.next_subperiod_start_era == expected_new_end,
                    "next_subperiod_start_era should be remainder-adjusted to the new schedule"
                );

                // next_era_start was not modified by this branch in this migration
                ensure!(
                    protocol_state.next_era_start == old_next_era_start,
                    "next_era_start should be unchanged in Build&Earn branch"
                );

                log::info!(target: LOG_TARGET, "Post-upgrade: Build&Earn branch OK");
            }

            // Verify not in maintenance mode
            ensure!(
                !protocol_state.maintenance,
                "Maintenance mode should not be enabled"
            );

            Ok(())
        }
    }
}
