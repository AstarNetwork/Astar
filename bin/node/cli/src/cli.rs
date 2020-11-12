use sc_cli::{RunCmd, KeySubcommand, SignCmd, VanityCmd, VerifyCmd};
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
    /// Key management cli utilities
    Key(KeySubcommand),

    /// Verify a signature for a message, provided on STDIN, with a given (public or secret) key.
    Verify(VerifyCmd),

    /// Generate a seed that provides a vanity address.
    Vanity(VanityCmd),

    /// Sign a message, with a given (secret) key.
    Sign(SignCmd),

    /// Build a chain specification.
    BuildSpec(sc_cli::BuildSpecCmd),

    /// Remove the whole chain.
    PurgeChain(sc_cli::PurgeChainCmd),
}
