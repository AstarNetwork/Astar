use crate::{load_spec, service, Cli, Subcommand};
use sc_cli::{error, VersionInfo};

/// Parse command line arguments into service configuration.
pub fn run<I, T>(args: I, version: VersionInfo) -> error::Result<()>
where
    I: Iterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args: Vec<_> = args.collect();
    let opt = sc_cli::from_iter::<Cli, _>(args.clone(), &version);

    let mut config = sc_service::Configuration::default();
    config.impl_name = "plasm-node";

    match opt.subcommand {
        None => sc_cli::run(
            config,
            opt.run,
            service::new_light,
            service::new_full,
            load_spec,
            &version,
        ),
        Some(Subcommand::Base(subcommand)) => sc_cli::run_subcommand(
            config,
            subcommand,
            load_spec,
            |config: service::NodeConfiguration| Ok(new_full_start!(config).0),
            &version,
        ),
    }
}
