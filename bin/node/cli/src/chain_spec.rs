///! Plasm chain configuration.

use chain_spec::ChainSpecExtension;
use primitives::{crypto::UncheckedInto, sr25519, Pair, Public};
use serde::{Serialize, Deserialize};
use plasm_primitives::{AccountId, Balance, Signature};
use plasm_runtime::{
    GenesisConfig, SystemConfig, SessionConfig, PlasmStakingConfig,
    BabeConfig, GrandpaConfig, IndicesConfig, BalancesConfig, ContractsConfig, SudoConfig,
    SessionKeys, Forcing, WASM_BINARY,
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
) -> (AccountId, BabeId, GrandpaId) {
    (
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<GrandpaId>(seed),
    )
}

fn session_keys(
    babe: BabeId,
    grandpa: GrandpaId,
) -> SessionKeys {
    SessionKeys { babe, grandpa, }
}

/// Helper function to create GenesisConfig
fn generate_config_genesis(
    initial_authorities: Vec<(AccountId, BabeId, GrandpaId)>,
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

    const ENDOWMENT: Balance = 1_000 * PLM;

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
        plasm_staking: Some(PlasmStakingConfig {
            storage_version: 1,
            force_era: Forcing::NotForcing,
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
    // Testnet authorities list
    let authorities = vec![
        (   // akru
            hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].into(),
            hex!["ac2bbc1877441591e997a7bd8043f4df4f7ca69bd05a762b0661ec376f64f551"].unchecked_into(),
            hex!["0e95fb00ea007cd02b7b0065840d4572aeab5dbf77f148a62330168e7092703d"].unchecked_into(),
        ),
        (   // staketech-01
            hex!["48cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
            hex!["70887b6d5241f2483fd7f199697a2f4ccfe3aedbfa60fe0c82fe476a4b08a320"].unchecked_into(),
            hex!["c62110354d58905bbfa894a1d82f0c175dfc7720758b28d18bc2118ef5f54f91"].unchecked_into(),
        ),
        (   // staketech-02
            hex!["38cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
            hex!["d409311bae981d87dee63d4c799723a33d509d7388db4c530a10e607937e547d"].unchecked_into(),
            hex!["36aaade466263a00ec16a1a1c301636ff8488fc28a08e6a7eca7ac8496e35dca"].unchecked_into(),
        ),
        (   // staketech-03
            hex!["28cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
            hex!["266f53d34490e10e6c818a1f6208dd285a74c01e022cb3b725cf5888bc89136f"].unchecked_into(),
            hex!["c379204b0b450bb62006a0df2b4abac72c79909248fc0f30ce0b05fcb9c102fa"].unchecked_into(),
        ),
        (   // staketech-04
            hex!["18cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
            hex!["96e2554353e7a8de10a388a5dda42096d3c7768403f3735d0a939bc3fd39bc54"].unchecked_into(),
            hex!["674bd4f2670c0e99edcccd5d3821c54b9d559580a31d8e2ca1e88c1e3db28021"].unchecked_into(),
        ),
        (   // Stir
            hex!["ce3e6bb7672586afaf31d1c5d7e73a69d468b461d1334917ff1f23d984f8c525"].into(),
            hex!["da001e43576e62a7d4984eb86fe3a330e83854129caac5a06c5587025d9be302"].unchecked_into(),
            hex!["a4b411edba991630917119135b82c1ee9ff15d30e1ff6f62e637c7527c7478c2"].unchecked_into(),
        ),
        (   // Knigh-star 
            hex!["ba103eb6c8b63de70b8410ec9d498d126234e56d51adfef3efa95fc466308d2c"].into(),
            hex!["ea711db476883a01dd2dc79a60656f66ee16a58ad33f6638c72fd647092d6b15"].unchecked_into(),
            hex!["597f96c1b19c1c2063fb35c8da64fee721ca900481f20d4b45693f517ef29acf"].unchecked_into(),
        ),
        (   // Roy 
            hex!["4219c9547619f8eaed24f507872df5168674c384fcbf4dd96e860bdc1a90b64d"].into(),
            hex!["56ffd328660aa360e9d73680bb93f255866356c8c480466177e8fecf39e0c204"].unchecked_into(),
            hex!["c54fb4015876028619d7572bffece205a0108821257112c60e0e3b779e5ff519"].unchecked_into(),
        ),
        (   // InChainWorks 
            hex!["10270887a7c74e7b858b70edba7d44c2905295d026e445933c094a0e29414236"].into(),
            hex!["f0bc957cf56363494f4bac16434e547a2e651166215ae409fe49cb376dd4c031"].unchecked_into(),
            hex!["9c1ee88efa1f48b1d5cea56df757897ecca77940336b24f0bd75cefaf7a6652f"].unchecked_into(),
        ),
        (   // Moonline 
            hex!["4041e4a5f581bb14f13036a34f6eb26346e67f03f1c1d41e4bfa0b822f60780e"].into(),
            hex!["c0f0af7ed4801cf9d07748cee789234ed1d94a3a49a64a00052287b49152f123"].unchecked_into(),
            hex!["1884916e0ec789a374739fc426b798ec6d76f41269ebe3d742a317ec7feff011"].unchecked_into(),
        ),
        (   // Witval 
            hex!["2ac41c4e82b7b3680a4e86486550557e7274363f413ec363ae03d2b7c11ceeef"].into(),
            hex!["6e1ce430daf4205ecabe491f5c1b2d84cf4b999f73d4f36f303e3e319162a267"].unchecked_into(),
            hex!["2fd38b8a4247fe685f3fb198b617f04b185044a0df07db0bf86f38fb23c1eb70"].unchecked_into(),
        ),
        (   // Spiritual 
            hex!["420a8d0c7c7971bbf1d32f5feefbfe2cb09d15b5c70b8258a880117c281f365f"].into(),
            hex!["6edf07f0743c09fd96e64d613777aec0225eb8cce3211f58b3ec0ced7f9e424a"].unchecked_into(),
            hex!["9ec82b6b22e47ec7dde324cfcca84a4e8785bb8ac03eda89e414c105fe13fdfa"].unchecked_into(),
        ),
        (   // cp287 
            hex!["380477b148049ca59005c2900f51467aaf438f113d5ab061c46e7fb35a145366"].into(),
            hex!["d87d24f1cc66e34b16529838654541e682971e78d4cd13595d25a5ff5f20be54"].unchecked_into(),
            hex!["4694c661d2c7d042c3b6d37305cd7c595b1d4c43f8b44173c304156f10f7a97a"].unchecked_into(),
        ),
    ];

    // Testnet endowements
    let mut endowed_accounts: Vec<AccountId> = authorities.iter().cloned().map(|x| x.0).collect();
    endowed_accounts.extend(vec![
        hex!["10270887a7c74e7b858b70edba7d44c2905295d026e445933c094a0e29414236"].into(),
        hex!["ce3e6bb7672586afaf31d1c5d7e73a69d468b461d1334917ff1f23d984f8c525"].into(),
        hex!["4cd4fbd4a2694d2a51a7012cb5f517096c150980465f2762f7a53dcb8bca0c56"].into(),
        hex!["240826b5b3a2d144086c7de1032c69209d6c381a96d0fa7eafc3d95798080e24"].into(),
        hex!["9450bb2d9c81781d159f6fe6f5be5b95a57be1a245da4315fe9baf3dd239bc06"].into(),
        hex!["2a40ea8a4d6c61e2aa0c5acf7e27f1b9d52014fe1d12e27e89b11fc0173e9277"].into(),
    ]);

    // 5Cakru1BpXPiezeD2LRZh3pJamHcbX9yZ13KLBxuqdTpgnYF
    let root_key = hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"];

    generate_config_genesis(
        authorities,
        root_key.into(),
        Some(endowed_accounts),
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
            |config| new_light(config),
        );
    }
}
