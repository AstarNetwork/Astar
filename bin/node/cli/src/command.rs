use crate::{chain_spec, service, Cli, Subcommand};
use plasm_runtime::Block;
use sc_cli::SubstrateCli;

impl SubstrateCli for Cli {
    fn impl_name() -> &'static str {
        "Plasm Node"
    }

    fn impl_version() -> &'static str {
        env!("SUBSTRATE_CLI_IMPL_VERSION")
    }

    fn description() -> &'static str {
        env!("CARGO_PKG_DESCRIPTION")
    }

    fn author() -> &'static str {
        env!("CARGO_PKG_AUTHORS")
    }

    fn support_url() -> &'static str {
        "https://github.com/staketechnologies/plasm/issues/new"
    }

    fn copyright_start_year() -> i32 {
        2019
    }

    fn executable_name() -> &'static str {
        "plasm-node"
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(match id {
            "dev" => Box::new(chain_spec::development_config()),
            "local" => Box::new(chain_spec::local_testnet_config()),
            "" | "dusty" => Box::new(chain_spec::dusty_config()),
            path => Box::new(chain_spec::ChainSpec::from_json_file(
                std::path::PathBuf::from(path),
            )?),
        })
    }
}

/// Parse command line arguments into service configuration.
pub fn run() -> sc_cli::Result<()> {
    sc_cli::reset_signal_pipe_handler()?;

    let cli = Cli::from_args();

    match &cli.subcommand {
        None => {
            let runner = cli.create_runner(&cli.run)?;
            runner.run_node(
                service::new_light,
                service::new_full,
                plasm_runtime::VERSION
            )
        },
        Some(Subcommand::Benchmark(cmd)) => {
            if cfg!(feature = "runtime-benchmarks") {
                let runner = cli.create_runner(cmd)?;

                runner.sync_run(|config| cmd.run::<Block, service::Executor>(config))
            } else {
                println!("Benchmarking wasn't enabled when building the node. \
                You can enable it with `--features runtime-benchmarks`.");
                Ok(())
            }
        },
        Some(Subcommand::Base(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;

            runner.run_subcommand(subcommand, |config| Ok(new_full_start!(config).0))
        },
    }
}
