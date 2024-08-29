// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

//! Astar collator CLI handlers.
use crate::{
    cli::{Cli, RelayChainCli, Subcommand},
    evm_tracing_types::EvmTracingConfig,
    local::{self, development_config},
    parachain::{self, chain_spec, service::AdditionalConfig},
};
use cumulus_primitives_core::ParaId;
use log::info;
use sc_cli::{
    ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
    NetworkParams, Result, SharedParams, SubstrateCli,
};
use sc_service::{
    config::{BasePath, PrometheusConfig},
    PartialComponents,
};
use sp_runtime::traits::AccountIdConversion;
use std::net::SocketAddr;

#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking_cli::{BenchmarkCmd, ExtrinsicFactory, SUBSTRATE_REFERENCE_HARDWARE};

trait IdentifyChain {
    fn is_astar(&self) -> bool;
    fn is_dev(&self) -> bool;
    fn is_shiden(&self) -> bool;
    fn is_shibuya(&self) -> bool;
}

impl IdentifyChain for dyn sc_service::ChainSpec {
    fn is_astar(&self) -> bool {
        self.id().starts_with("astar")
    }
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
    fn is_astar(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_astar(self)
    }
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

fn load_spec(id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
    Ok(match id {
        "dev" => Box::new(development_config()),
        "astar-dev" => Box::new(chain_spec::astar::get_chain_spec()),
        "shibuya-dev" => Box::new(chain_spec::shibuya::get_chain_spec()),
        "shiden-dev" => Box::new(chain_spec::shiden::get_chain_spec()),
        "astar" => Box::new(chain_spec::AstarChainSpec::from_json_bytes(
            &include_bytes!("../res/astar.raw.json")[..],
        )?),
        "shiden" => Box::new(chain_spec::ShidenChainSpec::from_json_bytes(
            &include_bytes!("../res/shiden.raw.json")[..],
        )?),
        "shibuya" => Box::new(chain_spec::ShibuyaChainSpec::from_json_bytes(
            &include_bytes!("../res/shibuya.raw.json")[..],
        )?),
        path => {
            let chain_spec = chain_spec::ShibuyaChainSpec::from_json_file(path.into())?;
            if chain_spec.is_astar() {
                Box::new(chain_spec::AstarChainSpec::from_json_file(path.into())?)
            } else if chain_spec.is_shiden() {
                Box::new(chain_spec::ShidenChainSpec::from_json_file(path.into())?)
            } else if chain_spec.is_shibuya() {
                Box::new(chain_spec)
            } else {
                Err("Unclear which chain spec to base this chain on. Name should start with astar, shiden or shibuya if custom name is used")?
            }
        }
    })
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "Astar Collator".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        format!(
            "Astar Collator\n\nThe command-line arguments provided first will be \
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
        "https://github.com/AstarNetwork/Astar/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2019
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        load_spec(id)
    }
}

impl SubstrateCli for RelayChainCli {
    fn impl_name() -> String {
        "Astar Collator".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        "Astar Collator\n\nThe command-line arguments provided first will be \
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
        if id == "tokyo" {
            Ok(Box::new(
                polkadot_service::WestendChainSpec::from_json_bytes(
                    &include_bytes!("../res/tokyo.raw.json")[..],
                )
                .unwrap(),
            ))
        } else {
            polkadot_cli::Cli::from_iter([RelayChainCli::executable_name()].iter()).load_spec(id)
        }
    }
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
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = parachain::new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = parachain::new_partial(&config)?;
                Ok((cmd.run(client, config.database), task_manager))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = parachain::new_partial(&config)?;
                Ok((cmd.run(client, config.chain_spec), task_manager))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = parachain::new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| {
                let polkadot_cli = RelayChainCli::new(
                    &config,
                    [RelayChainCli::executable_name()]
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
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    backend,
                    ..
                } = parachain::new_partial(&config)?;
                let aux_revert = Box::new(|client, _, blocks| {
                    sc_consensus_grandpa::revert(client, blocks)?;
                    Ok(())
                });
                Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
            })
        }
        Some(Subcommand::ExportGenesisState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| {
                let PartialComponents { client, .. } = parachain::new_partial(&config)?;
                cmd.run(client)
            })
        }
        Some(Subcommand::ExportGenesisWasm(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            runner.sync_run(|_config| {
                let spec = cli.load_spec(&cmd.shared_params.chain.clone().unwrap_or_default())?;
                cmd.run(&*spec)
            })
        }
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::Sign(cmd)) => cmd.run(),
        Some(Subcommand::Verify(cmd)) => cmd.run(),
        Some(Subcommand::Vanity(cmd)) => cmd.run(),
        #[cfg(feature = "runtime-benchmarks")]
        Some(Subcommand::Benchmark(cmd)) => {
            use crate::benchmarking::*;
            use sp_keyring::Sr25519Keyring;
            use sp_runtime::traits::HashingFor;

            let runner = cli.create_runner(cmd)?;
            let chain_spec = &runner.config().chain_spec;

            match cmd {
                BenchmarkCmd::Pallet(cmd) => {
                    if chain_spec.is_astar() {
                        runner.sync_run(|config| {
                            cmd.run_with_spec::<HashingFor<astar_runtime::Block>, parachain::HostFunctions>(
                                Some(config.chain_spec),
                            )
                        })
                    } else if chain_spec.is_shiden() {
                        runner.sync_run(|config| {
                            cmd.run_with_spec::<HashingFor<shiden_runtime::Block>, parachain::HostFunctions>(
                                Some(config.chain_spec),
                            )
                        })
                    } else if chain_spec.is_shibuya() {
                        runner.sync_run(|config| {
                            cmd.run_with_spec::<HashingFor<shibuya_runtime::Block>, parachain::HostFunctions>(
                                Some(config.chain_spec),
                            )
                        })
                    } else {
                        runner.sync_run(|config| {
                            cmd.run_with_spec::<HashingFor<local_runtime::Block>, local::HostFunctions>(
                                Some(config.chain_spec),
                            )
                        })
                    }
                }
                BenchmarkCmd::Block(cmd) => {
                    if chain_spec.is_dev() {
                        runner.sync_run(|config| {
                            let params = local::new_partial(&config)?;
                            cmd.run(params.client)
                        })
                    } else {
                        runner.sync_run(|config| {
                            let params = parachain::new_partial(&config)?;
                            cmd.run(params.client)
                        })
                    }
                }
                BenchmarkCmd::Storage(cmd) => {
                    if chain_spec.is_dev() {
                        runner.sync_run(|config| {
                            let params = local::new_partial(&config)?;
                            let db = params.backend.expose_db();
                            let storage = params.backend.expose_storage();

                            cmd.run(config, params.client, db, storage)
                        })
                    } else {
                        runner.sync_run(|config| {
                            let params = parachain::new_partial(&config)?;
                            let db = params.backend.expose_db();
                            let storage = params.backend.expose_storage();

                            cmd.run(config, params.client, db, storage)
                        })
                    }
                }
                BenchmarkCmd::Overhead(cmd) => {
                    if chain_spec.is_dev() {
                        runner.sync_run(|config| {
                            let params = local::new_partial(&config)?;
                            let ext_builder = RemarkBuilder::new(params.client.clone());
                            let inherent_data = local_benchmark_inherent_data()
                                .map_err(|e| format!("generating inherent data: {:?}", e))?;

                            cmd.run(
                                config,
                                params.client,
                                inherent_data,
                                Vec::new(),
                                &ext_builder,
                            )
                        })
                    } else {
                        runner.sync_run(|config| {
                            let params = parachain::new_partial(&config)?;

                            let ext_builder = RemarkBuilder::new(params.client.clone());
                            let inherent_data = para_benchmark_inherent_data()
                                .map_err(|e| format!("generating inherent data: {:?}", e))?;

                            cmd.run(
                                config,
                                params.client,
                                inherent_data,
                                Vec::new(),
                                &ext_builder,
                            )
                        })
                    }
                }
                BenchmarkCmd::Extrinsic(cmd) => {
                    if chain_spec.is_dev() {
                        runner.sync_run(|config| {
                            let params = local::new_partial(&config)?;
                            let remark_builder = RemarkBuilder::new(params.client.clone());
                            let tka_builder = TransferKeepAliveBuilder::new(
                                params.client.clone(),
                                Sr25519Keyring::Alice.to_account_id(),
                                params.client.existential_deposit(),
                            );
                            let ext_factory = ExtrinsicFactory(vec![
                                Box::new(remark_builder),
                                Box::new(tka_builder),
                            ]);
                            let inherent_data = local_benchmark_inherent_data()
                                .map_err(|e| format!("generating inherent data: {:?}", e))?;

                            cmd.run(params.client, inherent_data, Vec::new(), &ext_factory)
                        })
                    } else {
                        runner.sync_run(|config| {
                            let params = parachain::new_partial(&config)?;
                            let remark_builder = RemarkBuilder::new(params.client.clone());
                            let tka_builder = TransferKeepAliveBuilder::new(
                                params.client.clone(),
                                Sr25519Keyring::Alice.to_account_id(),
                                params.client.existential_deposit(),
                            );
                            let ext_factory = ExtrinsicFactory(vec![
                                Box::new(remark_builder),
                                Box::new(tka_builder),
                            ]);
                            let inherent_data = para_benchmark_inherent_data()
                                .map_err(|e| format!("generating inherent data: {:?}", e))?;

                            cmd.run(params.client, inherent_data, Vec::new(), &ext_factory)
                        })
                    }
                }
                BenchmarkCmd::Machine(cmd) => {
                    runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone()))
                }
            }
        }
        None => {
            let runner = cli.create_runner(&cli.run.normalize())?;
            let collator_options = cli.run.collator_options();

            let evm_tracing_config = EvmTracingConfig {
                ethapi: cli.eth_api_options.ethapi,
                ethapi_max_permits: cli.eth_api_options.ethapi_max_permits,
                ethapi_trace_max_count: cli.eth_api_options.ethapi_trace_max_count,
                ethapi_trace_cache_duration: cli.eth_api_options.ethapi_trace_cache_duration,
                eth_log_block_cache: cli.eth_api_options.eth_log_block_cache,
                eth_statuses_cache: cli.eth_api_options.eth_statuses_cache,
                max_past_logs: cli.eth_api_options.max_past_logs,
                tracing_raw_max_memory_usage: cli.eth_api_options.tracing_raw_max_memory_usage,
            };

            runner.run_node_until_exit(|config| async move {
                if config.chain_spec.is_dev() {
                    return local::start_node::<sc_network::NetworkWorker<_, _>>(
                        config,
                        evm_tracing_config,
                    )
                    .map_err(Into::into);
                }

                let polkadot_cli = RelayChainCli::new(
                    &config,
                    [RelayChainCli::executable_name()]
                        .iter()
                        .chain(cli.relaychain_args.iter()),
                );

                let para_id = ParaId::from(
                    chain_spec::Extensions::try_get(&*config.chain_spec)
                        .map(|e| e.para_id)
                        .ok_or("ParaId not found in chain spec extension")?,
                );

                let parachain_account =
                    AccountIdConversion::<polkadot_primitives::AccountId>::into_account_truncating(
                        &para_id,
                    );

                let polkadot_config = SubstrateCli::create_configuration(
                    &polkadot_cli,
                    &polkadot_cli,
                    config.tokio_handle.clone(),
                )
                .map_err(|err| format!("Relay chain argument error: {}", err))?;

                info!("Parachain id: {:?}", para_id);
                info!("Parachain Account: {}", parachain_account);
                info!(
                    "Is collating: {}",
                    if config.role.is_authority() {
                        "yes"
                    } else {
                        "no"
                    }
                );

                let hwbench = (!cli.no_hardware_benchmarks)
                    .then_some(config.database.path().map(|database_path| {
                        let _ = std::fs::create_dir_all(database_path);
                        sc_sysinfo::gather_hwbench(Some(database_path))
                    }))
                    .flatten();

                let additional_config = AdditionalConfig {
                    evm_tracing_config,
                    enable_evm_rpc: cli.enable_evm_rpc,
                    proposer_block_size_limit: cli.proposer_block_size_limit,
                    proposer_soft_deadline_percent: cli.proposer_soft_deadline_percent,
                    hwbench,
                };

                parachain::start_node(
                    config,
                    polkadot_config,
                    collator_options,
                    para_id,
                    additional_config,
                )
                .await
                .map(|r| r.0)
                .map_err(Into::into)
            })
        }
    }
}

