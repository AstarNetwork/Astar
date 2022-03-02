//! Astar chain specifications.

use astar_runtime::{
    wasm_binary_unwrap, AccountId, AuraId, Balance, BaseFeeConfig, EVMConfig, ParachainInfoConfig,
    Precompiles, Signature, SystemConfig, ASTR,
};
use cumulus_primitives_core::ParaId;
use sc_service::ChainType;
use sp_core::{crypto::Ss58Codec, sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

use super::{get_from_seed, Extensions};

/// Specialized `ChainSpec` for Astar Network.
pub type AstarChainSpec = sc_service::GenericChainSpec<astar_runtime::GenesisConfig, Extensions>;

/// Gen Astar chain specification for given parachain id.
pub fn get_chain_spec(para_id: u32) -> AstarChainSpec {
    // Alice as default
    let sudo_key = get_account_id_from_seed::<sr25519::Public>("Alice");
    let endowned = vec![
        (sudo_key.clone(), 100 * ASTR),
        (
            // Lockdrop
            AccountId::from_ss58check("5HEEAH8enKBb62mERpWY6cymE5pXjqp9vgptTTgj4wcMCunk").unwrap(),
            2_100_000_000 * ASTR,
        ),
        (
            // Parachain Auction
            AccountId::from_ss58check("5GEXA7G1idwiEDDNJMdpP27Vz3BRw82gziZB1GrPttaxXTGW").unwrap(),
            1_050_000_000 * ASTR,
        ),
        (
            // Parachain Auction Bonus
            AccountId::from_ss58check("5DveWyztFMF2bgqT8ZXNfALhuw6k3gqvA4eoUunQwm2Z2RmE").unwrap(),
            350_000_000 * ASTR,
        ),
        (
            // Parachain Reserve
            AccountId::from_ss58check("5C5CDyRQzCVvFjWx1FYzN1HY25CvoKH6CbzyBLVuUDzi8ESX").unwrap(),
            350_000_000 * ASTR,
        ),
        (
            // Protocol Development
            AccountId::from_ss58check("5F48VxQChQmMpCchszYHyhtARpBXXZ2y1zC4mNYsefMkc1fw").unwrap(),
            700_000_000 * ASTR,
        ),
        (
            // On Chain DAO
            AccountId::from_ss58check("5DyanbfmERU3X5EAEtVq9H12XDsbXLupPn47e4y2LjhDdwjL").unwrap(),
            350_000_000 * ASTR,
        ),
        (
            // Marketing
            AccountId::from_ss58check("5EL7GdB9Woz8oBL6NP4AvVWguDgi923UjLMGcdPusnNXFA7r").unwrap(),
            350_000_000 * ASTR,
        ),
        (
            // Institutional Investors
            AccountId::from_ss58check("5HdeVKsojzVAEaAaCscu7NiWG4X4aG88dVn67VgA5LG7macZ").unwrap(),
            700_000_000 * ASTR,
        ),
        (
            // Team
            AccountId::from_ss58check("5G3wWrPihXTbrk7pDDjeoqZz2Hoy823WB4dd4vLzZqmL5F3x").unwrap(),
            350_000_000 * ASTR,
        ),
        (
            // Foundation
            AccountId::from_ss58check("5EfMfGRp6vr8JLknyvx77nmyRfZSrFTBzyJQ856Gn2iuoFZc").unwrap(),
            700_000_000 * ASTR,
        ),
    ];

    AstarChainSpec::from_genesis(
        "Astar Testnet",
        "astar",
        ChainType::Development,
        move || make_genesis(endowned.clone(), sudo_key.clone(), para_id.into()),
        vec![],
        None,
        None,
        None,
        None,
        Extensions {
            bad_blocks: Default::default(),
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
        },
        sudo: astar_runtime::SudoConfig {
            key: Some(root_key),
        },
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
            accounts: Precompiles::used_addresses()
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
        base_fee: BaseFeeConfig::new(
            sp_core::U256::from(1_000_000_000),
            false,
            sp_runtime::Permill::from_parts(125_000),
        ),
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
