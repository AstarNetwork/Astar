//! Chain specification.

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use plasm_primitives::{AccountId, Balance, Signature};
use plasm_runtime::constants::currency::PLM;
use plasm_runtime::Block;
use plasm_runtime::{
    BabeConfig, BalancesConfig, ContractsConfig, EVMConfig, EthereumConfig, GenesisConfig,
    GrandpaConfig, ImOnlineConfig, IndicesConfig, SessionConfig, SessionKeys, StakerStatus,
    StakingConfig, SudoConfig, SystemConfig, VestingConfig, WASM_BINARY,
};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, Pair, Public, H160, U256};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};
type AccountPublic = <Signature as Verify>::Signer;
const STASH: Balance = 1_000_000 * PLM;

/*
use hex_literal::hex;
// use plasm_runtime::constants::currency::*;
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

const PLASM_PROPERTIES: &str = r#"
        {
            "ss58Format": 5,
            "tokenDecimals": 18,
            "tokenSymbol": "PLM"
        }"#;
const PLASM_PROTOCOL_ID: &str = "plm";
const DUSTY_PROPERTIES: &str = r#"
{
    "ss58Format": 5,
    "tokenDecimals": 18,
    "tokenSymbol": "PLD"
}"#;
const DUSTY_PROTOCOL_ID: &str = "pld";
*/


/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: sc_client_api::ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client_api::BadBlocks<Block>,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

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
pub fn get_authority_keys_from_seed(seed: &str) -> (AccountId, BabeId, GrandpaId, ImOnlineId) {
    (
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<ImOnlineId>(seed),
    )
}

fn session_keys(babe: BabeId, grandpa: GrandpaId, im_online: ImOnlineId) -> SessionKeys {
    SessionKeys {
        babe,
        grandpa,
        im_online,
    }
}

fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AccountId)>,
    keys: Vec<(AccountId, BabeId, GrandpaId, ImOnlineId)>,
    endowed_accounts: Option<Vec<AccountId>>,
    sudo_key: AccountId,
) -> GenesisConfig {
    const ENDOWMENT: Balance = 5_000_000 * PLM;

    let endowed_accounts: Vec<(AccountId, Balance)> = endowed_accounts
        .unwrap_or_else(|| {
            vec![
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                get_account_id_from_seed::<sr25519::Public>("Charlie"),
                get_account_id_from_seed::<sr25519::Public>("Dave"),
                get_account_id_from_seed::<sr25519::Public>("Eve"),
                get_account_id_from_seed::<sr25519::Public>("Ferdie"),
            ]
        })
        .iter()
        .cloned()
        .map(|acc| (acc, ENDOWMENT))
        .collect();

    make_genesis(initial_authorities, keys, endowed_accounts, sudo_key, true)
}

/// Helper function to create GenesisConfig
fn make_genesis(
    initial_authorities: Vec<(AccountId, AccountId)>,
    keys: Vec<(AccountId, BabeId, GrandpaId, ImOnlineId)>,
    balances: Vec<(AccountId, Balance)>,
    root_key: AccountId,
    enable_println: bool,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_babe: Some(BabeConfig {
            authorities: vec![],
        }),
        pallet_indices: Some(IndicesConfig { indices: vec![] }),
        pallet_balances: Some(BalancesConfig { balances }),
        pallet_staking: Some(StakingConfig {
            validator_count: initial_authorities.len() as u32,
            minimum_validator_count: initial_authorities.len() as u32,
            stakers: initial_authorities
            .iter()
            .map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
            .collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        }),
        pallet_session: Some(SessionConfig {
            keys: keys
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        session_keys(x.1.clone(), x.2.clone(), x.3.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: vec![],
        }),
        pallet_contracts: Some(ContractsConfig {
            current_schedule: pallet_contracts::Schedule {
                enable_println, // this should only be enabled on development chains
                ..Default::default()
            },
        }),
        pallet_sudo: Some(SudoConfig { key: root_key }),
        pallet_im_online: Some(ImOnlineConfig { keys: vec![] }),
        pallet_vesting: Some(VestingConfig { vesting: vec![] }),
        pallet_evm: Some(EVMConfig {
            accounts: vec![(
                H160::from(hex_literal::hex![
                    "7EF99B0E5bEb8ae42DbF126B40b87410a440a32a"
                ]),
                pallet_evm::GenesisAccount {
                    balance: U256::from(1_000_000_000_000_000_000_000u128),
                    nonce: Default::default(),
                    code: Default::default(),
                    storage: Default::default(),
                },
            )]
            .iter()
            .cloned()
            .collect(),
        }),
        pallet_ethereum: Some(EthereumConfig {}),
    }
}

