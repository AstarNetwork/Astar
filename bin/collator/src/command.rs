//! Astar collator CLI handlers.
use crate::{
    cli::{Cli, RelayChainCli, Subcommand},
    local::{self, development_config},
    parachain::{self, chain_spec, shibuya, shiden, start_shibuya_node, start_shiden_node},
    primitives::Block,
};
use codec::Encode;
use cumulus_client_service::genesis::generate_genesis_block;
use cumulus_primitives_core::ParaId;
use log::info;
use polkadot_parachain::primitives::AccountIdConversion;
use sc_cli::{
    ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
    NetworkParams, Result, RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::{
    config::{BasePath, PrometheusConfig},
    PartialComponents,
};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::Block as BlockT;
use std::{io::Write, net::SocketAddr};

trait IdentifyChain {
    fn is_dev(&self) -> bool;
    fn is_shiden(&self) -> bool;
    fn is_shibuya(&self) -> bool;
}

impl IdentifyChain for dyn sc_service::ChainSpec {
    fn is_dev(&self) -> bool {
        self.id().starts_with("dev")
    }
    fn is_shiden(&self) -> bool {
        self.id().starts_with("shiden")
    }
    fn is_shibuya(&self) -> bool {
        self.id().starts_with("shibuya")
    }
}

impl<T: sc_service::ChainSpec + 'static> IdentifyChain for T {
    fn is_dev(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_dev(self)
    }
    fn is_shiden(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_shiden(self)
    }
    fn is_shibuya(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_shibuya(self)
    }
}

fn load_spec(
    id: &str,
    para_id: u32,
) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
    Ok(match id {
        "dev" => Box::new(development_config()),
        "" | "shiden" => Box::new(chain_spec::ShidenChainSpec::from_json_bytes(
            &include_bytes!("../res/shiden.raw.json")[..],
        )?),
        "shibuya-dev" => Box::new(chain_spec::shibuya::get_chain_spec(para_id)),
        "shiden-dev" => Box::new(chain_spec::shiden::get_chain_spec(para_id)),
        "shibuya" => Box::new(chain_spec::ShibuyaChainSpec::from_json_bytes(
            &include_bytes!("../res/shibuya.raw.json")[..],
        )?),
        path => {
            let chain_spec = chain_spec::ShibuyaChainSpec::from_json_file(path.into())?;
            if chain_spec.is_shiden() {
                Box::new(chain_spec::ShidenChainSpec::from_json_file(path.into())?)
            } else {
                Box::new(chain_spec)
            }
        }
    })
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "Shiden Collator".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        format!(
            "Shiden parachain collator\n\nThe command-line arguments provided first will be \
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
        "https://github.com/PlasmNetwork/Astar/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2019
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        load_spec(id, self.run.parachain_id)
    }

    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        if chain_spec.is_dev() {
            &local_runtime::VERSION
        } else if chain_spec.is_shiden() {
            &shiden_runtime::VERSION
        } else {
            &shibuya_runtime::VERSION
        }
    }
}

impl SubstrateCli for RelayChainCli {
    fn impl_name() -> String {
        "Shiden Collator".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        "Shiden parachain collator\n\nThe command-line arguments provided first will be \
        passed to the parachain node, while the arguments provided after -- will be passed \
        to the relaychain node.\n\n\
        astar-collator [parachain-args] -- [relaychain-args]"
            .into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/AstarNetwork/Astar/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2019
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        if id == "azores" {
            Ok(Box::new(
                polkadot_service::WestendChainSpec::from_json_bytes(
                    &include_bytes!("../res/azores.raw.json")[..],
                )
                .unwrap(),
            ))
        } else if id == "tokyo" {
            Ok(Box::new(
                polkadot_service::WestendChainSpec::from_json_bytes(
                    &include_bytes!("../res/tokyo.raw.json")[..],
                )
                .unwrap(),
            ))
        } else {
            polkadot_cli::Cli::from_iter([RelayChainCli::executable_name().to_string()].iter())
                .load_spec(id)
        }
    }

    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        polkadot_cli::Cli::native_runtime_version(chain_spec)
    }
}

