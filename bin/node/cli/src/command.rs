use crate::{chain_spec, service, Cli, Subcommand};
use plasm_runtime::Block;
use sc_cli::{ChainSpec, Role, RuntimeVersion, SubstrateCli};
use sc_service::ServiceParams;

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "Plasm Node".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").into()
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
        Ok(match id {
            "dev" => Box::new(chain_spec::development_config()),
            "local" => Box::new(chain_spec::local_testnet_config()),
            "" | "dusty" => Box::new(chain_spec::dusty_config()),
            "plasm" => Box::new(chain_spec::plasm_config()),
            path => Box::new(chain_spec::ChainSpec::from_json_file(
                std::path::PathBuf::from(path),
            )?),
        })
    }

    fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        &plasm_runtime::VERSION
    }
}

/// Parse command line arguments into service configuration.
pub fn run() -> sc_cli::Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        None => {
            let runner = cli.create_runner(&cli.run)?;
            runner.run_node_until_exit(|config| match config.role {
                Role::Light => service::new_light(config),
                _ => service::new_full(config),
            })
        }
        Some(Subcommand::Benchmark(cmd)) => {
            if cfg!(feature = "runtime-benchmarks") {
                let runner = cli.create_runner(cmd)?;

                runner.sync_run(|config| cmd.run::<Block, service::Executor>(config))
            } else {
                println!(
                    "Benchmarking wasn't enabled when building the node. \
                You can enable it with `--features runtime-benchmarks`."
                );
                Ok(())
            }
        }
        Some(Subcommand::Base(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;

            runner.run_subcommand(subcommand, |config| {
                let (
                    ServiceParams {
                        client,
                        backend,
                        import_queue,
                        task_manager,
                        ..
                    },
                    ..,
                ) = service::new_full_params(config)?;
                Ok((client, backend, import_queue, task_manager))
            })
        }
        Some(Subcommand::LockdropOracle(config)) => {
            sc_cli::init_logger("");
            log::info!("Plasm Lockdrop oracle launched.");
            Ok(futures::executor::block_on(lockdrop_oracle::start(
                config.clone(),
            )))
        }
    }
}
