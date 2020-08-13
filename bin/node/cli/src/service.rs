//! Collator implementation. Specialized wrapper over substrate service.

use plasm_primitives::Block;
use plasm_runtime::RuntimeApi;
use cumulus_network::DelayedBlockAnnounceValidator;
use cumulus_service::{
    prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient};
use sp_runtime::traits::BlakeTwo256;
use sp_trie::PrefixedMemoryDB;
use polkadot_primitives::v0::CollatorPair;
pub use sc_executor::NativeExecutor;
use sc_informant::OutputFormat;
use sc_service::{Configuration, Role, TaskManager};
use polkadot_parachain::primitives::AccountIdConversion;
use polkadot_primitives::v0::Id as ParaId;
use sc_cli::{
    ChainSpec, CliConfiguration, ImportParams, KeystoreParams, NetworkParams, Result,
    RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_network::config::TransportConfig;
use sc_service::{
    config::{Configuration, NetworkConfiguration, NodeKeyConfig, PrometheusConfig},
    TaskManager,
};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::{Block as BlockT, Hash as HashT, Header as HeaderT, Zero};
use sp_runtime::BuildStorage;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

sc_executor::native_executor_instance!(
    pub Executor,
    plasm_runtime::api::dispatch,
    plasm_runtime::native_version,
);

pub fn new_partial(
    config: &mut Configuration,
) -> Result<
    PartialComponents<
        TFullClient<Block, RuntimeApi, Executor>,
        TFullBackend<Block>,
        (),
        sp_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
        sc_transaction_pool::FullPool<Block, TFullClient<Block, RuntimeApi, Executor>>,
        (),
    >,
    sc_service::Error,
> {
    let inherent_data_providers = sp_inherents::InherentDataProviders::new();

    let (client, backend, keystore, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
    let client = Arc::new(client);
    let registry = config.prometheus_registry();

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
    );

    let import_queue = cumulus_consensus::import_queue::import_queue(
        client.clone(),
        client.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        registry.clone(),
    )?;

    let params = PartialComponents {
        backend,
        client,
        import_queue,
        keystore,
        task_manager,
        transaction_pool,
        inherent_data_providers,
        select_chain: (),
        other: (),
    };

    Ok(params)
}

fn new_collator(
    parachain_config: Configuration,
    para_id: polkadot_primitives::v0::Id,
    collator_key: Arc<CollatorPair>,
    mut polkadot_config: polkadot_collator::Configuration,
    validator: bool,
) -> sc_service::error::Result<TaskManager> {
    if matches!(parachain_config.role, Role::Light) {
        return Err("Light client not supported!".into());
    }

    let mut parachain_config = prepare_node_config(parachain_config);

    parachain_config.informant_output_format = OutputFormat {
        enable_color: true,
        prefix: "[Parachain] ".to_string(),
    };
    polkadot_config.informant_output_format = OutputFormat {
        enable_color: true,
        prefix: "[Relaychain] ".to_string(),
    };

    let params = super::new_partial(&mut parachain_config)?;
    params
        .inherent_data_providers
        .register_provider(sp_timestamp::InherentDataProvider)
        .unwrap();

    let client = params.client.clone();
    let backend = params.backend.clone();
    let block_announce_validator = DelayedBlockAnnounceValidator::new();
    let block_announce_validator_builder = {
        let block_announce_validator = block_announce_validator.clone();
        move |_| Box::new(block_announce_validator) as Box<_>
    };

    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let transaction_pool = params.transaction_pool.clone();
    let mut task_manager = params.task_manager;
    let import_queue = params.import_queue;
    let (network, network_status_sinks, system_rpc_tx, start_network) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &parachain_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: None,
            block_announce_validator_builder: Some(Box::new(block_announce_validator_builder)),
            finality_proof_request_builder: None,
            finality_proof_provider: None,
        })?;

    let rpc_extensions_builder = Box::new(|_| ());
    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        on_demand: None,
        remote_blockchain: None,
        rpc_extensions_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        telemetry_connection_sinks: Default::default(),
        config: parachain_config,
        keystore: params.keystore,
        backend,
        network: network.clone(),
        network_status_sinks,
        system_rpc_tx,
    })?;

    let announce_block = Arc::new(move |hash, data| network.announce_block(hash, data));

    if validator {
        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            client.clone(),
            transaction_pool,
            prometheus_registry.as_ref(),
        );
        let params = StartCollatorParams {
            para_id,
            block_import: client.clone(),
            proposer_factory,
            inherent_data_providers: params.inherent_data_providers,
            block_status: client.clone(),
            announce_block,
            client: client.clone(),
            block_announce_validator,
            task_manager: &mut task_manager,
            polkadot_config,
            collator_key,
        };
        start_collator(params)?;
    } else {
        let params = StartFullNodeParams {
            client: client.clone(),
            announce_block,
            polkadot_config,
            collator_key,
            block_announce_validator,
            task_manager: &mut task_manager,
            para_id,
        };

        start_full_node(params)?;
    }

    start_network.start_network();

    Ok(task_manager)
}

fn generate_genesis_state() -> sc_service::error::Result<Block> {
    let storage = (&crate::chain_spec::parachain_testnet_config()).build_storage()?;

    let child_roots = storage.children_default.iter().map(|(sk, child_content)| {
        let state_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
            child_content.data.clone().into_iter().collect(),
        );
        (sk.clone(), state_root.encode())
    });
    let state_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
        storage.top.clone().into_iter().chain(child_roots).collect(),
    );

    let extrinsics_root =
        <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(Vec::new());

    Ok(Block::new(
        <<Block as BlockT>::Header as HeaderT>::new(
            Zero::zero(),
            extrinsics_root,
            state_root,
            Default::default(),
            Default::default(),
        ),
        Default::default(),
    ))
}

