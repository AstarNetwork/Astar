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
    pub type V10ToV11<T, TierParamsConfig, OldErasVoting, OldErasBnE> =
        frame_support::migrations::VersionedMigration<
            10,
            11,
            v11::VersionMigrateV10ToV11<T, TierParamsConfig, OldErasVoting, OldErasBnE>,
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

    pub struct VersionMigrateV10ToV11<T, P, OldErasVoting, OldErasBnE>(
        PhantomData<(T, P, OldErasVoting, OldErasBnE)>,
    );

    impl<T: Config, P: TierParamsV11Config, OldErasVoting: Get<u32>, OldErasBnE: Get<u32>>
        UncheckedOnRuntimeUpgrade for VersionMigrateV10ToV11<T, P, OldErasVoting, OldErasBnE>
    {
        fn on_runtime_upgrade() -> Weight {
            let old_eras_voting = OldErasVoting::get();
            let old_eras_bne = OldErasBnE::get();

            let mut reads: u64 = 0;
            let mut writes: u64 = 0;

            // 0. Safety: remove excess dApps if count exceeds new limit
            let current_integrated_dapps = IntegratedDApps::<T>::count();
            reads += 1;

            let max_dapps_allowed = T::MaxNumberOfContracts::get();

            if current_integrated_dapps > max_dapps_allowed {
                log::warn!(
                    target: LOG_TARGET,
                    "Safety net triggered: {} dApps exceed limit of {}. Removing {} excess dApps.",
                    current_integrated_dapps,
                    max_dapps_allowed,
                    current_integrated_dapps - max_dapps_allowed
                );

                let excess = current_integrated_dapps.saturating_sub(max_dapps_allowed);
                let victims: Vec<_> = IntegratedDApps::<T>::iter()
                    .take(excess as usize)
                    .map(|(contract, dapp_info)| (contract, dapp_info.id))
                    .collect();

                reads += excess as u64;

                for (contract, dapp_id) in victims {
                    ContractStake::<T>::remove(&dapp_id);
                    IntegratedDApps::<T>::remove(&contract);
                    reads += 2;
                    writes += 2;

                    let current_era = ActiveProtocolState::<T>::get().era;
                    Pallet::<T>::deposit_event(Event::<T>::DAppUnregistered {
                        smart_contract: contract,
                        era: current_era,
                    });
                    log::info!(
                        target: LOG_TARGET,
                        "Safety net removed dApp ID {} (contract: {:?})",
                        dapp_id,
                        core::any::type_name::<T::SmartContract>()
                    );
                }

                // ActiveProtocolState::get() for era => 1 read (done once for all events)
                reads += 1;
            }

            // 1. Migrate StaticTierParams
            let reward_portion = BoundedVec::<Permill, T::NumberOfTiers>::truncate_from(
                P::reward_portion().to_vec(),
            );
            let slot_distribution = BoundedVec::<Permill, T::NumberOfTiers>::truncate_from(
                P::slot_distribution().to_vec(),
            );
            let tier_thresholds = BoundedVec::<TierThreshold, T::NumberOfTiers>::truncate_from(
                P::tier_thresholds().to_vec(),
            );
            let tier_rank_multipliers = BoundedVec::<u32, T::NumberOfTiers>::truncate_from(
                P::tier_rank_multipliers().to_vec(),
            );

            let new_params = TierParameters::<T::NumberOfTiers> {
                reward_portion,
                slot_distribution,
                tier_thresholds,
                slot_number_args: P::slot_number_args(),
                tier_rank_multipliers,
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
                    let current_block: u32 =
                        frame_system::Pallet::<T>::block_number().saturated_into();

                    // Old/new voting period lengths (in blocks)
                    let old_voting_length: u32 =
                        old_eras_voting.saturating_mul(T::CycleConfiguration::blocks_per_era());
                    let new_voting_length: u32 = Pallet::<T>::blocks_per_voting_period()
                        .max(T::CycleConfiguration::blocks_per_era());

                    // Old schedule
                    let remaining_old: u32 = state.next_era_start.saturating_sub(current_block);
                    let elapsed: u32 = old_voting_length.saturating_sub(remaining_old);

                    // New schedule
                    let remaining_new: u32 = new_voting_length.saturating_sub(elapsed);

                    // If new period has already passed (elapsed >= new_voting_length),
                    // schedule for next block. Otherwise, use the calculated remainder.
                    state.next_era_start = if remaining_new == 0 {
                        current_block.saturating_add(1)
                    } else {
                        current_block.saturating_add(remaining_new)
                    };

                    log::info!(
                        target: LOG_TARGET,
                        "ActiveProtocolState updated: next_era_start (old_length={}, new_length={}, elapsed={}, remaining_new={})",
                        old_voting_length,
                        new_voting_length,
                        elapsed,
                        remaining_new
                    );
                }
                if state.period_info.subperiod == Subperiod::Voting {
                    // Recalculate next_era_start block
                    let current_block: u32 =
                        frame_system::Pallet::<T>::block_number().saturated_into();
                    let new_voting_length: u32 = Pallet::<T>::blocks_per_voting_period()
                        .max(T::CycleConfiguration::blocks_per_era());
                    let remaining_old: u32 = state.next_era_start.saturating_sub(current_block);
                    // Carry over remaining time, but never extend beyond the new voting length.
                    // If already overdue, schedule for the next block.
                    let remaining_new: u32 = remaining_old.min(new_voting_length).max(1);

                    state.next_era_start = current_block.saturating_add(remaining_new);

                    log::info!(
                        target: LOG_TARGET,
                        "ActiveProtocolState updated: next_era_start (remaining_old={}, remaining_new={})",
                        remaining_old,
                        remaining_new
                    );
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
            let current_block: u32 = frame_system::Pallet::<T>::block_number().saturated_into();

            let pre_dapp_count = IntegratedDApps::<T>::count();
            let max_allowed = T::MaxNumberOfContracts::get();

            log::info!(
                target: LOG_TARGET,
                "Pre-upgrade: dApp count={}, max={}, cleanup_needed={}",
                pre_dapp_count,
                max_allowed,
                pre_dapp_count > max_allowed
            );

            Ok((
                protocol_state.period_info.subperiod,
                protocol_state.era,
                protocol_state.next_era_start,
                protocol_state.period_info.next_subperiod_start_era,
                current_block,
                pre_dapp_count,
                max_allowed,
            )
                .encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(data: Vec<u8>) -> Result<(), TryRuntimeError> {
            let (
                old_subperiod,
                old_era,
                old_next_era_start,
                old_next_subperiod_era,
                pre_block,
                pre_dapp_count,
                max_allowed,
            ): (Subperiod, EraNumber, u32, EraNumber, u32, u32, u32) =
                Decode::decode(&mut &data[..])
                    .map_err(|_| TryRuntimeError::Other("Failed to decode pre-upgrade data"))?;
            let old_eras_voting = OldErasVoting::get();
            let old_eras_bne = OldErasBnE::get();

            // Verify storage version
            ensure!(
                Pallet::<T>::on_chain_storage_version() == StorageVersion::new(11),
                "Storage version should be 11"
            );

            // 1. Verify cleanup worked
            let post_dapp_count = IntegratedDApps::<T>::count();
            log::debug!(
                "post_dapp_count={}, max_allowed={}",
                post_dapp_count,
                max_allowed
            );
            ensure!(
                post_dapp_count <= max_allowed,
                "dApp count still exceeds limit",
            );

            if pre_dapp_count > max_allowed {
                let expected_removed = pre_dapp_count - max_allowed;
                let actual_removed = pre_dapp_count - post_dapp_count;
                log::debug!(
                    "Removed {} dApps, expected to remove {}",
                    actual_removed,
                    expected_removed
                );
                ensure!(
                    actual_removed == expected_removed,
                    "Mismatch in the expected dApps to be unregistered"
                );
            }

            // 2. Verify new StaticTierParams are valid
            let new_params = StaticTierParams::<T>::get();
            ensure!(new_params.is_valid(), "New tier params invalid");
            ensure!(
                new_params.reward_portion.as_slice() == P::reward_portion(),
                "reward_portion mismatch"
            );
            ensure!(
                new_params.tier_rank_multipliers.as_slice() == P::tier_rank_multipliers(),
                "tier_rank_multipliers mismatch"
            );

            // 3. Verify ActiveProtocolState update
            let protocol_state = ActiveProtocolState::<T>::get();
            ensure!(!protocol_state.maintenance, "Maintenance mode enabled");

            if old_subperiod == Subperiod::Voting {
                let old_voting_length: u32 =
                    old_eras_voting.saturating_mul(T::CycleConfiguration::blocks_per_era());
                let new_voting_length: u32 = Pallet::<T>::blocks_per_voting_period();

                let remaining_old: u32 = old_next_era_start.saturating_sub(pre_block);
                let elapsed: u32 = old_voting_length.saturating_sub(remaining_old);
                let remaining_new: u32 = new_voting_length.saturating_sub(elapsed);

                let expected = if remaining_new == 0 {
                    pre_block.saturating_add(1)
                } else {
                    pre_block.saturating_add(remaining_new)
                };

                ensure!(
                    protocol_state.next_era_start == expected,
                    "Voting: next_era_start incorrect"
                );
            } else {
                let new_total: EraNumber =
                    T::CycleConfiguration::eras_per_build_and_earn_subperiod();
                let remaining_old: EraNumber = old_next_subperiod_era.saturating_sub(old_era);
                let elapsed: EraNumber = old_eras_bne.saturating_sub(remaining_old);
                let remaining_new: EraNumber = new_total.saturating_sub(elapsed);
                let expected: EraNumber = old_era.saturating_add(remaining_new);

                ensure!(
                    protocol_state.period_info.next_subperiod_start_era == expected,
                    "BuildEarn: next_subperiod_start_era incorrect"
                );
                ensure!(
                    old_next_era_start > expected,
                    "next_era_start did not update as expected"
                );
            }

            log::info!(target: LOG_TARGET, "Post-upgrade: All checks passed");
            Ok(())
        }
    }
}

mod v10 {
    use super::*;
    use frame_support::storage_alias;

    /// v10 TierParameters (without tier_rank_multipliers)
    #[derive(Encode, Decode, Clone)]
    pub struct TierParameters<NT: Get<u32>> {
        pub reward_portion: BoundedVec<Permill, NT>,
        pub slot_distribution: BoundedVec<Permill, NT>,
        pub tier_thresholds: BoundedVec<TierThreshold, NT>,
        pub slot_number_args: (u64, u64),
    }

    #[storage_alias]
    pub type StaticTierParams<T: Config> =
        StorageValue<Pallet<T>, TierParameters<<T as Config>::NumberOfTiers>, OptionQuery>;
}
