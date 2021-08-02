//! Chain specifications.

use cumulus_primitives_core::ParaId;
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use shibuya_runtime::{
    wasm_binary_unwrap, AccountId, AuraConfig, AuraId, Balance, BalancesConfig, GenesisConfig,
    ImOnlineConfig, ImOnlineId, ParachainInfoConfig, SessionConfig, SessionKeys, Signature,
    StakerStatus, StakingConfig, SudoConfig, SystemConfig, VestingConfig, SDN,
};
use sp_core::{sr25519, Pair, Public};

use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

type AccountPublic = <Signature as Verify>::Signer;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
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

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, AuraId, ImOnlineId) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<AuraId>(seed),
        get_from_seed::<ImOnlineId>(seed),
    )
}

/// Gen chain specification for given parachain id
pub fn get_chain_spec(id: ParaId) -> ChainSpec {
    if id == ParaId::from(1000) {
        return shibuya_chain_spec();
    }

    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                None,
                id,
            )
        },
        vec![],
        None,
        None,
        None,
        Extensions {
            relay_chain: "westend-dev".into(),
            para_id: id.into(),
        },
    )
}

fn shibuya_chain_spec() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/shibuya.json")[..]).unwrap()
}

/*
fn shibuya_chain_spec() -> ChainSpec {
    use sp_core::crypto::Ss58Codec;

    let para_id: u32 = 1000;
    let sudo_key =
        AccountId::from_ss58check("5HbAP8GczDDfGL6K2BvbsDyCyL3qY2GSRrJAPNXSUnd95mRM").unwrap();

    ChainSpec::from_genesis(
        "Shibuya Testnet",
        "shibuya",
        ChainType::Live,
        move || {
            make_genesis(
                crate::balances::SHIDEN_HOLDERS.clone(),
                sudo_key.clone(),
                para_id.into(),
            )
        },
        vec![],
        None,
        None,
        None,
        Extensions {
            relay_chain: "tokyo".into(),
            para_id,
        },
    )
}
*/

fn testnet_genesis(
    sudo_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
    para_id: ParaId,
) -> GenesisConfig {
    const ENDOWMENT: Balance = 1_000_000_000 * SDN;

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

    make_genesis(endowed_accounts, sudo_key, para_id)
}

fn session_keys(aura: AuraId, im_online: ImOnlineId) -> SessionKeys {
    SessionKeys { aura, im_online }
}

/// Helper function to create GenesisConfig
fn make_genesis(
    mut balances: Vec<(AccountId, Balance)>,
    root_key: AccountId,
    parachain_id: ParaId,
) -> GenesisConfig {
    use sp_core::crypto::Ss58Codec;
    let authorities = vec![
        (
            AccountId::from_ss58check("5HbAP8GczDDfGL6K2BvbsDyCyL3qY2GSRrJAPNXSUnd95mRM").unwrap(),
            get_from_seed::<AuraId>("Alice"),
            get_from_seed::<ImOnlineId>("Alice"),
        ),
        (
            AccountId::from_ss58check("5Fhdbkg89StmVoMNrbukWpcD7ZgLc9gBYNyPBDi3zokPWC48").unwrap(),
            get_from_seed::<AuraId>("Bob"),
            get_from_seed::<ImOnlineId>("Bob"),
        ),
    ];
    let stakers = authorities
        .iter()
        .map(|x| {
            (
                x.0.clone(),
                x.0.clone(),
                1_000 * SDN,
                StakerStatus::Validator,
            )
        })
        .collect::<Vec<_>>();
    balances.extend(
        authorities
            .iter()
            .map(|x| (x.0.clone(), 1_000 * SDN))
            .collect::<Vec<_>>(),
    );
    GenesisConfig {
        system: SystemConfig {
            code: wasm_binary_unwrap().to_vec(),
            changes_trie_config: Default::default(),
        },
        sudo: SudoConfig { key: root_key },
        parachain_info: ParachainInfoConfig { parachain_id },
        balances: BalancesConfig { balances },
        vesting: VestingConfig { vesting: vec![] },
        session: SessionConfig {
            keys: authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        session_keys(x.1.clone(), x.2.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        },
        staking: StakingConfig {
            validator_count: authorities.len() as u32,
            minimum_validator_count: authorities.len() as u32,
            invulnerables: authorities.iter().map(|x| x.0.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            stakers,
            ..Default::default()
        },
        aura: AuraConfig {
            authorities: vec![],
        },
        im_online: ImOnlineConfig { keys: vec![] },
        aura_ext: Default::default(),
    }
}
