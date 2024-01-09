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

/// `OnRuntimeUpgrade` logic used to set & configure init dApp staking v3 storage items.
pub struct DAppStakingV3InitConfig<T, G>(PhantomData<(T, G)>);
impl<
        T: Config,
        G: Get<(
            EraNumber,
            TierParameters<T::NumberOfTiers>,
            TiersConfiguration<T::NumberOfTiers>,
        )>,
    > OnRuntimeUpgrade for DAppStakingV3InitConfig<T, G>
{
    fn on_runtime_upgrade() -> Weight {
        if Pallet::<T>::on_chain_storage_version() >= STORAGE_VERSION {
            return T::DbWeight::get().reads(1);
        }

        // 0. Unwrap arguments
        let (init_era, tier_params, init_tier_config) = G::get();

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

/// Legacy struct type
/// Should be deleted after the migration
#[derive(Encode, Decode, MaxEncodedLen, Copy, Clone, Debug, PartialEq, Eq, TypeInfo)]
struct OldDAppTier {
    #[codec(compact)]
    pub dapp_id: DAppId,
    pub tier_id: Option<TierId>,
}

/// Information about all of the dApps that got into tiers, and tier rewards
#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    RuntimeDebugNoBound,
    PartialEqNoBound,
    EqNoBound,
    CloneNoBound,
    TypeInfo,
)]
#[scale_info(skip_type_params(MD, NT))]
struct OldDAppTierRewards<MD: Get<u32>, NT: Get<u32>> {
    /// DApps and their corresponding tiers (or `None` if they have been claimed in the meantime)
    pub dapps: BoundedVec<OldDAppTier, MD>,
    /// Rewards for each tier. First entry refers to the first tier, and so on.
    pub rewards: BoundedVec<Balance, NT>,
    /// Period during which this struct was created.
    #[codec(compact)]
    pub period: PeriodNumber,
}

impl<MD: Get<u32>, NT: Get<u32>> Default for OldDAppTierRewards<MD, NT> {
    fn default() -> Self {
        Self {
            dapps: BoundedVec::default(),
            rewards: BoundedVec::default(),
            period: 0,
        }
    }
}

// Legacy convenience type for `DAppTierRewards` usage.
type OldDAppTierRewardsFor<T> =
    OldDAppTierRewards<<T as Config>::MaxNumberOfContracts, <T as Config>::NumberOfTiers>;

/// `OnRuntimeUpgrade` logic used to migrate DApp tiers storage item to BTreeMap.
pub struct DappStakingV3TierRewardAsTree<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for DappStakingV3TierRewardAsTree<T> {
    fn on_runtime_upgrade() -> Weight {
        let mut counter = 0;
        let mut translate = |pre: OldDAppTierRewardsFor<T>| -> DAppTierRewardsFor<T> {
            let mut dapps_tree = BTreeMap::new();
            for dapp_tier in &pre.dapps {
                if let Some(tier_id) = dapp_tier.tier_id {
                    dapps_tree.insert(dapp_tier.dapp_id, tier_id);
                }
            }

            let result = DAppTierRewardsFor::<T>::new(dapps_tree, pre.rewards.to_vec(), pre.period);
            if result.is_err() {
                // Tests should ensure this never happens...
                log::error!("Failed to migrate dApp tier rewards: {:?}", pre);
            }

            // For weight calculation purposes
            counter.saturating_inc();

            // ...if it does happen, there's not much to do except create an empty map
            result.unwrap_or(
                DAppTierRewardsFor::<T>::new(BTreeMap::new(), pre.rewards.to_vec(), pre.period)
                    .unwrap_or_default(),
            )
        };

        DAppTiers::<T>::translate(|_key, value: OldDAppTierRewardsFor<T>| Some(translate(value)));

        T::DbWeight::get().reads_writes(counter, counter)
    }
}
