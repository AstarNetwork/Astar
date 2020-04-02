//! Chain specification.

use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_consensus_babe::AuthorityId as BabeId;
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sc_chain_spec::ChainSpecExtension;
use plasm_primitives::{AccountId, Balance, Signature};
use plasm_runtime::constants::currency::*;
use plasm_runtime::Block;
use plasm_runtime::{
    BabeConfig, BalancesConfig, ContractsConfig, GenesisConfig, GrandpaConfig,
    IndicesConfig, DappsStakingConfig, SessionConfig, SessionKeys, SudoConfig, SystemConfig,
    WASM_BINARY,
};

type AccountPublic = <Signature as Verify>::Signer;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const PLASM_PROPERTIES: &str = r#"
        {
            "tokenDecimals": 15,
            "tokenSymbol": "PLM"
        }"#;
const PLASM_PROTOCOL_ID: &str = "plm";
const ENDOWMENT: Balance = 1_000 * PLM;

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
pub type ChainSpec = sc_service::GenericChainSpec<
    GenesisConfig,
    Extensions,
>;

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

/// Helper function to create GenesisConfig
fn generate_config_genesis(
    initial_authorities: Vec<(AccountId, BabeId, GrandpaId)>,
    root_key: AccountId,
    balances: Option<Vec<(AccountId, Balance)>>,
    enable_println: bool,
) -> GenesisConfig {
    let balances: Vec<(AccountId, Balance)> = balances.unwrap_or_else(|| {
        vec![
            (get_account_id_from_seed::<sr25519::Public>("Alice"), ENDOWMENT),
            (get_account_id_from_seed::<sr25519::Public>("Bob"), ENDOWMENT),
            (get_account_id_from_seed::<sr25519::Public>("Charlie"), ENDOWMENT),
            (get_account_id_from_seed::<sr25519::Public>("Dave"), ENDOWMENT),
            (get_account_id_from_seed::<sr25519::Public>("Eve"), ENDOWMENT),
            (get_account_id_from_seed::<sr25519::Public>("Ferdie"), ENDOWMENT),
        ]
    });

    GenesisConfig {
        system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        balances: Some(BalancesConfig {
            balances,
        }),
        indices: Some(IndicesConfig {
            indices: vec![],
        }),
        dapps_staking: Some(DappsStakingConfig {
            storage_version: 1,
            force_era: pallet_dapps_staking::Forcing::NotForcing,
            validators: initial_authorities.iter().map(|x| x.0.clone()).collect(),
        }),
        session: Some(SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.0.clone(), session_keys(x.1.clone(), x.2.clone())))
                .collect::<Vec<_>>(),
        }),
        babe: Some(BabeConfig {
            authorities: vec![],
        }),
        grandpa: Some(GrandpaConfig {
            authorities: vec![],
        }),
        contracts: Some(ContractsConfig {
            current_schedule: pallet_contracts::Schedule {
                enable_println, // this should only be enabled on development chains
                ..Default::default()
            },
            gas_price: 1 * MILLIPLM,
        }),
        sudo: Some(SudoConfig { key: root_key }),
    }
}

/// Plasm testnet file config.
pub fn plasm_testnet_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/testnet_v3.json")[..]).unwrap()
}

