use clap::Parser;
use std::path::PathBuf;

/// An overarching CLI command definition.
#[derive(Debug, clap::Parser)]
pub struct Cli {
    /// Possible subcommand with parameters.
    #[clap(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[allow(missing_docs)]
    #[clap(flatten)]
    pub run: cumulus_client_cli::RunCmd,

    /// Enable EVM tracing module on a non-authority node.
    #[clap(
        long,
        conflicts_with = "collator",
        conflicts_with = "validator",
        value_delimiter = ','
    )]
    pub ethapi: Vec<EthApi>,

    /// Number of concurrent tracing tasks. Meant to be shared by both "debug" and "trace" modules.
    #[clap(long, default_value = "10")]
    pub ethapi_max_permits: u32,

    /// Maximum number of trace entries a single request of `trace_filter` is allowed to return.
    /// A request asking for more or an unbounded one going over this limit will both return an
    /// error.
    #[clap(long, default_value = "500")]
    pub ethapi_trace_max_count: u32,

    /// Duration (in seconds) after which the cache of `trace_filter` for a given block will be
    /// discarded.
    #[clap(long, default_value = "300")]
    pub ethapi_trace_cache_duration: u64,

    /// Size in bytes of the LRU cache for block data.
    #[clap(long, default_value = "300000000")]
    pub eth_log_block_cache: usize,

    /// Size in bytes of the LRU cache for transactions statuses data.
    #[clap(long, default_value = "300000000")]
    pub eth_statuses_cache: usize,

    /// Size in bytes of data a raw tracing request is allowed to use.
    /// Bound the size of memory, stack and storage data.
    #[clap(long, default_value = "20000000")]
    pub tracing_raw_max_memory_usage: usize,

    /// Maximum number of logs in a query.
    #[clap(long, default_value = "10000")]
    pub max_past_logs: u32,

    /// Relaychain arguments
    #[clap(raw = true)]
    pub relaychain_args: Vec<String>,
}

/// Possible subcommands of the main binary.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Key management cli utilities
    #[clap(subcommand)]
    Key(sc_cli::KeySubcommand),

    /// Verify a signature for a message, provided on STDIN, with a given (public or secret) key.
    Verify(sc_cli::VerifyCmd),

    /// Generate a seed that provides a vanity address.
    Vanity(sc_cli::VanityCmd),

    /// Sign a message, with a given (secret) key.
    Sign(sc_cli::SignCmd),

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

#[derive(Debug, PartialEq, Clone)]
pub enum EthApi {
    Debug,
    Trace,
}

impl std::str::FromStr for EthApi {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "debug" => Self::Debug,
            "trace" => Self::Trace,
            _ => {
                return Err(format!(
                    "`{}` is not recognized as a supported Ethereum Api",
                    s
                ))
            }
        })
    }
}

pub struct EvmTracingConfig {
    pub ethapi: Vec<EthApi>,
    pub ethapi_max_permits: u32,
    pub ethapi_trace_max_count: u32,
    pub ethapi_trace_cache_duration: u64,
    pub eth_log_block_cache: usize,
    pub eth_statuses_cache: usize,
    pub max_past_logs: u32,
    pub tracing_raw_max_memory_usage: usize,
}
