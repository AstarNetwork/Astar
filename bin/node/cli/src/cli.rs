pub use substrate_cli::error;
pub use substrate_cli::{VersionInfo, IntoExit};
use tokio::prelude::Future;
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};
use substrate_cli::NoCustom;
use substrate_service::{AbstractService, Roles as ServiceRoles, Configuration};
use log::info;
use substrate_cli::{display_role, parse_and_prepare, ParseAndPrepare};
use crate::{service, load_spec};

/// Parse command line arguments into service configuration.
pub fn run<I, T, E>(args: I, exit: E, version: substrate_cli::VersionInfo) -> error::Result<()> where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
    E: IntoExit,
{
    type Config<A, B> = Configuration<(), A, B>;

    match parse_and_prepare::<NoCustom, NoCustom, _>(&version, "plasm-node", args) {
        ParseAndPrepare::Run(cmd) => cmd.run(load_spec, exit,
        |exit, _cli_args, _custom_args, config: Config<_, _>| {
            info!("{}", version.name);
            info!("  version {}", config.full_version());
            info!("  by Stake Technologies, 2018-2019");
            info!("Chain specification: {}", config.chain_spec.name());
            info!("Node name: {}", config.name);
            info!("Roles: {}", display_role(&config));
            let runtime = RuntimeBuilder::new().name_prefix("main-tokio-").build()
                .map_err(|e| format!("{:?}", e))?;
            match config.roles {
                ServiceRoles::LIGHT => run_until_exit(
                    runtime,
                    service::new_light(config)?,
                    exit
                ),
                _ => run_until_exit(
                    runtime,
                    service::new_full(config)?,
                    exit
                ),
                                                 }
                                             }),
        ParseAndPrepare::BuildSpec(cmd) => cmd.run::<NoCustom, _, _, _>(load_spec),
        ParseAndPrepare::ExportBlocks(cmd) => cmd.run_with_builder(|config: Config<_, _>|
            Ok(new_full_start!(config).0), load_spec, exit),
        ParseAndPrepare::ImportBlocks(cmd) => cmd.run_with_builder(|config: Config<_, _>|
            Ok(new_full_start!(config).0), load_spec, exit),
        ParseAndPrepare::CheckBlock(cmd) => cmd.run_with_builder(|config: Config<_, _>|
            Ok(new_full_start!(config).0), load_spec, exit),
        ParseAndPrepare::PurgeChain(cmd) => cmd.run(load_spec),
        ParseAndPrepare::RevertChain(cmd) => cmd.run_with_builder(|config: Config<_, _>|
            Ok(new_full_start!(config).0), load_spec),
        ParseAndPrepare::CustomCommand(_) => Ok(()),
    }
}

fn run_until_exit<T, E>(
    mut runtime: Runtime,
    service: T,
    e: E,
) -> error::Result<()>
    where
        T: AbstractService,
        E: IntoExit,
{
    use futures::{FutureExt, TryFutureExt, channel::oneshot, future::select, compat::Future01CompatExt};

    let (exit_send, exit) = oneshot::channel();

    let informant = substrate_cli::informant::build(&service);

    let future = select(informant, exit).map(|_| Ok(())).compat();
    runtime.executor().spawn(future);

    // we eagerly drop the service so that the internal exit future is fired,
    // but we need to keep holding a reference to the global telemetry guard
    let _telemetry = service.telemetry();

    let service_res = {
        let exit = e.into_exit();
        let service = service
            .map_err(|err| error::Error::Service(err))
            .compat();
        let select = select(service, exit)
            .map(|_| Ok(()))
            .compat();
        runtime.block_on(select)
    };

    let _ = exit_send.send(());

    // TODO [andre]: timeout this future #1318
    let _ = runtime.shutdown_on_idle().wait();

    service_res
}