/*
/// Plasm testnet native config.
pub fn plasm_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
        // akru
        "/ip4/95.216.202.55/tcp/30333/p2p/QmYyTG2eKpREh4J9BvySAkuNJDTnDXJBcJeiY1E5SdSsBv".into(),
        // Stake Technologies
        "/ip4/3.114.90.94/tcp/30333/p2p/QmW8EjUZ1f6RZe4YJ6tZAXzqYmjANbfdEYWMMaFgjkw9HN".into(),
        "/ip4/3.114.81.104/tcp/30333/p2p/QmTuouKCV9zXLrNRY71PkfggEUVrrzqofZecCfu7pz5Ntt".into(),
        "/ip4/3.115.175.152/tcp/30333/p2p/QmbKSyPY95NvJzoxP8q2DNaA9BRHZa5hy1q1pzfUoLhaUn".into(),
        "/ip4/54.64.145.3/tcp/30333/p2p/QmS9psuQJceiYQMe6swoheKXrpnyYDjaigrTqv45RWyvCh".into(),
    ];
    let properties = serde_json::from_str(PLASM_PROPERTIES).unwrap();
    ChainSpec::from_genesis(
        "Plasm Testnet v3",
        "plasm_testnet_v3",
        plasm_testnet_genesis,
        boot_nodes,
        Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(),0)])),
        Some(PLASM_PROTOCOL_ID),
        properties,
        Default::default(),
    )
}

fn plasm_testnet_genesis() -> GenesisConfig {
    // Testnet authorities list
    let authorities = vec![
        (   // akru
            hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].into(),
            hex!["ac2bbc1877441591e997a7bd8043f4df4f7ca69bd05a762b0661ec376f64f551"].unchecked_into(),
            hex!["0e95fb00ea007cd02b7b0065840d4572aeab5dbf77f148a62330168e7092703d"].unchecked_into(),
        ),
        (   // staketech-01
            hex!["48cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
            hex!["70887b6d5241f2483fd7f199697a2f4ccfe3aedbfa60fe0c82fe476a4b08a320"].unchecked_into(),
            hex!["c62110354d58905bbfa894a1d82f0c175dfc7720758b28d18bc2118ef5f54f91"].unchecked_into(),
        ),
        (   // staketech-02
            hex!["38cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
            hex!["d409311bae981d87dee63d4c799723a33d509d7388db4c530a10e607937e547d"].unchecked_into(),
            hex!["36aaade466263a00ec16a1a1c301636ff8488fc28a08e6a7eca7ac8496e35dca"].unchecked_into(),
        ),
        (   // staketech-03
            hex!["28cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
            hex!["266f53d34490e10e6c818a1f6208dd285a74c01e022cb3b725cf5888bc89136f"].unchecked_into(),
            hex!["c379204b0b450bb62006a0df2b4abac72c79909248fc0f30ce0b05fcb9c102fa"].unchecked_into(),
        ),
        (   // staketech-04
            hex!["18cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
            hex!["96e2554353e7a8de10a388a5dda42096d3c7768403f3735d0a939bc3fd39bc54"].unchecked_into(),
            hex!["674bd4f2670c0e99edcccd5d3821c54b9d559580a31d8e2ca1e88c1e3db28021"].unchecked_into(),
        ),
        (   // Stir
            hex!["ce3e6bb7672586afaf31d1c5d7e73a69d468b461d1334917ff1f23d984f8c525"].into(),
            hex!["da001e43576e62a7d4984eb86fe3a330e83854129caac5a06c5587025d9be302"].unchecked_into(),
            hex!["a4b411edba991630917119135b82c1ee9ff15d30e1ff6f62e637c7527c7478c2"].unchecked_into(),
        ),
        (   // Knigh-star
            hex!["ba103eb6c8b63de70b8410ec9d498d126234e56d51adfef3efa95fc466308d2c"].into(),
            hex!["ea711db476883a01dd2dc79a60656f66ee16a58ad33f6638c72fd647092d6b15"].unchecked_into(),
            hex!["597f96c1b19c1c2063fb35c8da64fee721ca900481f20d4b45693f517ef29acf"].unchecked_into(),
        ),
        (   // Roy
            hex!["4219c9547619f8eaed24f507872df5168674c384fcbf4dd96e860bdc1a90b64d"].into(),
            hex!["56ffd328660aa360e9d73680bb93f255866356c8c480466177e8fecf39e0c204"].unchecked_into(),
            hex!["c54fb4015876028619d7572bffece205a0108821257112c60e0e3b779e5ff519"].unchecked_into(),
        ),
        (   // InChainWorks
            hex!["10270887a7c74e7b858b70edba7d44c2905295d026e445933c094a0e29414236"].into(),
            hex!["f0bc957cf56363494f4bac16434e547a2e651166215ae409fe49cb376dd4c031"].unchecked_into(),
            hex!["9c1ee88efa1f48b1d5cea56df757897ecca77940336b24f0bd75cefaf7a6652f"].unchecked_into(),
        ),
        (   // Moonline
            hex!["4041e4a5f581bb14f13036a34f6eb26346e67f03f1c1d41e4bfa0b822f60780e"].into(),
            hex!["c0f0af7ed4801cf9d07748cee789234ed1d94a3a49a64a00052287b49152f123"].unchecked_into(),
            hex!["1884916e0ec789a374739fc426b798ec6d76f41269ebe3d742a317ec7feff011"].unchecked_into(),
        ),
        (   // Witval
            hex!["2ac41c4e82b7b3680a4e86486550557e7274363f413ec363ae03d2b7c11ceeef"].into(),
            hex!["6e1ce430daf4205ecabe491f5c1b2d84cf4b999f73d4f36f303e3e319162a267"].unchecked_into(),
            hex!["2fd38b8a4247fe685f3fb198b617f04b185044a0df07db0bf86f38fb23c1eb70"].unchecked_into(),
        ),
        (   // Spiritual
            hex!["420a8d0c7c7971bbf1d32f5feefbfe2cb09d15b5c70b8258a880117c281f365f"].into(),
            hex!["6edf07f0743c09fd96e64d613777aec0225eb8cce3211f58b3ec0ced7f9e424a"].unchecked_into(),
            hex!["9ec82b6b22e47ec7dde324cfcca84a4e8785bb8ac03eda89e414c105fe13fdfa"].unchecked_into(),
        ),
        (   // cp287
            hex!["380477b148049ca59005c2900f51467aaf438f113d5ab061c46e7fb35a145366"].into(),
            hex!["d87d24f1cc66e34b16529838654541e682971e78d4cd13595d25a5ff5f20be54"].unchecked_into(),
            hex!["4694c661d2c7d042c3b6d37305cd7c595b1d4c43f8b44173c304156f10f7a97a"].unchecked_into(),
        ),
    ];

    // Testnet endowements
    let mut balances: Vec<(AccountId, Balance)> = vec![];

    let endowed_authorities: Vec<(AccountId, Balance)> =
        authorities.iter().cloned().map(|x| (x.0, ENDOWMENT)).collect();
    balances.extend(endowed_authorities);

    let endowed_accounts: Vec<(AccountId, Balance)> = vec![
        (hex!["10270887a7c74e7b858b70edba7d44c2905295d026e445933c094a0e29414236"].into(), ENDOWMENT),
        (hex!["ce3e6bb7672586afaf31d1c5d7e73a69d468b461d1334917ff1f23d984f8c525"].into(), ENDOWMENT),
        (hex!["4cd4fbd4a2694d2a51a7012cb5f517096c150980465f2762f7a53dcb8bca0c56"].into(), ENDOWMENT),
        (hex!["240826b5b3a2d144086c7de1032c69209d6c381a96d0fa7eafc3d95798080e24"].into(), ENDOWMENT),
        (hex!["9450bb2d9c81781d159f6fe6f5be5b95a57be1a245da4315fe9baf3dd239bc06"].into(), ENDOWMENT),
        (hex!["2a40ea8a4d6c61e2aa0c5acf7e27f1b9d52014fe1d12e27e89b11fc0173e9277"].into(), ENDOWMENT),
    ];
    balances.extend(endowed_accounts);

    let lockdrop_results: Vec<(ecdsa::Public, Balance)> = vec![
        (hex!["72ea6ffa38d4f0b256ee1e12165f84e256266a163808bafb5a1cdec9127764707c0bc1dbff4d8d808345adbedd433c9c2ebfb6bca0c5e5f0fb7d1d9a841a9d22"].unchecked_into(), 7086849197963211748),
        (hex!["a0bd82a21ae8c035427069251db0de76b6501859500c2a1f249c7fa295a020416c42f73c33bb4c1d3dabca0ca644ef854dd85a53b44e9291331c5379b6745767"].unchecked_into(), 1476426916242335781029),
        (hex!["3e4960f13b3dd5d25e66f7bcc174f83d6afaec3a8f19ca25d51df8784f05ec794d3a778642272ade072aa31b2b754dc57f47bb2f52b385264d74e9a5b24dd1cc"].unchecked_into(), 34725561070019737569),
        (hex!["49bba4cd2109e677edd8374c6fc839b428ff549c1c2a8259732144beb59d246f62405399baf220bc868b3fda53bf8a331c53a9fcdd4827eea85d81e8dbcc3d31"].unchecked_into(), 70868491979632117489),
        (hex!["e8b340ecd69904ac5c0ee55180b0d6f40ff998098b0d82f36034c0741d5ea22cb8ad35dca8916f8b0441a98c9e2f23e402e1cb51ffc0c561bf5fefb58bb468b2"].unchecked_into(), 4724566131975474499293),
        (hex!["dacb4acb8315a094c5b62a1d383a037a4c9b92aff5d6d1fe3cf54564b2ade54d590d4b863bf5e2d45c2915f875f2ea230e8399091f0222f73007dcd2032c660f"].unchecked_into(), 25453600036017868864942),
        (hex!["88dacdff339718ad31eb7b641ab41759a3cb7dc1a2b5b245ff343fdeb8c628640425f2cf09bfdccd36e3413cb3c9541dcf8f1fee170df7b800f1449303098769"].unchecked_into(), 14882383315722744672773),
        (hex!["0eaa389de0bc6dbad1eb42ec211d629368ee97b380103b5edbe2a164f54811395c3ff49ad2bcaa8749cfdac0d3c40fab07e10b9c112a25de173d74bd24c46161"].unchecked_into(), 53151368984724088117049),
        (hex!["cbc386fdc95d42fb542ad7fc761b9d75e1525fefc872bf4ccc48d61d4a63f242f407f646d406e67d0e1c4ee97d34be65c1fe3dfa2d92deb07de94a69bc29db0f"].unchecked_into(), 637816427816689057404),
        (hex!["9308948ba005372aa9fb9ee70d088298e99da8dd87e79e24870b7e3af4f53d32439de3c3f4cf56fd8195fb16ec524ad60f3a1d1907a78f5bae601afea6a590dc"].unchecked_into(), 21260547593889635246819),
        (hex!["65d068328553959df19c46640e0326052e445f46496a1969da2c4e8c527bdb14f82f2bdbe07c5d1eb2fe4eb51b37bb0523c3d40afca97028270fbd7e7ba154c8"].unchecked_into(), 21260547593889635246819),
        (hex!["dc9381468a9712dfae81b5c88827cd824c189db9f7f49d4571cfe67b498b691204ae6d4c354a3677788828830649b682c91eeef379daec40f56a9ac7c4a669bd"].unchecked_into(), 20197520214195153484478),
        (hex!["7a4e1723b1d86a9f9fa9463a3bf9bf74bb77fed77da8133d57d786403a29b090595d0711616ed482fc2372e4222a18d28748a60104720ce60befe03726a63b80"].unchecked_into(), 5315136898472408811704),
        (hex!["18d4985842078b4e71fb3c6cf89404048d945f40b8091d8cb9d4c68325ecaa35f123068f6af41c46aa2eb68b0efefdf01b5ef3a3adebbc4cdf63a70888124a57"].unchecked_into(), 297647666314454893455),
        (hex!["0f1e92b59c57d71c1d547445542b59417b824bf69a16a7923ff6dca02ef492236f3869f57ba1147bd6acbbc147e64a15b1ed13f2864ea5b0c78516045a59d040"].unchecked_into(), 4762362661031278295287),
        (hex!["b67adc96f145bd8521576321f01b2a37de74c36af81b87fead022858de31d68a4443377ef3e517adf82f74bbea37e3b82c3d65f81451eb178f7127104fe6f753"].unchecked_into(), 10630273796944817623409),
        (hex!["5ed9d63912b7a0e1ebf719d3b191c05eaf74299ac81062f5911cd56e3d4c662c6fe078a33982b8df6dea6147ae53f2d74a69cce4070df5406b007472f788605b"].unchecked_into(), 10630273796944817623409),
        (hex!["0d719cf8c1f4b8343484a6e13c7102055903552787b9568041c3fe548a13791362f8a4ba73d42294ab5c0d32c1207781c45108b3e85c76ceaeb99bd64ab305d9"].unchecked_into(), 14882383315722744672773),
        (hex!["f70be2dc51b338ab5fb7b7607821d7a02dcbae1d6fcaad8377947c1f8b377ad8568524c0bc3ea7e46cfef6f5200b08cdc14534de34da34b9468e61d6040bc4f4"].unchecked_into(), 297647666314454893455),
        (hex!["ca5a3e9a9b1fa5151c42cfbae7e2c80b1a67a75aa42c5b248893861829405c58cab8c28050b2939a927191d7606580d77fce7e01223fada1d5673437e078f59c"].unchecked_into(), 106302737969448176234098),
        (hex!["9330721827066c3b165dd60c3b95368d08cdcc69229e7a892dcf848d3334a4ac206ad1e6f9ee498cb5a1e59262880d71dee3fa6889899e9f6665187da15fb6c2"].unchecked_into(), 31359307700987211989059),
        (hex!["5dee8ed95e0d9bed5d85b73db0271a0e67a034473e533a882cff19f0b0751f85ccbbe195da23e36f2eaaa64d05be4ea9f0010a4f1b7185d51dc76c2f2212dc48"].unchecked_into(), 2952853832484671562058),
        (hex!["f2b46504de471e2cf024c489e635f5b91c0a2073f7865c794bd8b946f8f3c79573baaf438101db6d9a0313f5225f49260597cfea2f86ee6554078dfc50d39d50"].unchecked_into(), 31890821390834452870229),
        (hex!["c5a5a32ff39eb08e7734f81658252ea2f13d2829366ee71f077880fdc41e291d1b78812a6ac224380b8d7756cb52c9f8aee3b1dc2ad4107a1a20b7b5c80618e8"].unchecked_into(), 42521095187779270493639)
    ];
    let lockdrop_balances: Vec<(AccountId, Balance)> = lockdrop_results
            .iter()
            .cloned()
            .map(|(a, b)| (AccountPublic::from(a).into_account(), b))
            .collect();
    balances.extend(lockdrop_balances);

    // 5Cakru1BpXPiezeD2LRZh3pJamHcbX9yZ13KLBxuqdTpgnYF
    let root_key = hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"];

    generate_config_genesis(
        authorities,
        root_key.into(),
        Some(balances),
        false,
    )
}
*/

fn development_config_genesis() -> GenesisConfig {
    generate_config_genesis(
        vec![get_authority_keys_from_seed("Alice")],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
        true,
    )
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Development",
        "dev",
        development_config_genesis,
        vec![],
        None,
        None,
        None,
        Default::default(),
    )
}

fn local_testnet_genesis() -> GenesisConfig {
    generate_config_genesis(
        vec![
            get_authority_keys_from_seed("Alice"),
            get_authority_keys_from_seed("Bob"),
        ],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
        false,
    )
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        local_testnet_genesis,
        vec![],
        None,
        None,
        None,
        Default::default(),
    )
}