fn extract_genesis_wasm(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Result<Vec<u8>> {
    let mut storage = chain_spec.build_storage()?;

    storage
        .top
        .remove(sp_core::storage::well_known_keys::CODE)
        .ok_or_else(|| "Could not find wasm file in genesis state!".into())
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::CheckBlock(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            if runner.config().chain_spec.is_shiden() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        import_queue,
                        ..
                    } = parachain::new_partial::<shiden::RuntimeApi, shiden::Executor, _>(
                        &config,
                        parachain::build_import_queue,
                    )?;
                    Ok((cmd.run(client, import_queue), task_manager))
                })
            } else {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        import_queue,
                        ..
                    } = parachain::new_partial::<shibuya::RuntimeApi, shibuya::Executor, _>(
                        &config,
                        parachain::build_import_queue,
                    )?;
                    Ok((cmd.run(client, import_queue), task_manager))
                })
            }
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            if runner.config().chain_spec.is_shiden() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        ..
                    } = parachain::new_partial::<shiden::RuntimeApi, shiden::Executor, _>(
                        &config,
                        parachain::build_import_queue,
                    )?;
                    Ok((cmd.run(client, config.database), task_manager))
                })
            } else {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        ..
                    } = parachain::new_partial::<shibuya::RuntimeApi, shibuya::Executor, _>(
                        &config,
                        parachain::build_import_queue,
                    )?;
                    Ok((cmd.run(client, config.database), task_manager))
                })
            }
        }
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            if runner.config().chain_spec.is_shiden() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        ..
                    } = parachain::new_partial::<shiden::RuntimeApi, shiden::Executor, _>(
                        &config,
                        parachain::build_import_queue,
                    )?;
                    Ok((cmd.run(client, config.chain_spec), task_manager))
                })
            } else {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        ..
                    } = parachain::new_partial::<shibuya::RuntimeApi, shibuya::Executor, _>(
                        &config,
                        parachain::build_import_queue,
                    )?;
                    Ok((cmd.run(client, config.chain_spec), task_manager))
                })
            }
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            if runner.config().chain_spec.is_shiden() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        import_queue,
                        ..
                    } = parachain::new_partial::<shiden::RuntimeApi, shiden::Executor, _>(
                        &config,
                        parachain::build_import_queue,
                    )?;
                    Ok((cmd.run(client, import_queue), task_manager))
                })
            } else {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        import_queue,
                        ..
                    } = parachain::new_partial::<shibuya::RuntimeApi, shibuya::Executor, _>(
                        &config,
                        parachain::build_import_queue,
                    )?;
                    Ok((cmd.run(client, import_queue), task_manager))
                })
            }
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| {
                let polkadot_cli = RelayChainCli::new(
                    &config,
                    [RelayChainCli::executable_name().to_string()]
                        .iter()
                        .chain(cli.relaychain_args.iter()),
                );
                let polkadot_config = SubstrateCli::create_configuration(
                    &polkadot_cli,
                    &polkadot_cli,
                    config.tokio_handle.clone(),
                )
                .map_err(|err| format!("Relay chain argument error: {}", err))?;

                cmd.run(config, polkadot_config)
            })
        }
        Some(Subcommand::Revert(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            if runner.config().chain_spec.is_shiden() {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        backend,
                        ..
                    } = parachain::new_partial::<shiden::RuntimeApi, shiden::Executor, _>(
                        &config,
                        parachain::build_import_queue,
                    )?;
                    Ok((cmd.run(client, backend), task_manager))
                })
            } else {
                runner.async_run(|config| {
                    let PartialComponents {
                        client,
                        task_manager,
                        backend,
                        ..
                    } = parachain::new_partial::<shibuya::RuntimeApi, shibuya::Executor, _>(
                        &config,
                        parachain::build_import_queue,
                    )?;
                    Ok((cmd.run(client, backend), task_manager))
                })
            }
        }
        Some(Subcommand::ExportGenesisState(params)) => {
            let mut builder = sc_cli::LoggerBuilder::new("");
            builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
            let _ = builder.init();

            let block: Block = generate_genesis_block(&load_spec(
                &params.chain.clone().unwrap_or_default(),
                params.parachain_id.into(),
            )?)?;
            let raw_header = block.header().encode();
            let output_buf = if params.raw {
                raw_header
            } else {
                format!("0x{:?}", HexDisplay::from(&block.header().encode())).into_bytes()
            };

            if let Some(output) = &params.output {
                std::fs::write(output, output_buf)?;
            } else {
                std::io::stdout().write_all(&output_buf)?;
            }

            Ok(())
        }
        Some(Subcommand::ExportGenesisWasm(params)) => {
            let mut builder = sc_cli::LoggerBuilder::new("");
            builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
            let _ = builder.init();

            let raw_wasm_blob =
                extract_genesis_wasm(&cli.load_spec(&params.chain.clone().unwrap_or_default())?)?;
            let output_buf = if params.raw {
                raw_wasm_blob
            } else {
                format!("0x{:?}", HexDisplay::from(&raw_wasm_blob)).into_bytes()
            };

            if let Some(output) = &params.output {
                std::fs::write(output, output_buf)?;
            } else {
                std::io::stdout().write_all(&output_buf)?;
            }

            Ok(())
        }
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::Sign(cmd)) => cmd.run(),
        Some(Subcommand::Verify(cmd)) => cmd.run(),
        Some(Subcommand::Vanity(cmd)) => cmd.run(),
        #[cfg(feature = "frame-benchmarking")]
        Some(Subcommand::Benchmark(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            let chain_spec = &runner.config().chain_spec;

            if chain_spec.is_shiden() {
                runner.sync_run(|config| cmd.run::<shiden_runtime::Block, shiden::Executor>(config))
            } else if chain_spec.is_shibuya() {
                runner
                    .sync_run(|config| cmd.run::<shibuya_runtime::Block, shibuya::Executor>(config))
            } else {
                runner.sync_run(|config| cmd.run::<Block, local::Executor>(config))
            }
        }
        #[cfg(feature = "try-runtime")]
        Some(Subcommand::TryRuntime(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            let chain_spec = &runner.config().chain_spec;

            if chain_spec.is_shiden() {
                runner.async_run(|config| {
                    let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
                    let task_manager =
                        sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
                            .map_err(|e| {
                                sc_cli::Error::Service(sc_service::Error::Prometheus(e))
                            })?;
                    Ok((
                        cmd.run::<shiden_runtime::Block, shiden::Executor>(config),
                        task_manager,
                    ))
                })
            } else if chain_spec.is_shibuya() {
                runner.async_run(|config| {
                    let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
                    let task_manager =
                        sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
                            .map_err(|e| {
                                sc_cli::Error::Service(sc_service::Error::Prometheus(e))
                            })?;
                    Ok((
                        cmd.run::<shibuya_runtime::Block, shibuya::Executor>(config),
                        task_manager,
                    ))
                })
            } else {
                runner.async_run(|config| {
                    let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
                    let task_manager =
                        sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
                            .map_err(|e| {
                                sc_cli::Error::Service(sc_service::Error::Prometheus(e))
                            })?;
                    Ok((cmd.run::<Block, local::Executor>(config), task_manager))
                })
            }
        }
        None => {
            let runner = cli.create_runner(&*cli.run)?;

            runner.run_node_until_exit(|config| async move {
                if config.chain_spec.is_dev() {
                    return local::start_node(config).map_err(Into::into);
                }

                let polkadot_cli = RelayChainCli::new(
                    &config,
                    [RelayChainCli::executable_name().to_string()]
                        .iter()
                        .chain(cli.relaychain_args.iter()),
                );

                let id = ParaId::from(cli.run.parachain_id);

                let parachain_account =
                    AccountIdConversion::<polkadot_primitives::v0::AccountId>::into_account(&id);

                let block: Block =
                    generate_genesis_block(&config.chain_spec).map_err(|e| format!("{:?}", e))?;
                let genesis_state = format!("0x{:?}", HexDisplay::from(&block.header().encode()));

                let polkadot_config = SubstrateCli::create_configuration(
                    &polkadot_cli,
                    &polkadot_cli,
                    config.tokio_handle.clone(),
                )
                .map_err(|err| format!("Relay chain argument error: {}", err))?;

                info!("Parachain id: {:?}", id);
                info!("Parachain Account: {}", parachain_account);
                info!("Parachain genesis state: {}", genesis_state);
                info!(
                    "Is collating: {}",
                    if config.role.is_authority() {
                        "yes"
                    } else {
                        "no"
                    }
                );

                if config.chain_spec.is_shiden() {
                    start_shiden_node(config, polkadot_config, id)
                        .await
                        .map(|r| r.0)
                        .map_err(Into::into)
                } else {
                    start_shibuya_node(config, polkadot_config, id)
                        .await
                        .map(|r| r.0)
                        .map_err(Into::into)
                }
            })
        }
    }
}

