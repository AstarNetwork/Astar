use plasm_primitives::{AccountId, Balance, Signature};
use grandpa_primitives::AuthorityId as GrandpaId;
use babe_primitives::AuthorityId as BabeId;
use chain_spec::ChainSpecExtension;
use serde::{Serialize, Deserialize};
use plasm_runtime::{
    constants::{currency::*, time::*},
    opaque::SessionKeys,
    GenesisConfig, SystemConfig, SessionConfig, SessionManagerConfig,
    BabeConfig, GrandpaConfig, IndicesConfig, BalancesConfig, ContractsConfig, SudoConfig,
    WASM_BINARY,
};
use primitives::{crypto::UncheckedInto, sr25519, Pair, Public};
use sr_primitives::{traits::{IdentifyAccount, Verify}, Perbill};
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
    pub fork_blocks: client::ForkBlocks<plasm_runtime::opaque::Block>,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = substrate_service::ChainSpec<
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

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(
    seed: &str,
) -> (AccountId, AccountId, GrandpaId, BabeId) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
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
    initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId)>,
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
                .map(|x| x.1.clone())
                .collect()
        }),
        session: Some(SessionConfig {
            keys: initial_authorities.iter().map(|x| {
                (x.0.clone(), session_keys(x.2.clone(), x.3.clone()))
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

pub fn plasm_testnet_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/v0.1.0.json")[..]).unwrap()
}

/*
/// Plasm testnet config.
pub fn plasm_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
        "/ip4/3.114.90.94/tcp/30333/p2p/QmW8EjUZ1f6RZe4YJ6tZAXzqYmjANbfdEYWMMaFgjkw9HN".to_string(),
        "/ip4/3.114.81.104/tcp/30333/p2p/QmTuouKCV9zXLrNRY71PkfggEUVrrzqofZecCfu7pz5Ntt"
            .to_string(),
        "/ip4/3.115.175.152/tcp/30333/p2p/QmbKSyPY95NvJzoxP8q2DNaA9BRHZa5hy1q1pzfUoLhaUn"
            .to_string(),
        "/ip4/54.64.145.3/tcp/30333/p2p/QmS9psuQJceiYQMe6swoheKXrpnyYDjaigrTqv45RWyvCh".to_string(),
    ];
    let properties = serde_json::from_str(PLASM_PROPERTIES).unwrap();
    ChainSpec::from_genesis(
        "PlasmTestnet v1",
        "plasm_testnet_v1",
        staging_testnet_genesis,
        boot_nodes,
        Some(TelemetryEndpoints::new(vec![(
            STAGING_TELEMETRY_URL.to_string(),
            0,
        )])),
        Some(PLASM_PROTOCOL_ID),
        properties,
        Default::default(),
    )
}

fn plasm_testnet_genesis() -> GenesisConfig {
    generate_config_genesis(
        vec![
            (
                // 5DwNtWotLKncBq1EYJSd74s4tf5fH8McK5XMoG8AyvwdFor6
                hex!["52e1e582076b036e8feb9104b18619795cceccc00fb80c18dd8df5a5c4ea1d52"].into(),
                // 5ELomezsSJhtedP3cFD4zqNDVvvwmdp6PpywACWEq1UP3fgq
                hex!["64c04cdc3237ff84dc94b294d66aff7c370c0cd2648fab05330368ef905cfa5a"].into(),
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
                hex!["7ed3e52399b93e05072d83bcbc18d80ff59da1439352057fb61eaf541e8f1c39"].into(),
                // 5GLQu9iyhRHDAHgCd8yFDD3dqFkxn4z8uwEwK8YyYa2GBTUu
                hex!["bcebc6faab0765ca020f33182410156517bc88994d1210a8a026bdc5d201ee7b"].into(),
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
            (
                // 5GQmtbg2xxckjAeAZzDkNK2oJmKcm91p9iSSZAoxbz4GarXN
                hex!["c03f6b4ab6098cf656c0c8f2454165fc00226f5b82e2c353de603c915ed3031e"].into(),
                // 5C5RmBoMGtgShP9p5vGdSfjkv7Mc25tqz9ZzRGN4z6mTgjEx
                hex!["008d11028493788b3d4d6c36fe3790fa9516e9ba5d034796b74a6cea7ae51d2f"].into(),
                // 5FxCnoKpWBgGKqooWvVb3KCdRbfCoMgKRQXLPrKry6vRGTwb
                hex!["abfb9d369944792a2d1db8d3ba2acd5992e092720a68e2bce719920ab6d72b7c"]
                    .unchecked_into(),
                // 5C5RmBoMGtgShP9p5vGdSfjkv7Mc25tqz9ZzRGN4z6mTgjEx
                hex!["008d11028493788b3d4d6c36fe3790fa9516e9ba5d034796b74a6cea7ae51d2f"]
                    .unchecked_into(),
                // 5C5RmBoMGtgShP9p5vGdSfjkv7Mc25tqz9ZzRGN4z6mTgjEx
                hex!["008d11028493788b3d4d6c36fe3790fa9516e9ba5d034796b74a6cea7ae51d2f"]
                    .unchecked_into(),
            ),
            (
                // 5CFJtZZZ2s8LEPTMiGyupwdBK9iCvZmbcaFP7xSDm7SLmwWg
                hex!["0816626ad05d91ac47de56ef3369f3f8db6942a66a1ea1130b7851415088a775"].into(),
                // 5DkRuuy4bTpodMNfnmV2bs6PNbSvCSKiQC6s8YUcBMBcMf7c
                hex!["4a88608ef40d00f043fa10250bd99dcb93f2ea9367264f8723bd4c777011c13f"].into(),
                // 5GhGYkurWP29fDtjGtTBFhR5DBSC8fqATeBmtgKVmPqMVuJp
                hex!["ccd3d1155c23c1febe0102f5d80a342696065f3072cadfa4e3f817618f80aa83"]
                    .unchecked_into(),
                // 5DkRuuy4bTpodMNfnmV2bs6PNbSvCSKiQC6s8YUcBMBcMf7c
                hex!["4a88608ef40d00f043fa10250bd99dcb93f2ea9367264f8723bd4c777011c13f"]
                    .unchecked_into(),
                // 5DkRuuy4bTpodMNfnmV2bs6PNbSvCSKiQC6s8YUcBMBcMf7c
                hex!["4a88608ef40d00f043fa10250bd99dcb93f2ea9367264f8723bd4c777011c13f"]
                    .unchecked_into(),
            ),
        ],
        // 5ELomezsSJhtedP3cFD4zqNDVvvwmdp6PpywACWEq1UP3fgq
        hex!["64c04cdc3237ff84dc94b294d66aff7c370c0cd2648fab05330368ef905cfa5a"].into(),
        None,
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
    use crate::service::{new_full, new_light};

    fn local_testnet_genesis_instant_single() -> GenesisConfig {
        testnet_genesis(
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
            |mut config| {
                // light nodes are unsupported
                config.roles = Roles::FULL;
                new_full(config)
            },
            true,
        );
    }
}
