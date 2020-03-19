use sc_cli::VersionInfo;
use crate::{Cli, service, load_spec, Subcommand};

/// Parse command line arguments into service configuration.
pub fn run(version: VersionInfo) -> sc_cli::Result<()> {
    let opt = sc_cli::from_args::<Cli>(&version);
    let mut config = sc_service::Configuration::from_version(&version);

    match opt.subcommand {
        None => {
            opt.run.init(&version)?;
            opt.run.update_config(&mut config, load_spec, &version)?;
            opt.run.run(
                config,
                service::new_light,
                service::new_full,
                &version,
            )
        },
        Some(Subcommand::Benchmark(cmd)) => {
            cmd.init(&version)?;
            cmd.update_config(&mut config, load_spec, &version)?;

            cmd.run::<plasm_runtime::Block, plasm_executor::Executor>(config)
        },
        Some(Subcommand::Base(subcommand)) => {
            subcommand.init(&version)?;
            subcommand.update_config(&mut config, load_spec, &version)?;
            subcommand.run(
                config,
                |config: sc_service::Configuration| Ok(new_full_start!(config).0),
            )
        },
    }
}
