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
const STASH: Balance = 10_000 * PLM;

/*
use hex_literal::hex;
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
use sp_core::crypto::UncheckedInto;

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
pub fn get_authority_keys_from_seed(
    seed: &str,
) -> (AccountId, AccountId, BabeId, GrandpaId, ImOnlineId) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
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
    initial_authorities: Vec<(
        AccountId, // stash
        AccountId, // ctrl
        BabeId,
        GrandpaId,
        ImOnlineId,
    )>,
    // keys: Vec<(AccountId, BabeId, GrandpaId, ImOnlineId)>,
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
    make_genesis(initial_authorities, endowed_accounts, sudo_key, true)
}

/// Helper function to create GenesisConfig
fn make_genesis(
    initial_authorities: Vec<(
        AccountId, // stash
        AccountId, // ctrl
        BabeId,
        GrandpaId,
        ImOnlineId,
    )>,
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
            validator_count: 2,
            minimum_validator_count: 2,
            stakers: initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
                .collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        }),
        pallet_session: Some(SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.1.clone(),
                        x.0.clone(),
                        session_keys(x.2.clone(), x.3.clone(), x.4.clone()),
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
        pallet_treasury: Default::default(),
    }
}

/// Dusty testnet file config.
pub fn dusty_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/dusty.raw.json")[..]).unwrap()
}

/*
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
        // Alice stash + ctrl
        // stash 5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY
        // ctrl 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
        (
            hex!["be5ddb1579b72e84524fc29e78609e3caf42e85aa118ebfe0b0ad404b5bdd25f"].into(),
            hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"].into(),
            hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"]
                .unchecked_into(),
            hex!["88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee"]
                .unchecked_into(),
            hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"]
                .unchecked_into(),
        ),

        // Bob stash + ctrl
        // stash 5HpG9w8EBLe5XCrbczpwq5TSXvedjrBGCwqxK1iQ7qUsSWFc
        // ctrl 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty
        (
            hex!["fe65717dad0447d715f660a0a58411de509b42e6efb8375f562f58a554d5860e"].into(),
            hex!["8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"].into(),
            hex!["8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"]
                .unchecked_into(),
            hex!["d17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69"]
                .unchecked_into(),
            hex!["8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"]
                .unchecked_into(),
        ),
    ];

    // Astar testnet root 5GrootH4UVFfSXJKLf5Rt1PtZ9HFBxGsUqnx7em9saHymCLY
    let root_key = hex!["d41a42362ef23d940e69e76636c9e32a1899812d20fae8d209e335dff1142639"];

    make_genesis(
        authorities,
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
