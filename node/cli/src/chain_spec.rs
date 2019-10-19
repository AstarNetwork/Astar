use babe_primitives::AuthorityId as BabeId;
use grandpa_primitives::AuthorityId as GrandpaId;
use hex_literal::hex;
use im_online::sr25519::AuthorityId as ImOnlineId;
pub use plasm_primitives::{AccountId, Balance};
use plasm_runtime::constants::{currency::*, time::*};
pub use plasm_runtime::GenesisConfig;
use plasm_runtime::{
    BabeConfig, BalancesConfig, ContractsConfig, GrandpaConfig, IndicesConfig, SudoConfig,
    SystemConfig, WASM_BINARY,
};
use primitives::{crypto::UncheckedInto, Pair, Public};
use sr_primitives::Perbill;
use std::collections::HashSet;
use substrate_service;
use substrate_telemetry::TelemetryEndpoints;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const PLASM_PROPERTIES: &str = r#"
		{
			"tokenDecimals": 15,
			"tokenSymbol": "PLM"
		}"#;
const PLASM_PROTOCOL_ID: &str = "plm";

/// Specialized `ChainSpec`.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

/// Flaming Fir testnet generator
pub fn flaming_fir_config() -> Result<ChainSpec, String> {
    ChainSpec::from_json_bytes(&include_bytes!("../res/flaming-fir.json")[..])
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(
    seed: &str,
) -> (AccountId, AccountId, GrandpaId, BabeId, ImOnlineId) {
    (
        get_from_seed::<AccountId>(&format!("{}//stash", seed)),
        get_from_seed::<AccountId>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<ImOnlineId>(seed),
    )
}

/// Helper function to create GenesisConfig
fn generate_config_genesis(
    initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId)>,
    root_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
    enable_println: bool,
) -> GenesisConfig {
    let default_endowed_accounts: HashSet<AccountId> = vec![
        get_from_seed::<AccountId>("Alice"),
        get_from_seed::<AccountId>("Bob"),
        get_from_seed::<AccountId>("Charlie"),
        get_from_seed::<AccountId>("Dave"),
        get_from_seed::<AccountId>("Eve"),
        get_from_seed::<AccountId>("Ferdie"),
        get_from_seed::<AccountId>("Alice//stash"),
        get_from_seed::<AccountId>("Bob//stash"),
        get_from_seed::<AccountId>("Charlie//stash"),
        get_from_seed::<AccountId>("Dave//stash"),
        get_from_seed::<AccountId>("Eve//stash"),
        get_from_seed::<AccountId>("Ferdie//stash"),
    ]
    .iter()
    .cloned()
    .chain(initial_authorities.iter().map(|x| x.1.clone()))
    .chain(initial_authorities.iter().map(|x| x.0.clone()))
    .into_iter()
    .collect();
    let endowed_accounts: Vec<AccountId> =
        endowed_accounts.unwrap_or_else(|| default_endowed_accounts.into_iter().collect());

    const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
    const STASH: Balance = 100 * DOLLARS;

    GenesisConfig {
        system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        balances: Some(BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, ENDOWMENT))
                .collect(),
            vesting: vec![],
        }),
        indices: Some(IndicesConfig {
            ids: endowed_accounts.iter().cloned().collect::<Vec<_>>(),
        }),
        contracts: Some(ContractsConfig {
            current_schedule: contracts::Schedule {
                enable_println, // this should only be enabled on development chains
                ..Default::default()
            },
            gas_price: 1 * MILLICENTS,
        }),
        sudo: Some(SudoConfig { key: root_key }),
        babe: Some(BabeConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.3.clone(), 1))
                .collect(),
        }),
        grandpa: Some(GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.2.clone(), 1))
                .collect(),
        }),
    }
}

/// Staging testnet config.
pub fn staging_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
		"/ip4/3.114.90.94/tcp/30333/p2p/QmTVUeF2fTe9m8MXw6EGihbvPzDpRm5sJbFGsj6WuagvJu".to_string(),
		"/ip4/3.114.81.104/tcp/30333/p2p/QmQU5Ac75U9d9hG9cWzjANxLK9k35mGnZRhFdGW9X7YLbN".to_string(),
	];
	let properties = serde_json::from_str(PLASM_PROPERTIES).unwrap();
    ChainSpec::from_genesis(
        "Miniplasm",
        "miniplasm",
        staging_testnet_genesis,
        boot_nodes,
        Some(TelemetryEndpoints::new(vec![(
            STAGING_TELEMETRY_URL.to_string(),
            0,
        )])),
		Some(PLASM_PROTOCOL_ID),
		None,
		properties,
    )
}

