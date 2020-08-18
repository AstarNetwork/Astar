//! Chain specification.

use plasm_primitives::{AccountId, Balance, Signature, Block};
use plasm_runtime::{
    BalancesConfig, ContractsConfig, GenesisConfig, IndicesConfig,
    SessionKeys, SudoConfig, SystemConfig, ParachainInfoConfig, WASM_BINARY,
    constants::currency,
};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use cumulus_primitives::ParaId;
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

type AccountPublic = <Signature as Verify>::Signer;

use hex_literal::hex;
use sp_core::crypto::{Ss58Codec, UncheckedInto};
use plasm_runtime::constants::currency::*;
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

const PLASM_PROPERTIES: &str = r#"
        {
            "ss58Format": 5,
            "tokenDecimals": 15,
            "tokenSymbol": "PLM"
        }"#;
const PLASM_PROTOCOL_ID: &str = "plm";

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
        pallet_balances: Some(BalancesConfig { balances }),
        pallet_indices: Some(IndicesConfig { indices: vec![] }),
        pallet_contracts: Some(ContractsConfig {
            current_schedule: pallet_contracts::Schedule {
                enable_println, // this should only be enabled on development chains
                ..Default::default()
            },
        }),
        parachain_info: Some(ParachainInfoConfig { parachain_id }),
        pallet_sudo: Some(SudoConfig { key: root_key }),
    }
}

fn parachain_testnet_genesis(
    endowed_accounts: Option<Vec<(AccountId, Balance)>>,
    sudo_key: AccountId,
    parachain_id: ParaId,
) -> GenesisConfig {
    const ENDOWMENT: Balance = 1_000_000_000_000_000_000;

    let endowed_accounts: Vec<(AccountId, Balance)> = endowed_accounts
        .unwrap_or_else(|| {
            vec![
                (get_account_id_from_seed::<sr25519::Public>("Alice"), ENDOWMENT),
                (get_account_id_from_seed::<sr25519::Public>("Bob"), ENDOWMENT),
                (get_account_id_from_seed::<sr25519::Public>("Charlie"), ENDOWMENT),
                (get_account_id_from_seed::<sr25519::Public>("Dave"), ENDOWMENT),
                (get_account_id_from_seed::<sr25519::Public>("Eve"), ENDOWMENT),
                (get_account_id_from_seed::<sr25519::Public>("Ferdie"), ENDOWMENT),
            ]
        });

    make_genesis(endowed_accounts, sudo_key, parachain_id, true)
}

/// Parachain testnet native config.
pub fn parachain_testnet_config() -> ChainSpec {
    // akru
    let sudo_key: AccountId =
        hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].into();

    let mut balances = currency::HOLDERS.clone();
    balances.extend(vec![(sudo_key.clone(), 50_000 * currency::PLM)]);

    let id: ParaId = 100.into();

    ChainSpec::from_genesis(
        "Plasm Test Parachain",
        "plasm_test_parachain",
        ChainType::Live,
        move || parachain_testnet_genesis(Some(balances.clone()), sudo_key.clone(), id),
        vec![],
        Some(sc_telemetry::TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(),0)]).unwrap()),
        Some(PLASM_PROTOCOL_ID),
        serde_json::from_str(PLASM_PROPERTIES).unwrap(),
		Extensions {
			relay_chain: "rococo_local_testnet".into(),
			para_id: id.into(),
		},
    )
}
