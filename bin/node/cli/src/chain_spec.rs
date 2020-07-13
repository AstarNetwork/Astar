//! Chain specification.

use pallet_plasm_lockdrop::sr25519::AuthorityId as LockdropId;
use plasm_primitives::{AccountId, Balance, Signature};
use plasm_runtime::Block;
use plasm_runtime::{
    BabeConfig, BalancesConfig, ContractsConfig, GenesisConfig, GrandpaConfig, IndicesConfig,
    PlasmLockdropConfig, PlasmRewardsConfig, PlasmValidatorConfig, SessionConfig, SessionKeys,
    SudoConfig, SystemConfig, WASM_BINARY,
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
use sp_core::crypto::UncheckedInto;
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
pub fn get_authority_keys_from_seed(seed: &str) -> (AccountId, BabeId, GrandpaId, LockdropId) {
    (
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<LockdropId>(seed),
    )
}

fn session_keys(babe: BabeId, grandpa: GrandpaId, lockdrop: LockdropId) -> SessionKeys {
    SessionKeys {
        babe,
        grandpa,
        lockdrop,
    }
}

fn testnet_genesis(
    initial_authorities: Vec<AccountId>,
    keys: Vec<(AccountId, BabeId, GrandpaId, LockdropId)>,
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

    make_genesis(initial_authorities, keys, endowed_accounts, sudo_key, true)
}

/// Helper function to create GenesisConfig
fn make_genesis(
    initial_authorities: Vec<AccountId>,
    keys: Vec<(AccountId, BabeId, GrandpaId, LockdropId)>,
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
        // pallet_dapps_staking: Some(DappsStakingConfig {
        //     ..Default::default()
        // }),
        pallet_plasm_lockdrop: Some(PlasmLockdropConfig {
            // Alpha2: 0.44698108660714747
            alpha: Perbill::from_parts(446_981_087),
            // Price in cents: BTC $9000, ETH $200
            dollar_rate: (9_000, 200),
            vote_threshold: 1,
            positive_votes: 1,
            // Max time bounds for testing purposes
            time_bounds: (0, 2_594_459_790_000u64),
        }),
        pallet_session: Some(SessionConfig {
            keys: keys
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        session_keys(x.1.clone(), x.2.clone(), x.3.clone()),
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
        "dusty2",
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
(
  hex!["fc1aae21d970b0b6afcb78328cf7aac17e6e9b111be2954613a7e845eb292259"].into(),
  hex!["ee1c3251c6d423e6a3d5fa3b52526044835a7b754f4c0d83a3ce9cfada23e357"].unchecked_into(),
  hex!["3d531ea1cb447b7ba1477b5373c69dbd0b990a15093ef23d82cd62f486f248bc"].unchecked_into(),
),
(
  hex!["c04d072348343f8df51745269e90f2914eb3ce1046da3d51cda7e664b71d7047"].into(),
  hex!["acbd9a668bf8ef12769db3970d31387cea3d0ebe6230bc57d0d027290fed0865"].unchecked_into(),
  hex!["c237a2dbd94db917b17dc66fefe25bde814ba6578daec2d572a5dd1d2c05150a"].unchecked_into(),
),
(
  hex!["aaed9c9603a9b7689d0758230796529d250a10b1491a423324f7fdeab31c7c29"].into(),
  hex!["f8c8726828f75c764eed6e7a7eb6005523c827a7438e7981c96d3862b5ef444d"].unchecked_into(),
  hex!["5b0dd3c4f247ba582bffc5839dd5ed4e48ef3ec436c62762354f0ebbfab4b147"].unchecked_into(),
),
(
  hex!["541e44342fcf707927f33a053b0f1af7ea5d7bfbded04e56f06d0f07b1d1a872"].into(),
  hex!["66204f3605b5f19540179374673a0de0ce32f1a0432093346e6c150b1cc27657"].unchecked_into(),
  hex!["b1dca18428ef3999cf5e0464007fa5578b254c02887c83ac91866d68b8f0608f"].unchecked_into(),
),
(
  hex!["107d4ed95a4a60c30c2aad70e8db902a937a1bbe65533de18bd6e61b00971400"].into(),
  hex!["38397545b0cbda6ed09ab7c8afee6fc13948356ef606faf6164857e509e48b78"].unchecked_into(),
  hex!["4134f2e2c31dfc6848b513de7caf3588a5640dc3fea15caf0821b14b91e9ec8d"].unchecked_into(),
),
(
  hex!["e6c2da882ea18eb43ce2e3193e35dc89279192eae9a75552b908f150cbf9b367"].into(),
  hex!["149af4d7e3e2b24e1f316f2902e73918c74c5947dc737d02e9998e67c810b848"].unchecked_into(),
  hex!["574016f121d1874a3e06c69a29960e3c0382c58ec7bf9c246c4086d591438c39"].unchecked_into(),
),
(
  hex!["38cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
  hex!["d409311bae981d87dee63d4c799723a33d509d7388db4c530a10e607937e547d"].unchecked_into(),
  hex!["36aaade466263a00ec16a1a1c301636ff8488fc28a08e6a7eca7ac8496e35dca"].unchecked_into(),
),
(
  hex!["de03a8a4f772c7d02f2491f95a8dd5cc2fc293fe07758164fcb7884e225fff14"].into(),
  hex!["f0fb6d0c7e69642eff3e3299b04278300d7f3fe8a126a6a163183860bbd5a233"].unchecked_into(),
  hex!["b4e47756b593a656407d1c30d00a7e3ef8b1a65676d86c244db231be4760b29e"].unchecked_into(),
),
(
  hex!["2013c8dab5879faf64f9b38ba00cb38711eeb9ad8d0cd7b71433e53e4ee82c20"].into(),
  hex!["e2e11da3cc2c4e3f74a771211672a5e4cc01d1ab209efd1b8ee9479b22992d19"].unchecked_into(),
  hex!["b9f5ca1dd840c0691443de7215f849808a606c2428d0481e04a22394fba366d0"].unchecked_into(),
),
(
  hex!["08514468942850978698ddaba8321a2a94aaf45e62071c3eea1f4b0cb488b80a"].into(),
  hex!["384d97f1e92052c325e79c2d07338cbd0b0772aad2f8d6731d48b0684a920103"].unchecked_into(),
  hex!["014e74d4646986a0d7b9d5e2566d9b235c7c6b814e94f893e5ab675fecb13039"].unchecked_into(),
),
(
  hex!["48cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
  hex!["70887b6d5241f2483fd7f199697a2f4ccfe3aedbfa60fe0c82fe476a4b08a320"].unchecked_into(),
  hex!["c62110354d58905bbfa894a1d82f0c175dfc7720758b28d18bc2118ef5f54f91"].unchecked_into(),
),
(
  hex!["d8a52ea6f51afc4b356e1bf77525eb5ccf6f10ff1896a0a77108d4e1915d2214"].into(),
  hex!["cce1120b343fc80d66147d67ffc03731e40e026578e605939b6d872f40133271"].unchecked_into(),
  hex!["eab0601b064b991604311e2a08132176edd6948bb9f88b856c55e91886cbdc13"].unchecked_into(),
),
(
  hex!["1e7bb944c3caef2c30c3b1493a293f5a422a2b673903a7be4365664297bb8f2d"].into(),
  hex!["92eea8c97f28fbd3905d741cd5895dd544e1ff641b08042c96787dbe157a4675"].unchecked_into(),
  hex!["cfc3a70abb269da4984a06d4f8264bdacbbff3e53ea7d7ceb5baf342533d52a3"].unchecked_into(),
),
(
  hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].into(),
  hex!["ac2bbc1877441591e997a7bd8043f4df4f7ca69bd05a762b0661ec376f64f551"].unchecked_into(),
  hex!["0e95fb00ea007cd02b7b0065840d4572aeab5dbf77f148a62330168e7092703d"].unchecked_into(),
),
(
  hex!["f614c390ecc98e9097b6f214b5c2f6ddd4b671344d10dcdc0946b297b35e9668"].into(),
  hex!["0ab54955fdb81295515e7485edf056c305730f03de724d8d147d2bb45336bb37"].unchecked_into(),
  hex!["a197b24b68c9bd6de1b9a2056640dea68937d486f793b14d54c53324bc91b077"].unchecked_into(),
),
(
  hex!["a0ee2ec1b74695028a995e43e35e6528f945eb119c80affbcfb53bfcd0029266"].into(),
  hex!["786e41ca6b5300d45e3abe1388e0e5c00e3462f226621c9aacc094d99132cd5a"].unchecked_into(),
  hex!["c7c04477cce92f1ef5362f1ee0d4d91ea3dedfb7fcb55dbd07bef51c2f2955fc"].unchecked_into(),
),
(
  hex!["cd4f9d12775488ee2457d1e0389cd0d07c77d055f5e1477a13fb92c24a761221"].into(),
  hex!["9ebdf67000283f4b475547fb844478c68e81caa64a0da55e244910018f3c3e0a"].unchecked_into(),
  hex!["9eab5bc89dbb9f7d4b376f70f8a7643492a5693285f387f2e79e90489778f08c"].unchecked_into(),
),
(
  hex!["2eaf514f45bfc1611dbd8aa3191a3985dea267d8b59aae9efb6af068b2d6853b"].into(),
  hex!["0812bd6e89091b8c99d51f11f0e5e6d425bdfa13c80095c7491211b0ef45ab11"].unchecked_into(),
  hex!["f08291af69651f253ac168c81f6a3d03f1a6668c2e56032c8e1e947510d6f9e5"].unchecked_into(),
),
(
  hex!["602e2ee1447eeb0662cfdb617f96cbf76a9d3a0fcf667cfe202aa2627db14e4a"].into(),
  hex!["eaae3115e78550f2dfbaf548cc2862b79557ce762c063c766171a57730cc2c70"].unchecked_into(),
  hex!["70bf98c6b0ab7a61aefa99cd462a03c64829e05e61b0eabb838faa952cfcd811"].unchecked_into(),
),
(
  hex!["0c695fb78934ff885c4cec07327787c0be3d83079a3c4074a397248a5804733d"].into(),
  hex!["026a1e18a3d29028fe9f8797e79f3c7335b23905d9dbb427f92fcc08bc6b4041"].unchecked_into(),
  hex!["92e475a2ee36409bcb57ef00dd766888c50f99394672fa6d219570f1c201ee98"].unchecked_into(),
),
(
  hex!["060ef168436268aad1a48384a82308d1a9e70c90064e385ee166f421532aa324"].into(),
  hex!["1827624d5bbfdf6219cd1af151d522d5a5acfe2dd3d2bce664e9749d3b9d7865"].unchecked_into(),
  hex!["9767cae477eb66b5b05f0d5c689805ce19af689f58f99f31b068bdd80c1f588a"].unchecked_into(),
),
(
  hex!["ad6e9c078a9744d27b74dde59b4eb74165007a630e1231259e2688d10e14b376"].into(),
  hex!["8427589e9bb3f16cb584a9fccdc0a64e9397a987bf5e7b955967a0b6c01ea775"].unchecked_into(),
  hex!["d25f1a4214f13a2102d0007881c4e468be1020155f505f2bf1e46f3f8631d646"].unchecked_into(),
),
(
  hex!["063ca9dbfff008a45135d4f1ddb29431c56711ddf4d705198b4fde441b2f125d"].into(),
  hex!["5619c0512a47f90f791192393631c21aba0cd27dfea427cc449aef59d40ca61b"].unchecked_into(),
  hex!["f477042eb78682ee01b7d62f1de056d9cd41bde6b5c2b0d97fa8737e13b30fc3"].unchecked_into(),
),
(
  hex!["006e82dff492a445c51290b83f386f575b9f828bbe9cd50cc4787c2db8e3d039"].into(),
  hex!["64ea6db51ae4747df8b8b997a4076f6c31e92e2d6244aa2701f7dff15592d129"].unchecked_into(),
  hex!["7ed76fb858498586de4157426b09e5d84a0eec83149f06e4ce55b501def1059f"].unchecked_into(),
),
(
  hex!["02d8afbd3a0cd892deebaac4db9aec1d99d6a909de745956da334e0837ac293a"].into(),
  hex!["1c74b2b3b0281be3c465a827a89fdbb5ca0274b3e34e08ac4a0be7b229ccbf6d"].unchecked_into(),
  hex!["ff5583653f62a53d5821f036841db63deb0a68225bfa59a94360a4536cfe3270"].unchecked_into(),
),
(
  hex!["21a656d9462cd3efc67f7c8b4ffec3a786b581c0237aa850c08ec8c622e0a42c"].into(),
  hex!["26a6cf41799cf1da305427833b8f83a863b4e3918cff066adc81ea973f18e055"].unchecked_into(),
  hex!["2a2c2e1dfcc930a1e9d05d8b268fe428dad1c58f830c082f77b51046e41079fc"].unchecked_into(),
),
(
  hex!["24f5a801612f9f7de9da11c3f2229b858f279cc694158ce68be87de01c7ca7fc"].into(),
  hex!["c2c38791c020acb4866772dec67b41bef489b4d2335a1e097bddc62bfdbed46f"].unchecked_into(),
  hex!["1b89862a8a21059c678b9d22ab65442de7443c85860fce2f6d7288ecb10756ed"].unchecked_into(),
),
(
  hex!["5e9358d949d35ec66c56e16358471ff8a8a67835fb719fd78e356ce935a3982b"].into(),
  hex!["76f0c33fe35a156839a340cea6db4cebe9338acd8d4fea4701bcb5547d04eb60"].unchecked_into(),
  hex!["6a857bf01b82589ce8c96243056c0c7a04558abec31ca9c165dfad49655fa77e"].unchecked_into(),
),
(
  hex!["000e79ea1782c8e00840cd29fed1f68d000bc34a8ed101e251048f4046046743"].into(),
  hex!["82c96aa6fffcb8c956db1d540f1d2ca9433fb6d67be6dc686369638d4321103d"].unchecked_into(),
  hex!["e186e108390662ed812270f518b178b679e9a5b160d3d9184bfd229cdd48fdb3"].unchecked_into(),
),
(
  hex!["7032393626db6a8238a275ae047c9704aac7445f94cccdca95eb26102ef6fc75"].into(),
  hex!["3ce30be3fa13e35357809ec0e011c522dc796338ab688fd3fd6fd5e41fd00b44"].unchecked_into(),
  hex!["f76b93f631c5dcc0f94eff47591200967c72b30fd83a9a9e04710f460b1dc4de"].unchecked_into(),
),
(
  hex!["de56110ce7ab81e4777c16b0cf2880cc4473462e88550c985644546ed46e383b"].into(),
  hex!["42bfcdb8668f9432389a7971b51065590f2d28d077b52efebd24f120af5f4407"].unchecked_into(),
  hex!["863dd4f9a38e301e3d1be371df46d5def055a93993e9a3273f833e12db69a671"].unchecked_into(),
),
(
  hex!["56edbf0dfecfcbec73edb5701d56b3488b97e0388589d3ce70eb4d25bf6acf3d"].into(),
  hex!["c4f72265a7a5cd16f1cfb8461dc64bb4efcebf552f03dc8acad672a73380c054"].unchecked_into(),
  hex!["27db04964074313a8cf6653090702d706492508531408fc4c2f5665bd0584c91"].unchecked_into(),
),
(
  hex!["94ac70f400728b7fe1030a703853f05413e9386d50f513466367fbef46f41316"].into(),
  hex!["32a00f91cd9d83bdea08e66bcb3c4cc852ad42e5b8860ba20930d0c45d0f6117"].unchecked_into(),
  hex!["63cfe9651258ef2397f57f8dd43dbfc455d6a4cbba9277c5adb04a4024283719"].unchecked_into(),
),
(
  hex!["d860803924ca921e916ea56fcb396093fb99b36b052cad2739b9a8c23744a52e"].into(),
  hex!["364e5a1a9c229dc165b22768757c013b4af3302e55b11e788b5c3b04c205e57a"].unchecked_into(),
  hex!["0521653470d697fb151c7e13788d3e4a3f4a793e41bf2fa638d8467f8d924a19"].unchecked_into(),
),
(
  hex!["52dcfe67119a6717dd8cacf36d262e247f604769a5b71700da9a5a605b759366"].into(),
  hex!["60dde8b38c6f6e187d512adcfbb6c02dc2ed88c0d2281657693e6d0cdd4a7818"].unchecked_into(),
  hex!["87afbb714cde47eef1a1e441fec98bd28bfe8d3cb690f4dfc8e9668e5e0665ed"].unchecked_into(),
),
(
  hex!["62e88eb06eac67700dc018429743435f893a3ef850802f4666f55fcfe443342d"].into(),
  hex!["92ed3ee02ce65d7bb91ffd12bb686a7da77dd94df5de69b18e3fa5bdd1bfb543"].unchecked_into(),
  hex!["d66aff5e3e065ec92d90181bbdfc138f28879e6834ea094411aafb8d64a7f83e"].unchecked_into(),
),
(
  hex!["fa6fceebf2766e46e75155faa520e30f34d80623dbef9fb26f087ec37b896861"].into(),
  hex!["24317f9e97d88ee88fe0056cce22cce075852319e772e26a323f7288f744e340"].unchecked_into(),
  hex!["d1df0d13369d952df1732552c58197deaf77978ce1d9d3fc8386af67ee8a717e"].unchecked_into(),
),
(
  hex!["945f0d53f5b08a1d33fb55d997c07444c2eaca05ffb9eb7bae5cdc7fea23c318"].into(),
  hex!["28af14391f19aab7c6199b6b538050dfbbaea7d24e0cbc5da8f8aca60b9b1f45"].unchecked_into(),
  hex!["9b6673aeb5236dc51255ca3843ce23f17ca572c34936fa8d5d5edb373fe475ed"].unchecked_into(),
),
(
  hex!["9aa1da29936ea43568224e14e13f61385034b8c25a24e4ae98a0a7f6b71b9b55"].into(),
  hex!["b070cdd1f2a825b1227c846cd1cb3cd9074bfd4faed9f3faa99fefeef9d18967"].unchecked_into(),
  hex!["6d3fece47142ea4e1cf6d53b729827ed6f13d756aec1af773f55ebd1e2b3eb18"].unchecked_into(),
),
(
  hex!["2e40fad89da106f5f4250fb8f28d28a0961fe73c88f3101228f53be08aaab06c"].into(),
  hex!["36877d9006c381a128a955582d9761feadbd3f7f37b08208a6b347654472ac6a"].unchecked_into(),
  hex!["1fad60e62397d7909dd04e8f94698fcea625ed87e1a6c2af7e6e153c076046aa"].unchecked_into(),
),
(
  hex!["821fa8107191bbc16eef51b9e3a35cdb402aefbb36de6e9d1583f6ee91c6be49"].into(),
  hex!["2c5d07dd23f7392128f82b1d973f21fed047b8391c3b3def4bbd06cf7df41b04"].unchecked_into(),
  hex!["3d0c24f39ca982633e5b63511302a067a9f936681e6c04527784218b0ff710d2"].unchecked_into(),
),
(
  hex!["e09e5e7c39f0848410b0a3617fa8c920be4598d976d11a025dfd8eb4d7ce8f2f"].into(),
  hex!["f4360627770becece1d09da5130670e74217af351c108af600b509d99265203a"].unchecked_into(),
  hex!["0f6eb79b35b0f3ddeb6cb5776b803052acd916a53fcefa186a68b30cdf9ac68c"].unchecked_into(),
),
(
  hex!["4e778f6ece78fa8524b14bde9adb2be75e74f13b73ddd8cdb4a700c21fd3d978"].into(),
  hex!["d2e5848e4fc29c3c01f6cca442fe140f2988ae1b157513e875687e21a2e7906b"].unchecked_into(),
  hex!["ceb2a6d105554a47ddab55a9cfb76774844cf8b543baaa3ea3e726310bc79dd3"].unchecked_into(),
),
(
  hex!["66364e54f63081d4f956c5c510a8eed37850a765474f01aba0d7f0dcd65faa7a"].into(),
  hex!["0089d9a9cfce6077b049643acb01888bb85484691864a2a0730f41750af6304a"].unchecked_into(),
  hex!["946ca07c9b08994f50a626886b1127e4630e2faed9d487eb2af60383bbb6550e"].unchecked_into(),
),
(
  hex!["dced9ad2bc97292196fcc12990188c3ec126164cfe4d36d1a51cc8705d42202c"].into(),
  hex!["c6ebbfffac85fb080cd3f3167f185ebb25c39958757932b3efa8c68e5e10df7c"].unchecked_into(),
  hex!["2eed080f401001178600bf706ff621ef623c6de318a58c81298513fa17fe2d05"].unchecked_into(),
),
(
  hex!["68de548ba668b1048aa523d6a750bd29d7a530b3928b6cfe632b516bd615aa14"].into(),
  hex!["3c47f526e568b15d1a76613f13fbd3e4df523d6511fc0fda8da655f041a84b0e"].unchecked_into(),
  hex!["e1d0dd965adfa10341c7e902132731808e2848eb59ea4327f9c05032c4b94a67"].unchecked_into(),
),
(
  hex!["0c59155a3aa107e57cc6461ba08ef77febfd7578b52e946d17e86a7bc8d52645"].into(),
  hex!["f406fe06c20fa69038a330b7c9378bec802db4738a9e208911fa0dbf79cf5476"].unchecked_into(),
  hex!["c8ebff4a77e9fdfcd937677b00d1cab112eeeece8923077002ec7404b5374ed0"].unchecked_into(),
),
(
  hex!["e48c4a22611aa3f0714ec271941b09e6be092027ca5c005d0c78d6ee1fef2d53"].into(),
  hex!["2675c4ab6a2afbcbefb57d60508cf81235bf961c06bc99658c0dedcc9d5ec86f"].unchecked_into(),
  hex!["fed146a416464ebefdb0d48f382fd74865c9d302edfe53ed90e89f8db74ab399"].unchecked_into(),
),
(
  hex!["92d3b3f42b2f444b8a1330d8bf1eecaee3ae6c3ead00b1f63bc31a4231468206"].into(),
  hex!["26a26e41dbf41218cd61b76256db70bcb8a6a437105800f5410ad74519ab792e"].unchecked_into(),
  hex!["63a09431a5de4b597094cc188c917ff418e0812d0ff56f9c053c1caeb3e337aa"].unchecked_into(),
),
(
  hex!["c8fc74ea0431f0c642fdfaae6a2e6173cb4a43cfd94b2afb3e30b343bfe15c6c"].into(),
  hex!["9c23b65c2b253440a2c48f3b46bcbb4f213346a7ec3851b8ad17ea4dda28c73f"].unchecked_into(),
  hex!["946c780ee034ce68eb97eb479331c436b89341c5980259b8c793d6cf8aed849a"].unchecked_into(),
),
(
  hex!["d8a5747a71fb9e8b49d7d87948bdec26656d6aa0c70ec81d09f9b6d5744c5e63"].into(),
  hex!["3a5e7a51fd41f9ee4cd0c3d727a409754d98d487a38c63cc68f7277cc8c84256"].unchecked_into(),
  hex!["61a3db89d6624751a2fb01f03e988c567d305305e3a91639ab87c29e299d8834"].unchecked_into(),
),
(
  hex!["ea3b4a5835fd45a5a870df5ca5785e4d8b00881e01d095226553fb7a1bc85f34"].into(),
  hex!["a2918577bfb33e3c47a32596e0616f379d244bee6d82e442879f6ccc3ea2556b"].unchecked_into(),
  hex!["9db7da88f760c60278f1970a8625cf3be76951b635eb422c49f30ab97e6a8c97"].unchecked_into(),
),
(
  hex!["ba491c2dd981181506989741a602f794b5a5c63063b27005e6e224bd681a8501"].into(),
  hex!["72dbce9fc860c3020118a7e37144fadc1fe73df7c15d526ffcc082e6317aa30f"].unchecked_into(),
  hex!["5f0944debcda5b264e2dc52a50dda1c563208d5fce1a8a410bf2f9341bd7762b"].unchecked_into(),
),
(
  hex!["164772c049dbf312f43bb3f081ff37efe23f4fcf1e49c87e0c194ef1cfb9553d"].into(),
  hex!["0882a3631204c3c727ae9d72217edd3e5a300dd6158e869e4d2362c5096f6b1a"].unchecked_into(),
  hex!["9657c96149f9d9cf96b47c1a67e74711add5f21f74c76c4216f13dce7da4a010"].unchecked_into(),
),
(
  hex!["268fec229c5f13af5f5e3288cbe281a4a2c56e73c286c183f56457dfcfb9333c"].into(),
  hex!["b6b9bc4772099c545698a4f5af3b38cf56a297079a89b4ce060c7debd7b43433"].unchecked_into(),
  hex!["3a7bcc0ee0031b09c4c00c41a892a154c1e472c7504e03ff8c5d860c3fc44dfc"].unchecked_into(),
),
(
  hex!["0c576d549721022073b2ab856d2138e9d144059660423063e36ec38511131c59"].into(),
  hex!["5e15369efa0e0f13c0de08b7f84a67db9e71dec360a15dad40a979000060251f"].unchecked_into(),
  hex!["4e8e4fab3271274874d12ae5de3f7bd9916a0a3b24a349343ebd6aea6f401221"].unchecked_into(),
),
(
  hex!["1c33d683df494bd9c669d1c57eb5d3e2b12ec267dc4ca204ab14e38a1cf7991f"].into(),
  hex!["b86524703276797c1d4ed3e718bf1e8ecc2789fe3bcd6186d49692d758330248"].unchecked_into(),
  hex!["a226f3b5bf0ba22cd584e83300347dc78283fae1ff9415fa5da0078e803dd3e7"].unchecked_into(),
),
(
  hex!["8a679616b3ea3c01cf6b3cffa447b544d3a51c50dd0a6429ed6e3b9626d19fbb"].into(),
  hex!["9e97f067374823516cc4a70e63546550064c402a0ec12347d5e35ea27460b47f"].unchecked_into(),
  hex!["39e726269bffe1276672d1f89b51b02d25a7ee419268341507633d773e97bbc2"].unchecked_into(),
),
(
  hex!["6e645eed244a003b4316b756a6e5197585b0b54f0027efcf65f54a48f2f85b0c"].into(),
  hex!["3ebd9bca4cd0436ad649ffe60d93de7c95dce91c9034c6872f228618d5c1af43"].unchecked_into(),
  hex!["71037ecef701f8e439976b580dc778b90509a77faaaa1fc1c7c685fe5a5828eb"].unchecked_into(),
),
(
  hex!["1d101d2ce73c5b5a0636a1384ca9e486f63f7e19c792ef3b0eaaa9925065e594"].into(),
  hex!["c23a99a537091758f463f76a3691616a958c41fa4ec7e1533d364ecf78b78c00"].unchecked_into(),
  hex!["cfb62ef7d85a4cec8ea4a84b23d0d40c5ad893d2021f5b91d3fd151992729a1a"].unchecked_into(),
),
(
  hex!["7c2222c61bafd6ae1196f15c8930626e371cfe63a2c88b4a721e023d9b07d83e"].into(),
  hex!["ae5d17ab235e9f958f6ff06db97112e446ff1d06664d818f158760976b38f11d"].unchecked_into(),
  hex!["6e965bba6c73afc891d27be82e22e898d1d2b9de1d3cf9b98ec6f445d72bf1db"].unchecked_into(),
),
(
  hex!["30c6ab03f332c9b941137856e9604a506aa9a4dd8280af941ca50b11589fa47b"].into(),
  hex!["a8dbfb58e61523c4998d3280aa3b7d56f8ee46a5d7dfb0e7d4d02e78519d8f3c"].unchecked_into(),
  hex!["868b6297e54d048570e0b077e4c267c25518e87a8fcfdcfba85eb182b209dce7"].unchecked_into(),
),
(
  hex!["2e1005ecea36fc3af8c7a03992f9f383a8c0b065ced0d08006544c20b0390811"].into(),
  hex!["cc54752f6204766548247988997e861f5e17bcffc7e46f071d4cf5c8786e1110"].unchecked_into(),
  hex!["0bf19f1e4157b3dd9f9d31ef063777e3a98a1c88612eb677f5ed90708aefcfaf"].unchecked_into(),
),
(
  hex!["4270c49a7fcabf7194d2fd31f0f0c36a308ce20429fd67d3343c7e2f92e74137"].into(),
  hex!["5483b9f1ebd9ea96bcf4282edb448a52347b6c21ffc47ada1393e3c0ca065a03"].unchecked_into(),
  hex!["fc9c3bab3c7e7f6eb7b38e015d1fd21b07a6e96c2cab9833dfb1ce153f86180c"].unchecked_into(),
),
(
  hex!["18cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
  hex!["96e2554353e7a8de10a388a5dda42096d3c7768403f3735d0a939bc3fd39bc54"].unchecked_into(),
  hex!["674bd4f2670c0e99edcccd5d3821c54b9d559580a31d8e2ca1e88c1e3db28021"].unchecked_into(),
),
(
  hex!["380477b148049ca59005c2900f51467aaf438f113d5ab061c46e7fb35a145366"].into(),
  hex!["bcc8c36a42db2d202f5138bbe52253d9f5a05fb87f24d334372c48111443f834"].unchecked_into(),
  hex!["7e6cf47ef32500e0ab2ff5c29e1c23857ab75b99e688c0866a505a783d7f5178"].unchecked_into(),
),
(
  hex!["cab842b5b724c427d49f3707d688dfa20dc65a488ad81b4cc4a850ad2435c146"].into(),
  hex!["501b11e5926f9b39e3088ed570b70870cb0315a6135fb5f88772c65b8268b62c"].unchecked_into(),
  hex!["0a1e9a1ae76fd1e286e6fb7f554640581c360745b4f5ddadcdcf17cc93e63114"].unchecked_into(),
),
(
  hex!["dad22ccd9575d9c5a7290e9ce4984d08752b9d0ba0b9d1daac064a3c3cae0a62"].into(),
  hex!["ba442fadbb1cfa6ea327a38411c0c765c0993d9131ac0c9267be85556b9aaf77"].unchecked_into(),
  hex!["adc071861fede64a37c39d9a7bc5492baf8a83dab8214ddc9be240e8f2355438"].unchecked_into(),
),
(
  hex!["10270887a7c74e7b858b70edba7d44c2905295d026e445933c094a0e29414236"].into(),
  hex!["f0bc957cf56363494f4bac16434e547a2e651166215ae409fe49cb376dd4c031"].unchecked_into(),
  hex!["9c1ee88efa1f48b1d5cea56df757897ecca77940336b24f0bd75cefaf7a6652f"].unchecked_into(),
),
(
  hex!["806452676c107d381b4009e30b8485a055f4b452f59ba2ada16544609d83bb3b"].into(),
  hex!["682ad0818dc86d028656df783b93d06e7ecb8dde784de5c88b8aa98b3b5a567a"].unchecked_into(),
  hex!["1091b11ea548b414559c271ca57b9f1d1bfc34484a69ca34dee9fdbdf032f248"].unchecked_into(),
),
(
  hex!["6cdd7d6626702113ab7efab250706e1e4088ea992e6731d15359dcadd145bb43"].into(),
  hex!["082c87abc6c50905062361509041620725a5232e2022c41d36d0504c734d7e39"].unchecked_into(),
  hex!["b19ff0313bad241b19fb56e181ffa264927a537997c89decdf402a9452eb10f9"].unchecked_into(),
),
(
  hex!["a63d081cdb513000e391fbf73429146d996e06fb0b5582fa7df03b957325cf67"].into(),
  hex!["0499455fc11905091355226fe0302e8a65cea48ce3d476d5881c946630c10446"].unchecked_into(),
  hex!["ab6e39db5927225ac5ae2286209a5fc1eb8f16115269b2bd3cdfcc26d47861dc"].unchecked_into(),
),
(
  hex!["580d98601f4fe25b7695117052812ee386029b35dcb919285d83e7077ac4f43d"].into(),
  hex!["44803406bb5d9dc6662c1e51f9b98610c669fcbe9ea2b0672c46a65b6fdc4f32"].unchecked_into(),
  hex!["1612f3416b1d5aaddffbce4f97b3a86fbf3eab2439880249276869b862310930"].unchecked_into(),
),
(
  hex!["28cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
  hex!["266f53d34490e10e6c818a1f6208dd285a74c01e022cb3b725cf5888bc89136f"].unchecked_into(),
  hex!["c379204b0b450bb62006a0df2b4abac72c79909248fc0f30ce0b05fcb9c102fa"].unchecked_into(),
),
(
  hex!["7221943c63a943fea3385b99ea36238e6fe01a0b08292b32f4b733fe1fbbe400"].into(),
  hex!["4468330ab7161db932a50dfe871acf81d5a55cfe6671e9d2be28f6d577249477"].unchecked_into(),
  hex!["bc80657a8ecf6680c1f1106216c06ba2c2e0a17401b2a0a2fcb8efa40ed6b0b2"].unchecked_into(),
),
(
  hex!["e6c925435af92c9d33201361951f4680bafdbc5269b54f9295a2dbedd63d857e"].into(),
  hex!["5261a999c5f63712758d103799325fe167abb1caa6bcd2e77f523bda8f80f527"].unchecked_into(),
  hex!["7aa5822d8c0900a21240740349863378e3f82d779e94d42b627243ee2198738a"].unchecked_into(),
),
(
  hex!["34b3baf809571ac6d477f7e6210488843c931c29d8e3273c06cdf802c93eef48"].into(),
  hex!["7ec007ca1082275cd14ae2a829ce1a39e10eba60669f315ffd5d274050c5fe70"].unchecked_into(),
  hex!["8e3099fdebef9d44a4e86c71c3899fbe488548fb031189f8d39b3629bee21f54"].unchecked_into(),
),
    ];

    let balances = vec![
(hex!["f4fcabcfad50d265f2d8b3d4aff875b3f60319ac215f89152a7be0556025ae05"].into(), 29989516668868002288),
(hex!["fc1aae21d970b0b6afcb78328cf7aac17e6e9b111be2954613a7e845eb292259"].into(), 9751936046333727),
(hex!["c04d072348343f8df51745269e90f2914eb3ce1046da3d51cda7e664b71d7047"].into(), 16387596802882729),
(hex!["f1bd686f19561df0f0db32d5f7fc19a70a15034eedfeb3ba3bb5d9ccba08ebe8"].into(), 11135813174837015555),
(hex!["e723d57452606fc5cc469a0ebb2da56f4aec0bd9b93c0a849fb1a9e3a6a06e41"].into(), 7349636695392430266),
(hex!["aaed9c9603a9b7689d0758230796529d250a10b1491a423324f7fdeab31c7c29"].into(), 8743683578724740),
(hex!["296fa030d9a93244ddb4ccf31ed57ce585587b566f80dc562270d69e9b667980"].into(), 105845352948945890),
(hex!["541e44342fcf707927f33a053b0f1af7ea5d7bfbded04e56f06d0f07b1d1a872"].into(), 498530780769917636),
(hex!["e47105725f05de907e73607879fc5050b7bb335c9471d7ea1846dac423c9db2b"].into(), 22271626349674031111),
(hex!["f2da18ae94f25ced9d57ae68e3b9fe0f19f712716179e687ff6c9443cac157f6"].into(), 1984600367792735445553),
(hex!["f61de67c7cbb4ee1d7cc0622056481161b317d418f4e038e0510ad6b7a3f4898"].into(), 176408921581576484049),
(hex!["6ddbd58a7e9d60ed113b669d4237f3b34b013a48bbd3ffc689e22f35b8508d30"].into(), 297548928031645055643),
(hex!["b9131f11d6cfc2c9b0e33cba28e7ba97977d0adebfb8f28a8ba01b9d5a05adab"].into(), 756317890738416),
(hex!["020828aec218c4a75719d054d25662567328e6790127b2aef0b64d0d6c155e83"].into(), 1764089215815764840492),
(hex!["eaaa47341fad7f360095b2f8f943ae75bd806b3b55ed9c1c9c873c5fbdf56e86"].into(), 26461338237236472),
(hex!["107d4ed95a4a60c30c2aad70e8db902a937a1bbe65533de18bd6e61b00971400"].into(), 8554578310153003),
(hex!["4ed1e97e05bd07ed228d24bd57408ea70ea0ba2cf473e350e72dbadee494a509"].into(), 193924324676133150),
(hex!["e09de189fee3550ff0010208f74b8431ecce3814c49cc90155a0164aa4760d32"].into(), 54767442126382083),
(hex!["83d390f8fedb2c04119013ec7553a63704e500890b17c49ef6fe76e21301822d"].into(), 31753605884683767128),
(hex!["e6c2da882ea18eb43ce2e3193e35dc89279192eae9a75552b908f150cbf9b367"].into(), 8524143581738812),
(hex!["d556541527a56424170cc7670ec4d0b0dd17dd4f40ed9df885a20c587d459c49"].into(), 2756389399712132563268),
(hex!["2ffa193a2d221cfeb3d2123bf61d87e90bad6503a16559cd1dbbef64515ccb65"].into(), 286664497570061786580),
(hex!["2d4f14d0de122f5d1d3561818d1cebde491677fc5a134536d7c9cbaaa070ef2f"].into(), 66814879049022093333),
(hex!["92530e994a5a459b2f918437389283c2dcc8ef57c5cb8acd307c684046a9a01b"].into(), 16423097270249630541),
(hex!["2ef2e6dd79b88249c53afd5bbe6024686cf414177702860930b56314c053d18d"].into(), 1764089215815764840),
(hex!["793c504f9c4955c1ce0ea89ba33e436e21042e2561b1d5ed4e5880ec8e27de18"].into(), 17640892158157648404923),
(hex!["de03a8a4f772c7d02f2491f95a8dd5cc2fc293fe07758164fcb7884e225fff14"].into(), 498314973974264488),
(hex!["2013c8dab5879faf64f9b38ba00cb38711eeb9ad8d0cd7b71433e53e4ee82c20"].into(), 8613296426913769),
(hex!["88f2294352edc4fc9d43b5d7dbbd7fe6eebbe381d72e83af717613041387716c"].into(), 303116834619063563422),
(hex!["3b87fd707ea6d907d2b1c4d4c87dda3228aacf4994352fdcf2314ed9f77dbb02"].into(), 6615334559309118150),
(hex!["bf88967fac581f67e49185bdc826dd63d38659d3875daff7c82376bf48dc8759"].into(), 739021074735619285802),
(hex!["08514468942850978698ddaba8321a2a94aaf45e62071c3eea1f4b0cb488b80a"].into(), 98580301108476270),
(hex!["d8a52ea6f51afc4b356e1bf77525eb5ccf6f10ff1896a0a77108d4e1915d2214"].into(), 8738576731439376),
(hex!["509ebc3da085b3d640850f9c5eb51d30ff79f61bc946784a4a63bb0aa21165bb"].into(), 2205111519769706050615),
(hex!["3daf5f990e878e787d7995f9762d931a6ba2fa60ccc9a8b4afeefe30f7572d08"].into(), 6879947941681482877),
(hex!["d0bfc5b76b9a163bf74d7001080b76125cbdeaa63c76976f342314a2f7152368"].into(), 3969200735585470891107),
(hex!["62657a49010c09b9aa33204119d49203579d4964c7a28dccc2d51de40a6259ef"].into(), 111358131748370155555),
(hex!["1e7bb944c3caef2c30c3b1493a293f5a422a2b673903a7be4365664297bb8f2d"].into(), 8613537144374761),
(hex!["a39c92eaa1c8a20b02f671d836312c1a02889bd472351813d1c5dd881261a774"].into(), 869695983397172066362),
(hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].into(), 70998190125519226647515),
(hex!["ddebabf4d0311b50398c54a1bc7ea4b6869f4abc82877c361cade088ce5157a5"].into(), 529226764744729452147),
(hex!["f614c390ecc98e9097b6f214b5c2f6ddd4b671344d10dcdc0946b297b35e9668"].into(), 548586697324634080),
(hex!["e377c08ef23ddced91056e4fbfdd31b2d5269aab20cea7c72ebd5a3b3dbf7482"].into(), 198460036779273544555),
(hex!["48f0543d0340008d0df38ef3146528227ee9fcbdb20e9d7b1bd3a3dbc8409056"].into(), 1378791465243306),
(hex!["7d537592ba54cf7a97ab7700d07fa770bca56f27120d5d764d0b9373da5cf544"].into(), 38589451595969855885),
(hex!["a0ee2ec1b74695028a995e43e35e6528f945eb119c80affbcfb53bfcd0029266"].into(), 8743975880710232),
(hex!["9163f646098ab794e399ce6b4b4cf4157c3ffb2a85dfb75e1cff291fcf197991"].into(), 957679933035983337780),
(hex!["fb7265e48e839efcc981a607c3dfedeb18accc8bb3f44042f2364f0089f519ff"].into(), 209485594378122074808),
(hex!["cd4f9d12775488ee2457d1e0389cd0d07c77d055f5e1477a13fb92c24a761221"].into(), 94260284077112259),
(hex!["b084b10aa13d278659af1c00e130ed6c79743375664da994ffef8bc7823bed14"].into(), 174644832365760719208),
(hex!["09eff193d7bf27f5b4193ae4e79707a2067c06c96b7612020dd24ea9c0da4a3a"].into(), 30066695572059941999),
(hex!["7a626aa828ac70bcc6fb6776be8c854ea5b9b663c732868c96736f56ba6972ec"].into(), 17640892158157648404),
(hex!["7b94d031c16d160b6c2a5accf4e8ecb92635df7b1ab89d1e71e817241b9ee8f3"].into(), 3836894044399288528),
(hex!["8c5022804400cbc3e2468760b4c46f8e8b1759a6f716c2193c9ef54e014d2dc0"].into(), 44543252699348062222),
(hex!["8c94b46390156340ecd142f8ef5876143e0ea9782545aa2d5981246d029e2333"].into(), 136716914225721775137),
(hex!["45586bdc2876afeb807a1fab01db150070ebd0cc750d8761ea27e1df8472e879"].into(), 1268380146171534920314),
(hex!["2eaf514f45bfc1611dbd8aa3191a3985dea267d8b59aae9efb6af068b2d6853b"].into(), 296584854046590731),
(hex!["7e5befdefbd3fadb5d72ad8b96f89ae6bbdf601e95c0b059ce98c45edcead992"].into(), 2227162634967403111121),
(hex!["8f1984912b7b82a98398694449e7a18d42629bbe2191f2180b08a95548ece6ec"].into(), 2458832092256712382736),
(hex!["a9b86a28ea926907aff80a3f4e83d9ed215bbb39a555a8a614a7d258d5202b87"].into(), 19977648835657605906),
(hex!["602e2ee1447eeb0662cfdb617f96cbf76a9d3a0fcf667cfe202aa2627db14e4a"].into(), 98755221107530920),
(hex!["8cd22c2c5bab3e428f9e65e8374d4c79d7e805ab6d6c9407b7f447ef90da12a9"].into(), 306030854210654913653),
(hex!["9b19ac496ecbc01cd6d36dba81be90fa3c82a4018cd7cbd95a6ae5fa29fc5ddd"].into(), 52922676474472945),
(hex!["f519aa2dc42ca68223d46d27dd90b08b50e2119651cf5607fa623537d6527153"].into(), 222733904388898468759),
(hex!["9f8b12050bbef77748216aa0f5d5a3f50f2fcbabaa970ef84eb521f69e2bf3c0"].into(), 7406984764158105),
(hex!["e7d00c7b07de67381499db0d607ff91e804e898885f1348433e5404a366d4573"].into(), 15434457571476080527),
(hex!["d519ac8fe5ec131821b918eade8dde8c5c911e88cd2fc72a70556044130ed355"].into(), 517649929265938495381),
(hex!["31ac0ba13db6edeb3acfebf06d2955326ed53700df0e000a1d7e5edd359f9a6f"].into(), 7216006937294386080033),
(hex!["fd8896fd5b69b5a43c81faf20f13162d36f1f8896a114ca45016fddab1c95e84"].into(), 1664969453002116553516),
(hex!["836e5f8a61fc8faf26867e30f8a404355558dae9b17c6f50cd8e9c8667e480ae"].into(), 365410573519101828439),
(hex!["103b1480b9c2b99512ca56f68017f47692ed7e407206492b334a57ffafb56eee"].into(), 400889274294132560001),
(hex!["c97f4b603a3ed5fcafe0783eb6b64ca0076c7eef53184021c4a04a01203bdf52"].into(), 1362758919217678339280),
(hex!["9cc3e564788a9fb93eab610853e514f16956967cf4d2c95779edf3e4a70c20ae"].into(), 5512778799424265126),
(hex!["c7705512367c49ad317b17da02f6c583d450413e6cdef74703cbbfb943009086"].into(), 129219535058504774565),
(hex!["f25b5faf78fbddf7af30585686941b6ac4932d390a08874957ab01283b3c0f93"].into(), 17815000000000000000),
(hex!["0c695fb78934ff885c4cec07327787c0be3d83079a3c4074a397248a5804733d"].into(), 8523799695228822),
(hex!["a7e3d73d3aa34a8c0f764dd39b69a2deb96bb234ec11ed79f85b1f1edb921dc7"].into(), 33598137369716771040),
(hex!["ceef2a85f4b72a498217d4a072b86df144e78611c623c40103930c6916df04d9"].into(), 44102230395394121012),
(hex!["ce3b804035c56ded55a5ed3c66b886a1c40909d29e4eb6ed013f31ade639453d"].into(), 220511151976970605),
(hex!["00b99f797dc130e14526ab62f6c6f795aa0cd92b7efe5dec22b16dadbf0c9b7e"].into(), 8908650539869612444486),
(hex!["30a14d47c51991c329877bca224488c3dde42f8f9cf840c5bfebe85bad3c4c82"].into(), 224921375016510017162),
(hex!["0faad22606464cd112a0244887d2976e8838949d76a895074b1284fef2574af1"].into(), 867579076338193148553),
(hex!["060ef168436268aad1a48384a82308d1a9e70c90064e385ee166f421532aa324"].into(), 508537022487061972),
(hex!["3f2d2b09c7ea8d59518689db3970e3c832682fd6ec317716504b9422958fea46"].into(), 2019868921439932124125),
(hex!["3d94fbcb5b04f3bcaffe1b31fc6e92df012ffeb9f1039327cd51694438cfab1a"].into(), 78590174564592323643),
(hex!["6b156f197b410ed3b21de970c22a6d5d11a4c462911601b60cf0d0abd5bcf8ba"].into(), 2205111519769706050),
(hex!["269c3229659dd4f04f2967298d958a43c0f1f2e70d593dceadac7eacdfdb2198"].into(), 531872898568453099407),
(hex!["8dd1b68696ed262f30b359c80956374dc9c6d31a49d925f2a33ee91588380521"].into(), 672382604608178768953),
(hex!["40a69b7b56f5de9fa94f527f15cf50c35eaf08dad53f8888a0a88290b8322a54"].into(), 11135813174837015555),
(hex!["ad6e9c078a9744d27b74dde59b4eb74165007a630e1231259e2688d10e14b376"].into(), 37184343156375958724),
(hex!["564c7b9358a3228593d238020aa485537775340f17d0450eed90818e18ec88ef"].into(), 3964349490241977537795),
(hex!["7ae1ac63ee143c46c94ec085c0c8a46c90f550fcbbe23acd26ec653353b8c3ba"].into(), 1143129811848615616639),
(hex!["063ca9dbfff008a45135d4f1ddb29431c56711ddf4d705198b4fde441b2f125d"].into(), 497277226123967254),
(hex!["006e82dff492a445c51290b83f386f575b9f828bbe9cd50cc4787c2db8e3d039"].into(), 98744852796986707),
(hex!["a7048bcee20cfef99a6f6b328af9d6541a1f6d805f1f8304bd217c272e270f5d"].into(), 60861077945643886995),
(hex!["d9d60d94be6f4df6efd0bb90531ef05aa90046149120aa72aafb4d8f4b5202ee"].into(), 783961247508525895114),
(hex!["d32d7a36b66247e43e0295034f74fb3342c477af0e80bdd26263b8202b0e4ef4"].into(), 110255575988485302530),
(hex!["249f0c0859fef45b60b0de5b4f6f16ec0bb86ab6520703db680d5603aff60c6c"].into(), 55679065874185077777),
(hex!["f2bdd63779b6e2bb4e66828f3663b8a02993fa573339c8399125e274ce0e12cb"].into(), 4410223039539412101230),
(hex!["27eef26606b5d22bc74a4a388a47288e6a3cc339f18c1bd80efb5dc609f25a14"].into(), 26461338237236472607386),
(hex!["e1f59bd4acabf3ba8257fb25fdd0d9e0d2d948c55a773d4db6d6a3e77b6d7de4"].into(), 264613382372364726),
(hex!["e344bd5bf2f2f3351dfdc209c2d5254f59c89dd04bcbf3d04e98b46f5d405cbc"].into(), 27122871693167384422),
(hex!["34a3636829da24ab2aec8b19069b5c7cf90b875efbc59403bd9e13bbd910ad8f"].into(), 12249394492320717110),
(hex!["865adbf7513359dbe7c0a088bec21c757875053271f78b003f2442b42720dc31"].into(), 10000000000000000),
(hex!["d72cc4f3518791291deac9f2b6fcd63152aed955c5503a7f53d14c51398d1db6"].into(), 690420416839894964445),
(hex!["60d4e639e5d196c9e3bb13e533d72afcae5580da0fdcde0e90a7cc84e640a1e4"].into(), 1380840833679789928894),
(hex!["7cea4ad6a64a3268f9fddfbc520996d876b0e7532accdb991996e41f2e9fb32c"].into(), 10000000000000000),
(hex!["8dba7a41c60574691345c87773dbbfedfa10f58364c47766d2e1e64e713e76bc"].into(), 8017785485882651199),
(hex!["02d8afbd3a0cd892deebaac4db9aec1d99d6a909de745956da334e0837ac293a"].into(), 494247593303947289),
(hex!["21a656d9462cd3efc67f7c8b4ffec3a786b581c0237aa850c08ec8c622e0a42c"].into(), 9050473414439665),
(hex!["3453d101b548442d73f34f5fb9c8808cd550ee41581e2f983744fe441d800c3f"].into(), 99009507237659801672),
(hex!["cd5c7864b095fa09afa7980ff756a5136c4a759e4c3ce758ea7f91626b0c1600"].into(), 765263029540066),
(hex!["16b9129a07f70d3fa66666d27dec4fd25d7261c7f08a23883b082e945ea50462"].into(), 1377918210708470432620),
(hex!["24f5a801612f9f7de9da11c3f2229b858f279cc694158ce68be87de01c7ca7fc"].into(), 3521822319416376),
(hex!["5e9358d949d35ec66c56e16358471ff8a8a67835fb719fd78e356ce935a3982b"].into(), 498939883776862),
(hex!["c4305fb88b6ccb43d6552dc11d18e7b0ee3185247adcc6e885eb284adf6c563d"].into(), 8821445079078824202),
(hex!["59e5b18ac29296d3d903f696660a21ea48505f13224292949e5cebef9edce2b7"].into(), 26725951619608837333),
(hex!["692a89cd3f6f79d6a98c8947718f9cdf8d4f2ff2b73d68a92ea1a1141dfccecb"].into(), 134070780401998127876),
(hex!["7706f2cf1202cfc5cc6e1cd4a6cee6f2677c429b85548e2a644db7e5f8d1eb1d"].into(), 14994758334434001144),
(hex!["116817e45dcd842ae9e4452d338d08704de098f6103dad3889c2b8bcd845f9d5"].into(), 53451903239217674666),
(hex!["f57f37295623a8cd7bb2944bd16c59126ca572dd0579a7edf0f4fff7b26a6969"].into(), 55127787994242651265),
(hex!["ce346a575587ad8636c217d29c55378308d4f8dd026f4a0e17cf1035d07491f3"].into(), 1190760220675641267332),
(hex!["c1e6e835faed6339a6019b31b1fc5b9f1a956c0f5c6923b152133efcf5d12276"].into(), 55127787994242651265),
(hex!["5e35661594f0b8adecf537ad913a78efd57a4d732392e2a972814c8b5a2a3dca"].into(), 200444637147066280000),
(hex!["6506ccc7c7953893695d20b046899acc3c7c8683abd387f9554f9c4f1b1fc1fb"].into(), 4998000000000000),
(hex!["65e88211dec057fb6e83be0fd6088474301bd189abab914d274d49698d179153"].into(), 5167017313124375217800),
(hex!["1d803f1e59d7037225e9623c22ba6226d01b54f0098c1e127c4fd8940b9453f2"].into(), 1113581317483701555560),
(hex!["000e79ea1782c8e00840cd29fed1f68d000bc34a8ed101e251048f4046046743"].into(), 98745145098972198),
(hex!["94269f01ad5c086a60ce9932a3cfd38485a42639f4705e4fe7e340f48809699f"].into(), 16935256471831342468),
(hex!["ec9d43f9edf4bd61b90ccb00061999d0a133dd962a260f74bf9fdb16157a1dd1"].into(), 1113581317483701555560),
(hex!["638963f44a7fb4fd0f4c640013e2af09a839f9068022465102ce341ceb918da3"].into(), 2271529478745169596859),
(hex!["7032393626db6a8238a275ae047c9704aac7445f94cccdca95eb26102ef6fc75"].into(), 98521770745211879),
(hex!["de56110ce7ab81e4777c16b0cf2880cc4473462e88550c985644546ed46e383b"].into(), 498313546905620033),
(hex!["bc386859f250524e701b452e9266732f077ab9523ed9cfc5be18a7aec8116fa9"].into(), 8908650539869612444),
(hex!["252288db9cd08946309e9f491accc9c15cdea686e20a6a5280efcdce99e246f3"].into(), 343997397084074143895),
(hex!["d6989bdc03145b8b180bcfc4b5111a50c90f35ae53c68f7861772187249c965a"].into(), 267259516196087),
(hex!["80845acc5eed464d434f3d99f94458ce3c4352a710ad74996e6b855c4413b371"].into(), 534519032392176746668),
(hex!["e6188a6be61ebb512f4e9f09ea9908501f50c7f776057dfae4394bc847a4af4d"].into(), 793840147117094178),
(hex!["3ba7c59e2a9bd5e09fd66d0d5cd31f08a2618327b7ddaa0be3b28be56b1daf07"].into(), 69916368401578184893),
(hex!["56edbf0dfecfcbec73edb5701d56b3488b97e0388589d3ce70eb4d25bf6acf3d"].into(), 497663813994033504),
(hex!["4f5488fafd10aadcbb9c013836dea8e12b55183e5af3cf3bc59cf90821c92326"].into(), 1093715076284718),
(hex!["79eeb8003ef704f3ae3835a1744e5b4ee46820f46920d1c6606658e50051f361"].into(), 55127787994242651265),
(hex!["215a9a3e38ba3dcaf8120046e3f4b385b25016575ab8564973edfdb64528493b"].into(), 17639128068941832639),
(hex!["22093b1c03d7e31ef99707087096780905707d0e047d8e0d604c72779ed28306"].into(), 242551241617068817037),
(hex!["9af39f4d9ab2089e8416e7bc323be09e2eda58a43707a57cc299c85680412059"].into(), 1670371976225552333340),
(hex!["76cedb6ed6ad49510f7eb239be061f04e1f09e9d8f7d3b6c1e26b82b43d948a3"].into(), 53451903239217674666),
(hex!["d80c922667c05e1c735c225ea5902ca56a1c83ec35b3c225c2816e2cfa798ad7"].into(), 15999204365324),
(hex!["e58fbd604dadbf6ad2d97478f3d516d1d475130ce913f57abf8af36d9e6672e7"].into(), 4454325269934806222243),
(hex!["861bc9b51a0283a8b32ef194072bfbb19b7abec7d8ac0c59cfc52d1e45dc325d"].into(), 5679264719166877933),
(hex!["ed22cad0579df6a12bf77f99bb8d3ce3331108bf4055a4e275489f6f994919fd"].into(), 1290096084418226985498),
(hex!["b3ea6174c0fb7d2af155c23e8135cfdb33f76b482f737abdef8a8b19dd8c7296"].into(), 26725951619608837333),
(hex!["9c637e1b72be8a4e315479ed25352981f536451ddbfe1fdd26d89900508f3980"].into(), 308715612767758847086),
(hex!["12c6e5fb069e481836caac799117852d85c29b2a8f8a258fa6a967023ddc761c"].into(), 501111592867665700001),
(hex!["94ac70f400728b7fe1030a703853f05413e9386d50f513466367fbef46f41316"].into(), 393343907605665718),
(hex!["8cbd1ab7426acfdcf5a59209d433428b0b66104d8790558c0b46c64586f98605"].into(), 200444637147066280000),
(hex!["c3784e7f6e7db80cda7d629b9e8a4dc9b2b2a668d7a5d310c4200dc6fb3bb694"].into(), 11025557598848530253),
(hex!["5cf42b8b29b3ba88bdc8cbecc6abcd562cd224620f5dd556b3c7f62650df972c"].into(), 170675631630175248317),
(hex!["d860803924ca921e916ea56fcb396093fb99b36b052cad2739b9a8c23744a52e"].into(), 498597116600502737),
(hex!["52dcfe67119a6717dd8cacf36d262e247f604769a5b71700da9a5a605b759366"].into(), 498994916970994627),
(hex!["343b451d11d7d2dc79822986662d2f6d7bfa5b3a0afa2cc0f55f3f58812ac5e9"].into(), 52922676474472945214),
(hex!["78055b8221fcaf9ed7440901bac69a70249bab6b511259d1778fbc5488b42482"].into(), 629173444378291378890),
(hex!["1623e3dadb9d9c55fc7587ad02545f8e20dc3c88c530a9a73fa36d90afad3298"].into(), 2807229121472869),
(hex!["7eb44a5e0beb08dba940bb341757b0127c5492e3805f086121b4ba0c7dae3d52"].into(), 50276542650749297954),
(hex!["7e296a381a68e694449d53ef3577e254e2d5ba6a8c79d79ea67e34a4ba86f920"].into(), 111358131748370155555),
(hex!["82334b4dd791aa80762e89311f113e6e412085f0359dc439b3f8f1e5c8e02019"].into(), 356346021594784497778),
(hex!["5908f50504b99f748936bd593856cd221acc503b0862afb751b56dce1e3b69a9"].into(), 52922676474472945214),
(hex!["c4097d08dbb9785f7a56d9bd8110b0b84851dc9edc102afc39eba31acbe37124"].into(), 1102555759884853025307),
(hex!["b2fff308d26fa73b561edbbb30bed88c99a8f4271f02a335321fde83b29f99b9"].into(), 119076022067564126733),
(hex!["62e88eb06eac67700dc018429743435f893a3ef850802f4666f55fcfe443342d"].into(), 8524556247614800),
(hex!["fa6fceebf2766e46e75155faa520e30f34d80623dbef9fb26f087ec37b896861"].into(), 9094084980930209),
(hex!["5569a76687ecf8cba9689fd033e55d742de05014e26566bceb7a9d12bd2cae79"].into(), 22271626349674031111),
(hex!["945f0d53f5b08a1d33fb55d997c07444c2eaca05ffb9eb7bae5cdc7fea23c318"].into(), 498314733277443496),
(hex!["f26f5f898311d5018be6040dd0d78b587f9f2cc180cc19357fe4bec78f759616"].into(), 144320138745887721599),
(hex!["9aa1da29936ea43568224e14e13f61385034b8c25a24e4ae98a0a7f6b71b9b55"].into(), 248315266276249979),
(hex!["44082db8ab51866c17ca8c97fe8ba8827c87b54d7c08181792c6ee633cb3038a"].into(), 368694646105494851662),
(hex!["16675d8e5e96d482557aa31f2c7b1fbf5a97294e973caa452426453702e3f954"].into(), 26461338237236472606),
(hex!["9053859fafbc989fdf75ca4826c7fdf035ea1a8c0781f1c04a032b4da19e45a0"].into(), 39180421483268137105),
(hex!["b9f7297a73f82d8e99aa3cefa1aa50fdf814244e4e8158d68436ab32e63fe1b2"].into(), 1524173082464820822184),
(hex!["5d32edfe553031dda6f76a84ee8f2b2cc59d19112886e9d4861cbbf4f2d17427"].into(), 606405667936669163919),
(hex!["b0e65d413f1db486157ea97bbc03a9ea4bd27ec0cf72991512ce724007282be7"].into(), 7772797596036236857),
(hex!["895777b2b3aa3f909a5ecb035b51dbd6490fb799293a56bb47cdec8d67c3899c"].into(), 60135395590491354663083),
(hex!["bf0da924fd8100297e5972b91cb5f9a1f3604985204a5dd83318c6d65412935d"].into(), 2421212448707137243575),
(hex!["764caaabf8d645b19ad8239e03af9b3784a3b515a792c53fde4d71b8d370b072"].into(), 178173010797392248889),
(hex!["d026641ff61be1f6a79275b70f73209862f7bb3464dd7602864702179376d43c"].into(), 3619845254339315),
(hex!["ce3c4da26685de067b88b1db4b43e6f0fd665ce085b61e0436c29e53e9983743"].into(), 110255575988485302530),
(hex!["69f7930c99db741180e47ff8cb035399d1767297f895cb6d000c9dcb18313d8e"].into(), 110255575988485302530),
(hex!["9722cd3b979a13da6ccd3c0ac0c2d6b8aff49f917e51c770d722abc23822411f"].into(), 264613382372364726073),
(hex!["c84430432bf81bce8d3d354435e6d1004b73fe2eda45a5d1ad66d1c56396f729"].into(), 222716263496740311111),
(hex!["2e40fad89da106f5f4250fb8f28d28a0961fe73c88f3101228f53be08aaab06c"].into(), 2458794916799064779464),
(hex!["4783435e7e90d6fea539f9654a67d27712ac1937f28a404729c9f0f8904df736"].into(), 31312583580729825918),
(hex!["9a930b26c521f3a618fa581abbdf92198a0fdcac2f3416e4c9ae77ef938c9678"].into(), 2205111519769706050615),
(hex!["04b6698730c472fbd6f51e4bbc64a24f209e71199181b5ae90ae6a3cec37538b"].into(), 1234862451071035388344),
(hex!["a7188bda68648c89fd4a5939856e9435fa9ff47ffaa8fdc1140e11a73163b094"].into(), 3034233451203115525646),
(hex!["a76ae49a9e2d175f2a87fb0e6e3a0f105e201b9bbc3c027da0c848fbd9e5b3d8"].into(), 4643634093907035486688),
(hex!["821fa8107191bbc16eef51b9e3a35cdb402aefbb36de6e9d1583f6ee91c6be49"].into(), 498315713290528963),
(hex!["e09e5e7c39f0848410b0a3617fa8c920be4598d976d11a025dfd8eb4d7ce8f2f"].into(), 98314681703238999),
(hex!["1c6b016d812dc63ce19994551ede778c5319e253ab9b6367f058b9ed8e28f788"].into(), 68645121610430949354),
(hex!["0938cb561056a376c4d29223ef52bf8323b8b4b08909548a4ae9237183ea34bf"].into(), 612469724616035855558),
(hex!["de54452c34f991fc0be8ff37e542aaff44c6e4573ac25a02fff1fb6257c55f26"].into(), 50000000000000000),
(hex!["50623e86c83e646b7a179f51a41506746f8bd07c58af27f4e7ca53442a93f949"].into(), 595380110337820633666),
(hex!["4e778f6ece78fa8524b14bde9adb2be75e74f13b73ddd8cdb4a700c21fd3d978"].into(), 498604490340393005),
(hex!["66364e54f63081d4f956c5c510a8eed37850a765474f01aba0d7f0dcd65faa7a"].into(), 8545121236768258),
(hex!["81ac99e2d0122a26e0181eff02694ef9c16100178164fca7958086b254bfe82b"].into(), 39692007355854708911),
(hex!["becce72e9d8ce40a728f712165049f218929b9a193d9e9dc2b5772021e449f78"].into(), 226613798107933266556),
(hex!["c2774e7afb358b2ca05bc2a60b20ca596932fc790a54323cb4358f131f3b891d"].into(), 111358131748370155555),
(hex!["564ed342c7b035d7cb9757bfe795664ec283f992869dbbcb702179092f725d91"].into(), 2756389399712132563269),
(hex!["b960d1400a81b74083192f569a4e3bfadd0d944709d580d4d02d0fd1c8b6646d"].into(), 17640892158157648404),
(hex!["63895912e4a50dfe4e64786fce45311e92997ae7e784d7f95ba6946429794aa4"].into(), 2132730939244785219209),
(hex!["b37da9676d421fd79b184f5e591c74215c2b1e4eec2702040fed72a1d3d79f92"].into(), 551277879942426512),
(hex!["0ba4885ce14357b9b489b49db270c07ead6e3fddb2eea16528b1465cd329a486"].into(), 609128980663584750891),
(hex!["a942bc4a71396429ec503c41949d672d0a6922648c657b3244687aca54bcef96"].into(), 2646133823723647260738),
(hex!["90ff7fef956105e696bb49787942ec80cb95ba8f693847a1534376b4e9524962"].into(), 4082436848273064866),
(hex!["94beaa7e63c093f6d6dbfeb9806865336822cc5ebe9a8fe0adf7ee10d1a0797c"].into(), 3572275848940660000000),
(hex!["ce55679c4ba18240d52c8df56f301574bf8c0e428119caa3a43d3ca7548bf5d7"].into(), 37486895836085002860),
(hex!["971657e00d92e1b4949bbb8f8e48dfdcfe1f0d3ff825701ff253c1172a3ee011"].into(), 178173010797392248889),
(hex!["dced9ad2bc97292196fcc12990188c3ec126164cfe4d36d1a51cc8705d42202c"].into(), 48572959131315982),
(hex!["f099333da90418ba162224306893a1ff160925c761b76942178f276843a66042"].into(), 941847232324036848337),
(hex!["bcda14fd709fb8b8c667e5c84dce604047cf5c13e91b0197d62fce1ee91ff159"].into(), 16703719762255523332),
(hex!["fe74f018297259cbdebbf58a5e755f4d74b95b4d45244f736493a7a195e6c14c"].into(), 549282279574633777206),
(hex!["68de548ba668b1048aa523d6a750bd29d7a530b3928b6cfe632b516bd615aa14"].into(), 8612488290003292),
(hex!["0735df3d0aac21407d74041faf52f458e16d1f9c7394db11dfb3a03dbd66cd4a"].into(), 3969200735585470891107),
(hex!["f6b9b0b3d5efccf8c4edf53d743e6da879d1288319a44b7e63ce6bb49121af0b"].into(), 1675884755024976598467),
(hex!["597617055da94cdeb559767183e02c55a8af0b11bd557deff49d7776acb9223d"].into(), 2672595161960883732),
(hex!["0c59155a3aa107e57cc6461ba08ef77febfd7578b52e946d17e86a7bc8d52645"].into(), 395876951452969654),
(hex!["405672d65369bb5cb22b24e2ef42ab4e1561cbb048656580ec24a724d24e0ecf"].into(), 598136499737532766229),
(hex!["e48c4a22611aa3f0714ec271941b09e6be092027ca5c005d0c78d6ee1fef2d53"].into(), 498635683862866161),
(hex!["3eda6aec1946a3844a50eee716611706cdad559bc90812ba92733378984e3b8a"].into(), 21751450244651044439),
(hex!["92d3b3f42b2f444b8a1330d8bf1eecaee3ae6c3ead00b1f63bc31a4231468206"].into(), 8668593943256226),
(hex!["a3e58bbca608edc629ab62db0d2676ac068fc0271d2949375fa5dc42752855a1"].into(), 213617091321562596064),
(hex!["c8fc74ea0431f0c642fdfaae6a2e6173cb4a43cfd94b2afb3e30b343bfe15c6c"].into(), 8229962812050724),
(hex!["d8a5747a71fb9e8b49d7d87948bdec26656d6aa0c70ec81d09f9b6d5744c5e63"].into(), 245117804027413545198),
(hex!["28308d3c35abedb437a5b3681e2493584e02606cdcf991383f61a3c16089dfbf"].into(), 190521635308102602773),
(hex!["ea3b4a5835fd45a5a870df5ca5785e4d8b00881e01d095226553fb7a1bc85f34"].into(), 211693573440576557968),
(hex!["b3cd6c3c1506c188de9a34d58e0e6397bf86e695c7459e47893cee78104bc6f8"].into(), 1764089215815764840),
(hex!["46d30cddda3742a4476ed710e4f784301a542aa923422d7d0e1a70b10292a6f6"].into(), 26543699152499871127),
(hex!["ba491c2dd981181506989741a602f794b5a5c63063b27005e6e224bd681a8501"].into(), 98752745208594999),
(hex!["db19156a60d2369c5e262909ba95bbb69fedcd53c4109d89f5ca19a28673f343"].into(), 178173010797392248889),
(hex!["50546a124cfa51e1da8061eb8321af5906e9fd2ecee19fd4a7632c2d56083324"].into(), 1102555759884853025307),
(hex!["5026d4880a84ca1635b356140fe34a587d2b85f31e4684f2466079fe0e5439cf"].into(), 748326645349047445336),
(hex!["9fe303bf1fcedfc467aea341b325d10d624ef556e7020de37eb7e622e5bb89e5"].into(), 7056356863263059361),
(hex!["c46a619de6e2d812d9137ff919cf614ad1d3d93253f497f5cd7fb47ee17075db"].into(), 198460036779273544555),
(hex!["0dcdc317d3bd4691f14270853b72ef81f7bb69918d1e3be834ddb3c010b7bbf0"].into(), 18708166133726186133),
(hex!["4ec698f97a552d423fc5b04fdbe39f62254695cad42df770ea5ed092ebf03858"].into(), 198460036779273544555),
(hex!["738db85cb7207d722073ae3825881304cfb785599c65c56668e5a570990fbddc"].into(), 3527157464997876307081),
(hex!["dcfbefbe9feddbea010203ec80bed2759d5936f35ac06e0b9bf395c8aadfc876"].into(), 49710270012472437439),
(hex!["164772c049dbf312f43bb3f081ff37efe23f4fcf1e49c87e0c194ef1cfb9553d"].into(), 498896201533865724),
(hex!["79536ca846e81b15f80e046edb563701b84424891ff3be36cbeca07b647e139c"].into(), 8820446079078824202),
(hex!["3e19a4556f1a3901b5c7257ee67d43a0895f6b3b377dc4f88f0b0d742361c41d"].into(), 793840147117094178),
(hex!["e0c66e848204bedce56ffcae67564355363dc1119e70aeb1cff5ff58537bd2fc"].into(), 17817301079739224888),
(hex!["b591c9caccb3cce8f4589634ce99391422d2944e28ab0097170d71356959beef"].into(), 33341286178917955485),
(hex!["268fec229c5f13af5f5e3288cbe281a4a2c56e73c286c183f56457dfcfb9333c"].into(), 98754602172356943),
(hex!["10ea487ef45ee6ce8df1b38f87542b8f21211f1459ba003b2adeba721a7c71ff"].into(), 2877670533299466396053),
(hex!["4dafa770f39a3e450863347cddf24d39689ed4ecb5160b19475e78667415f39c"].into(), 909573220120687430579),
(hex!["0c576d549721022073b2ab856d2138e9d144059660423063e36ec38511131c59"].into(), 4068618607173112),
(hex!["18a86fca77b454f4651ef1b87e3f4b699d0f8bd7c2edfb1e49ae0d857d6d305f"].into(), 5345190323921767466),
(hex!["0ce5142b4246d526a9cddc3e20201450294c7815227e4aa356e15c687e1d325c"].into(), 15523781872970286),
(hex!["38b165b6a740de151a850a9a746185b9d7f6edd89a3bb3d71f03a177d20e7a86"].into(), 535768385641867309458),
(hex!["a06272f9d22415b908045cfa198d7d0425a9865bfae07eb85b14f299f2c0c5a2"].into(), 2672595161960883732),
(hex!["8e9dd69bb7fb561e74a8d4eccfe08a579c417eaef2bbc71b471ef0ef0572b77c"].into(), 238152044135128253466),
(hex!["5cd818b5b0724b7cfa35384910f7176141fb3f8fefaeb193e9def4787734c527"].into(), 3307667279654559075),
(hex!["1c33d683df494bd9c669d1c57eb5d3e2b12ec267dc4ca204ab14e38a1cf7991f"].into(), 8613812251518753),
(hex!["d873250b266b81a510f42958c4a09085cf28d10be30c86eac9061afe14b0bad7"].into(), 551277879942426512653),
(hex!["84f39ee093965f1342244e07337b3af7a406b92de394c63a614d35c445351cea"].into(), 11025557598848530253),
(hex!["60d7059105652857ac00b6827dfeda166a4c25c80f06710d21c1357559ac4ca5"].into(), 66153345593091181517),
(hex!["8a679616b3ea3c01cf6b3cffa447b544d3a51c50dd0a6429ed6e3b9626d19fbb"].into(), 103587470944581547),
(hex!["447270ac5f4640438a00322f5f1230c09c2e7bea1a27d33bdd6ebddcbbf85e36"].into(), 110255575988485302),
(hex!["50ad41e2798992be1a87a70be49323d1cc0883cc249cee2acb4524bdb018d9f6"].into(), 178173010797392248889),
(hex!["6e645eed244a003b4316b756a6e5197585b0b54f0027efcf65f54a48f2f85b0c"].into(), 498596583581056253),
(hex!["7bb4076ae5e164cee876a342c2ca8e39fca2e160bf3abab063e630211344275f"].into(), 278282000000000000000),
(hex!["1d101d2ce73c5b5a0636a1384ca9e486f63f7e19c792ef3b0eaaa9925065e594"].into(), 74595578598839807817),
(hex!["7c2222c61bafd6ae1196f15c8930626e371cfe63a2c88b4a721e023d9b07d83e"].into(), 497610826972536323),
(hex!["7a404130dfb5906321f1e6d73f866f84d70c88aacbd9c6b49028a2de95c4723a"].into(), 100553085301498595908),
(hex!["db42677e9cbdf87814b3404413b2735bcec75463a5557a68db650b069fcfcd50"].into(), 1521526948641097174924),
(hex!["b2b5e2eba80c6c8276441f025310365b132a8d18f98f606f576e14be889f74e5"].into(), 176408921581576484049),
(hex!["2b67c39c5833e92804787c849d4b16f8785ca447065e54313e0be2e8f8e5dcb7"].into(), 745327693682160645107),
(hex!["19127aeaf37d568087cd5e664d6e76789c2de7c32b26e1e0809848663d4ae5f3"].into(), 43529893700437893806),
(hex!["30c6ab03f332c9b941137856e9604a506aa9a4dd8280af941ca50b11589fa47b"].into(), 8613021319769777),
(hex!["b22033a7542a2191c3c1873d66ab06a874b9c3c6c34b18c61c0846d43bd4513e"].into(), 17505498310843788452),
(hex!["2e1005ecea36fc3af8c7a03992f9f383a8c0b065ced0d08006544c20b0390811"].into(), 9250191123496906),
(hex!["8e780ac6e6123f23dae73e0a250c50437555df1395cef32714f230c39475ecc6"].into(), 4454325269934806222),
(hex!["4270c49a7fcabf7194d2fd31f0f0c36a308ce20429fd67d3343c7e2f92e74137"].into(), 498315163127840983),
(hex!["8d9672430f5a6b1af0340408e7ff0f3d106defba1759abe9f89d5f5dce494fd8"].into(), 176408921581576484),
(hex!["534e3da55fcfdba702cdf293f6ed1510e1c09b5618728610bacb1ed6b1aaef97"].into(), 396920073558547089110),
(hex!["380477b148049ca59005c2900f51467aaf438f113d5ab061c46e7fb35a145366"].into(), 8509611334358674),
(hex!["d29636ae232d2b9f3edf2ba2d46becb654ea77d7b4586c245d8a84612affd271"].into(), 376972788023357),
(hex!["8212370cf6cce6657dc2b9672109e7af243fd5e3eab0443b41a3e050b0345647"].into(), 793840147117094178221),
(hex!["46adca6ca4b5cb7380555a4829f96db63726fc355f6a0be7dda3f9248095fc64"].into(), 2205111519769706050615),
(hex!["498a45130cfef583c26bfa01a9f39c20f9f591b73493459ff328662f205603d5"].into(), 17146947177729234249),
(hex!["0614b363eb92b3c2ddc69e7349d2ed849e6b9fd3ec53d3f66d70aa679802914f"].into(), 2023841987578557),
(hex!["40713394d2bcff487b717019831cea08573c45f5bd1267c73ae4c981f9321ed6"].into(), 793840147117094178221),
(hex!["0dd6952a20e581f2115884db0cc19b72554df7dad7e099f5c75bfe5106b27878"].into(), 3775150921845736),
(hex!["cab842b5b724c427d49f3707d688dfa20dc65a488ad81b4cc4a850ad2435c146"].into(), 248315266276249979),
(hex!["dad22ccd9575d9c5a7290e9ce4984d08752b9d0ba0b9d1daac064a3c3cae0a62"].into(), 98744422936269219),
(hex!["c9fcc9c9b4961dc49e43764b07806879373a50bc7505c2e1fe95897c920c05b9"].into(), 11135701816705267185451),
(hex!["252ca6702a3704a14f3226010015316eba003318318c5173db2000fb1dead3f3"].into(), 106124299556196758244),
(hex!["284014d40338bdd9e5ca5284f46cdae91b052b5790481a9460ec446f10990f50"].into(), 3437388096333838),
(hex!["46fb4cd1cb506753d95db839af87f39100971df8e1f78224374efe2b4ee8fcaf"].into(), 1675884755024976598467),
(hex!["10270887a7c74e7b858b70edba7d44c2905295d026e445933c094a0e29414236"].into(), 8552153916621573),
(hex!["b151781b912b67e8458601c64555b854715cb4bbadec921ca98b3b269ae3ed75"].into(), 591979828374335746933),
(hex!["db7954157cf4ea031b14c885e744e8ff94170e7e57049747c77bd40602b39b6b"].into(), 132306691186182363036),
(hex!["b6ec9ce9d5ab972e51fb236cbfd0d903c6356a47fd4ec9a1ae1f55aa9884f721"].into(), 29933092275300135049946),
(hex!["b2520c8cbad971c316e92e37dff5076fea66d94a08acbee637053ca67248d50e"].into(), 8195958496680043448927),
(hex!["808d860a9eaecd9e3ff032c7589e3f0a85c9a0586728c9615df3ca5734740ec5"].into(), 219038137481764441416),
(hex!["cd5d2dacd3df28449eaccd95219c4fe8118022f40afed6cfb7e962c1f689b13f"].into(), 12160307986922020986),
(hex!["8dcc84e0a2f0dcf4241c3542c968951e56d2d0fc259932bfd285102cff92e57c"].into(), 2205111519769706050),
(hex!["44d0941908aab5b78b3ac9a8702b8665ee92177dc65bffae36e991da7fa3e3d9"].into(), 6236937422516636593),
(hex!["7286230d7b39ece8703f4bc304cd53484afaf7c24a869dd6bd7420bde5bc4b28"].into(), 556790658741850777779),
(hex!["59a93f3e857503da7aa918cf0963f3d092062b619cd2460bb3b9ea6c9323153e"].into(), 136981527608094139863),
(hex!["f22985ac845f33cfe3c93427410078435f8debd01575f38bfec784fc17bc052a"].into(), 1058453529489458904295),
(hex!["45db0edb8c988128070268748499788c99b64488b0a2087d1c3ec93ddddae60f"].into(), 1781730107973922488896),
(hex!["4217f22e9a29af49fd087008d593d07b73d628867f95402885c0651da2c8a432"].into(), 5000000000000000),
(hex!["8d4f97c17b3d5b85ae853889a3c2f6d53309571e7e5079d9ce6bf6f98323d858"].into(), 259613404902996729801),
(hex!["e21247bfe35d10ec100b68e305843803f0614a2d5da308f8e68cc77681a80f69"].into(), 230985431695876708801),
(hex!["51886bd0b1f7a529994f6dfd551cfa564cc849f7db87198b11323894c87b91df"].into(), 352817843163152968098),
(hex!["8dc260edd0d5bfec8e9b479121f15cc8df7508440d0b0fa0ed8654c6f329ae78"].into(), 1051220763704614268448),
(hex!["4923e8954b7b90f6c13eb795956f3d39e6727ffff6f8bca998f74323a20bfac2"].into(), 59979033337736004576),
(hex!["806452676c107d381b4009e30b8485a055f4b452f59ba2ada16544609d83bb3b"].into(), 498596789908834247),
(hex!["251bc8e97917f3c94d498cdfe842cb3444e3dbca8efc05c80bcc15ef243e8d65"].into(), 17846258868830223041193),
(hex!["f85614895a96daf5944a9d577a55a1f3b133b87f3abbacdd4eb0e4371fa1c37c"].into(), 160355709717653024000),
(hex!["ad6a4f2a2fe1e26466bd8d634db5f95920bb656d660383efef789a64b90bf2ea"].into(), 3563460215947844977793),
(hex!["a529053b68c21fa38fb9d4755139ab429b02212d85c6f6cd5e9990193b7d7a1f"].into(), 10155861615451358185),
(hex!["614b32bbb12795b3f5962639b721ac0631148ebb9faa9a3051481937b7511a17"].into(), 1323066911861823630),
(hex!["288b515a8d86888f643b78765feb73770af7d668bcca4396984fc6a973ada91a"].into(), 216982973545339075380),
(hex!["6cdd7d6626702113ab7efab250706e1e4088ea992e6731d15359dcadd145bb43"].into(), 98575778995881900),
(hex!["d61ce0ac83de8437e7c3ffea39d10cd8f61163de12c24823ff1ad910127bae3e"].into(), 5512778799424265126),
(hex!["7d75a28578fc8af424e2d45fd810f5681fae3506e9da59d00284ddf39f070790"].into(), 17640892158157648404),
(hex!["d1d96c2bbac2cea6967c1e4a7a262cab2a6aa9a389626468d320a6b8490ff0bc"].into(), 389753461119295543),
(hex!["09dc8da9dddf6582327651fbd7c4c01915ed436d3647190d9976d47fa9644f53"].into(), 220511151976970605061),
(hex!["ce8a8c6dcdda22841df58ecae8a0f3e7b998ff653ac94bfd13a5b4cb5eb68811"].into(), 176408921581576484049),
(hex!["a63d081cdb513000e391fbf73429146d996e06fb0b5582fa7df03b957325cf67"].into(), 98521650391641383),
(hex!["580d98601f4fe25b7695117052812ee386029b35dcb919285d83e7077ac4f43d"].into(), 98314905194898489),
(hex!["ae2342c3014aa619b066f8219cf295bc4d6fcc60fcfe4b8b103633305234bfc1"].into(), 6911172320801421915597),
(hex!["01ce9ba0e10ab6818596fe2a76fe7772473af5ac53b319b74f7353eb3a65e5bf"].into(), 53451903239217674666),
(hex!["1a18f4d1b8f2cb4ab309e9e38e00528731bb657e9528846f4be89070e6ebaaa8"].into(), 97995155938565736888),
(hex!["7221943c63a943fea3385b99ea36238e6fe01a0b08292b32f4b733fe1fbbe400"].into(), 8523129106730340),
(hex!["d574bc0ee6a31c136d75c27947d35f145058074624b3055352661232fe895436"].into(), 1781730107973922488),
(hex!["da9f81516906f43b0732cc4498973208022fe22b72f98b4107240d570e41ddcf"].into(), 1194729421411226737),
(hex!["2b7dbb3acc6e227825f3006231711549efb49ad53f0cb287d58c439b6b037037"].into(), 33076672796545590759),
(hex!["9eb2e9f15ba972e18fa855e3619b7290587bd03a7ac8eb2de43e9fa4868cfa2f"].into(), 3563460215947844977793),
(hex!["ca43e4a9aedefaa60d70137b18506a0168b45bae08822b4cd1809e4123f3b0db"].into(), 66285652284277363881500),
(hex!["e6c925435af92c9d33201361951f4680bafdbc5269b54f9295a2dbedd63d857e"].into(), 98314492559982504),
(hex!["8b49fe4cb915ade1121fe8e68f553f1bd848c0b46f484c24eb89c5110c5b3448"].into(), 7126920431895689954),
(hex!["4f545762f0b47721a4530b436b78b226a2a4615b515926324ca478184bf33da3"].into(), 13362975809804418666729),
(hex!["2f1b5044e15cea3aed14fbb24de1b80898a849b9500c3a4b4b03f392d33a668a"].into(), 5578932145017356308056),
(hex!["c8d513a5d866af83832c32c3c8c6d26ac0ef8a9add93f28ed7c2d8c3e8491b96"].into(), 882044607907882420246),
(hex!["2cd36ac2a10ddd0e7e797955158ecbbf683a099cfb314a803f37df491e5e13c0"].into(), 27563893997121325632),
(hex!["a8671d7520c42f20f38f6a7e84131e66d4294cad1e68ca2b327c7bf9d2300d27"].into(), 5000000000000000),
(hex!["9cbc5e3dd080c241d081d1d50d310fbcb946c8479bd2114b54081afd78387142"].into(), 2772928322862294173),
(hex!["34b3baf809571ac6d477f7e6210488843c931c29d8e3273c06cdf802c93eef48"].into(), 8542335682765332),
(hex!["a8ca3849137080fc0d2bda8b460d5b5b3a8f780011482bb38a37c5640e0e41d9"].into(), 26725951619608837333),
(hex!["161cea308e5c5574a27112e88793ad1f8f493335848a80c5a03c2e3710645000"].into(), 5345190323921767466),
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
