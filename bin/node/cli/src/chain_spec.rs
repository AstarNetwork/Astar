///! Plasm chain configuration.

use chain_spec::ChainSpecExtension;
use primitives::{crypto::UncheckedInto, sr25519, Pair, Public};
use serde::{Serialize, Deserialize};
use plasm_primitives::{AccountId, Balance, Signature};
use plasm_runtime::{
    GenesisConfig, SystemConfig, SessionConfig, SessionManagerConfig,
    BabeConfig, GrandpaConfig, IndicesConfig, BalancesConfig, ContractsConfig, SudoConfig,
    SessionKeys, WASM_BINARY,
};
use plasm_runtime::constants::currency::*;
use plasm_runtime::Block;
use grandpa_primitives::AuthorityId as GrandpaId;
use babe_primitives::AuthorityId as BabeId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use telemetry::TelemetryEndpoints;
use hex_literal::hex;

type AccountPublic = <Signature as Verify>::Signer;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const PLASM_PROPERTIES: &str = r#"
        {
            "tokenDecimals": 15,
            "tokenSymbol": "PLM"
        }"#;
const PLASM_PROTOCOL_ID: &str = "plm";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: client::ForkBlocks<Block>,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::ChainSpec<
    GenesisConfig,
    Extensions,
>;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
    where
        AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate controller and session key from seed
pub fn get_authority_keys_from_seed(
    seed: &str,
) -> (AccountId, GrandpaId, BabeId) {
    (
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
    )
}

fn session_keys(
    grandpa: GrandpaId,
    babe: BabeId,
) -> SessionKeys {
    SessionKeys { grandpa, babe, }
}

/// Helper function to create GenesisConfig
fn generate_config_genesis(
    initial_authorities: Vec<(AccountId, GrandpaId, BabeId)>,
    root_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
    enable_println: bool,
) -> GenesisConfig {
    let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| vec![ 
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        get_account_id_from_seed::<sr25519::Public>("Bob"),
        get_account_id_from_seed::<sr25519::Public>("Charlie"),
        get_account_id_from_seed::<sr25519::Public>("Dave"),
        get_account_id_from_seed::<sr25519::Public>("Eve"),
        get_account_id_from_seed::<sr25519::Public>("Ferdie"),
    ]);

    const ENDOWMENT: Balance = 10_000_000 * PLM;

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
            ids: endowed_accounts
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>(),
        }),
        session_manager: Some(SessionManagerConfig {
            validators: initial_authorities
                .iter()
                .map(|x| x.0.clone())
                .collect()
        }),
        session: Some(SessionConfig {
            keys: initial_authorities.iter().map(|x| {
                (x.0.clone(), session_keys(x.1.clone(), x.2.clone()))
            }).collect::<Vec<_>>(),
        }),
        babe: Some(BabeConfig {
            authorities: vec![] 
        }),
        grandpa: Some(GrandpaConfig {
            authorities: vec![] 
        }),
        contracts: Some(ContractsConfig {
            current_schedule: contracts::Schedule {
                enable_println, // this should only be enabled on development chains
                ..Default::default()
            },
            gas_price: 1 * MILLIPLM,
        }),
        sudo: Some(SudoConfig { key: root_key }),
    }
}

/// Plasm testnet file config.
pub fn plasm_testnet_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/testnet_v2.json")[..]).unwrap()
}

/*
/// Plasm testnet native config.
pub fn plasm_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
        // akru
        "/ip4/95.216.202.55/tcp/30333/p2p/QmYyTG2eKpREh4J9BvySAkuNJDTnDXJBcJeiY1E5SdSsBv".into(),
        // Stake Technologies
        "/ip4/3.114.90.94/tcp/30333/p2p/QmW8EjUZ1f6RZe4YJ6tZAXzqYmjANbfdEYWMMaFgjkw9HN".into(),
        "/ip4/3.114.81.104/tcp/30333/p2p/QmTuouKCV9zXLrNRY71PkfggEUVrrzqofZecCfu7pz5Ntt".into(),
        "/ip4/3.115.175.152/tcp/30333/p2p/QmbKSyPY95NvJzoxP8q2DNaA9BRHZa5hy1q1pzfUoLhaUn".into(),
        "/ip4/54.64.145.3/tcp/30333/p2p/QmS9psuQJceiYQMe6swoheKXrpnyYDjaigrTqv45RWyvCh".into(),
    ];
    let properties = serde_json::from_str(PLASM_PROPERTIES).unwrap();
    ChainSpec::from_genesis(
        "Plasm Testnet v2",
        "plasm_testnet_v2",
        plasm_testnet_genesis,
        boot_nodes,
        Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(),0)])),
        Some(PLASM_PROTOCOL_ID),
        properties,
        Default::default(),
    )
}

fn plasm_testnet_genesis() -> GenesisConfig {
    let authorities = vec![(
        hex!["58cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
        hex!["daf0535a46d8187446471bf619ea9104bda443366c526bf6f2cd4e9a1fcf5dd7"].unchecked_into(),
        hex!["36cced69f5f1f07856ff0daac944c52e286e10184e52be76ca9377bd0406d90b"].unchecked_into(),
    )];
    // 5Cakru1BpXPiezeD2LRZh3pJamHcbX9yZ13KLBxuqdTpgnYF
    let root_key = hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"];
    generate_config_genesis(
        authorities,
        root_key.clone().into(),
        Some(vec![root_key.into()]),
        false,
    )
}
*/

fn development_config_genesis() -> GenesisConfig {
    generate_config_genesis(
        vec![get_authority_keys_from_seed("Alice")],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
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
        Default::default(),
    )
}

fn local_testnet_genesis() -> GenesisConfig {
    generate_config_genesis(
        vec![
            get_authority_keys_from_seed("Alice"),
            get_authority_keys_from_seed("Bob"),
        ],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
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
        Default::default(),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::service::new_full;
    use sc_service::Roles;
    use service_test;

    fn local_testnet_genesis_instant_single() -> GenesisConfig {
        generate_config_genesis(
            vec![get_authority_keys_from_seed("Alice")],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
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
            Default::default(),
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
            Default::default(),
        )
    }

    #[test]
    #[ignore]
    fn test_connectivity() {
        service_test::connectivity(
            integration_test_config_with_two_authorities(),
            |config| new_full(config),
            |mut config| {
                // light nodes are unsupported
                config.roles = Roles::FULL;
                new_full(config)
            },
            true,
        );
    }
}
