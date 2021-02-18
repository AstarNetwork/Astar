//! Chain specification.

use cumulus_primitives_core::ParaId;
use plasm_primitives::{AccountId, Balance, CurrencyId, Signature, TokenSymbol};
use plasm_runtime::constants::currency::PLM;
use plasm_runtime::{
    BalancesConfig, ContractsConfig, GenesisConfig, ParachainInfoConfig, SudoConfig, SystemConfig,
    TokensConfig, WASM_BINARY,
};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{crypto::Ss58Codec, sr25519, Pair, Public};

use sp_runtime::traits::{IdentifyAccount, Verify};

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
    pub fn try_get(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Option<&Self> {
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

/// Gen chain specification for given parachain id
pub fn get_chain_spec(id: ParaId) -> ChainSpec {
    if id == ParaId::from(5000) {
        return plasm_chain_spec();
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

fn plasm_chain_spec() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/plasm_parachain.json")[..]).unwrap()
}

/*
fn plasm_chain_spec() -> ChainSpec {
    let para_id: u32 = 5000;
    let sudo_key = AccountId::from_ss58check("5GvHmdxMzYLrWCVLeEcGy6YwDxSS47dsTDRGhMvhthJAfMWf")
        .unwrap();

    ChainSpec::from_genesis(
        "Plasm PC3",
        "plasm-parachain",
        ChainType::Live,
        move || {
            make_genesis(
                crate::balances::HOLDERS.clone(),
                sudo_key.clone(),
                para_id.into(),
                false,
            )
        },
        vec![],
        None,
        None,
        None,
        Extensions {
            relay_chain: "rococo".into(),
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
    const ENDOWMENT: Balance = 1_000_000_000 * PLM;

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

    make_genesis(endowed_accounts, sudo_key, para_id, true)
}

/// Helper function to create GenesisConfig
fn make_genesis(
    balances: Vec<(AccountId, Balance)>,
    root_key: AccountId,
    parachain_id: ParaId,
    enable_println: bool,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        orml_tokens: Some(TokensConfig {
            endowed_accounts: balances
                .iter()
                .cloned()
                .map(|(a, b)| (a, CurrencyId::Token(TokenSymbol::PLM), b))
                .collect(),
        }),
        pallet_balances: Some(BalancesConfig { balances }),
        pallet_contracts: Some(ContractsConfig {
            current_schedule: pallet_contracts::Schedule {
                enable_println, // this should only be enabled on development chains
                ..Default::default()
            },
        }),
        pallet_sudo: Some(SudoConfig { key: root_key }),
        parachain_info: Some(ParachainInfoConfig { parachain_id }),
    }
}
