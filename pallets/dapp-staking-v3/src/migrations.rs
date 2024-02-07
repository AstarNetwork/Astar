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
use sp_arithmetic::fixed_point::FixedU64;

/// `OnRuntimeUpgrade` logic used to set & configure init dApp staking v3 storage items.
pub struct DAppStakingV3InitConfig<T, G, P>(PhantomData<(T, G, P)>);
impl<
        T: Config,
        G: Get<(
            EraNumber,
            TierParameters<T::NumberOfTiers>,
            TiersConfiguration<T::NumberOfTiers>,
        )>,
        P: Get<FixedU64>,
    > OnRuntimeUpgrade for DAppStakingV3InitConfig<T, G, P>
{
    fn on_runtime_upgrade() -> Weight {
        if Pallet::<T>::on_chain_storage_version() >= STORAGE_VERSION {
            return T::DbWeight::get().reads(1);
        }

        // 0. Unwrap arguments
        let (init_era, tier_params, base_tier_config) = G::get();

        // 1. Prepare init active protocol state
        let now = frame_system::Pallet::<T>::block_number();
        let voting_period_length = Pallet::<T>::blocks_per_voting_period();

        let period_number = 1;
        let protocol_state = ProtocolState {
            era: init_era,
            next_era_start: now.saturating_add(voting_period_length),
            period_info: PeriodInfo {
                number: period_number,
                subperiod: Subperiod::Voting,
                next_subperiod_start_era: init_era.saturating_add(1),
            },
            maintenance: true,
        };

        // 2. Prepare init current era info - need to set correct eras
        let init_era_info = EraInfo {
            total_locked: 0,
            unlocking: 0,
            current_stake_amount: StakeAmount {
                voting: 0,
                build_and_earn: 0,
                era: init_era,
                period: period_number,
            },
            next_stake_amount: StakeAmount {
                voting: 0,
                build_and_earn: 0,
                era: init_era.saturating_add(1),
                period: period_number,
            },
        };

        let average_price = P::get();
        let init_tier_config = base_tier_config.calculate_new(average_price, &tier_params);

        // 3. Write necessary items into storage
        ActiveProtocolState::<T>::put(protocol_state);
        StaticTierParams::<T>::put(tier_params);
        TierConfig::<T>::put(init_tier_config);
        STORAGE_VERSION.put::<Pallet<T>>();
        CurrentEraInfo::<T>::put(init_era_info);

        // 4. Emit events to make indexers happy
        Pallet::<T>::deposit_event(Event::<T>::NewEra { era: init_era });
        Pallet::<T>::deposit_event(Event::<T>::NewSubperiod {
            subperiod: Subperiod::Voting,
            number: 1,
        });

        log::info!("dApp Staking v3 storage initialized.");

        T::DbWeight::get().reads_writes(2, 5)
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
        assert_eq!(Pallet::<T>::on_chain_storage_version(), STORAGE_VERSION);
        let protocol_state = ActiveProtocolState::<T>::get();
        assert!(protocol_state.maintenance);

        let number_of_tiers = T::NumberOfTiers::get();

        let tier_params = StaticTierParams::<T>::get();
        assert_eq!(tier_params.reward_portion.len(), number_of_tiers as usize);
        assert!(tier_params.is_valid());

        let tier_config = TierConfig::<T>::get();
        assert_eq!(tier_config.reward_portion.len(), number_of_tiers as usize);
        assert_eq!(tier_config.slots_per_tier.len(), number_of_tiers as usize);
        assert_eq!(tier_config.tier_thresholds.len(), number_of_tiers as usize);

        let current_era_info = CurrentEraInfo::<T>::get();
        assert_eq!(
            current_era_info.current_stake_amount.era,
            protocol_state.era
        );
        assert_eq!(
            current_era_info.next_stake_amount.era,
            protocol_state.era + 1
        );

        Ok(())
    }
}