/// Dusty testnet file config.
pub fn dusty_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/dusty5.json")[..]).unwrap()
}

/*
use sp_core::crypto::{Ss58Codec, UncheckedInto};
/// Dusty native config.
pub fn dusty_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Dusty",
        "dusty5",
        ChainType::Live,
        dusty_genesis,
        vec![],
        Some(
            sc_telemetry::TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
                .unwrap(),
        ),
        Some(DUSTY_PROTOCOL_ID),
        serde_json::from_str(DUSTY_PROPERTIES).unwrap(),
        Default::default(),
    )
}

fn dusty_genesis() -> GenesisConfig {
    let authorities = vec![
        // akru ctrl + stash
        (hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].into(),
        hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].into()),
        
        // Mario ctrl + stash
        // 5Ff4YsfTpFDiJsfTR7UcmJFj2ysCwtxX5ixLN8gjnaahGbhG
        // 5CiTLYjgjkmx5mB85Y1ATCdL1rijUvS3mkFnYakGoHCN2R91
        (hex!["9ee8b420d6705162524d290b0134faab7f38ab6dc57f0c6d538c644e8f693366"].into(),
        hex!["1cca72ef6e569fc7491b6d116fe9414325c52b9ea34f1652a405e10bf020294f"].into()),
        
        // Mario2 ctrl + stash
        // 5FThDusmRcd4enuaBJrCx9QsMccFEkkEUBENdM4xygPPVyrn
        // 5GxQbApcLQuDYDYNhbiv2t6tmyoPTELbwf1ZsHBv9bVYUsXi
        (hex!["963d361a290e31eb661d886a81e9cb794e4dbb0c81cf37723be3c1f1aecba14f"].into(),
        hex!["d85f9175c7ed2cb6e8b1f7eb907c8505856f2fd23f68ca316173ca0a01dd2532"].into()),
    ];

    let keys = vec![
        (
            AccountId::from_ss58check("5Ff4YsfTpFDiJsfTR7UcmJFj2ysCwtxX5ixLN8gjnaahGbhG").unwrap(),
            hex!["46ebddef8cd9bb167dc30878d7113b7e168e6f0646beffd77d69d39bad76b47a"]
                .unchecked_into(),
            hex!["345071da55e5dccefaaa440339415ef9f2663338a38f7da0df21be5ab4e055ef"]
                .unchecked_into(),
            // hex!["46ebddef8cd9bb167dc30878d7113b7e168e6f0646beffd77d69d39bad76b47a"]
            //     .unchecked_into(),
        ),
    ];

    // akru
    let root_key = hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"];

    make_genesis(
        authorities,
        keys.iter()
            .cloned()
            .map(|(a, b, c)| {
                (
                    a.clone(),
                    b,
                    c,
                    get_from_seed::<ImOnlineId>(a.to_string().as_str()),
                )
            })
            .collect(),
        crate::balances::DUSTY_HOLDERS.clone(),
        root_key.into(),
        false,
    )
}
*/

/// Plasm mainnet file config.
pub fn plasm_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/plasm.json")[..]).unwrap()
}

fn development_config_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![(
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
        )],
        vec![
            get_authority_keys_from_seed("Alice"),
            get_authority_keys_from_seed("Bob"),
        ],
        None,
        get_account_id_from_seed::<sr25519::Public>("Alice"),
    )
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        development_config_genesis,
        vec![],
        None,
        None,
        None,
        Default::default(),
    )
}

fn local_testnet_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![
            (
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
            ),
            (
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
            ),
        ],
        vec![
            get_authority_keys_from_seed("Alice"),
            get_authority_keys_from_seed("Bob"),
        ],
        None,
        get_account_id_from_seed::<sr25519::Public>("Alice"),
    )
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        local_testnet_genesis,
        vec![],
        None,
        None,
        None,
        Default::default(),
    )
}
