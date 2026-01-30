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
    dapp_staking::MAX_ENCODED_RANK, evm::EVM_REVERT_CODE, genesis::GenesisAccount,
    parachain::SHIBUYA_ID,
};

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &sp_genesis_builder::PresetId) -> Option<Vec<u8>> {
    let genesis = match id.as_str() {
        "development" => default_config(SHIBUYA_ID),
        _ => return None,
    };
    Some(
        serde_json::to_string(&genesis)
            .expect("serialization to json is expected to work. qed.")
            .into_bytes(),
    )
}

/// Get the default genesis config for the Shibuya runtime.
pub fn default_config(para_id: u32) -> serde_json::Value {
    let alice = GenesisAccount::<sr25519::Public>::from_seed("Alice");
    let bob = GenesisAccount::<sr25519::Public>::from_seed("Bob");
    let charlie = GenesisAccount::<sr25519::Public>::from_seed("Charlie");
    let dave = GenesisAccount::<sr25519::Public>::from_seed("Dave");
    let eve = GenesisAccount::<sr25519::Public>::from_seed("Eve");

    let authorities = vec![&alice, &bob];
    let accounts = vec![&alice, &bob, &charlie, &dave, &eve]
        .iter()
        .map(|x| x.account_id())
        .collect::<Vec<_>>();

    let balances = accounts
        .iter()
        .chain(
            vec![
                TreasuryPalletId::get().into_account_truncating(),
                CommunityTreasuryPalletId::get().into_account_truncating(),
            ]
            .iter(),
        )
        .map(|x| (x.clone(), 1_000_000_000 * SBY))
        .collect::<Vec<_>>();

    let slots_per_tier = vec![0, 6, 10, 0];
    let rank_points: Vec<Vec<u8>> = slots_per_tier
        .iter()
        .map(|&slots| (1..=slots.min(MAX_ENCODED_RANK as u16) as u8).collect())
        .collect();

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
            candidacy_bond: 32_000 * SBY,
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
        evm_chain_id: EVMChainIdConfig {
            chain_id: 0x51,
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
                    required_percentage: Perbill::from_parts(11_600_000), // 1.16%
                },
                TierThreshold::FixedPercentage {
                    required_percentage: Perbill::from_parts(5_800_000), // 0.58%
                },
                // Tier 3: unreachable dummy
                TierThreshold::FixedPercentage {
                    required_percentage: Perbill::from_parts(0), // 0%
                },
            ],
            slots_per_tier,
            // Force fixed 16 slots
            slot_number_args: (0, 16),
            safeguard: Some(false),
            rank_points,
            base_reward_portion: Permill::from_percent(10),
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
        council_membership: CouncilMembershipConfig {
            members: accounts
                .clone()
                .try_into()
                .expect("Should support at least 5 members."),
            phantom: Default::default(),
        },
        technical_committee_membership: TechnicalCommitteeMembershipConfig {
            members: accounts[..3]
                .to_vec()
                .try_into()
                .expect("Should support at least 3 members."),
            phantom: Default::default(),
        },
        community_council_membership: CommunityCouncilMembershipConfig {
            members: accounts
                .try_into()
                .expect("Should support at least 5 members."),
            phantom: Default::default(),
        },
        council: Default::default(),
        technical_committee: Default::default(),
        community_council: Default::default(),
        democracy: Default::default(),
        treasury: Default::default(),
        community_treasury: Default::default(),
        safe_mode: Default::default(),
        tx_pause: Default::default(),
    };

    serde_json::to_value(&config).expect("Could not build genesis config.")
}