/// Run a collator node with the given parachain `Configuration`
pub fn run_collator(
    config: Configuration,
    parachain_id: u32,
    relaychain_args: &Vec<String>,
    validator: bool,
) -> sc_service::error::Result<TaskManager> {
    let key = Arc::new(sp_core::Pair::generate().0);
    let parachain_id = ParaId::from(parachain_id);

    let block = generate_genesis_state()?;
    let header_hex = format!("0x{:?}", HexDisplay::from(&block.header().encode()));
    let parachain_account =
        AccountIdConversion::<polkadot_primitives::v0::AccountId>::into_account(&parachain_id);

    info!("[Para] ID: {}", parachain_id);
    info!("[Para] Account: {}", parachain_account);
    info!("[Para] Genesis State: {}", header_hex);

    let mut polkadot_cli = PolkadotCli::from_iter(
        [PolkadotCli::executable_name().to_string()]
            .iter()
            .chain(relaychain_args.iter()),
    );
    polkadot_cli.base_path = config.base_path.as_ref().map(|x| x.path().join("polkadot"));

    let task_executor = config.task_executor.clone();
    let polkadot_config =
        SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, task_executor).unwrap();

    new_collator(config, parachain_id, key, polkadot_config, validator)
}

#[derive(Debug, structopt::StructOpt)]
pub struct PolkadotCli {
    #[structopt(flatten)]
    pub base: polkadot_cli::RunCmd,

    #[structopt(skip)]
    pub base_path: Option<std::path::PathBuf>,
}

impl SubstrateCli for PolkadotCli {
    fn impl_name() -> String {
        "Plasm Test Parachain Collator".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        format!(
            "plasm test parachain collator\n\nThe command-line arguments provided first will be \
        passed to the parachain node, while the arguments provided after -- will be passed \
        to the relaychain node.\n\n\
        {} [parachain-args] -- [relaychain-args]",
            Self::executable_name()
        )
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/staketechnologies/plasm/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2019
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        let chain_spec = match id {
            "" => polkadot_service::WestendChainSpec::from_json_bytes(
                &include_bytes!("../../res/polkadot_chainspec.json")[..],
            )?,
            path => {
                polkadot_service::WestendChainSpec::from_json_file(std::path::PathBuf::from(path))?
            }
        };
        Ok(Box::new(chain_spec))
    }

    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        polkadot_cli::Cli::native_runtime_version(chain_spec)
    }
}

impl CliConfiguration for PolkadotCli {
    fn shared_params(&self) -> &SharedParams {
        self.base.base.shared_params()
    }

    fn import_params(&self) -> Option<&ImportParams> {
        self.base.base.import_params()
    }

    fn network_params(&self) -> Option<&NetworkParams> {
        self.base.base.network_params()
    }

    fn keystore_params(&self) -> Option<&KeystoreParams> {
        self.base.base.keystore_params()
    }

    fn base_path(&self) -> Result<Option<sc_service::config::BasePath>> {
        Ok(self
            .shared_params()
            .base_path()
            .or_else(|| self.base_path.clone().map(Into::into)))
    }

    fn rpc_http(&self) -> Result<Option<SocketAddr>> {
        let rpc_port = self.base.base.rpc_port;
        Ok(Some(parse_address(
            &format!("127.0.0.1:{}", 9934),
            rpc_port,
        )?))
    }

    fn rpc_ws(&self) -> Result<Option<SocketAddr>> {
        let ws_port = self.base.base.ws_port;
        Ok(Some(parse_address(
            &format!("127.0.0.1:{}", 9945),
            ws_port,
        )?))
    }

    fn prometheus_config(&self) -> Result<Option<PrometheusConfig>> {
        Ok(None)
    }

    // TODO: we disable mdns for the polkadot node because it prevents the process to exit
    //       properly. See https://github.com/paritytech/cumulus/issues/57
    fn network_config(
        &self,
        chain_spec: &Box<dyn sc_service::ChainSpec>,
        is_dev: bool,
        net_config_dir: PathBuf,
        client_id: &str,
        node_name: &str,
        node_key: NodeKeyConfig,
    ) -> Result<NetworkConfiguration> {
        let (mut network, allow_private_ipv4) = self
            .network_params()
            .map(|x| {
                (
                    x.network_config(
                        chain_spec,
                        is_dev,
                        Some(net_config_dir),
                        client_id,
                        node_name,
                        node_key,
                    ),
                    !x.no_private_ipv4,
                )
            })
            .expect("NetworkParams is always available on RunCmd; qed");

        network.transport = TransportConfig::Normal {
            enable_mdns: false,
            allow_private_ipv4,
            wasm_external_transport: None,
            use_yamux_flow_control: false,
        };

        Ok(network)
    }

    fn init<C: SubstrateCli>(&self) -> Result<()> {
        unreachable!("PolkadotCli is never initialized; qed");
    }
}

// copied directly from substrate
fn parse_address(address: &str, port: Option<u16>) -> std::result::Result<SocketAddr, String> {
    let mut address: SocketAddr = address
        .parse()
        .map_err(|_| format!("Invalid address: {}", address))?;
    if let Some(port) = port {
        address.set_port(port);
    }

    Ok(address)
}
