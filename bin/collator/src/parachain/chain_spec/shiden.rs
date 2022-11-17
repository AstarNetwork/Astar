//! Shiden chain specifications.

use cumulus_primitives_core::ParaId;
use sc_service::ChainType;
use shiden_runtime::{
    wasm_binary_unwrap, AccountId, AuraId, Balance, BaseFeeConfig, BlockRewardConfig, EVMConfig,
    ParachainInfoConfig, Precompiles, Signature, SystemConfig, SDN,
};
use sp_core::{sr25519, Pair, Public};

use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

use super::{get_from_seed, Extensions};

const PARA_ID: u32 = 2007;

/// Specialized `ChainSpec` for Shiden Network.
pub type ShidenChainSpec = sc_service::GenericChainSpec<shiden_runtime::GenesisConfig, Extensions>;

/// Gen Shiden chain specification for given parachain id.
pub fn get_chain_spec() -> ShidenChainSpec {
    // Alice as default
    let sudo_key = get_account_id_from_seed::<sr25519::Public>("Alice");
    let endowned = vec![
        (
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            1_000_000_000 * SDN,
        ),
        (
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            1_000_000_000 * SDN,
        ),
    ];

    let mut properties = serde_json::map::Map::new();
    properties.insert("tokenSymbol".into(), "SDN".into());
    properties.insert("tokenDecimals".into(), 18.into());

    ShidenChainSpec::from_genesis(
        "Shiden Testnet",
        "shiden",
        ChainType::Development,
        move || make_genesis(endowned.clone(), sudo_key.clone(), PARA_ID.into()),
        vec![],
        None,
        None,
        None,
        Some(properties),
        Extensions {
            bad_blocks: Default::default(),
            relay_chain: "tokyo".into(),
            para_id: PARA_ID,
        },
    )
}

fn session_keys(aura: AuraId) -> shiden_runtime::SessionKeys {
    shiden_runtime::SessionKeys { aura }
}

/// Helper function to create GenesisConfig.
fn make_genesis(
    balances: Vec<(AccountId, Balance)>,
    root_key: AccountId,
    parachain_id: ParaId,
) -> shiden_runtime::GenesisConfig {
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

    shiden_runtime::GenesisConfig {
        system: SystemConfig {
            code: wasm_binary_unwrap().to_vec(),
        },
        sudo: shiden_runtime::SudoConfig {
            key: Some(root_key),
        },
        parachain_info: ParachainInfoConfig { parachain_id },
        balances: shiden_runtime::BalancesConfig { balances },
        block_reward: BlockRewardConfig {
            // Make sure sum is 100
            reward_config: pallet_block_reward::RewardDistributionConfig {
                base_treasury_percent: Perbill::from_percent(40),
                base_staker_percent: Perbill::from_percent(25),
                dapps_percent: Perbill::from_percent(25),
                collators_percent: Perbill::from_percent(10),
                adjustable_percent: Perbill::from_percent(0),
                ideal_dapps_staking_tvl: Perbill::from_percent(0),
            },
        },
        vesting: shiden_runtime::VestingConfig { vesting: vec![] },
        session: shiden_runtime::SessionConfig {
            keys: authorities
                .iter()
                .map(|x| (x.0.clone(), x.0.clone(), session_keys(x.1.clone())))
                .collect::<Vec<_>>(),
        },
        aura: shiden_runtime::AuraConfig {
            authorities: vec![],
        },
        aura_ext: Default::default(),
        collator_selection: shiden_runtime::CollatorSelectionConfig {
            desired_candidates: 32,
            candidacy_bond: 32_000 * SDN,
            invulnerables: authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
        },
        evm: EVMConfig {
            // We need _some_ code inserted at the precompile address so that
            // the evm will actually call the address.
            accounts: Precompiles::used_addresses()
                .map(|addr| {
                    (
                        addr,
                        fp_evm::GenesisAccount {
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
            sp_runtime::Permill::zero(),
        ),
        ethereum: Default::default(),
        polkadot_xcm: Default::default(),
        assets: Default::default(),
        parachain_system: Default::default(),
        transaction_payment: Default::default(),
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
