//! Chain specification.

use plasm_primitives::{AccountId, Balance, Signature};
use plasm_runtime::constants::currency::*;
use plasm_runtime::Block;
use plasm_runtime::{
    BabeConfig, BalancesConfig, ContractsConfig, GenesisConfig, GrandpaConfig, IndicesConfig,
    PlasmRewardsConfig, PlasmValidatorConfig, SessionConfig, SessionKeys, SudoConfig, SystemConfig,
    PlasmLockdropConfig, WASM_BINARY,
};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_runtime::Perbill;

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

    let endowed_accounts: Vec<(AccountId, Balance)> = endowed_accounts
        .unwrap_or_else(|| {
            vec![
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_account_id_from_seed::<sr25519::Public>("Bob"),
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

    make_genesis(initial_authorities, endowed_accounts, sudo_key, true)
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
        // pallet_dapps_staking: Some(DappsStakingConfig {
        //     ..Default::default()
        // }),
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
        pallet_plasm_lockdrop: Some(PlasmLockdropConfig {
            // Alpha2: 0.44698108660714747
            alpha: Perbill::from_parts(446_981_087),
            // Price in dollars: BTC $11000, ETH $400
            dollar_rate: (11_000, 400),
            vote_threshold: 1,
            positive_votes: 1,
            // Start from launch for testing purposes
            lockdrop_bounds: (0, 1_000),
            keys: vec![],
        }),
        pallet_sudo: Some(SudoConfig { key: root_key }),
    }
}

/// Plasm testnet file config.
pub fn dusty_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/dusty.json")[..]).unwrap()
}

/// Plasm mainnet file config.
pub fn plasm_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/plasm.json")[..]).unwrap()
}

/*
/// Mainnet native config.
pub fn plasm_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Plasm",
        "plasm",
        ChainType::Live,
        plasm_genesis,
        vec![],
        Some(sc_telemetry::TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(),0)]).unwrap()),
        Some(PLASM_PROTOCOL_ID),
        serde_json::from_str(PLASM_PROPERTIES).unwrap(),
        Default::default(),
    )
}

fn plasm_genesis() -> GenesisConfig {
    // Plasm initial authorities
    let authorities = vec![
        (   // akru
            hex!["34141c0c21335e3d0ee1a036793cd329a1a24abd617fc37ce2382c340be96a56"].into(),
            hex!["e0c4aae64015c19224b8a054a072a375168ccde72d3d960e8b06f2bb30167d4a"].unchecked_into(),
            hex!["9cc6e9120f5fae0ec6d2b1d6ca8a14bed7a5055a66daf8e64e41cb2700678584"].unchecked_into(),
        ),
        (   // staketech-01
            hex!["84fb8020ed0b8e4ca4b574b9480ff2f4d37a0b46ce46e65d05468f9d65150d21"].into(),
            hex!["fe6d0ed26feab814e4c844f639dd7b5c9c1da84f130bf047e4a37d9b57c5a214"].unchecked_into(),
            hex!["f5c5a9d0a9d19f9ee41a8e442758674294035cde4703c4ace5d4f2683ca2243f"].unchecked_into(),
        ),
        (   // staketech-02
            hex!["8e067f3e41cdd90c11ac2f7f3b1a70ee511867fa4e7dfd85f08ff16c3245ad01"].into(),
            hex!["6c35e8a3eb4839ea8b7438ae59d7d49fe43529943b2812ea5f704d6f9cee640e"].unchecked_into(),
            hex!["7a6a1d203f0ee6112b108faa17808f88b89b5f3fdfea9e4434ae51d28a81508f"].unchecked_into(),
        ),
        (   // staketech-03
            hex!["32b0c306a3f85902e504ed971ca0323f42c3fd209cb275aaabcc22f1c054da79"].into(),
            hex!["f617e63ea7f69f5d83e3718b30db7a5b1d41abb24835a92053bc8bcd252c861c"].unchecked_into(),
            hex!["8f6d7375f702f327b3779ef8ba567530764ea9f71dc638dafbadd29786640eec"].unchecked_into(),
        ),
        (   // staketech-04
            hex!["1e4f5e33dfb4fc38e3b2c3bf91eae6c5443095627d1e0a8595354fcbb2163a2c"].into(),
            hex!["80a9e57aace4b42c158ab336360bca4b8373ae049b58357c04df29a37b564f35"].unchecked_into(),
            hex!["139e66014a330d35cbd662905e9e4ad4fb8d6ef0d3681d95f53cbf9c0abb7967"].unchecked_into(),
        ),
    ];

    // Stake Technologies
    let root_key = hex!["4217f22e9a29af49fd087008d593d07b73d628867f95402885c0651da2c8a432"];

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
