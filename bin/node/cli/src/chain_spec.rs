//! Chain specification.

use plasm_primitives::{AccountId, Balance, Signature};
use plasm_runtime::constants::currency::*;
use plasm_runtime::Block;
use plasm_runtime::{
    BabeConfig, BalancesConfig, ContractsConfig, DappsStakingConfig, GenesisConfig, GrandpaConfig,
    IndicesConfig, PlasmRewardsConfig, PlasmValidatorConfig, SessionConfig, SessionKeys,
    SudoConfig, SystemConfig, WASM_BINARY,
};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

type AccountPublic = <Signature as Verify>::Signer;

/*
use hex_literal::hex;
use sp_core::crypto::UncheckedInto;
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

const PLASM_PROPERTIES: &str = r#"
        {
            "ss58Format": 5,
            "tokenDecimals": 15,
            "tokenSymbol": "PLM"
        }"#;
const PLASM_PROTOCOL_ID: &str = "plm";

const DUSTY_PROPERTIES: &str = r#"
        {
            "ss58Format": 5,
            "tokenDecimals": 15,
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
    pub fork_blocks: sc_client::ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client::BadBlocks<Block>,
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
pub fn get_authority_keys_from_seed(seed: &str) -> (AccountId, BabeId, GrandpaId) {
    (
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<GrandpaId>(seed),
    )
}

fn session_keys(babe: BabeId, grandpa: GrandpaId) -> SessionKeys {
    SessionKeys { babe, grandpa }
}

fn testnet_genesis(
    initial_authorities: Vec<(AccountId, BabeId, GrandpaId)>,
    endowed_accounts: Option<Vec<AccountId>>,
    sudo_key: AccountId,
) -> GenesisConfig {
    const ENDOWMENT: Balance = 1_000_000_000_000_000_000;

    let endowed_accounts: Vec<(AccountId, Balance)> = endowed_accounts.unwrap_or_else(|| {
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
            get_account_id_from_seed::<sr25519::Public>("Dave"),
            get_account_id_from_seed::<sr25519::Public>("Eve"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie"),
        ]
    }).iter().cloned().map(|acc| (acc, ENDOWMENT)).collect();

    make_genesis(
        initial_authorities,
        endowed_accounts,
        sudo_key,
        true,
    )
}

/// Helper function to create GenesisConfig
fn make_genesis(
    initial_authorities: Vec<(AccountId, BabeId, GrandpaId)>,
    balances: Vec<(AccountId, Balance)>,
    root_key: AccountId,
    enable_println: bool,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_balances: Some(BalancesConfig { balances }),
        pallet_indices: Some(IndicesConfig { indices: vec![] }),
        pallet_plasm_rewards: Some(PlasmRewardsConfig {
            ..Default::default()
        }),
        pallet_plasm_validator: Some(PlasmValidatorConfig {
            validators: initial_authorities
                .iter()
                .map(|x| x.0.clone())
                .collect::<Vec<_>>(),
        }),
        pallet_dapps_staking: Some(DappsStakingConfig {
            ..Default::default()
        }),
        pallet_session: Some(SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        session_keys(x.1.clone(), x.2.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        }),
        pallet_babe: Some(BabeConfig {
            authorities: vec![],
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: vec![],
        }),
        pallet_contracts: Some(ContractsConfig {
            current_schedule: pallet_contracts::Schedule {
                enable_println, // this should only be enabled on development chains
                ..Default::default()
            },
            gas_price: 1 * MILLIPLM,
        }),
        pallet_sudo: Some(SudoConfig { key: root_key }),
    }
}

/// Plasm testnet file config.
pub fn dusty_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/dusty.json")[..]).unwrap()
}

/*
/// Dusty native config.
pub fn dusty_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Dusty",
        "dusty",
		ChainType::Live,
        dusty_genesis,
        vec![],
        Some(sc_telemetry::TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(),0)]).unwrap()),
        Some(DUSTY_PROTOCOL_ID),
        serde_json::from_str(DUSTY_PROPERTIES).unwrap(),
        Default::default(),
    )
}

fn dusty_genesis() -> GenesisConfig {
    // Dusty initial authorities
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
    ];

    // akru 
    let root_key = hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"];

    // token holders
    let holders = HOLDERS.to_vec();
    // quick check
    let total_amount = holders.iter().fold(0, |sum, (_, v)| sum + v);
    assert!(total_amount == 500_000_000 * plasm_runtime::constants::currency::PLM);

    make_genesis(
        authorities,
        HOLDERS.to_vec(),
        root_key.into(),
        false,
    )
}
*/

fn development_config_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![get_authority_keys_from_seed("Alice")],
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
