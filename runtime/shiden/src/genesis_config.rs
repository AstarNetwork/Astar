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

use crate::*;
use astar_primitives::{
    dapp_staking::FIXED_TIER_SLOTS_ARGS, evm::EVM_REVERT_CODE, genesis::GenesisAccount,
    parachain::SHIDEN_ID,
};

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &sp_genesis_builder::PresetId) -> Option<Vec<u8>> {
    let genesis = match id.as_str() {
        "development" => default_config(SHIDEN_ID),
        _ => return None,
    };
    Some(
        serde_json::to_string(&genesis)
            .expect("serialization to json is expected to work. qed.")
            .into_bytes(),
    )
}

/// Get the default genesis config for the Shiden runtime.
pub fn default_config(para_id: u32) -> serde_json::Value {
    let alice = GenesisAccount::<sr25519::Public>::from_seed("Alice");
    let bob = GenesisAccount::<sr25519::Public>::from_seed("Bob");

    let balances: Vec<(AccountId, Balance)> = vec![
        (alice.account_id(), 1_000_000_000_000 * SDN),
        (bob.account_id(), 1_000_000_000_000 * SDN),
        (
            TreasuryPalletId::get().into_account_truncating(),
            1_000_000_000 * SDN,
        ),
    ];

    let slots_per_tier = vec![0, 6, 10, 0];
    let tier_rank_multipliers: Vec<u32> = vec![0, 24_000, 46_700, 0];

    let authorities = vec![&alice, &bob];

    let config = RuntimeGenesisConfig {
        system: Default::default(),
        sudo: SudoConfig {
            key: Some(alice.account_id()),
        },
        parachain_info: ParachainInfoConfig {
            parachain_id: para_id.into(),
            ..Default::default()
        },
        balances: BalancesConfig {
            balances,
            ..Default::default()
        },
        vesting: VestingConfig { vesting: vec![] },
        session: SessionConfig {
            keys: authorities
                .iter()
                .map(|x| {
                    (
                        x.account_id(),
                        x.account_id(),
                        SessionKeys {
                            aura: x.pub_key().into(),
                        },
                    )
                })
                .collect::<Vec<_>>(),
            ..Default::default()
        },
        aura: AuraConfig {
            authorities: vec![],
        },
        aura_ext: Default::default(),
        collator_selection: CollatorSelectionConfig {
            desired_candidates: 32,
            candidacy_bond: 32_000 * SDN,
            invulnerables: authorities
                .iter()
                .map(|x| x.account_id())
                .collect::<Vec<_>>(),
        },
        evm: EVMConfig {
            // We need _some_ code inserted at the precompile address so that
            // the evm will actually call the address.
            accounts: Precompiles::used_addresses_h160()
                .map(|addr| {
                    (
                        addr,
                        fp_evm::GenesisAccount {
                            nonce: Default::default(),
                            balance: Default::default(),
                            storage: Default::default(),
                            code: EVM_REVERT_CODE.into(),
                        },
                    )
                })
                .collect(),
            ..Default::default()
        },
        ethereum: Default::default(),
        polkadot_xcm: Default::default(),
        assets: Default::default(),
        parachain_system: Default::default(),
        transaction_payment: Default::default(),
        dapp_staking: DappStakingConfig {
            reward_portion: vec![
                Permill::from_percent(0),
                Permill::from_percent(70),
                Permill::from_percent(30),
                Permill::from_percent(0),
            ],
            slot_distribution: vec![
                Permill::from_percent(0),
                Permill::from_parts(375_000), // 37.5%
                Permill::from_parts(625_000), // 62.5%
                Permill::from_percent(0),
            ],
            // percentages below are calculated based on a total issuance at the time when dApp staking v3 was revamped (8.6B)
            tier_thresholds: vec![
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
            ],
            slots_per_tier,
            slot_number_args: FIXED_TIER_SLOTS_ARGS,
            safeguard: Some(false),
            tier_rank_multipliers,
            ..Default::default()
        },
        inflation: Default::default(),
        oracle_membership: OracleMembershipConfig {
            members: vec![alice.account_id(), bob.account_id()]
                .try_into()
                .expect("Assumption is that at least two members will be allowed."),
            ..Default::default()
        },
        price_aggregator: PriceAggregatorConfig {
            circular_buffer: vec![CurrencyAmount::from_rational(5, 10)]
                .try_into()
                .expect("Must work since buffer should have at least a single value."),
        },
    };

    serde_json::to_value(&config).expect("Could not build genesis config.")
}
