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
    evm::EVM_REVERT_CODE,
    genesis::{get_from_seed, GenesisAccount},
};

/// Get the default genesis config for the local runtime.
pub fn default_config() -> serde_json::Value {
    let alice = GenesisAccount::<sr25519::Public>::from_seed("Alice");
    let bob = GenesisAccount::<sr25519::Public>::from_seed("Bob");
    let charlie = GenesisAccount::<sr25519::Public>::from_seed("Charlie");
    let dave = GenesisAccount::<sr25519::Public>::from_seed("Dave");
    let eve = GenesisAccount::<sr25519::Public>::from_seed("Eve");

    let balances: Vec<(AccountId, Balance)> = vec![
        (alice.account_id(), 1_000_000_000 * AST),
        (bob.account_id(), 1_000_000_000 * AST),
        (
            TreasuryPalletId::get().into_account_truncating(),
            1_000_000_000 * AST,
        ),
        (
            CommunityTreasuryPalletId::get().into_account_truncating(),
            1_000_000_000 * AST,
        ),
    ];

    let accounts = vec![&alice, &bob, &charlie, &dave, &eve]
        .iter()
        .map(|x| x.account_id())
        .collect::<Vec<_>>();

    let config = RuntimeGenesisConfig {
        system: Default::default(),
        sudo: SudoConfig {
            key: Some(alice.account_id()),
        },
        balances: BalancesConfig { balances },
        vesting: VestingConfig { vesting: vec![] },
        aura: AuraConfig {
            authorities: vec![get_from_seed::<AuraId>("Alice")],
            ..Default::default()
        },
        grandpa: GrandpaConfig {
            authorities: vec![(get_from_seed::<GrandpaId>("Alice"), 1)],
            ..Default::default()
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
        assets: Default::default(),
        transaction_payment: Default::default(),
        dapp_staking: DappStakingConfig {
            reward_portion: vec![
                Permill::from_percent(40),
                Permill::from_percent(30),
                Permill::from_percent(20),
                Permill::from_percent(10),
            ],
            slot_distribution: vec![
                Permill::from_percent(10),
                Permill::from_percent(20),
                Permill::from_percent(30),
                Permill::from_percent(40),
            ],
            tier_thresholds: vec![
                TierThreshold::DynamicPercentage {
                    percentage: Perbill::from_parts(35_700_000), // 3.57%
                    minimum_required_percentage: Perbill::from_parts(23_800_000), // 2.38%
                },
                TierThreshold::DynamicPercentage {
                    percentage: Perbill::from_parts(8_900_000), // 0.89%
                    minimum_required_percentage: Perbill::from_parts(6_000_000), // 0.6%
                },
                TierThreshold::DynamicPercentage {
                    percentage: Perbill::from_parts(23_800_000), // 2.38%
                    minimum_required_percentage: Perbill::from_parts(17_900_000), // 1.79%
                },
                TierThreshold::FixedPercentage {
                    required_percentage: Perbill::from_parts(600_000), // 0.06%
                },
            ],
            slots_per_tier: vec![10, 20, 30, 40],
            safeguard: Some(false),
            ..Default::default()
        },
        inflation: InflationConfig {
            params: InflationParameters {
                max_inflation_rate: Perquintill::from_percent(7),
                treasury_part: Perquintill::from_percent(5),
                collators_part: Perquintill::from_percent(3),
                dapps_part: Perquintill::from_percent(20),
                base_stakers_part: Perquintill::from_percent(25),
                adjustable_stakers_part: Perquintill::from_percent(35),
                bonus_part: Perquintill::from_percent(12),
                ideal_staking_rate: Perquintill::from_percent(50),
            },
            ..Default::default()
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
    };

    serde_json::to_value(&config).expect("Could not build genesis config.")
}
