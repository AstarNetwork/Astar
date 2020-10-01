//! Chain specification.

use plasm_primitives::{AccountId, Balance, Signature};
use plasm_runtime::constants::currency::PLM;
use plasm_runtime::Block;
use plasm_runtime::{
    BabeConfig, BalancesConfig, ContractsConfig, EthereumConfig, GenesisConfig, GrandpaConfig,
    IndicesConfig, PlasmLockdropConfig, PlasmRewardsConfig, PlasmValidatorConfig, SessionConfig,
    SessionKeys, SudoConfig, SystemConfig, WASM_BINARY,
};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

type AccountPublic = <Signature as Verify>::Signer;

/*
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
    initial_authorities: Vec<AccountId>,
    keys: Vec<(AccountId, BabeId, GrandpaId)>,
    endowed_accounts: Option<Vec<AccountId>>,
    sudo_key: AccountId,
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

    make_genesis(initial_authorities, keys, endowed_accounts, sudo_key, true)
}

/// Helper function to create GenesisConfig
fn make_genesis(
    initial_authorities: Vec<AccountId>,
    keys: Vec<(AccountId, BabeId, GrandpaId)>,
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
            validators: initial_authorities,
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
        pallet_session: Some(SessionConfig {
            keys: keys
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
        }),
        pallet_ethereum: Some(EthereumConfig {}),
        pallet_sudo: Some(SudoConfig { key: root_key }),
    }
}

/// Dusty testnet file config.
pub fn dusty_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/dusty.json")[..]).unwrap()
}

/*
/// Dusty native config.
pub fn dusty_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Dusty",
        "dusty3",
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
    let authorities = vec![
        hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].into(),
        hex!["48cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
        hex!["38cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
        hex!["28cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
        hex!["18cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
    ];

    let keys = vec![
        (AccountId::from_ss58check("bdrHXtQkGHTJrpNErtyGmfhh3wtwy3uX7wGRZDHJShS2aHF").unwrap(), hex!["ee1c3251c6d423e6a3d5fa3b52526044835a7b754f4c0d83a3ce9cfada23e357"].unchecked_into(), hex!["3d531ea1cb447b7ba1477b5373c69dbd0b990a15093ef23d82cd62f486f248bc"].unchecked_into()),
        (AccountId::from_ss58check("aHSNN7fWryhBJn7o1uEBW1vLXsKSnRDKWUEBjJW6FPBZoiv").unwrap(), hex!["acbd9a668bf8ef12769db3970d31387cea3d0ebe6230bc57d0d027290fed0865"].unchecked_into(), hex!["c237a2dbd94db917b17dc66fefe25bde814ba6578daec2d572a5dd1d2c05150a"].unchecked_into()),
        (AccountId::from_ss58check("ZoR24Kp1bzo4ASdwffZ5h7Z9UpX9Gx5DiJTEeqGVboULVdP").unwrap(), hex!["b26dae5ee57a3a7637052cfc47f8e91bbd71561346a30fb6867dba1f60b6d77a"].unchecked_into(), hex!["11422fde2ccfc948fc5e0792119afce987ef644c0f12bb439c62032f69aa5905"].unchecked_into()),
        (AccountId::from_ss58check("XqbJjWyRVFcZKvrtszJC7ebzJCe7zQMaPhekQquVcYSVTQY").unwrap(), hex!["7809f562dfa9dd3e29a8e9e01f8cb5bf95bbc4ad4b356e644f0c28849e35bc7a"].unchecked_into(), hex!["c3dc3fa3fb49725786ea45f4b72e3484f9d9d23723ea6b27a5f631aca379a856"].unchecked_into()),
        (AccountId::from_ss58check("WJvHQSTh7rgd95CVeLzwTdcx6TqvM5rXB5ppmq5C7Y8s4NN").unwrap(), hex!["38397545b0cbda6ed09ab7c8afee6fc13948356ef606faf6164857e509e48b78"].unchecked_into(), hex!["4134f2e2c31dfc6848b513de7caf3588a5640dc3fea15caf0821b14b91e9ec8d"].unchecked_into()),
        (AccountId::from_ss58check("b9sBzZkJ8QPfwv8PvZA1T3q73bXNQ74HE5ZrhvaMc3WFiSr").unwrap(), hex!["d099b5d7ff7ee62f54710384f1b75a9033985e420efce77397df324a4986f061"].unchecked_into(), hex!["38b179c8cf54a502e2d59121c74d615658de6ee083e9751a565e7773463a8c04"].unchecked_into()),
        (AccountId::from_ss58check("XDn71pH1XxRg2NYQaNH9pcgwoR5t2Yk4YHtNY6yrAbp34Wi").unwrap(), hex!["d409311bae981d87dee63d4c799723a33d509d7388db4c530a10e607937e547d"].unchecked_into(), hex!["36aaade466263a00ec16a1a1c301636ff8488fc28a08e6a7eca7ac8496e35dca"].unchecked_into()),
        (AccountId::from_ss58check("axQ1Zez8u3wVXEsJ29jpUuBCMfmCZpde1u7j1JSKqfGXBYR").unwrap(), hex!["324fac4c952701e4d8cf37548818d36825060008d4c576b8c63485075799035f"].unchecked_into(), hex!["52def62aec2fddf2774c0ad3d7f3a0c8570d53f0b95a3baf0ed9b02d56a11346"].unchecked_into()),
        (AccountId::from_ss58check("WfMhiUCcS3gL2Rs6UYN11MkdKAHA8mfQ1bzPdCXU3FWTmmr").unwrap(), hex!["7c1d4ae577eb4b86fd8854dbb4dd1a66daf4bf3eaff8bdc62473d62f4db4a955"].unchecked_into(), hex!["d90d04a2ae2bf9e19c03d02c3154489e487dbb402a4e456ab59a318b1ac7d510"].unchecked_into()),
        (AccountId::from_ss58check("W8CpP5J3nkp4737S3eNXyLi5cnaZzthEheGhbYpwEL9EjtT").unwrap(), hex!["384d97f1e92052c325e79c2d07338cbd0b0772aad2f8d6731d48b0684a920103"].unchecked_into(), hex!["014e74d4646986a0d7b9d5e2566d9b235c7c6b814e94f893e5ab675fecb13039"].unchecked_into()),
        (AccountId::from_ss58check("XaksTAN6Z1bZLi8iMoDcZkuMtoMSrLXSyT9M9CwKDhbJfej").unwrap(), hex!["70887b6d5241f2483fd7f199697a2f4ccfe3aedbfa60fe0c82fe476a4b08a320"].unchecked_into(), hex!["c62110354d58905bbfa894a1d82f0c175dfc7720758b28d18bc2118ef5f54f91"].unchecked_into()),
        (AccountId::from_ss58check("aqMhtFmPgUE64aALHQ1di3pQ39XvEEjvRmZAL3EzG5rFVcp").unwrap(), hex!["8ee1dfc724145662b5959d0e423940ffb904ed5703ae3c9e0c1f85c5a96bf305"].unchecked_into(), hex!["844b53bad5e0e27b7ff10e5d2d305b5ebec8d0f6f5e53889b6b94c1ebecc5d92"].unchecked_into()),
        (AccountId::from_ss58check("WdGUzQFw8B3zsxuoL48ns9WkxcphYgV6Ct82oCfxmD1Kqsn").unwrap(), hex!["a452b1bc9cc394cfe98f86ac0212f2a153490edfc6108fe4d746867e5d5e5747"].unchecked_into(), hex!["fda65cf2488771b13e7561936537c66355ae5c8d4f884d9529e0083e4169f5fd"].unchecked_into()),
        (AccountId::from_ss58check("ZUCDxtBEWVi59qaa4B4TfSGE9thYpP7nVGgqRPn7jczXeEt").unwrap(), hex!["2a1d65bebf3773270767d8d4f73e4e5da2cf937ebf63490c85aec36db706810a"].unchecked_into(), hex!["a7ef9c3ab3aa7c842424ac9e5d420a4be70e38c755e104f8fad03f5125552543"].unchecked_into()),
        (AccountId::from_ss58check("WTMJByGbm2pspg3WdsgihyafozjZjriaCYTZuAmhzTnGvax").unwrap(), hex!["ac2bbc1877441591e997a7bd8043f4df4f7ca69bd05a762b0661ec376f64f551"].unchecked_into(), hex!["0e95fb00ea007cd02b7b0065840d4572aeab5dbf77f148a62330168e7092703d"].unchecked_into()),
        (AccountId::from_ss58check("bVxEw9hcY3gKRGb9THwCocAZ7Ww3cQRhgj7EUtnosBoNket").unwrap(), hex!["06ce1856ceaf3743f70deef838b03f76cb5c91d8a0d847dca9cb4ebdbbebf620"].unchecked_into(), hex!["200ffa89515b5a4327d06d1a6bd27e8a2656c52445f07f73419470ae9b7cfe94"].unchecked_into()),
        (AccountId::from_ss58check("ZaJi85BQtBAAWjhK29qoqZrLo79CC2S3REenpD88H5jETEx").unwrap(), hex!["887821e1e1cd5f0679b13142c00a86a7aaf57e98dae16ec1ef43beb2837a246c"].unchecked_into(), hex!["5feb5753bf43bd1ce69222ad717f7f3c4915408225c05e697501b20a2c6a6f24"].unchecked_into()),
        (AccountId::from_ss58check("Wr1VKUaNbSMLgiMq983TyetzX7v3utWZzGoWpXXcKutJopB").unwrap(), hex!["8e1bfd7fe9ecfbdcb7e3ba011bef469ffbcaa4ce150ce9583dfcd1a87257220f"].unchecked_into(), hex!["2e29dca815189062c5b126018ccdae84b2e2c88327265e0eaeebc442f5b22fd9"].unchecked_into()),
        (AccountId::from_ss58check("aaVm288nFvtEDfSoAequ55YQHLGnTZsvcNR6iwGDeNMggbR").unwrap(), hex!["9ebdf67000283f4b475547fb844478c68e81caa64a0da55e244910018f3c3e0a"].unchecked_into(), hex!["9eab5bc89dbb9f7d4b376f70f8a7643492a5693285f387f2e79e90489778f08c"].unchecked_into()),
        (AccountId::from_ss58check("WzWaNQWbLyLBoghehG8r3cvVLpGyf4GsyRZ3LL5qpWZyV3B").unwrap(), hex!["667ec6d388cce6195ed7eeeb7b6815fa2e3337a1e447688256a56aaa8706b313"].unchecked_into(), hex!["5b21152d340b935d7a4ed71e45fa4f4fd6ed9c17d05f691b3043367a9c556c00"].unchecked_into()),
        (AccountId::from_ss58check("Y7QcJMaDrzanYjd9nNhH6LA3ihYEcDDfAsxfXAd8zGe4817").unwrap(), hex!["8e148ae69412d5849c1f4ba18c685606b988cee5a24bb3f23f36f21bb7e95273"].unchecked_into(), hex!["e3dfc45ea92e6c5c810691c9015628b1466408422410a2e15bce548d112a1ed9"].unchecked_into()),
        (AccountId::from_ss58check("Z84kUCaZhX9ZG7fqPMPFwerxvnyVyP1DNVVd8g5RuFtSAxw").unwrap(), hex!["6ca70364b997833e3e850edac76a237b5197df04a82f6cd42ecfe28040017d75"].unchecked_into(), hex!["6eb343b89b8cda00253fec59a1b99a4a92bdd99119090a776be842c99a78a27d"].unchecked_into()),
        (AccountId::from_ss58check("WDaAqcYqYFhk6aYHefQ5CaT1b8yWE7w1ULwZHg9vhpg2iyx").unwrap(), hex!["d0505c8f98dda4ec56fdbdd7683f031d8b1b9c6257e391f5d2bf940195988a4a"].unchecked_into(), hex!["b516305ec623e416b5025da8ad1f16cc4175d9ba639e47c372cf72f8524319d1"].unchecked_into()),
        (AccountId::from_ss58check("W5F25zxK2ZF3Uv6NnVuzuieHrg8RzzWa9qvtfSCA1HDCr52").unwrap(), hex!["1827624d5bbfdf6219cd1af151d522d5a5acfe2dd3d2bce664e9749d3b9d7865"].unchecked_into(), hex!["9767cae477eb66b5b05f0d5c689805ce19af689f58f99f31b068bdd80c1f588a"].unchecked_into()),
        (AccountId::from_ss58check("ZrhSCBNxVh5pJJsdX1q2k3tu1yffwVwcSVXNZsS5puEdKax").unwrap(), hex!["221b7fa916a0f024f325ffeb058d3b3d8eef7419d38fc2b54fab6c841631d542"].unchecked_into(), hex!["dc44a116db865646ad6d93715ae39bcc6153cba227c2d314fd75554c5cab0fa7"].unchecked_into()),
        (AccountId::from_ss58check("W5UbqH1MtvduPZ2p2RKTVrwLRhQwvEGZxDT9N89Zyy8j7P8").unwrap(), hex!["5619c0512a47f90f791192393631c21aba0cd27dfea427cc449aef59d40ca61b"].unchecked_into(), hex!["f477042eb78682ee01b7d62f1de056d9cd41bde6b5c2b0d97fa8737e13b30fc3"].unchecked_into()),
        (AccountId::from_ss58check("Vws82RrTsECumPe3R1rWpU5AG7tTn1oa9PuVtrinMij2vLa").unwrap(), hex!["d62308dc369791b113b0e060864a3e985f8e4fab8a13863caf2826082d693868"].unchecked_into(), hex!["50663a61c0cf0b42c35f47cd25032b2ec008fe25d71075a3d3de8b89b78e6764"].unchecked_into()),
        (AccountId::from_ss58check("W12kwJXtEXhWnEsH3LcNBoPkE8qLCxrakKrB5RdmPJfqU37").unwrap(), hex!["e4b875c761494b5e24e690ca6dfaf9cdcef20936f1bc8e3cbab76608de64bd44"].unchecked_into(), hex!["45ffd954ffd3b4c2c9ee1e223cff916bf199dbbb05a7eee7ec12e483a7aeebde"].unchecked_into()),
        (AccountId::from_ss58check("WhRHa5pLc5K1K3aujMN2wLto7aZovHCdqG3M5rWvLbZL8dr").unwrap(), hex!["26a6cf41799cf1da305427833b8f83a863b4e3918cff066adc81ea973f18e055"].unchecked_into(), hex!["2a2c2e1dfcc930a1e9d05d8b268fe428dad1c58f830c082f77b51046e41079fc"].unchecked_into()),
        (AccountId::from_ss58check("WmkzWAEMK5DSgd96MX1U9vpXCyzSahoMyXfZeFsG8YEbVuG").unwrap(), hex!["c2c38791c020acb4866772dec67b41bef489b4d2335a1e097bddc62bfdbed46f"].unchecked_into(), hex!["1b89862a8a21059c678b9d22ab65442de7443c85860fce2f6d7288ecb10756ed"].unchecked_into()),
        (AccountId::from_ss58check("Y5JZm1K5HpCAx4Zj55W9cf2d5PuFFhd6F5xPXbSTQv46puC").unwrap(), hex!["76f0c33fe35a156839a340cea6db4cebe9338acd8d4fea4701bcb5547d04eb60"].unchecked_into(), hex!["6a857bf01b82589ce8c96243056c0c7a04558abec31ca9c165dfad49655fa77e"].unchecked_into()),
        (AccountId::from_ss58check("Z9roQrB41XbTtkmDw8xNRB9LbQGLQGSh8qRkm9cgXPW4eQ8").unwrap(), hex!["2ec9caffb50a8d42866a8290505dfc10a08c3551696e4a0cb99556df802dea7d"].unchecked_into(), hex!["bb37e724a749187ce7b0d0e1b4b819833428af5898c8986bb980bfa10ea0e2af"].unchecked_into()),
        (AccountId::from_ss58check("VwNbP3Zx77te3URWmjD17af2mVHa1W8ictc9wF6K3vYXBkR").unwrap(), hex!["d67c905b801adc22f2a921e578a9d9aaf728f4175e4dccc95887b568813d791c"].unchecked_into(), hex!["ba37199d5020c3c054580499d41e971132feeead0edceac60add00fb5f1fd526"].unchecked_into()),
        (AccountId::from_ss58check("YUQaMJHwKmzGGFUfSeH4EKxV9MGwdtr52QG46uAce2iVjAh").unwrap(), hex!["1c73f1167c093c43dc70bddb7fdf0bbcff7244a3d1c785e557b99ce27ecfcb7f"].unchecked_into(), hex!["75204493b3b00b7d754d0dd38b6e2d4b549607c4e44f2e7e1574680ce95f7ab6"].unchecked_into()),
        (AccountId::from_ss58check("axpVRBhe8zoUkWiXc4wQnWjiiiWRYNAv6sraqg2Eo47uVy8").unwrap(), hex!["34e224544566cc363ba62bb16089ff9df1bebd0a9b04e6d783f53a9c3b31011b"].unchecked_into(), hex!["bcb0cc5cac58dffc8c98d9d42b04b1bef44a2ff149ba7a96f57c6468ee341354"].unchecked_into()),
        (AccountId::from_ss58check("XuH35opmreeFtvWLDkNddLFEbLQJCUDGZ8MDGf16uFTxZnT").unwrap(), hex!["e6f92d2abf5c04f80c3672eacf06197fcc677f20d11632893888c15067fa8051"].unchecked_into(), hex!["5d1e303418c845d699b8c230c8b61fa68a82449d486a26eb9ff2bbd27997ef94"].unchecked_into()),
        (AccountId::from_ss58check("ZJEc6ZXwZ1oq76Hc37ocUEHP8H54KYEoP8x1h53qLjFmdc4").unwrap(), hex!["841da123916096e57cb99657b01aec4f534baf327537068a295bccfdbe030260"].unchecked_into(), hex!["cbac19e85eb65be000f8b128e70198bc71538135bc379343ee2a003ca3afb6f5"].unchecked_into()),
        (AccountId::from_ss58check("aq1JXpWCoPaFnHYAeicbW84DEsS7p1pCBKUE9RZDMRotNEG").unwrap(), hex!["54924faba85e54e358ee0818959411b8e8d3b191a7ab0bcc2950de2a82914c60"].unchecked_into(), hex!["d5861c2e41d6e290c1736b52b1bd60660e16f6816f8aeccd36de2aa3c161248a"].unchecked_into()),
        (AccountId::from_ss58check("XowsLNTrd17Qd3bwapmqtXZ7SuBqKJJAMNrX1nEbSVGJpHu").unwrap(), hex!["60dde8b38c6f6e187d512adcfbb6c02dc2ed88c0d2281657693e6d0cdd4a7818"].unchecked_into(), hex!["87afbb714cde47eef1a1e441fec98bd28bfe8d3cb690f4dfc8e9668e5e0665ed"].unchecked_into()),
        (AccountId::from_ss58check("YAz51SDiPNNfYhdr2T58q31cf773t6nq7CZdJomXMada2EE").unwrap(), hex!["20889f8214adbd13c5c8eb20c7582f8359a2119f0246d6970e05b0e5c9435469"].unchecked_into(), hex!["b6f4f535a7ad2ebda1f67364673660a9ec86212810187872c91f04019b5321d0"].unchecked_into()),
        (AccountId::from_ss58check("bbfUhdwC6U4JCwF1VeXgFcMq7heb4tnfVFAe8LxsN5iE2hk").unwrap(), hex!["24317f9e97d88ee88fe0056cce22cce075852319e772e26a323f7288f744e340"].unchecked_into(), hex!["d1df0d13369d952df1732552c58197deaf77978ce1d9d3fc8386af67ee8a717e"].unchecked_into()),
        (AccountId::from_ss58check("ZHqciH6HaE18yxxkcRLFrxLikxrrw6Bxcmhg1X8dNoryika").unwrap(), hex!["cefcba8441e3976a428ec3df93eca7e2ecaefa8f8018a4524691fa7073ec316f"].unchecked_into(), hex!["0ab0a5dd2c04fc6ada67018f2571250cda8ddb95361928fe0109c607690cbd07"].unchecked_into()),
        (AccountId::from_ss58check("ZS3kKGvt4aqXE74DC4ShfNp8PBao3P4NBmaQsWEogMxx71C").unwrap(), hex!["b8f7710ec62e375c95cde5a26f22740162d0b4e997ec85290a19409c88f36571"].unchecked_into(), hex!["ef8150b8fb26753320f5a27339c7481a946bc36fbffa12f08aebd5585fc91f3d"].unchecked_into()),
        (AccountId::from_ss58check("YkXGKXvee1doH2Y8xthtBZmHkH1jtcKuje4KKZ2HngoXZ2J").unwrap(), hex!["d46d9de8bb05271408633cf1850a012f10aa94cf1bb3a1e6649234839d930114"].unchecked_into(), hex!["fc2c17f8fd5bad1ef4ad7a3fe70505442c776a8349fdc8b4ca18d2664daef8ac"].unchecked_into()),
        (AccountId::from_ss58check("WywoJ3jqnpCeFchBV3dJpjHWhDiD2C5UaDm2Nk4FaV2aNLi").unwrap(), hex!["2a473c2832df694e0dc524b2fdd1e1dd7550a2f3465436b3bd0669ecbac8d05a"].unchecked_into(), hex!["4e4b09496d932aeeb0ab7c4ade5d9229e20e5eb6724d1cc8492c33264aa3b4c5"].unchecked_into()),
        (AccountId::from_ss58check("ZiPahHnNRVUZ4218CtD527aecG6XyKKwpj14GhnMZxABPY5").unwrap(), hex!["9ecb630e13394efc84f5c3cd7c758c08395cb7c4db153b5a558950a2e0782b6d"].unchecked_into(), hex!["8908d8b3016fa42c9ea155a0bba237b594c4451a227bf2199c40f3a7f64344e2"].unchecked_into()),
        (AccountId::from_ss58check("YsuvSX1EDV3uV5XbMZRxY4TQdog3wxt3cd5NkQzdniJxvBe").unwrap(), hex!["cad39dca91616235cd8f7dff7fce266f1a582b17f23d80f576a8b3f10f4bca07"].unchecked_into(), hex!["6e55f238c3eae2027cad34649ca3b4edc38853c4bf2430db31c119da34350b75"].unchecked_into()),
        (AccountId::from_ss58check("b1p4iRzaSjBPMPpH3KAdHbN32Rr5fZngU4vjZXPg5bNjKia").unwrap(), hex!["284e86e1c14b6b25cfe75eafd30d9be92e0caf7a64e676e449189f5c8a5cf324"].unchecked_into(), hex!["d2a58370180c8c5209c4445f9e48e4c02542894c2a1c187842361ead285e141b"].unchecked_into()),
        (AccountId::from_ss58check("XiBYZs6cARmdJDs99XtJ3YfaSzLwjoQcMTvhPuNnUMPXjKb").unwrap(), hex!["38bde8c91f47eed69163deae8ee50f461b970984240e51ce7b53a714b097a40b"].unchecked_into(), hex!["ec51119b0b9e9b38001fd46ae41a414c113101793407a8da75cb8e2881b0dcdb"].unchecked_into()),
        (AccountId::from_ss58check("YFKJvAnECGc9pnTvUrBWXJ3oVbMcZwSSiUHVurGSdkvyoud").unwrap(), hex!["0089d9a9cfce6077b049643acb01888bb85484691864a2a0730f41750af6304a"].unchecked_into(), hex!["946ca07c9b08994f50a626886b1127e4630e2faed9d487eb2af60383bbb6550e"].unchecked_into()),
        (AccountId::from_ss58check("avyQoeiu2PtA2qACiFsxj39H87nqRL8v6UPqEzi6VFm7wXn").unwrap(), hex!["26d7a23340d7e042f8508bcbfcf613736f09b8143c58a7d8c404fc75a6791935"].unchecked_into(), hex!["216290515d94786304c18c196356713d08d9ae5f6b7f36cf5ae98a970e1bf840"].unchecked_into()),
        (AccountId::from_ss58check("YJoKTocu1eQKAgfjkkUQ3wcMCGgqKWF9PPj97cRfHyo8JKg").unwrap(), hex!["d057130ce56f62b38f36c6367c70af5cc7e6772f176b107078d143af913ba50c"].unchecked_into(), hex!["af04b9bd62c75ce2f36a2c27b44d500b0f6dc6a010114bdd8536ba2ce22341e9"].unchecked_into()),
        (AccountId::from_ss58check("WDVLAFvqAkoyGu6yxi8gqkxB6siXLwyxa5PDEfL5fpq8dvi").unwrap(), hex!["fccb67ba08eb0b50b7d1fc476d8d52f87ae86582db297f0658a0ac1098a3615d"].unchecked_into(), hex!["1e93d6772ef80f2d35e97d1f2147e42b4474d35140197b24d5ee4bdac140f3eb"].unchecked_into()),
        (AccountId::from_ss58check("b6xtKdEEmjA5R3nqagVkQaae31X5D4KJw5bb8TdCMAJ4ZJN").unwrap(), hex!["2675c4ab6a2afbcbefb57d60508cf81235bf961c06bc99658c0dedcc9d5ec86f"].unchecked_into(), hex!["fed146a416464ebefdb0d48f382fd74865c9d302edfe53ed90e89f8db74ab399"].unchecked_into()),
        (AccountId::from_ss58check("ZFpB1Hh3JH6EYRENiMUoqj8kKWGhuEaze1GgGaZjsdCbsGi").unwrap(), hex!["26a26e41dbf41218cd61b76256db70bcb8a6a437105800f5410ad74519ab792e"].unchecked_into(), hex!["63a09431a5de4b597094cc188c917ff418e0812d0ff56f9c053c1caeb3e337aa"].unchecked_into()),
        (AccountId::from_ss58check("aUps9jmZcgGer18dGjkxJn8THGexWRszbSVe2Yvu72GtmZ3").unwrap(), hex!["483201e9246f302cdd81e00f7c89594fccd5631d712ceed70288dee72f463f69"].unchecked_into(), hex!["a8b9854cb5f66ae5041e763f134fa34131aec9bda80d5fa8369e0fa3e83c6040"].unchecked_into()),
        (AccountId::from_ss58check("aqMnapwNPeLx6Mx68Uzm1wcgTq7vxYF2cDys9nu9NgmgDSn").unwrap(), hex!["2c1ed62add96f08b50e6505adda0db755e54d48f39574fb0b6efbd9d5781d329"].unchecked_into(), hex!["7f141531eb6aa2798b968e440eff5ef70a4b69513d03cc607ba83e22acdc6bf7"].unchecked_into()),
        (AccountId::from_ss58check("bER7QJ6kw4RgNdgL1e8DTWb3psNb3q8F7ym3BNhnjyffiy1").unwrap(), hex!["82cbce16c3c40cf8e50a035c345dfe315d9004a1483d6ab3c6506e33b7815f3c"].unchecked_into(), hex!["dab763c0ee858856b0bda67bee5be2124129ba108beb30ff74dba27f06b7e1a0"].unchecked_into()),
        (AccountId::from_ss58check("a9YvCUomVGmx2GUQ3p671u3P7NF7yiTPmt7VTwYxrQGRoPR").unwrap(), hex!["b443a9599dd6b71c4412105187e0812f463a4b38bad320456a4db231d80ac95e"].unchecked_into(), hex!["c901cad13c39cd8b8ab1c4489eb6ae41ee61ffb29ca7a577c6f6f76c4846301e"].unchecked_into()),
        (AccountId::from_ss58check("WSWa5zEtZLyAiq6cx1ykjRpq4Q1JheMpSEpe5UQmLGJkfKm").unwrap(), hex!["0882a3631204c3c727ae9d72217edd3e5a300dd6158e869e4d2362c5096f6b1a"].unchecked_into(), hex!["9657c96149f9d9cf96b47c1a67e74711add5f21f74c76c4216f13dce7da4a010"].unchecked_into()),
        (AccountId::from_ss58check("WorsDyJRpBkFTXMAcuKGYF1sJdvG8dZSmY9gwKWZEzkGU4u").unwrap(), hex!["c83571feb79c8d4f7c943ee2dce403f9e404e0f41f2aa5d2ddabcf202cae2e1c"].unchecked_into(), hex!["87a26bde22d797ece29d5e3743fdc72f21aa0fef6d382b7a8a3222291b2114a3"].unchecked_into()),
        (AccountId::from_ss58check("WDUqd45VLsHnu223f3yuSyvWgLw6fnVTuxDHEwE8HdAYBCD").unwrap(), hex!["deb91392510a1d7df0162b181e09471adf078af12afb2b6a0c3624876530793a"].unchecked_into(), hex!["8e56235e8c343ff6a22b7e09fd3640158f6d54b1d07011b54f127be0cc1b22ab"].unchecked_into()),
        (AccountId::from_ss58check("WaH2sbSw53bTH6tm4txNx1Uq1ZXrsaEvnSzeXnyt2ir3sKo").unwrap(), hex!["0ad2a32c09a7bff03e328c656ec8ed3bd8089f412d4e209068df2376d8ead714"].unchecked_into(), hex!["18ce277c54103e7b2760dde83a203d6fd8c67199d4f6ade013fe3e2d7b07816d"].unchecked_into()),
        (AccountId::from_ss58check("Z4mfypvwUyLUQBATD1JMjSAc9Rxn4X78YdJBivdtgai2GvV").unwrap(), hex!["9e97f067374823516cc4a70e63546550064c402a0ec12347d5e35ea27460b47f"].unchecked_into(), hex!["39e726269bffe1276672d1f89b51b02d25a7ee419268341507633d773e97bbc2"].unchecked_into()),
        (AccountId::from_ss58check("YS3NpBz6k18m5irxW5Z5iKBaYj7fbCLSZiUmGdU5X7Djn4t").unwrap(), hex!["3ebd9bca4cd0436ad649ffe60d93de7c95dce91c9034c6872f228618d5c1af43"].unchecked_into(), hex!["71037ecef701f8e439976b580dc778b90509a77faaaa1fc1c7c685fe5a5828eb"].unchecked_into()),
        (AccountId::from_ss58check("WbQU971v6fYfDkifpWfZRcRZ7JBYaW9u91Gkb5LrMpLwLVv").unwrap(), hex!["c23a99a537091758f463f76a3691616a958c41fa4ec7e1533d364ecf78b78c00"].unchecked_into(), hex!["cfb62ef7d85a4cec8ea4a84b23d0d40c5ad893d2021f5b91d3fd151992729a1a"].unchecked_into()),
        (AccountId::from_ss58check("Yk4NVesFj3JJetvzTC2GB8nxZBLyER7sRKVLBi5jKbY8pBy").unwrap(), hex!["ae5d17ab235e9f958f6ff06db97112e446ff1d06664d818f158760976b38f11d"].unchecked_into(), hex!["6e965bba6c73afc891d27be82e22e898d1d2b9de1d3cf9b98ec6f445d72bf1db"].unchecked_into()),
        (AccountId::from_ss58check("X3FcF2sdAdq8kE84bLLNo8hFrbHMMM5hG9CQLPFPSafcc3A").unwrap(), hex!["64d85553848ccf1b7372b75ed708766375b0b85174d420ad088f102790154d37"].unchecked_into(), hex!["5969329fea45900ddf4505121eb9fe36e466cd86ec73e87f92a3cc5d637c3991"].unchecked_into()),
        (AccountId::from_ss58check("WyhFnn1tQp3ttUzSrrfZ3uFdgsrqkc2HMkHuJewzC1aDR4r").unwrap(), hex!["cc54752f6204766548247988997e861f5e17bcffc7e46f071d4cf5c8786e1110"].unchecked_into(), hex!["0bf19f1e4157b3dd9f9d31ef063777e3a98a1c88612eb677f5ed90708aefcfaf"].unchecked_into()),
        (AccountId::from_ss58check("XSQxDCBpL9C3gMUoTtgd3QJEcj63UqjLf4WUDH1weiEodmf").unwrap(), hex!["c0f7472d386a0f33676d7592021b96436f4462595139697a3be5296931fe925a"].unchecked_into(), hex!["283bdf91ec781d298a424d147d07b311bf4304c25930caf1b5e62da53a83839d"].unchecked_into()),
        (AccountId::from_ss58check("WVpa986qVr5uPhMo1WQELLG7ceYkNyBHfyNRKu4v4QFVxeu").unwrap(), hex!["96e2554353e7a8de10a388a5dda42096d3c7768403f3735d0a939bc3fd39bc54"].unchecked_into(), hex!["674bd4f2670c0e99edcccd5d3821c54b9d559580a31d8e2ca1e88c1e3db28021"].unchecked_into()),
        (AccountId::from_ss58check("XCkJTv7Ncven3y7rsqkeV9dS9biqGVwhwVaunyQrJEZydzQ").unwrap(), hex!["9ae93f7a7a0aeb64ccaa4a7593ca4cebac3d1ffb1a6d263ce52fe36cc8fd884b"].unchecked_into(), hex!["15935dd38138ff3bd02cf0f778db932bf96f8431ac4edf7f33a5d02a15c92614"].unchecked_into()),
        (AccountId::from_ss58check("aX6hiPgpR5AE5TnWT2hSh9b5ShsR4xT5TbWkBcmA4jLkJxR").unwrap(), hex!["ac245d4f57f440383f5e8bd73b834c0ba3966ee5d16638c05d5d4d044d5cd300"].unchecked_into(), hex!["e1a327a14e102e246971a1ce8a9bba05f96b2f1bcc9942b06575e473c33fd56b"].unchecked_into()),
        (AccountId::from_ss58check("atDAeTYvvM5c48mE7nDQYyHKX11g4oq1kKKBVgDcXDVs1tc").unwrap(), hex!["da9d4213bc70b9fd7b8d91b2a33355b91e516629d82e8bead89148f728df4637"].unchecked_into(), hex!["21d5e60f14a9407af3057323b1156e990954a620436284601feecf129161b495"].unchecked_into()),
        (AccountId::from_ss58check("WJUevho3n9RuQtdEwVPgThoRPvDttPKh2wqoecNCeT2U4Jb").unwrap(), hex!["1cb8f1e1f8094e6e7dd866047b6b97f13adf451571acc26fd8a576a0eb4d5b36"].unchecked_into(), hex!["633bfb31c6d72ca4e7c658db876848c6a451ab9bb8dbe912a93040dd04395222"].unchecked_into()),
        (AccountId::from_ss58check("YqeDxpgMG8w7Qo3qo5vxfMakB5DwNrBEKP6HxRBQQjPnzAe").unwrap(), hex!["682ad0818dc86d028656df783b93d06e7ecb8dde784de5c88b8aa98b3b5a567a"].unchecked_into(), hex!["1091b11ea548b414559c271ca57b9f1d1bfc34484a69ca34dee9fdbdf032f248"].unchecked_into()),
        (AccountId::from_ss58check("YQ3G6JKbvRHZbvweppbh2vyMnT9vwVXVGEHSeqt5AgZydSx").unwrap(), hex!["8001a5c4952f730995a919cad913c56956abc90dd51b8f6fa4de35f82beb712d"].unchecked_into(), hex!["a486c3285908ddf0a71b93828a6212469bbd7c80f792dbabe91f581c6aaa9a77"].unchecked_into()),
        (AccountId::from_ss58check("ZhGNYfZs4BQJgHX5ob5PJbZBmTipHvatrJwS6T7hRuZUQWa").unwrap(), hex!["eecfa354b02d7a96e52d308de23e265a5554749dfce7b733a9738874b77d9433"].unchecked_into(), hex!["78792be4552afadd58fdcff8205667d4792b5d0d9babf506554bb3b8cdae6e9f"].unchecked_into()),
        (AccountId::from_ss58check("XvkYc3JFoJAHSAhfUU1o3DFPAZGefm9nRGfKaFFNNH6uPwf").unwrap(), hex!["2e2e0bccae58d5b4089f44a877e67c0fa5254f36b4b1b67551564e9988238727"].unchecked_into(), hex!["8ce66bcacb01ec173ed26ea5640d79db45d48d97ccd691deaf695fd66b4aa769"].unchecked_into()),
        (AccountId::from_ss58check("WroLaUBvWuFni2x6nwLh5UUXi2pKCkxg78dPw12P7W2mjA6").unwrap(), hex!["266f53d34490e10e6c818a1f6208dd285a74c01e022cb3b725cf5888bc89136f"].unchecked_into(), hex!["c379204b0b450bb62006a0df2b4abac72c79909248fc0f30ce0b05fcb9c102fa"].unchecked_into()),
        (AccountId::from_ss58check("YWwj8QNRLTWEYVD1QoRw1NVZovynnRC5dzVp14t3jRfwWRU").unwrap(), hex!["4c61527059d7d04129d6734c500ad7a1da73a3c065a41f84d1f268ce7e71a071"].unchecked_into(), hex!["f891d698fc9a11158a4c5d824e99fa8735132a54b9e0c87939e6e85b38f2dbe4"].unchecked_into()),
        (AccountId::from_ss58check("b9u4QBgFmZ5gt91rdJ4KjwgPibTD44H7G8ecxfe7eEUNxAY").unwrap(), hex!["f8400c82a8c4dbaf6be405ddb63a369c543bba096462fff596bf5581d401ea55"].unchecked_into(), hex!["aa0b905a56462a6a51f500dfd4231b85da09f4cfa6a6246d0dae6dae76f78b0c"].unchecked_into()),
        (AccountId::from_ss58check("X8QB4dLhX41vuqtqCAidG38mhd4bKbXKwx4zt5W4unAmjT6").unwrap(), hex!["7ec007ca1082275cd14ae2a829ce1a39e10eba60669f315ffd5d274050c5fe70"].unchecked_into(), hex!["8e3099fdebef9d44a4e86c71c3899fbe488548fb031189f8d39b3629bee21f54"].unchecked_into()),
    ];

    let balances = vec![
        (AccountId::from_ss58check("bUX33je1sSEQN1WerLnpYqdnjV87KPdYYv1EBHVzkS9Nwx8").unwrap(), 29989516668868002288),
        (AccountId::from_ss58check("bdrHXtQkGHTJrpNErtyGmfhh3wtwy3uX7wGRZDHJShS2aHF").unwrap(), 9751936046333727),
        (AccountId::from_ss58check("aHSNN7fWryhBJn7o1uEBW1vLXsKSnRDKWUEBjJW6FPBZoiv").unwrap(), 16387596802882729),
        (AccountId::from_ss58check("bQG6iWvD6warVBnTt4r7UHR1nesZ8iRvy7MUDi1MCQmPqXk").unwrap(), 11135813174837015555),
        (AccountId::from_ss58check("bAMzvU6TpN8EzL4Rjuv7HgMErEdQZ7j2vFkGUGmRKyrPdax").unwrap(), 7349636695392430266),
        (AccountId::from_ss58check("ZoR24Kp1bzo4ASdwffZ5h7Z9UpX9Gx5DiJTEeqGVboULVdP").unwrap(), 8546129014196552),
        (AccountId::from_ss58check("WsdR6dh2yQ8qwHFGZZL75EHvt5DZ397orXBTqFMST9YzDJb").unwrap(), 105845352948945890),
        (AccountId::from_ss58check("XqbJjWyRVFcZKvrtszJC7ebzJCe7zQMaPhekQquVcYSVTQY").unwrap(), 21387434130596294420),
        (AccountId::from_ss58check("b6pnVwqTCv85gvHMMSp5ZM1QxfgSSkTY1BrUfgvtTgJ3UTU").unwrap(), 22271626349674031111),
        (AccountId::from_ss58check("bRifnU5LavQRQJGfQ8JKAjZL1kP68yGKnyf27s9NXmXx2jj").unwrap(), 1984600367792735445553),
        (AccountId::from_ss58check("bVzxMKELvi7Lg2TRiAdqskgAhWR3marwVSNbGg8mZKP2neg").unwrap(), 176408921581576484049),
        (AccountId::from_ss58check("YRLpLpUscgpBwRc1qwF2NZZcUHzpwLrEWLQuoLYrmai6Bop").unwrap(), 297548928031645055643),
        (AccountId::from_ss58check("a7xqDaN2GD881JNihJvRZLunDoYSNSt16D1oQQQq1S8iEJi").unwrap(), 756317890738416),
        (AccountId::from_ss58check("Vyxp6DcCJST2z5Lz7EHe1gsZAxu2ToBkZvtZgqGto1aCvKG").unwrap(), 1764089215815764840492),
        (AccountId::from_ss58check("bEz5gC6oiZPM7yS1WfzxagRiPEfniJW5YkthXnyw1SFMAEU").unwrap(), 26461338237236472),
        (AccountId::from_ss58check("WJvHQSTh7rgd95CVeLzwTdcx6TqvM5rXB5ppmq5C7Y8s4NN").unwrap(), 18845228979006941076),
        (AccountId::from_ss58check("XiePHvby6DSBKyDWpnDeCc53Bt2iutJ2CYod5uaw38S9PJC").unwrap(), 193924324676133150),
        (AccountId::from_ss58check("b1ovJhD5CfLoU1YWb3XGgWoouKJUwL5TLFcB47ioVdQp5Qh").unwrap(), 54767442126382083),
        (AccountId::from_ss58check("Yv9QzUUzQe7wsL2YD9cQnZMf5j1kMduLdrMXnNFdah8CY6q").unwrap(), 31753605884683767128),
        (AccountId::from_ss58check("b9sBzZkJ8QPfwv8PvZA1T3q73bXNQ74HE5ZrhvaMc3WFiSr").unwrap(), 8327869899311575),
        (AccountId::from_ss58check("am28w6PXVPJXzkoQn1YoXk129TWx8XbB8KTAGyp2HGrqZvY").unwrap(), 2756389399712132563268),
        (AccountId::from_ss58check("X2CqbKhYitPF7iaw4hNXUfeWy19TSHFjnZx8pyXDdpQwdhh").unwrap(), 286664497570061786580),
        (AccountId::from_ss58check("WxhwUmo5atM7g3KJv7YdhZZ4s6YH4oA35uKcj5E8Z43qs11").unwrap(), 66814879049022093333),
        (AccountId::from_ss58check("ZF9xVGcCDTRZsniuVMPBbpgaWkCTe7Jq9mUXP8miF7GKAAe").unwrap(), 16423097270249630541),
        (AccountId::from_ss58check("WzrepRrYgAwiw1dTBtSV2K8Q5LCvSMM1LdBmKAqxvrTywua").unwrap(), 1764089215815764840),
        (AccountId::from_ss58check("XDn71pH1XxRg2NYQaNH9pcgwoR5t2Yk4YHtNY6yrAbp34Wi").unwrap(), 9594121765572483658),
        (AccountId::from_ss58check("YgG1CPpJEqPTA7JhvrVrfbkVDTm6mQy3BrQ1ms2kAvypkzt").unwrap(), 17640892158157648404923),
        (AccountId::from_ss58check("axQ1Zez8u3wVXEsJ29jpUuBCMfmCZpde1u7j1JSKqfGXBYR").unwrap(), 19333303675468731132),
        (AccountId::from_ss58check("WfMhiUCcS3gL2Rs6UYN11MkdKAHA8mfQ1bzPdCXU3FWTmmr").unwrap(), 16104227919921018669),
        (AccountId::from_ss58check("Z2rk1siCNMzrEwAZypXcFQLmVEYNSq7EFP1GmcZuHMEbxtA").unwrap(), 303116834619063563422),
        (AccountId::from_ss58check("XHMWskdy4N3jRaYx3bYNYJR7JchYTrvSHenRTq88fTLtJ6N").unwrap(), 6615334559309118150),
        (AccountId::from_ss58check("aGS1nLfRqSrpc9g1M45yLtzsqByWXbviHoFWqq2oWFojuqL").unwrap(), 739021074735619285802),
        (AccountId::from_ss58check("W8CpP5J3nkp4737S3eNXyLi5cnaZzthEheGhbYpwEL9EjtT").unwrap(), 16197532110963386631),
        (AccountId::from_ss58check("XaksTAN6Z1bZLi8iMoDcZkuMtoMSrLXSyT9M9CwKDhbJfej").unwrap(), 9594121765572483658),
        (AccountId::from_ss58check("aqMhtFmPgUE64aALHQ1di3pQ39XvEEjvRmZAL3EzG5rFVcp").unwrap(), 8542952021074210),
        (AccountId::from_ss58check("Xm1H6JAWHCqDFZWKjNp2NF8s4DAK4qQ5gEsVraMP4QgmSFW").unwrap(), 2205111519769706050615),
        (AccountId::from_ss58check("XLBJzSgTsrWst4VhMAP6evGGHjk7g4V8gmt4YUuYkb25xA7").unwrap(), 6879947941681482877),
        (AccountId::from_ss58check("af1DoWUqorcMu2S4b9HGKxLG6FvHTCyKWKaHUQazSCeiNmy").unwrap(), 3969200735585470891107),
        (AccountId::from_ss58check("YAK8ZJ3MpqeXErVGYq9kGaWdD3QPvJC9ZKrFx89L7yjRUML").unwrap(), 111358131748370155555),
        (AccountId::from_ss58check("WdGUzQFw8B3zsxuoL48ns9WkxcphYgV6Ct82oCfxmD1Kqsn").unwrap(), 16105747568365465212),
        (AccountId::from_ss58check("ZdpcNLPNTMZXMZwUQ1ynEeTCsGSy3zDWvsY5ud9PFPiSAVM").unwrap(), 869695983397172066362),
        (AccountId::from_ss58check("ZUCDxtBEWVi59qaa4B4TfSGE9thYpP7nVGgqRPn7jczXeEt").unwrap(), 90996999625000000),
        (AccountId::from_ss58check("WTMJByGbm2pspg3WdsgihyafozjZjriaCYTZuAmhzTnGvax").unwrap(), 17037646505162858958),
        (AccountId::from_ss58check("axGtGyvwKUi5wNujf3tj1yVmmSo7jQBad6XEvg3G4tn8Pz6").unwrap(), 529226764744729452147),
        (AccountId::from_ss58check("bVxEw9hcY3gKRGb9THwCocAZ7Ww3cQRhgj7EUtnosBoNket").unwrap(), 19383972822788199788),
        (AccountId::from_ss58check("b5YjhAh2RLiubh4t9RV4dAqYRiuahRAQTsVY7FwpRsLwQLF").unwrap(), 198460036779273544555),
        (AccountId::from_ss58check("Xaw8hcZvBis2cgS1dUSz28vsZREMNT4P13shJJAegDLk8CH").unwrap(), 1378791465243306),
        (AccountId::from_ss58check("WGHQfDJFKEuEjpuiW95aiuk9errvhRSGB3JHJs6uc2mpoL9").unwrap(), 500000000000000000),
        (AccountId::from_ss58check("Ymd56UJivQLRjiKBdU6ZSZqw6TweyJmVVrak6B78ddSig2V").unwrap(), 38589451595969855885),
        (AccountId::from_ss58check("ZaJi85BQtBAAWjhK29qoqZrLo79CC2S3REenpD88H5jETEx").unwrap(), 7059389150241079),
        (AccountId::from_ss58check("ZDvvyccY5tt7nrpk8hDTx5zUMBAQg75BwqjTgmw6gd5qT15").unwrap(), 957679933035983337780),
        (AccountId::from_ss58check("bczJ6gamrvnTJ76ewyoDKbz4oUjSQBWv1Fwj4VxfSfnYGJQ").unwrap(), 209485594378122074808),
        (AccountId::from_ss58check("Wr1VKUaNbSMLgiMq983TyetzX7v3utWZzGoWpXXcKutJopB").unwrap(), 99999000000000000),
        (AccountId::from_ss58check("aaVm288nFvtEDfSoAequ55YQHLGnTZsvcNR6iwGDeNMggbR").unwrap(), 18930934684773900332),
        (AccountId::from_ss58check("Zvk8zRU5guMggTP2eJZX2Dsn8cEKJCJYooWw2cUjcGLeAfz").unwrap(), 174644832365760719208),
        (AccountId::from_ss58check("WAL16BXdW9DZG56ybn1rstC1iwAHeQypXVeyKS9GKgW2Jhu").unwrap(), 30066695572059941999),
        (AccountId::from_ss58check("YhmNUMhAm27pYsnadDLmCVVPTZFexDETTq3T6pYr9TydcR1").unwrap(), 17640892158157648404),
        (AccountId::from_ss58check("YjLPZdySPegq8tq4cEkzePv7gsDTPz2UEfBNr7hvmqUU9Fm").unwrap(), 3836894044399288528),
        (AccountId::from_ss58check("Z7GoUWEZs6fdUgBoQEXmavmW4xWYM8efBzE96A7tmMbFjQe").unwrap(), 44543252699348062222),
        (AccountId::from_ss58check("Z7dAuXaNyKmm641eij56Fmh3dXj4Bdx2LSvkurAfQyMH8iS").unwrap(), 136716914225721775137),
        (AccountId::from_ss58check("XWDs4RiaPWEpWFfnFSbDG24j13kAnGX7teR5fX2u9fHfKex").unwrap(), 1268380146171534920314),
        (AccountId::from_ss58check("WzWaNQWbLyLBoghehG8r3cvVLpGyf4GsyRZ3LL5qpWZyV3B").unwrap(), 294944824222821365),
        (AccountId::from_ss58check("YnydwVLDJyDwqnufAmV6QwFeRCiYTrbzjSvCc4GfkMzoiSE").unwrap(), 2227162634967403111121),
        (AccountId::from_ss58check("bJoiZZkvUyBadQffbx5ti2bqFWoHMWPutLf8xncDj1LSV6h").unwrap(), 100000000000000000),
        (AccountId::from_ss58check("ZAvin9PwDdqhsxEZsn32AErHvYhAj4tPunoifLzFxRuosxH").unwrap(), 2458832092256712382736),
        (AccountId::from_ss58check("ZmqAicR5P8b5fcU4wANmF3cjo7qyouWRfaDfb4UTjwDGPie").unwrap(), 19977648835657605906),
        (AccountId::from_ss58check("Y7QcJMaDrzanYjd9nNhH6LA3ihYEcDDfAsxfXAd8zGe4817").unwrap(), 98429903999901947),
        (AccountId::from_ss58check("Z7wRySHUkcVVuKUHCcUxHXUsYRM6jNsEssU4VXHBzNkV49a").unwrap(), 306030854210654913653),
        (AccountId::from_ss58check("ZSfLnCppbtBjdADUJAX1msBNphzXizxw1oCvsSJLaXoqStT").unwrap(), 52922676474472945),
        (AccountId::from_ss58check("bUfebYnUSk7Pd4hUnEfsfThmfFvsvQSYnXsmkFBWEQS9ZiS").unwrap(), 222733904388898468759),
        (AccountId::from_ss58check("ZYVDgTLBTfPrhngBnA5pvzqhYhbpB7AZPvPrpTJtev5bh68").unwrap(), 7406984764158105),
        (AccountId::from_ss58check("bBFA7kCXoRQtrq8kWkRmmsNyFUYpxc9KxotsT213UtDRfnb").unwrap(), 15434457571476080527),
        (AccountId::from_ss58check("aki7tADKC2WgTJC9QYekk4kLpecxAQPCYZWb2HkqcZ3Buka").unwrap(), 517649929265938495381),
        (AccountId::from_ss58check("X4RkKhzg81HWcZVMru8891TbXefWEUCmL6yw8qX4tbwwwJC").unwrap(), 7216006937294386080033),
        (AccountId::from_ss58check("bfiz1UQewvg1Vnd2Gg4c6swuaYRpwmijMGxqnkkuaby8t1a").unwrap(), 1664969453002116553516),
        (AccountId::from_ss58check("YudMUXFF8rgyr7r5XzfAm15v7msx8s3ZzcCBEEkgfLkxmEj").unwrap(), 365410573519101828439),
        (AccountId::from_ss58check("WJacKmyGWX1NS7jxEmgFUnjneiGRfgza7fVWcGKwnmoCGRd").unwrap(), 400889274294132560001),
        (AccountId::from_ss58check("b1bx7VVxFpdWbvAZMNfU1nuGqxQ8irFEgnDnciF8aLEEzMN").unwrap(), 1000000000000000),
        (AccountId::from_ss58check("aVVjS5k8fYAtfwCSngWTv2bkfYtP7TA4DQ7PXpHVFrGea11").unwrap(), 1362758919217678339280),
        (AccountId::from_ss58check("ZUqxS3s3sg9CvLHxShqH8K6WU83ecisUGrtgf6LXBPh81MR").unwrap(), 5512778799424265126),
        (AccountId::from_ss58check("aSoC61WDPxGPtgZX3nhiPZtsVFpWWdb7KvzkXjYmgtMd3rB").unwrap(), 129219535058504774565),
        (AccountId::from_ss58check("bR52QQjtjVygLZRScmUgLBYrZravsV3CZDK9m559r77BV66").unwrap(), 17815000000000000000),
        (AccountId::from_ss58check("Z84kUCaZhX9ZG7fqPMPFwerxvnyVyP1DNVVd8g5RuFtSAxw").unwrap(), 249968580971027997),
        (AccountId::from_ss58check("WDaAqcYqYFhk6aYHefQ5CaT1b8yWE7w1ULwZHg9vhpg2iyx").unwrap(), 16107281607278242449),
        (AccountId::from_ss58check("ZjRyNrzhgLXML9nm7JLQBfRyC24h1aJSfRpTZZNgNgJnnmD").unwrap(), 33598137369716771040),
        (AccountId::from_ss58check("XXsZh6ujcZocSgqkJK6HhUA4vxuiHdgyrKJ7e5mh23gvAvK").unwrap(), 976000000000000000),
        (AccountId::from_ss58check("acdCpja2aaEw1m6c9QjDBkzrQkfdWAPPHb5DsGTaLpkU2iH").unwrap(), 44102230395394121012),
        (AccountId::from_ss58check("abhqGNEdXra7koSvdgAgQVNbKHX2EUVxir77pKkZzD1HpL9").unwrap(), 220511151976970605),
        (AccountId::from_ss58check("VxFSAprVPQzehnDdfth46rEvTFuDJo5orNBSgRcj1NLWQ4f").unwrap(), 8908650539869612444486),
        (AccountId::from_ss58check("X34WSQpM7YL4MDTZRwbffVSWcAcmaHtm19sF8KBdaobdi3Y").unwrap(), 224921375016510017162),
        (AccountId::from_ss58check("WHqko9UZqBGv4dUihjVuMWkjMXdkPH5HPSTDdKowjVpSvzQ").unwrap(), 867579076338193148553),
        (AccountId::from_ss58check("W5F25zxK2ZF3Uv6NnVuzuieHrg8RzzWa9qvtfSCA1HDCr52").unwrap(), 19345211423183850045),
        (AccountId::from_ss58check("bCP338VBLsmk636T4xg913BLrKtWcmpHchM7JtK3fMR5Hq5").unwrap(), 1000000000000000),
        (AccountId::from_ss58check("XN8jAGM3z468tPwJ7ArUHcJEeWjbY2wZgugyvGTEGE3xnmE").unwrap(), 2019868921439932124125),
        (AccountId::from_ss58check("XL3UJcm1iBcurbhkmBKA9jDhTQnx4yP9dnnGtHZ9RnC5yvF").unwrap(), 78590174564592323643),
        (AccountId::from_ss58check("YMhnS3oTJyLg3HgJKUdqegq56z49zvutvzfXD7s9DNEKxXy").unwrap(), 2205111519769706050),
        (AccountId::from_ss58check("WovWh8HjaWXzUS4ks2aBHxee8gSzNT4mrG6CuPq2fb9bpB4").unwrap(), 531872898568453099407),
        (AccountId::from_ss58check("Z9FLqGrjPyNjbRnYkz6QRrbAzB3tVV43ke6Rwaoi65VS7gd").unwrap(), 672382604608178768953),
        (AccountId::from_ss58check("ZvbNr57RLVhXWyw6yDwMEmL4E6zJZiSDCcBhojEWievREcf").unwrap(), 20000000000000000),
        (AccountId::from_ss58check("XQ4rHcAdv6ajNfkPo6wTRK5QHZNQtnWY9HodzsfXmLcnGw2").unwrap(), 11135813174837015555),
        (AccountId::from_ss58check("ZrhSCBNxVh5pJJsdX1q2k3tu1yffwVwcSVXNZsS5puEdKax").unwrap(), 37182751443974313817),
        (AccountId::from_ss58check("XtT8aQNCYCJDw53T3xt2WBh3ZYWY74AgrJfJkH493zN6YEg").unwrap(), 3964349490241977537795),
        (AccountId::from_ss58check("YiRB4BL3Eb46Gstu1zufAEdCTfRjsREHTPGz2iNzh3sWAVr").unwrap(), 1143129811848615616639),
        (AccountId::from_ss58check("W5UbqH1MtvduPZ2p2RKTVrwLRhQwvEGZxDT9N89Zyy8j7P8").unwrap(), 21386987051821069021),
        (AccountId::from_ss58check("Vws82RrTsECumPe3R1rWpU5AG7tTn1oa9PuVtrinMij2vLa").unwrap(), 97057073270248095),
        (AccountId::from_ss58check("ZiHe6nBGr7k11GZXPR1j4biuWMw9DNK1X936wDn9pyv9R4c").unwrap(), 60839817720911300292),
        (AccountId::from_ss58check("arvGgmFB53XBxY4KsjGXYGxa1CRSJp4RZF739EwXZ5ZZiY2").unwrap(), 783961247508525895114),
        (AccountId::from_ss58check("aiBuXnbxWn6ieKcM6weRkugT9oYjREpXuTY6JGXRSrNCNoq").unwrap(), 110255575988485302530),
        (AccountId::from_ss58check("XXK2bf5yPJ4mYGPLuPaYvDAtzEdDm6hSxhNgac4J8xDNn6S").unwrap(), 70785993996998750000000),
        (AccountId::from_ss58check("WmKGG4vSczmQ25b2VCkYsg4WyKxRnGnH3iP7W7NGiGsuUs2").unwrap(), 55679065874185077777),
        (AccountId::from_ss58check("bRaGt9T9purdkYiPf5xbY9Ug8sCSCTUJjwjGSdvpNMaNqDV").unwrap(), 4410223039539412101230),
        (AccountId::from_ss58check("Wqf9EbTdnR2HWVbansVnsm9WzfaiBbxctWXwos6rP5LSSv7").unwrap(), 26461338237236472607386),
        (AccountId::from_ss58check("b3a2bYDFoHm26rbzqDMjEiFa2sN7V9x9rCPhfcubuLBzA7h").unwrap(), 264613382372364726),
        (AccountId::from_ss58check("b5HamXTqrVkUXGzuCiT7uLMCHocaAkwTThsdNCUfQzY1eea").unwrap(), 27122871693167384422),
        (AccountId::from_ss58check("X8KKVkhUgebcRcsEYLHD9L5jiFDCtdvJX2CSzdjCetrXaVt").unwrap(), 12249394492320717110),
        (AccountId::from_ss58check("YyThb9NXd1ug3dz1sNy1Rr4soQNxnnrBLjb6MtvCBhS6WLZ").unwrap(), 9499999625000000),
        (AccountId::from_ss58check("aoRtSYjDF3B5vHKu64ps4D7nJWzizcyxEkibdP4eRU5ZDXg").unwrap(), 690420416839894964445),
        (AccountId::from_ss58check("Y8G8ke6FUknfeBHyphUpDsgwpu44mYoaj8kCf91ZhMndz7E").unwrap(), 1380840833679789928894),
        (AccountId::from_ss58check("aX49sX75rC4RyHKzYusS1EdWyKWPoSjU68k6LRQbgT9VtBG").unwrap(), 2007999998500000000),
        (AccountId::from_ss58check("Ym5q7PvtK5UhKrpYQWX9JcgCrYRSGXeGn7XGFBuqf4Z382F").unwrap(), 10000000000000000),
        (AccountId::from_ss58check("XdY8Mnm3ucrVUjFWm4SF88LBsZf8yBX5vbi1WRP4YAK1gfp").unwrap(), 1000000000000000),
        (AccountId::from_ss58check("Z7nvTz2YT1HY4BpAwjvKvDxM3n6RLwTYjt6Ur9MFrYKt1rp").unwrap(), 1001000000000000000),
        (AccountId::from_ss58check("Z98SVj1aHqCyMsvvqJSByXom3BCpiHUnAz8zstgfbLEReR7").unwrap(), 8017785485882651199),
        (AccountId::from_ss58check("W12kwJXtEXhWnEsH3LcNBoPkE8qLCxrakKrB5RdmPJfqU37").unwrap(), 19329229448645184094),
        (AccountId::from_ss58check("WhRHa5pLc5K1K3aujMN2wLto7aZovHCdqG3M5rWvLbZL8dr").unwrap(), 9050473414439665),
        (AccountId::from_ss58check("X7ugWFHPBbK66qgmqd9XD311Kc6tic4vK8mGn4cL9qDS7wc").unwrap(), 99009507237659801672),
        (AccountId::from_ss58check("b9QAwYAtU2EadFRQmeeziv6CKKeh4quVTTkjaneLYcVgd41").unwrap(), 100000000000000000),
        (AccountId::from_ss58check("aaZaY4b1xj9MWs6tnb5T1wdJyymNb2NRWJ92my8poGYxFTh").unwrap(), 765263029540066),
        (AccountId::from_ss58check("WT6KnmWysXt2n2YiQv8QB69sFTSFFgFZBh7vzfTdLTkr3KV").unwrap(), 1377918210708470432620),
        (AccountId::from_ss58check("WmkzWAEMK5DSgd96MX1U9vpXCyzSahoMyXfZeFsG8YEbVuG").unwrap(), 18840196223016204449),
        (AccountId::from_ss58check("Y5JZm1K5HpCAx4Zj55W9cf2d5PuFFhd6F5xPXbSTQv46puC").unwrap(), 498939883776862),
        (AccountId::from_ss58check("aNY3n4KnCbepMDbbaYkvcmRYFPT32MZNjPq5Dk6UQVUtjUC").unwrap(), 8821445079078824202),
        (AccountId::from_ss58check("XyAnfn8pgavroJHWyZLxyVukGKfDchK1gXuDEzbCHvoc9Z4").unwrap(), 26725951619608837333),
        (AccountId::from_ss58check("YKBxVmQkeDFyH6WjbwiyfqkJsuy4J99mSaEVSrK4hiVkxEQ").unwrap(), 134070780401998127876),
        (AccountId::from_ss58check("YdN4BSpCm1WiBAN56xeds5v6NKEqf6NNZ2S5a2CYBDEng54").unwrap(), 14994758334434001144),
        (AccountId::from_ss58check("Z9roQrB41XbTtkmDw8xNRB9LbQGLQGSh8qRkm9cgXPW4eQ8").unwrap(), 499999999875000000),
        (AccountId::from_ss58check("WL82fLsjbX9jo46qeVx8mrwkRToE6Bxc4q7Lq2kPhJp3zxP").unwrap(), 53451903239217674666),
        (AccountId::from_ss58check("bVBpGsnXc6rZwjH6j66fz1nyd13cyNSCQ9uoTugh27XR2ZM").unwrap(), 55127787994242651265),
        (AccountId::from_ss58check("abfjBd86PknUG25PTaitg9qu3azzwQHeUPpmRc28tQoifLq").unwrap(), 1190760220675641267332),
        (AccountId::from_ss58check("aKY8RFX7UgiRGKgfqQEv6TihQJ5tHUzSLgkSE9YroxGhgsm").unwrap(), 55127787994242651265),
        (AccountId::from_ss58check("Y4pf4u6brVgnNMJUGqrjNqGwcRBksRkHKjkrWkjLi81nvVe").unwrap(), 200444637147066280000),
        (AccountId::from_ss58check("YDm9dV2S6bNHnaNAEno7kUuJL5EnUnXCkfrxJtJauGosQ8h").unwrap(), 4998000000000000),
        (AccountId::from_ss58check("YEvCVRjdjy7VomTy9orDgaKsgHBxD7f5xcRSeFbB5z3pCv3").unwrap(), 5167017313124375217800),
        (AccountId::from_ss58check("Wbyn95X26NJMnUUDSxt84EnXrtRhaHQAdamwRo1dwhES2ZV").unwrap(), 1113581317483701555560),
        (AccountId::from_ss58check("VwNbP3Zx77te3URWmjD17af2mVHa1W8ictc9wF6K3vYXBkR").unwrap(), 97065391508262452),
        (AccountId::from_ss58check("ZHYrRtiexRL8PXgjev3CcmbbeX69Erw2q7jvBrRZANkKZYZ").unwrap(), 16935256471831342468),
        (AccountId::from_ss58check("bHYK2eMSxuN1zu8ShMAnccogyeqrdprGpRcWKK2w1fhSLEr").unwrap(), 1113581317483701555560),
        (AccountId::from_ss58check("YBor6cNg7P6z6pbVriHkvDWrZXcfawFRtL5nd4WKwAy5bun").unwrap(), 2271529478745169596859),
        (AccountId::from_ss58check("YUQaMJHwKmzGGFUfSeH4EKxV9MGwdtr52QG46uAce2iVjAh").unwrap(), 16197228400702967849),
        (AccountId::from_ss58check("axpVRBhe8zoUkWiXc4wQnWjiiiWRYNAv6sraqg2Eo47uVy8").unwrap(), 19333304483688201615),
        (AccountId::from_ss58check("aC63ycFa2KGBj2tyigs7VUrmXvVhCEE8iuLX1jZNmZYweHR").unwrap(), 8908650539869612444),
        (AccountId::from_ss58check("WmzKjpsNAjuT1HXZoQC5NeBFBbBEmbuV8s5HZiaLjZRAuvD").unwrap(), 343997397084074143895),
        (AccountId::from_ss58check("anfshGYjHScvZbxn5WpUK9cVs7hKtRwwVG5xEbKuNDb7usy").unwrap(), 267259516196087),
        (AccountId::from_ss58check("Yqojsj5cno5GLBjviNPCGWKEHtDXZmHGZB34UnvDw72U5Aq").unwrap(), 534519032392176746668),
        (AccountId::from_ss58check("WBD9ggSeNqAwiBjwkd1M5NgB1AAnwx1B81dsPnAMzdk1yZY").unwrap(), 9000000000000000),
        (AccountId::from_ss58check("b8zba17PpySJNwUrngAHH5q1a62xs7bTwoJ64uhJVQ8M9bD").unwrap(), 793840147117094178),
        (AccountId::from_ss58check("XHWxTz4QE7K8jmRdDuRW8vjrZziSpDwRy4Nkdhn4pJgpjYq").unwrap(), 69916368401578184893),
        (AccountId::from_ss58check("XuH35opmreeFtvWLDkNddLFEbLQJCUDGZ8MDGf16uFTxZnT").unwrap(), 16593547132937360093),
        (AccountId::from_ss58check("XjKBsg8UvGYdVJeh8NUBB4UKM21J7o4J1UP9JVG3TU6ChyE").unwrap(), 1093715076284718),
        (AccountId::from_ss58check("YhB13XzqexE67LkhDZktHQc6JeawCCvf31Bh6oQXqW1Q8gE").unwrap(), 55127787994242651265),
        (AccountId::from_ss58check("Wh2nf6F5ZNJguoQu22Z361xo6VFqX1Y2BuQMcJBSJxERh5E").unwrap(), 17638128068816832639),
        (AccountId::from_ss58check("WhvfSA7tEiR43WDXzmLKUVxqAe1QihzpnsXSXDJXBAfnCob").unwrap(), 242551241617068817037),
        (AccountId::from_ss58check("ZSU3BV8XvkeD17sua1yQh3eTzqa46wY5WojJJeTdBHfM89t").unwrap(), 1670371976225552333340),
        (AccountId::from_ss58check("Yd5PkTaHKmxvV1QRDLndfT21a5DV1fv2eyEYBUsBEc8dHUe").unwrap(), 53451903239217674666),
        (AccountId::from_ss58check("apaNTC8x1KW5gCb8qetAS1swbvDSukoUJKzfyKy4UGnWYPY").unwrap(), 15999204365324),
        (AccountId::from_ss58check("b8HxYWsZshqxr2uSffQXMbY5D6AjikxjXk6dAUqHcBR87J7").unwrap(), 4454325269934806222243),
        (AccountId::from_ss58check("Yy8xtv84TE7ZqwLmTtjmBec6Y2T8MRZmbqNJ24dK9P5tVRZ").unwrap(), 5679264719166877933),
        (AccountId::from_ss58check("bJDye7uVsQbs2qYdF2C2DNv1sRt8sNzQXJrkBKz37L7dujw").unwrap(), 1290096084418226985498),
        (AccountId::from_ss58check("a1CVPK9sx9mLUJadH1dT6ogVJYVMEcZMH21i4QJjULeEkws").unwrap(), 26725951619608837333),
        (AccountId::from_ss58check("ZUMKSTi5e5pB1298Smi83B2qemsbqCtr47aMiuW4c5cyXkU").unwrap(), 308715612767758847086),
        (AccountId::from_ss58check("WMvEtgcxfv95fKbt4inBPcVU442t5dWcCbqBpSfnHs6S3Ro").unwrap(), 501111592867665700001),
        (AccountId::from_ss58check("ZJEc6ZXwZ1oq76Hc37ocUEHP8H54KYEoP8x1h53qLjFmdc4").unwrap(), 19228332248029100873),
        (AccountId::from_ss58check("Z7qAye1S5Zxjc7q9zFNddpFXf2GdMRSTFYHdfNvd8EHftHF").unwrap(), 200444637147066280000),
        (AccountId::from_ss58check("aMbNNYgpUXmZGyKoWDWBx9yVLmaZXdY9yQR4usKiZek8bAi").unwrap(), 11025557598848530253),
        (AccountId::from_ss58check("Y3BERi4WVrwE9DvwwgY7Hp5QY2vKwHxZo8U3M7hnRsJrLc9").unwrap(), 170675631630175248317),
        (AccountId::from_ss58check("aq1JXpWCoPaFnHYAeicbW84DEsS7p1pCBKUE9RZDMRotNEG").unwrap(), 21386715178321755100),
        (AccountId::from_ss58check("XowsLNTrd17Qd3bwapmqtXZ7SuBqKJJAMNrX1nEbSVGJpHu").unwrap(), 16597946726825904988),
        (AccountId::from_ss58check("X7nPaZWVN8qgkoyiSKBCtzUwnoZm3avQzSmdgnXC5m6EqX6").unwrap(), 52922676474472945214),
        (AccountId::from_ss58check("YefdYbic7f4bRAt7qYoG9V2D6Njg52jii4iy6A39RScmV7F").unwrap(), 629173444378291378890),
        (AccountId::from_ss58check("ac7qCpY11SaBc1AbV94Exzk6jimS43HUiFyrubveXRTfPu6").unwrap(), 1000000000000000000),
        (AccountId::from_ss58check("WSL1S6TxfETdTWVt88g8w7J7KGb9h6JvDNBbjSrCvmvsVpm").unwrap(), 2807229121472869),
        (AccountId::from_ss58check("YoRtEd1puKw3QzYk5FbEBBA2Dfjh2Z5hFn3HHmAvpYYb7rC").unwrap(), 50276542650749297954),
        (AccountId::from_ss58check("YnidTvzCFeA24u98Dto9vCXEVjSwMEFG9RXr8o1YPGtZtRW").unwrap(), 111358131748370155555),
        (AccountId::from_ss58check("Yt1knkoVo1e4Ya5YGUcZQh8EFvUacxmtZ1mvGMNoztYRMUj").unwrap(), 356346021594784497778),
        (AccountId::from_ss58check("Xx3DUBfkFaVTAgKCaVXHshVvGLdVq2MYt1ZtBFvM28T325y").unwrap(), 52922676474472945214),
        (AccountId::from_ss58check("aNLVo8iu6xGDgmxc1XqYgjuVQ7DQ5ooViLaNdDZH3DTDkHL").unwrap(), 1102555759884853025307),
        (AccountId::from_ss58check("ZyzrEA8Ckhr5waMSdfUtWMWyVssaZuPwXF1M7SDkMYd5qSS").unwrap(), 119076022067564126733),
        (AccountId::from_ss58check("YAz51SDiPNNfYhdr2T58q31cf773t6nq7CZdJomXMada2EE").unwrap(), 16107229913009468313),
        (AccountId::from_ss58check("bbfUhdwC6U4JCwF1VeXgFcMq7heb4tnfVFAe8LxsN5iE2hk").unwrap(), 9094084980930209),
        (AccountId::from_ss58check("XsHkQaY9bb4q4f66j36TUN4YAtpeVMEfwzSYHeM8gvhKKBG").unwrap(), 22271626349674031111),
        (AccountId::from_ss58check("ZHqciH6HaE18yxxkcRLFrxLikxrrw6Bxcmhg1X8dNoryika").unwrap(), 19333303194054449147),
        (AccountId::from_ss58check("bRAxzAacTKqyRrJwpkT2GBsLRA4o6pGnKVvfpiShdNQ4Fen").unwrap(), 144320138745887721599),
        (AccountId::from_ss58check("ZS3kKGvt4aqXE74DC4ShfNp8PBao3P4NBmaQsWEogMxx71C").unwrap(), 19083307991291387511),
        (AccountId::from_ss58check("XUVyjSXJPmVRfcn26PFwqx19PgmR69vDH63dpGRwG2WAJN7").unwrap(), 368694646105494851662),
        (AccountId::from_ss58check("WSg41PPaPE6VBVyen3i7wBy1Mcm5rAaN66bVorFcQpbnHdV").unwrap(), 26461338237236472606),
        (AccountId::from_ss58check("ZPT2Q74Ng4DUTyQLMCje3ULdTBEyt7ptTxZG4dh8uzxVsnV").unwrap(), 976000000000000000),
        (AccountId::from_ss58check("ZCXzwj25Mbs8ZwhAjcTvdYZ6kmYGKBNk6Ypuw7CsFUUCU1C").unwrap(), 39180421483268137105),
        (AccountId::from_ss58check("a98aGRTsvFFifCb1i8LbHsPnvujtgFki8evL8DcuvKh8P6m").unwrap(), 1524173082464820822184),
        (AccountId::from_ss58check("Y3VskPzPp8fTatjwhoAiz1My9HNAar8gUYT631aVHFH6MHT").unwrap(), 606405667936669163919),
        (AccountId::from_ss58check("ZwF9rPr1fk9iwoJr6mC8U2q1jDeHDxkQVLgumf3ADav8fTT").unwrap(), 7772797596036236857),
        (AccountId::from_ss58check("YkXGKXvee1doH2Y8xthtBZmHkH1jtcKuje4KKZ2HngoXZ2J").unwrap(), 499999000000000000),
        (AccountId::from_ss58check("Z3NqV3SwJ4xatCTY2kQLfBwzF5WhkdMSP2KqhYGUfpPjiLM").unwrap(), 60135395590491354663083),
        (AccountId::from_ss58check("aFoVoAd2p2w9Nq2GZJ88pUCB3bPf5ouQZy4rRs5Yxasqg1r").unwrap(), 2421212448707137243575),
        (AccountId::from_ss58check("YcQicwUwxqRDBopEsdd1kdaa9Be3xgEU172e8VQxCY7GuwK").unwrap(), 178173010797392248889),
        (AccountId::from_ss58check("aeDf76H2gR9YoB5Up2pdPvVhqJksMXDYHDeA42fe3bvtH9q").unwrap(), 3619845254339315),
        (AccountId::from_ss58check("YJxtF3uPj5yPh4btCHxuD7SUtzfHpPRQXV2NMwjSiBWke7Q").unwrap(), 1001000000000000),
        (AccountId::from_ss58check("abi566etE2yRKrM9FMTzJ8Cika9dRGQkfZcWtXf6xEBLBgP").unwrap(), 110255575988485302530),
        (AccountId::from_ss58check("YLEsBo64PjFMuRBct1KamrM3nmWN6VhS4mbDp5ybZMGi3fR").unwrap(), 110255575988485302530),
        (AccountId::from_ss58check("ZMTrxKrPahfoxXbLRbJTYg2GKpfMUvjB3Ss49u2N29JDRtC").unwrap(), 264613382372364726073),
        (AccountId::from_ss58check("aTt8HUHtswmDFT8hSgtTN5uXozfK9MG6t8rXPPg2jUUjUz3").unwrap(), 222716263496740311111),
        (AccountId::from_ss58check("WywoJ3jqnpCeFchBV3dJpjHWhDiD2C5UaDm2Nk4FaV2aNLi").unwrap(), 20899185264960309003),
        (AccountId::from_ss58check("XZ4gm9yDYz8pDrfuGZfFCkTNWZsM8nWYPWDjkkUPbwR7UEL").unwrap(), 31312583580729825918),
        (AccountId::from_ss58check("ZRyMAksBgZ3VVirY6AjTkqF2wZq2mu6fUJPkMvmh6tbA4Gv").unwrap(), 2205111519769706050615),
        (AccountId::from_ss58check("W3UfxdN5FgZqbSFzBZ2re7ivTFW3kCZdjruN9S2Fg4ziN46").unwrap(), 1234862451071035388344),
        (AccountId::from_ss58check("ZiPahHnNRVUZ4218CtD527aecG6XyKKwpj14GhnMZxABPY5").unwrap(), 3052965727450977834487),
        (AccountId::from_ss58check("Zip3Vi3UMyCTfUEWZ4zCn4k853vZrMnFCQZnMk9rN113EE1").unwrap(), 4643634093907035486688),
        (AccountId::from_ss58check("YsuvSX1EDV3uV5XbMZRxY4TQdog3wxt3cd5NkQzdniJxvBe").unwrap(), 19333306512535018550),
        (AccountId::from_ss58check("b1p4iRzaSjBPMPpH3KAdHbN32Rr5fZngU4vjZXPg5bNjKia").unwrap(), 18933307475487422528),
        (AccountId::from_ss58check("WaZRPXag23gXPDigaxGWxuMGG8ajg8uVHUj3FiDJWPXjo6k").unwrap(), 68645121610430949354),
        (AccountId::from_ss58check("W9PbVwaFRLT3upGi6UgAsNEe87sFg1s8YerwMsZbrXppToF").unwrap(), 612469724616035855558),
        (AccountId::from_ss58check("axoxU1qv2HhThQGJnAEBBFuoLbTo1YQf83saVfUUiVdsdnh").unwrap(), 50000000000000000),
        (AccountId::from_ss58check("XkhJrZFoJAfzCHoJ9eiEWAW58Y3ehS9QNZ9vyMfXBcjwviS").unwrap(), 595380110337820633666),
        (AccountId::from_ss58check("XiBYZs6cARmdJDs99XtJ3YfaSzLwjoQcMTvhPuNnUMPXjKb").unwrap(), 16595863806413956596),
        (AccountId::from_ss58check("YFKJvAnECGc9pnTvUrBWXJ3oVbMcZwSSiUHVurGSdkvyoud").unwrap(), 18845219521933556331),
        (AccountId::from_ss58check("YsKk5kKSTGbYqNBvBkG7yTgSYFoeMyrwakLH8gtXKJ3wPGv").unwrap(), 39692007355854708911),
        (AccountId::from_ss58check("aFUG3kKaVcsYinuPzZya1Czi8MGX9K1pw666qbr5tsyWeMv").unwrap(), 226613798107933266556),
        (AccountId::from_ss58check("aLH2N5VyRpjChJkBuBb6mppp1JKsGNwwKnxugxto8zeNdg1").unwrap(), 111358131748370155555),
        (AccountId::from_ss58check("XtTpwKjQ4g3uu8ZrmmzUHAB2BfGf8LYGkh3zqFttySHMnWH").unwrap(), 2756389399712132563269),
        (AccountId::from_ss58check("a8MutWLGKXrLAqnxoptmRQ56ffT2JZQeModru4wvBG4NvQR").unwrap(), 17640892158157648404),
        (AccountId::from_ss58check("YBoqN8xKFSaTyHnD4vDYvPhxwoKYL1jgDeef4QbYUC5g67q").unwrap(), 2132730939244785219209),
        (AccountId::from_ss58check("ZzeCCeKhpHkujFsb8AGTibqBdTAANaPRqSyFk9fj8CGWucH").unwrap(), 551277879942426512),
        (AccountId::from_ss58check("WCZhLt5KBhSWwose3ba2Vff5bDtfm2FKW22SCMV3Qw4SKiv").unwrap(), 609128980663584750891),
        (AccountId::from_ss58check("ZmED9qmFNKonrUyYU6qd1jQAc7Rccgit5N57NorC5dUgDdn").unwrap(), 2646133823723647260738),
        (AccountId::from_ss58check("ZDR63zyvqMaq4RFNTomieUSBuEkbFG9Fh75X7CHT9GvbZjo").unwrap(), 1582434848023064866),
        (AccountId::from_ss58check("ZJL26sqQCaaN4N3WbC6hMuuEfPc4cBPsuw6GDoDbVNqbF15").unwrap(), 3572275848940660000000),
        (AccountId::from_ss58check("abqXaR7wTjychNdRxZe8R3LJjvvgmeLgcyT6yQ7bM5uD34e").unwrap(), 37486895836085002860),
        (AccountId::from_ss58check("ZMQAJQkFSTWp9xv5APReP4qUxwQHv7xZzPcdYp7FTi2mf9X").unwrap(), 178173010797392248889),
        (AccountId::from_ss58check("avyQoeiu2PtA2qACiFsxj39H87nqRL8v6UPqEzi6VFm7wXn").unwrap(), 16245836060717602495),
        (AccountId::from_ss58check("bNmJ6PN2NpmpBG8rf1n6jQ1cyiXuwizwuTkMVmVw1bqdQam").unwrap(), 941847232324036848337),
        (AccountId::from_ss58check("aCv5ZWe9jwvjXCtXMKvrBtrrtzKZQbqXgoJ9uixE1W9v3ZU").unwrap(), 16703719762255523332),
        (AccountId::from_ss58check("bgwCC3sJF3x4rmJmWyQ9WWTG8BcBLP3vnt8jRwnYTJet666").unwrap(), 549282279574633777206),
        (AccountId::from_ss58check("YJoKTocu1eQKAgfjkkUQ3wcMCGgqKWF9PPj97cRfHyo8JKg").unwrap(), 16107319664639564174),
        (AccountId::from_ss58check("W6kdbWj2mSDJgqe74hb36ykwvDcF26G1K3yN9Z9gtt1NQ6Z").unwrap(), 3969200735585470891107),
        (AccountId::from_ss58check("bWoEYmyzaCMf7nRGbp98b9wK6iJk849UBXn9TF27CACPgy3").unwrap(), 1675884755024976598467),
        (AccountId::from_ss58check("XxbdnSEef6B9DLGmNyxrmUM2bupuTXS2SHekpnCq7oA3C53").unwrap(), 2672595161960883732),
        (AccountId::from_ss58check("WDVLAFvqAkoyGu6yxi8gqkxB6siXLwyxa5PDEfL5fpq8dvi").unwrap(), 20832518910623824143),
        (AccountId::from_ss58check("XPf3BcGbtsWKsk56MLKEb4KJhkQvvfyythaUnKnLhTAPS5f").unwrap(), 598136499737532766229),
        (AccountId::from_ss58check("b6xtKdEEmjA5R3nqagVkQaae31X5D4KJw5bb8TdCMAJ4ZJN").unwrap(), 16597587493717776522),
        (AccountId::from_ss58check("XMi9QMRfakYTd5YxU6QiWMYfPnGT2ax9zxDZiGRQFGPi9KG").unwrap(), 21751450244651044439),
        (AccountId::from_ss58check("ZFpB1Hh3JH6EYRENiMUoqj8kKWGhuEaze1GgGaZjsdCbsGi").unwrap(), 8668593943256226),
        (AccountId::from_ss58check("ZeCHeRvLbjgQoNAMHUvjfk84EJNaHPirsZUsxA4HJ3fcUCA").unwrap(), 213617091321562596064),
        (AccountId::from_ss58check("aUps9jmZcgGer18dGjkxJn8THGexWRszbSVe2Yvu72GtmZ3").unwrap(), 20896253622336597547),
        (AccountId::from_ss58check("aqMnapwNPeLx6Mx68Uzm1wcgTq7vxYF2cDys9nu9NgmgDSn").unwrap(), 266005929689234100360),
        (AccountId::from_ss58check("Wqzdabm6mWpuuCUzecqHHY3G8G2RocB8nFguJhQaM7YzcBK").unwrap(), 190521635308102602773),
        (AccountId::from_ss58check("bER7QJ6kw4RgNdgL1e8DTWb3psNb3q8F7ym3BNhnjyffiy1").unwrap(), 232379428694301037313),
        (AccountId::from_ss58check("a13tTCmbMDg7tg5Dch4Xb3A2ThifikKZp1RiQDGJtvnEpoq").unwrap(), 1764089215815764840),
        (AccountId::from_ss58check("XYALgexCDH2aHva6W6A8osULCLyp3i3rCCZmThPiCfRNRD9").unwrap(), 26543699152499871127),
        (AccountId::from_ss58check("a9YvCUomVGmx2GUQ3p671u3P7NF7yiTPmt7VTwYxrQGRoPR").unwrap(), 92735749602868770),
        (AccountId::from_ss58check("ataENekW4Am7CY1BxxU8mFyFtEA9kabpfJqKDF9iZQuSqHW").unwrap(), 178173010797392248889),
        (AccountId::from_ss58check("XkdCa6zKQj5dpLAosaNJSVoZk7qsL9vmw2iceBA489bDgak").unwrap(), 1102555759884853025307),
        (AccountId::from_ss58check("XkPfB7HZcvUQQaJK8tyGBSgbJyAMHu297n5XDX9QYDTuLFS").unwrap(), 748326645349047445336),
        (AccountId::from_ss58check("ZYwLvcQ9Phqnu6XYV9Mm6JL6ediTvAKNpCAEcYdyUqr5Pas").unwrap(), 7056356863263059361),
        (AccountId::from_ss58check("aNqHDq5D8Lav1FN3iwvyTpoqRsVtzKRDGrLHQPfNnb8s4Av").unwrap(), 198460036779273544555),
        (AccountId::from_ss58check("WFQ3GJ1qtc1mHjdFZbdwvZTmSLzTBcWTvn8iN28QG8HvyT7").unwrap(), 18708166133726186133),
        (AccountId::from_ss58check("Xib2M8grNHy2qbqieQbHS3WYFAjLkPMLt8Y58Gbx7Q32czc").unwrap(), 198460036779273544555),
        (AccountId::from_ss58check("YhKdHQRShdEGLxsQbCYhxo1vcB92m3k9LpTuYjkvBUoMV4a").unwrap(), 2000000000000000),
        (AccountId::from_ss58check("YYou8iYYk298sNwNsQd9ftcEGsGKuXy2nsk1K4xNpkXcnyb").unwrap(), 3527157464997876307081),
        (AccountId::from_ss58check("aw3fjan9JSnMMmDBY16nAo3kydQp8yfDuo1VWkHf7bMDe79").unwrap(), 49710270012472437439),
        (AccountId::from_ss58check("WSWa5zEtZLyAiq6cx1ykjRpq4Q1JheMpSEpe5UQmLGJkfKm").unwrap(), 498896201533865724),
        (AccountId::from_ss58check("YgNsPLB1LVzeMZnwB4UWW8SbRitVxH5jzWT82V4tq4Fw1mj").unwrap(), 8820446079078824202),
        (AccountId::from_ss58check("XLisxLHwaCJLqmyNLqEnRug58FziyTNiKycgDGtzkRhYAzc").unwrap(), 793840147117094178),
        (AccountId::from_ss58check("b21xygfahx9BeYVmqdzaXnJWXfGR8ggMBkUQjYHt8d1271L").unwrap(), 17817301079739224888),
        (AccountId::from_ss58check("a3NGXaRqG6WAxmL6TWMf4v9hBUSvrQs99EyqzcPda8MriPb").unwrap(), 33341286178917955485),
        (AccountId::from_ss58check("WorsDyJRpBkFTXMAcuKGYF1sJdvG8dZSmY9gwKWZEzkGU4u").unwrap(), 97069793665605736),
        (AccountId::from_ss58check("WKUf1ATuB7e5thaKYNG6Hpc5a4qGHupEU4ACQUrrsrJyxVn").unwrap(), 2877670533299466396053),
        (AccountId::from_ss58check("XhAAGEet5CR4iEmeKoLWHFTmfxPhUrFsqyiXs5j9mgkTkvD").unwrap(), 909573220120687430579),
        (AccountId::from_ss58check("WDUqd45VLsHnu223f3yuSyvWgLw6fnVTuxDHEwE8HdAYBCD").unwrap(), 2680982618747194),
        (AccountId::from_ss58check("WVdUiKjNebTqC7pK2iCnT2Sagx88jZRaSg8pn1vN8qyjfrq").unwrap(), 5345190323921767466),
        (AccountId::from_ss58check("WECvDysKUFHCWrN5T1c2d1BXtmuEzbQGMc8UdpHjUaeh2Ka").unwrap(), 15523781872970286),
        (AccountId::from_ss58check("XDdfyXeAezGFPHMQQeNLGKPMHPcWfypf1ozpW6eYyWpR9WG").unwrap(), 535768385641867309458),
        (AccountId::from_ss58check("ZZbCa2dp7GdMJS6msr8XYtMLWw7p9FUSSjZ2PZF3xYxJHZk").unwrap(), 2672595161960883732),
        (AccountId::from_ss58check("WFSe4kJ2YpvGM8PP4z4i2kQGKZvqgX9yu5AbAHB7agdfHfu").unwrap(), 1000000000000000),
        (AccountId::from_ss58check("ZAHyq8CWEb5jr6QbH8eg4zJ9uBgkEyE5UseoAtoHsemRCe6").unwrap(), 238152044135128253466),
        (AccountId::from_ss58check("Y32tjKYjmuKs7Vnc9ZdTfJGGzBcgLA6gnTs1UxEPFjCipqp").unwrap(), 3307667279654559075),
        (AccountId::from_ss58check("WaH2sbSw53bTH6tm4txNx1Uq1ZXrsaEvnSzeXnyt2ir3sKo").unwrap(), 16105748099609355108),
        (AccountId::from_ss58check("aq6qkumYGX54CBb1HhKNLDADppgn61yJq8M2uvfktGSq2Yx").unwrap(), 551277879942426512653),
        (AccountId::from_ss58check("Ywcz3yLQZGRx9abd8ywUsNDaAYfWUte2d9UQFeUSAtXjwjw").unwrap(), 11025557598848530253),
        (AccountId::from_ss58check("Y8GmKcXKSeeq72JL9fZaL7veLtzXRuRBh6DVHzADbJ6oADg").unwrap(), 66153345593091181517),
        (AccountId::from_ss58check("Z4mfypvwUyLUQBATD1JMjSAc9Rxn4X78YdJBivdtgai2GvV").unwrap(), 103587470944581547),
        (AccountId::from_ss58check("XV3YaNFSmi4j3Kj2UgSJM91zmMBgVNzvbagDosPFrEpKNYU").unwrap(), 110255575988485302),
        (AccountId::from_ss58check("Xm5bJRNP1CTAFLGwVMAyp6zuy3s3ZVKJ3VzNbirRKgL8Y1V").unwrap(), 178173010797392248889),
        (AccountId::from_ss58check("YS3NpBz6k18m5irxW5Z5iKBaYj7fbCLSZiUmGdU5X7Djn4t").unwrap(), 16597548393435966614),
        (AccountId::from_ss58check("YjVfQ2RXeTxMXUX4gH51y1AZyVroY8KWGyLxdGDAe8Z4Ze3").unwrap(), 278282000000000000000),
        (AccountId::from_ss58check("WbQU971v6fYfDkifpWfZRcRZ7JBYaW9u91Gkb5LrMpLwLVv").unwrap(), 74595578598839807817),
        (AccountId::from_ss58check("Yk4NVesFj3JJetvzTC2GB8nxZBLyER7sRKVLBi5jKbY8pBy").unwrap(), 16596562636827446684),
        (AccountId::from_ss58check("YhbDsit1AkAtT6nf2tYe1V1ntAh5tTMBhrUfroEUFE6MrFT").unwrap(), 100553085301498595908),
        (AccountId::from_ss58check("atnWK3GnAKNoPDo8RUteiKsM2g55eKi1fEVP7EyMLrNNvYF").unwrap(), 1521526948641097174924),
        (AccountId::from_ss58check("Zycr9pP4pg9UKaKx5QAcV49ow7W87cE2bJbpMfn4FKDWo6r").unwrap(), 176408921581576484049),
        (AccountId::from_ss58check("WvDBCXpoLJ9VG1amyjv6zF2XTU4mfRvPQz6qxxX9atRnF4D").unwrap(), 745327693682160645107),
        (AccountId::from_ss58check("WWAyoLEMjboFWT2CPkPvL3Ep11qNJJGN9zUbLhGP5SSMirQ").unwrap(), 43529893700437893806),
        (AccountId::from_ss58check("X3FcF2sdAdq8kE84bLLNo8hFrbHMMM5hG9CQLPFPSafcc3A").unwrap(), 16107320539168365356),
        (AccountId::from_ss58check("ZxrP9TP5KTHKpM1NgvNCtCMxZtgQzi8chhovGvJnEErHmaK").unwrap(), 17505498310843788452),
        (AccountId::from_ss58check("WyhFnn1tQp3ttUzSrrfZ3uFdgsrqkc2HMkHuJewzC1aDR4r").unwrap(), 9250191123496906),
        (AccountId::from_ss58check("ZA6kcjWCcQFszzLr9A4XFKW7n2yvK8PFASiRmrQiUwgdFT2").unwrap(), 4454325269934806222),
        (AccountId::from_ss58check("XSQxDCBpL9C3gMUoTtgd3QJEcj63UqjLf4WUDH1weiEodmf").unwrap(), 19333308060070753509),
        (AccountId::from_ss58check("X5StEJiq5iMGMRoVpTY7Pig7um426KrPvE9NymyMKZLY2jc").unwrap(), 1000000000000000),
        (AccountId::from_ss58check("WVpa986qVr5uPhMo1WQELLG7ceYkNyBHfyNRKu4v4QFVxeu").unwrap(), 9594121765572483658),
        (AccountId::from_ss58check("Z8wjh7YXJisokoNeqGCBXGcU37E1W7GXrFpuz6TbvSD1oeV").unwrap(), 176408921581576484),
        (AccountId::from_ss58check("XpXWY38fUHUnKv8pFTZhYsrGorm8gQJzHffJMUQmqTkAE9V").unwrap(), 396920073558547089110),
        (AccountId::from_ss58check("XCkJTv7Ncven3y7rsqkeV9dS9biqGVwhwVaunyQrJEZydzQ").unwrap(), 20898217437031460441),
        (AccountId::from_ss58check("b92jciXPMGXY9JJ5jUjqyNK2RpJz4nEB11evqyGsWShCJLB").unwrap(), 926000000000000000),
        (AccountId::from_ss58check("ahQyKLY1a3vvXMs8ymeHiPk72to2u9irnEPdUDJQuVeWULV").unwrap(), 376972788023357),
        (AccountId::from_ss58check("YsqvrF3EodmscyAN4cWhduupSFPjuizZc3oz8gThBJTnwmA").unwrap(), 793840147117094178221),
        (AccountId::from_ss58check("XXyGiaD3r3eefxo7AqYrDJLPXikmY3A1EEL9CskoVRvErNb").unwrap(), 2205111519769706050615),
        (AccountId::from_ss58check("Xbis3CRCg8e5DTKSLj6UswU8przAHT5rvuhwk2Z3SnV66gR").unwrap(), 17146947177729234249),
        (AccountId::from_ss58check("YJjwZXmnX941jmS2K9GP2ktgxNmyFTrEVcRLKXBaLj71a3y").unwrap(), 847973626197519622),
        (AccountId::from_ss58check("W5GjHp4RzF5FbRmRsLTTJ38BDxeP2UWZrkPQt6Kdvk3Kaj5").unwrap(), 2023841987578557),
        (AccountId::from_ss58check("XPnz8EfHZtx1jJrvyUAyWH1wAqVvx79TfGzfmHc9RAgdfXE").unwrap(), 793840147117094178221),
        (AccountId::from_ss58check("WFSfErnMPuHAxvvTRdHpbbXDvLGwYikmbsYz1NJzZKW4Ped").unwrap(), 3775150921845736),
        (AccountId::from_ss58check("aX6hiPgpR5AE5TnWT2hSh9b5ShsR4xT5TbWkBcmA4jLkJxR").unwrap(), 19083306340607243556),
        (AccountId::from_ss58check("atDAeTYvvM5c48mE7nDQYyHKX11g4oq1kKKBVgDcXDVs1tc").unwrap(), 97063251879553826),
        (AccountId::from_ss58check("aW91e8jB2q9Ar4iuzWAss8tyqydNi2UGTGXeF47MVtwQJjf").unwrap(), 11135701816705267185451),
        (AccountId::from_ss58check("Wn3L2Tn9F594zGBn8kgGQHYbogCfuNpLVpamNx4VsVochar").unwrap(), 106124299556196758244),
        (AccountId::from_ss58check("Wr5F9ecdE5g7oP2zFtWDDL7coZS9pLkcnaz6i4H4YNBosMH").unwrap(), 3437388096333838),
        (AccountId::from_ss58check("XYNJAyCtcfKKChGXXNJVCW7d7JUj9AQuj8vNVoBVTfrPJCH").unwrap(), 1675884755024976598467),
        (AccountId::from_ss58check("WJUevho3n9RuQtdEwVPgThoRPvDttPKh2wqoecNCeT2U4Jb").unwrap(), 20896617719807415811),
        (AccountId::from_ss58check("ZwnyE7BnmkTksGNaGZXeWe8o98hGHWyBERWoQduCquuRsgD").unwrap(), 591979828374335746933),
        (AccountId::from_ss58check("au4pdgwQBPYUFatawkDREFWHDo4SzVZAnwjr3UrCtutVoHJ").unwrap(), 132306691186182363036),
        (AccountId::from_ss58check("a59JBJ8wDU4GWEZEUJ4jbNydQWuGx19ofEuLxrB6arVBj9T").unwrap(), 29933092275300135049946),
        (AccountId::from_ss58check("Zy7BzeVH4owDdSe7nbxTjNwmqZ6Bc3QSkGxCFP4bDex8brP").unwrap(), 8195958496680043448927),
        (AccountId::from_ss58check("YqrTrNzyjP1gbw31DrpdHiLsxqCxTYbCNruQrR6FCXNxFeD").unwrap(), 219038137481764441416),
        (AccountId::from_ss58check("aaZnjiBNfgYExfUhyQCNHqURtccy9bgqXWw9BKMeszBTiDt").unwrap(), 12160307986922020986),
        (AccountId::from_ss58check("Yr7ZxQJX6Ho4qBFywoA5hvFSxpBtLQv5T4LnLpzy3MffKJa").unwrap(), 2458784000000000000000),
        (AccountId::from_ss58check("Z9DoLtFvo9DFmLCGC3BnY9gxxj2deQsMvbDu784Gmfc4rnD").unwrap(), 2205111519769706050),
        (AccountId::from_ss58check("XVXWYRTar584RVWxod9AsVURrK848rkHnzHZXQ1wX9XwLWe").unwrap(), 6234794915296563297),
        (AccountId::from_ss58check("YXTbhaFLPaKmokJkzvGoZnbZAfsoSADT1TE5vzoxPgC6nhZ").unwrap(), 556790658741850777779),
        (AccountId::from_ss58check("XxrqCbze6N9Li5R475wouL9BQgBdCJDe9MPpXLk6977fNAe").unwrap(), 136981527608094139863),
        (AccountId::from_ss58check("bQpDUs633WwivU5uQsLxqf5XNno8xgsQe5AmuzC35XHpgQs").unwrap(), 1058453529489458904295),
        (AccountId::from_ss58check("XWtfsspFDz7mCNobsqp2za1e1R6hoR5DmEuRE8mdkt1qDbo").unwrap(), 1781730107973922488896),
        (AccountId::from_ss58check("XRxZqvXkQVJ83P1RcLV8iZWLTB7krCJjCqR1yYU9JvArMgo").unwrap(), 5000000000000000),
        (AccountId::from_ss58check("Z8aguz83ut8RuYqq8rZd57ogBHPUk4uUek3zhF2fNZS5q6u").unwrap(), 259613404902996729801),
        (AccountId::from_ss58check("b3iYbWL6mZuCPWUH1u46vXfUicWsEJh9bJvZjPWUsjHgZFm").unwrap(), 230985431695876708801),
        (AccountId::from_ss58check("XnChQTwDsSVFbBLqkUbnJryLTEXSNKg4ucUNogeuFJfVjot").unwrap(), 352817843163152968098),
        (AccountId::from_ss58check("Z9AndPXjFBUJ1aR6ZQDAic4wfLyR9zPjaBCwngKJrdjaCHT").unwrap(), 1051220763704614268448),
        (AccountId::from_ss58check("XbCTPqtPNvDEiTBmX9L68jMD1MjcNNgWva2TnPoV4bNfkZj").unwrap(), 59979033337736004576),
        (AccountId::from_ss58check("YqeDxpgMG8w7Qo3qo5vxfMakB5DwNrBEKP6HxRBQQjPnzAe").unwrap(), 16597548599763744608),
        (AccountId::from_ss58check("WmxKSedF9ZwVZDSWbfKDCuP9ZvMM6Wdg6jso1iPHh1kMY1x").unwrap(), 17846258868830223041193),
        (AccountId::from_ss58check("bYujryt1mmF1e7c8Wo5HRo6FJRX5mgVCcb2ixmJc3uGfEuo").unwrap(), 160355709717653024000),
        (AccountId::from_ss58check("ZrgA6s9MskgWkR2bDafZsH8MXYCCd7DrfbxssDx1tqtPVEG").unwrap(), 3563460215947844977793),
        (AccountId::from_ss58check("ZfrNz158nt9UZsVMyUhy57ir6K5EZWLWT7fTVqKqNhdj85b").unwrap(), 10155861615451358185),
        (AccountId::from_ss58check("Y8sGzfbpBaAgvomVfQXqYkE1eQyvRP2Prtzn12AdBZ3DCeX").unwrap(), 1323066911861823630),
        (AccountId::from_ss58check("WrTbSfrx11QMGwfn8GbmErX5PbsSEpEm1akkeyzbUNCF3Cs").unwrap(), 216982973545339075380),
        (AccountId::from_ss58check("YQ3G6JKbvRHZbvweppbh2vyMnT9vwVXVGEHSeqt5AgZydSx").unwrap(), 16194550020744975065),
        (AccountId::from_ss58check("an37rciQjbSCWqBJG7VDd56jUwkK7JaQGkqLjtjW79oHykE").unwrap(), 5512778799424265126),
        (AccountId::from_ss58check("YmoDvhXkp6JXWoB63oYWrJBFrnJwkKXr5Lob133iFP1pSHg").unwrap(), 17640892158157648404),
        (AccountId::from_ss58check("agStXHAmix2p1RoPjd63g2xNJGK5bkJHEAfbQotVmvgJm8w").unwrap(), 389753461119295543),
        (AccountId::from_ss58check("XyQLcyiwhJofshVTqhBi21UV53ozsPYvzApFKm1F5Ev9bBA").unwrap(), 500000000000000),
        (AccountId::from_ss58check("WAEF19CHESYYi8fpwuRsQKzjnWT3My8LJUp1oERSWdR3Uwk").unwrap(), 220511151976970605061),
        (AccountId::from_ss58check("ac7KDvjNTgwBzqMwUzFo1M8pk1Bhg1dF1vNf7wA98mhD6Gr").unwrap(), 176408921581576484049),
        (AccountId::from_ss58check("ZhGNYfZs4BQJgHX5ob5PJbZBmTipHvatrJwS6T7hRuZUQWa").unwrap(), 16197228519393733688),
        (AccountId::from_ss58check("XvkYc3JFoJAHSAhfUU1o3DFPAZGefm9nRGfKaFFNNH6uPwf").unwrap(), 18933306340607243556),
        (AccountId::from_ss58check("Xp5rb4ioj84w8CUL2hZ95BGih4AT1NVDt1av6hZwKrR8n8t").unwrap(), 210007000000000000000),
        (AccountId::from_ss58check("WroLaUBvWuFni2x6nwLh5UUXi2pKCkxg78dPw12P7W2mjA6").unwrap(), 9594121765572483658),
        (AccountId::from_ss58check("Zsd6k3Zf84zyNRzCBr2XB7D1ecjTKKV6SEY4PYEtPUL1Zos").unwrap(), 6911172320801421915597),
        (AccountId::from_ss58check("VyfiWYipCPWA2S3ciQB7G8mHB192sMEhVfEgq2vE4FNqwzw").unwrap(), 53451903239217674666),
        (AccountId::from_ss58check("WBekicsAd5axpGoQgrqPntrytVB6vQohSLReAJzhMPyCGgx").unwrap(), 100000000000000000),
        (AccountId::from_ss58check("WXWxAAmA24CkBeQeBarEtKrTjKy7XtKusVGESqGi3Y4DzKL").unwrap(), 97995155938565736888),
        (AccountId::from_ss58check("YWwj8QNRLTWEYVD1QoRw1NVZovynnRC5dzVp14t3jRfwWRU").unwrap(), 16107281500268557541),
        (AccountId::from_ss58check("amBApUNrKaHUHaam2KpkkFcvBu4RQ5SHLstnS4TTjzFkwYX").unwrap(), 1781730107973922488),
        (AccountId::from_ss58check("asx7dDV2D5iH7qu1zWemmJuQ6fwEDpjgSqerBLRaZho6iGq").unwrap(), 1194729421411226737),
        (AccountId::from_ss58check("WvKhgnSstbFxAyYxvo6soBnN66VM9pXRcMmoVyVzTnSpuKo").unwrap(), 33076672796545590759),
        (AccountId::from_ss58check("ZXP1PbZDFAERGphy2zhKCAeSRyeX8a9nwFh7HsqqRPEuab1").unwrap(), 3563460215947844977793),
        (AccountId::from_ss58check("aWW8kXjGYySyLap6CZCaUEtc4cah5iRg5bAa7wHLA8bNpfn").unwrap(), 66285652284277363881500),
        (AccountId::from_ss58check("b9u4QBgFmZ5gt91rdJ4KjwgPibTD44H7G8ecxfe7eEUNxAY").unwrap(), 18933297829315501293),
        (AccountId::from_ss58check("Z5vvtCsLqFWmtZzHcJ41s26xPgoXSf8D3QLzcrxte8wZJbj").unwrap(), 7126920431895689954),
        (AccountId::from_ss58check("XjK8Y5gQvvBXDNqfBZHr1TuioPaV8HTmtAHuZ7H37rbdczf").unwrap(), 13362975809804418666729),
        (AccountId::from_ss58check("X14f6YwV1H8RDeA48MbaKjrXt45xoGwf3qZ3KvnGa1fWqxc").unwrap(), 5578932145017356308056),
        (AccountId::from_ss58check("aUdAehUdRbTmosyjvJmQfn683n1ZWiL9Std9n2vG3KdogoW").unwrap(), 882044607907882420246),
        (AccountId::from_ss58check("Wx5CnzJmJmAsohjYwSHv8YWCuBGGxupfHX6z5F7Ty1DwzWp").unwrap(), 27563893997121325632),
        (AccountId::from_ss58check("Zk6yBTXLn8e1XsEcQKznKPLmDbuDjwSHQ3HK1eQQqwE3Sc3").unwrap(), 5000000000000000),
        (AccountId::from_ss58check("ZUoijG3QLgBpY9Q77ZpNkgmb6tzaoTbWnMBcAKdU6TWoHbw").unwrap(), 2772928322862294173),
        (AccountId::from_ss58check("X8QB4dLhX41vuqtqCAidG38mhd4bKbXKwx4zt5W4unAmjT6").unwrap(), 18843611027397567866),
        (AccountId::from_ss58check("ZkcQiYGGGFkbTVubLVgscFFiti1jkJx2kuq91yoCtufvegW").unwrap(), 26725951619608837333),
        (AccountId::from_ss58check("WSHwFc7JCpzJzSAbLqy1FcTJyjCmZXf4QJRn814A3FsoDVj").unwrap(), 5345190323921767466),
    ];

    // akru
    let root_key = hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"];

    make_genesis(
        authorities,
        keys,
        balances,
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
        vec![get_account_id_from_seed::<sr25519::Public>("Alice")],
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
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
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