impl DefaultConfigurationValues for RelayChainCli {
    fn p2p_listen_port() -> u16 {
        30334
    }

    fn rpc_listen_port() -> u16 {
        9945
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
            .base_path()?
            .or_else(|| self.base_path.clone().map(Into::into)))
    }

    fn rpc_addr(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
        self.base.base.rpc_addr(default_listen_port)
    }

    fn prometheus_config(
        &self,
        default_listen_port: u16,
        chain_spec: &Box<dyn ChainSpec>,
    ) -> Result<Option<PrometheusConfig>> {
        self.base
            .base
            .prometheus_config(default_listen_port, chain_spec)
    }

    fn init<F>(
        &self,
        _support_url: &String,
        _impl_version: &String,
        _logger_hook: F,
        _config: &sc_service::Configuration,
    ) -> Result<()>
    where
        F: FnOnce(&mut sc_cli::LoggerBuilder, &sc_service::Configuration),
    {
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

    fn transaction_pool(&self, is_dev: bool) -> Result<sc_service::config::TransactionPoolOptions> {
        self.base.base.transaction_pool(is_dev)
    }

    fn trie_cache_maximum_size(&self) -> Result<Option<usize>> {
        self.base.base.trie_cache_maximum_size()
    }

    fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
        self.base.base.rpc_methods()
    }

    fn rpc_max_connections(&self) -> Result<u32> {
        self.base.base.rpc_max_connections()
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