impl DefaultConfigurationValues for RelayChainCli {
    fn p2p_listen_port() -> u16 {
        30334
    }

    fn rpc_ws_listen_port() -> u16 {
        9945
    }

    fn rpc_http_listen_port() -> u16 {
        9934
    }

    fn prometheus_listen_port() -> u16 {
        9616
    }
}

impl CliConfiguration<Self> for RelayChainCli {
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

    fn base_path(&self) -> Result<Option<BasePath>> {
        Ok(self
            .shared_params()
            .base_path()
            .or_else(|| self.base_path.clone().map(Into::into)))
    }

    fn rpc_http(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
        self.base.base.rpc_http(default_listen_port)
    }

    fn rpc_ipc(&self) -> Result<Option<String>> {
        self.base.base.rpc_ipc()
    }

    fn rpc_ws(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
        self.base.base.rpc_ws(default_listen_port)
    }

    fn prometheus_config(&self, default_listen_port: u16) -> Result<Option<PrometheusConfig>> {
        self.base.base.prometheus_config(default_listen_port)
    }

    fn init<C: SubstrateCli>(&self) -> Result<()> {
        unreachable!("PolkadotCli is never initialized; qed");
    }

    fn chain_id(&self, is_dev: bool) -> Result<String> {
        let chain_id = self.base.base.chain_id(is_dev)?;

        Ok(if chain_id.is_empty() {
            self.chain_id.clone().unwrap_or_default()
        } else {
            chain_id
        })
    }

    fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
        self.base.base.role(is_dev)
    }

    fn transaction_pool(&self) -> Result<sc_service::config::TransactionPoolOptions> {
        self.base.base.transaction_pool()
    }

    fn state_cache_child_ratio(&self) -> Result<Option<usize>> {
        self.base.base.state_cache_child_ratio()
    }

    fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
        self.base.base.rpc_methods()
    }

    fn rpc_ws_max_connections(&self) -> Result<Option<usize>> {
        self.base.base.rpc_ws_max_connections()
    }

    fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
        self.base.base.rpc_cors(is_dev)
    }

    fn default_heap_pages(&self) -> Result<Option<u64>> {
        self.base.base.default_heap_pages()
    }

    fn force_authoring(&self) -> Result<bool> {
        self.base.base.force_authoring()
    }

    fn disable_grandpa(&self) -> Result<bool> {
        self.base.base.disable_grandpa()
    }

    fn max_runtime_instances(&self) -> Result<Option<usize>> {
        self.base.base.max_runtime_instances()
    }

    fn announce_block(&self) -> Result<bool> {
        self.base.base.announce_block()
    }

    fn telemetry_endpoints(
        &self,
        chain_spec: &Box<dyn ChainSpec>,
    ) -> Result<Option<sc_telemetry::TelemetryEndpoints>> {
        self.base.base.telemetry_endpoints(chain_spec)
    }
}
