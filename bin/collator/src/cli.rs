use clap::Parser;
use cumulus_client_cli::CollatorOptions;
use sc_cli::{KeySubcommand, SignCmd, VanityCmd, VerifyCmd};
use std::path::PathBuf;
use url::Url;

/// An overarching CLI command definition.
#[derive(Debug, clap::Parser)]
pub struct Cli {
    /// Possible subcommand with parameters.
    #[clap(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[allow(missing_docs)]
    #[clap(flatten)]
    pub run: RunCmd,

    /// Relaychain arguments
    #[clap(raw = true)]
    pub relaychain_args: Vec<String>,
}

/// Possible subcommands of the main binary.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Key management cli utilities
    #[clap(subcommand)]
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
    ExportGenesisState(cumulus_client_cli::ExportGenesisStateCommand),

    /// Export the genesis wasm of the parachain.
    ExportGenesisWasm(cumulus_client_cli::ExportGenesisWasmCommand),

    /// The custom benchmark subcommmand benchmarking runtime pallets.
    #[cfg(feature = "runtime-benchmarks")]
    #[clap(name = "benchmark", about = "Benchmark runtime pallets.")]
    #[clap(subcommand)]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),

    /// Try some command against runtime state.
    #[cfg(feature = "try-runtime")]
    TryRuntime(try_runtime_cli::TryRuntimeCmd),
}

fn validate_relay_chain_url(arg: &str) -> Result<Url, String> {
    let url = Url::parse(arg).map_err(|e| e.to_string())?;

    if url.scheme() == "ws" {
        Ok(url)
    } else {
        Err(format!(
            "'{}' URL scheme not supported. Only websocket RPC is currently supported",
            url.scheme()
        ))
    }
}

#[allow(missing_docs)]
#[derive(Debug, clap::Parser)]
pub struct RunCmd {
    #[allow(missing_docs)]
    #[clap(flatten)]
    pub base: cumulus_client_cli::RunCmd,

    /// Id of the parachain this collator collates for.
    ///
    /// Default: 2007 (shiden)
    #[clap(long, default_value = "2007")]
    pub parachain_id: u32,

    /// EXPERIMENTAL: Specify an URL to a relay chain full node to communicate with.
    #[clap(
		long,
		value_parser = validate_relay_chain_url,
		conflicts_with_all = &["alice", "bob", "charlie", "dave", "eve", "ferdie", "one", "two"]	)
	]
    pub relay_chain_rpc_url: Option<Url>,
}

impl std::ops::Deref for RunCmd {
    type Target = cumulus_client_cli::RunCmd;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl RunCmd {
    /// Create [`CollatorOptions`] representing options only relevant to parachain collator nodes
    pub fn collator_options(&self) -> CollatorOptions {
        CollatorOptions {
            relay_chain_rpc_url: self.relay_chain_rpc_url.clone(),
        }
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
        let extension = crate::parachain::chain_spec::Extensions::try_get(&*para_config.chain_spec);
        let chain_id = extension.map(|e| e.relay_chain.clone());
        let base_path = para_config
            .base_path
            .as_ref()
            .map(|x| x.path().join("polkadot"));
        Self {
            base_path,
            chain_id,
            base: polkadot_cli::RunCmd::parse_from(relay_chain_args),
        }
    }
}