fn development_config_genesis() -> GenesisConfig {
    generate_config_genesis(
        vec![get_authority_keys_from_seed("Alice")],
        get_from_seed::<AccountId>("Alice"),
        None,
        true,
    )
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Development",
        "dev",
        development_config_genesis,
        vec![],
        None,
        None,
        None,
        None,
    )
}

fn staging_testnet_genesis() -> GenesisConfig {
    generate_config_genesis(
        vec![
            (
                // 5DwNtWotLKncBq1EYJSd74s4tf5fH8McK5XMoG8AyvwdFor6
                hex!["52e1e582076b036e8feb9104b18619795cceccc00fb80c18dd8df5a5c4ea1d52"]
                    .unchecked_into(),
                // 5ELomezsSJhtedP3cFD4zqNDVvvwmdp6PpywACWEq1UP3fgq
                hex!["64c04cdc3237ff84dc94b294d66aff7c370c0cd2648fab05330368ef905cfa5a"]
                    .unchecked_into(),
                // 5GdhABzAQBQYdEFw31veGtjoHbtSUUxTkte53ZUGuUDu73Ra
                hex!["ca19aecbb6f621eb9aea26914916a73135df6766e146b993803065474abed3fa"]
                    .unchecked_into(),
                // 5ELomezsSJhtedP3cFD4zqNDVvvwmdp6PpywACWEq1UP3fgq
                hex!["64c04cdc3237ff84dc94b294d66aff7c370c0cd2648fab05330368ef905cfa5a"]
                    .unchecked_into(),
                // 5ELomezsSJhtedP3cFD4zqNDVvvwmdp6PpywACWEq1UP3fgq
                hex!["64c04cdc3237ff84dc94b294d66aff7c370c0cd2648fab05330368ef905cfa5a"]
                    .unchecked_into(),
            ),
            (
                // 5EvzqUdvcifpT9oevGXy3DRqLB9CL6ptWdpAQtNt1hAGJMP1
                hex!["7ed3e52399b93e05072d83bcbc18d80ff59da1439352057fb61eaf541e8f1c39"]
                    .unchecked_into(),
                // 5GLQu9iyhRHDAHgCd8yFDD3dqFkxn4z8uwEwK8YyYa2GBTUu
                hex!["bcebc6faab0765ca020f33182410156517bc88994d1210a8a026bdc5d201ee7b"]
                    .unchecked_into(),
                // 5HJWD9xdcPvXW2ajEUJEgAXbP4DBGfjDxRh3Nq8PAvnZM8AP
                hex!["e7b365779d16bf9e51164f63d5b1ff986ba58420636d007576549f0da03547ae"]
                    .unchecked_into(),
                // 5GLQu9iyhRHDAHgCd8yFDD3dqFkxn4z8uwEwK8YyYa2GBTUu
                hex!["bcebc6faab0765ca020f33182410156517bc88994d1210a8a026bdc5d201ee7b"]
                    .unchecked_into(),
                // 5GLQu9iyhRHDAHgCd8yFDD3dqFkxn4z8uwEwK8YyYa2GBTUu
                hex!["bcebc6faab0765ca020f33182410156517bc88994d1210a8a026bdc5d201ee7b"]
                    .unchecked_into(),
            ),
        ],
        // 5ELomezsSJhtedP3cFD4zqNDVvvwmdp6PpywACWEq1UP3fgq
        hex!["64c04cdc3237ff84dc94b294d66aff7c370c0cd2648fab05330368ef905cfa5a"].unchecked_into(),
        None,
        false,
    )
}

fn local_testnet_genesis() -> GenesisConfig {
    generate_config_genesis(
        vec![
            get_authority_keys_from_seed("Alice"),
            get_authority_keys_from_seed("Bob"),
        ],
        get_from_seed::<AccountId>("Alice"),
        None,
        false,
    )
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        local_testnet_genesis,
        vec![],
        None,
        None,
        None,
        None,
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::service::{new_full, new_light};
    use service_test;

    fn local_testnet_genesis_instant_single() -> GenesisConfig {
        testnet_genesis(
            vec![get_authority_keys_from_seed("Alice")],
            get_from_seed::<AccountId>("Alice"),
            None,
            false,
        )
    }

    /// Local testnet config (single validator - Alice)
    pub fn integration_test_config_with_single_authority() -> ChainSpec {
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            local_testnet_genesis_instant_single,
            vec![],
            None,
            None,
            None,
            None,
        )
    }

    /// Local testnet config (multivalidator Alice + Bob)
    pub fn integration_test_config_with_two_authorities() -> ChainSpec {
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            local_testnet_genesis,
            vec![],
            None,
            None,
            None,
            None,
        )
    }

    #[test]
    #[ignore]
    fn test_connectivity() {
        service_test::connectivity(
            integration_test_config_with_two_authorities(),
            |config| new_full(config),
            |config| new_light(config),
        );
    }
}
