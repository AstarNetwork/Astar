use sc_cli::{KeySubcommand, SignCmd, VanityCmd, VerifyCmd};
use std::path::PathBuf;
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

    /// Relaychain arguments
    #[structopt(raw = true)]
    pub relaychain_args: Vec<String>,
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

    /// Validate blocks.
    CheckBlock(sc_cli::CheckBlockCmd),

    /// Export blocks.
    ExportBlocks(sc_cli::ExportBlocksCmd),

    /// Export the state of a given block into a chain spec.
    ExportState(sc_cli::ExportStateCmd),

    /// Import blocks.
    ImportBlocks(sc_cli::ImportBlocksCmd),

    /// Remove the whole chain.
    PurgeChain(cumulus_client_cli::PurgeChainCmd),

    /// Revert the chain to a previous state.
    Revert(sc_cli::RevertCmd),

    /// Export the genesis state of the parachain.
    #[structopt(name = "export-genesis-state")]
    ExportGenesisState(ExportGenesisStateCommand),

    /// Export the genesis wasm of the parachain.
    #[structopt(name = "export-genesis-wasm")]
    ExportGenesisWasm(ExportGenesisWasmCommand),
}

/// Command for exporting the genesis state of the parachain
#[derive(Debug, StructOpt)]
pub struct ExportGenesisStateCommand {
    /// Output file name or stdout if unspecified.
    #[structopt(parse(from_os_str))]
    pub output: Option<PathBuf>,

    /// Write output in binary. Default is to write in hex.
    #[structopt(short, long)]
    pub raw: bool,

    /// Id of the parachain this state is for.
    ///
    /// Default: 2007 (shiden)
    #[structopt(long, default_value = "2007")]
    pub parachain_id: u32,

    /// The name of the chain for that the genesis state should be exported.
    #[structopt(long)]
    pub chain: Option<String>,
}

/// Command for exporting the genesis wasm file.
#[derive(Debug, StructOpt)]
pub struct ExportGenesisWasmCommand {
    /// Output file name or stdout if unspecified.
    #[structopt(parse(from_os_str))]
    pub output: Option<PathBuf>,

    /// Write output in binary. Default is to write in hex.
    #[structopt(short, long)]
    pub raw: bool,

    /// The name of the chain for that the genesis wasm file should be exported.
    #[structopt(long)]
    pub chain: Option<String>,
}

#[allow(missing_docs)]
#[derive(Debug, StructOpt)]
pub struct RunCmd {
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub base: sc_cli::RunCmd,

    /// Id of the parachain this collator collates for.
    ///
    /// Default: 2007 (shiden)
    #[structopt(long, default_value = "2007")]
    pub parachain_id: u32,
}

impl std::ops::Deref for RunCmd {
    type Target = sc_cli::RunCmd;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

#[derive(Debug)]
#[allow(missing_docs)]
pub struct RelayChainCli {
    /// The actual relay chain cli object.
    pub base: polkadot_cli::RunCmd,

    /// Optional chain id that should be passed to the relay chain.
    pub chain_id: Option<String>,

    /// The base path that should be used by the relay chain.
    pub base_path: Option<PathBuf>,
}

impl RelayChainCli {
    /// Parse the relay chain CLI parameters using the para chain `Configuration`.
    pub fn new<'a>(
        para_config: &sc_service::Configuration,
        relay_chain_args: impl Iterator<Item = &'a String>,
    ) -> Self {
        let extension = crate::chain_spec::Extensions::try_get(&*para_config.chain_spec);
        let chain_id = extension.map(|e| e.relay_chain.clone());
        let base_path = para_config
            .base_path
            .as_ref()
            .map(|x| x.path().join("polkadot"));
        Self {
            base_path,
            chain_id,
            base: polkadot_cli::RunCmd::from_iter(relay_chain_args),
        }
    }
}
