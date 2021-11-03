//! Astar chain specifications.

use astar_runtime::{
    wasm_binary_unwrap, AccountId, AstarNetworkPrecompiles, AuraId, Balance, EVMConfig,
    ParachainInfoConfig, Signature, SystemConfig, ASTR,
};
use cumulus_primitives_core::ParaId;
use sc_service::ChainType;
use sp_core::{sr25519, Pair, Public};

use sp_runtime::traits::{IdentifyAccount, Verify};

use super::{get_from_seed, Extensions};

/// Specialized `ChainSpec` for Astar Network.
pub type AstarChainSpec = sc_service::GenericChainSpec<astar_runtime::GenesisConfig, Extensions>;

/// Gen Astar chain specification for given parachain id.
pub fn get_chain_spec(para_id: u32) -> AstarChainSpec {
    // Alice as default
    let sudo_key = get_account_id_from_seed::<sr25519::Public>("Alice");
    let endowned = vec![(
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        7_000_000_000 * ASTR,
    )];

    AstarChainSpec::from_genesis(
        "Astar Testnet",
        "astar",
        ChainType::Live,
        move || make_genesis(endowned.clone(), sudo_key.clone(), para_id.into()),
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

fn session_keys(aura: AuraId) -> astar_runtime::SessionKeys {
    astar_runtime::SessionKeys { aura }
}

/// Helper function to create GenesisConfig.
fn make_genesis(
    balances: Vec<(AccountId, Balance)>,
    root_key: AccountId,
    parachain_id: ParaId,
) -> astar_runtime::GenesisConfig {
    let authorities = vec![
        (
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_from_seed::<AuraId>("Alice"),
        ),
        (
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_from_seed::<AuraId>("Bob"),
        ),
    ];

    // This is supposed the be the simplest bytecode to revert without returning any data.
    // We will pre-deploy it under all of our precompiles to ensure they can be called from
    // within contracts.
    // (PUSH1 0x00 PUSH1 0x00 REVERT)
    let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];

    astar_runtime::GenesisConfig {
        system: SystemConfig {
            code: wasm_binary_unwrap().to_vec(),
            changes_trie_config: Default::default(),
        },
        sudo: astar_runtime::SudoConfig { key: root_key },
        parachain_info: ParachainInfoConfig { parachain_id },
        balances: astar_runtime::BalancesConfig { balances },
        vesting: astar_runtime::VestingConfig { vesting: vec![] },
        session: astar_runtime::SessionConfig {
            keys: authorities
                .iter()
                .map(|x| (x.0.clone(), x.0.clone(), session_keys(x.1.clone())))
                .collect::<Vec<_>>(),
        },
        aura: astar_runtime::AuraConfig {
            authorities: vec![],
        },
        aura_ext: Default::default(),
        collator_selection: astar_runtime::CollatorSelectionConfig {
            desired_candidates: 200,
            candidacy_bond: 3_200_000 * ASTR,
            invulnerables: authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
        },
        evm: EVMConfig {
            // We need _some_ code inserted at the precompile address so that
            // the evm will actually call the address.
            accounts: AstarNetworkPrecompiles::<()>::used_addresses()
                .map(|addr| {
                    (
                        addr,
                        pallet_evm::GenesisAccount {
                            nonce: Default::default(),
                            balance: Default::default(),
                            storage: Default::default(),
                            code: revert_bytecode.clone(),
                        },
                    )
                })
                .collect(),
        },
        ethereum: Default::default(),
    }
}

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate an account ID from seed
fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}
