use sc_cli::RunCmd;
use structopt::StructOpt;

/// An overarching CLI command definition.
#[derive(Debug, StructOpt)]
pub struct Cli {
    /// Possible subcommand with parameters.
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub run: RunCmd,
}

/// Possible subcommands of the main binary.
#[derive(Debug, StructOpt)]
pub enum Subcommand {
    /// A set of base subcommands handled by `sc_cli`.
    #[structopt(flatten)]
    Base(sc_cli::Subcommand),
    /// Plasm Lockdrop oracle authority support module.
    #[structopt(name = "oracle", about = "Launch lockdrop oracle module.")]
    LockdropOracle(lockdrop_oracle::Config),
}
