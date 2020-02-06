use sc_cli::RunCmd;
use structopt::StructOpt;

#[allow(missing_docs)]
#[derive(Clone, Debug, StructOpt)]
#[structopt(settings = &[
structopt::clap::AppSettings::GlobalVersion,
structopt::clap::AppSettings::ArgsNegateSubcommands,
structopt::clap::AppSettings::SubcommandsNegateReqs,
])]
pub struct Cli {
    #[allow(missing_docs)]
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub run: RunCmd,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, StructOpt)]
pub enum Subcommand {
    #[allow(missing_docs)]
    #[structopt(flatten)]
    Base(sc_cli::Subcommand),
}
