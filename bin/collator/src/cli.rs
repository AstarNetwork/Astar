// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

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
    #[cfg(feature = "evm-tracing")]
    #[clap(
        long,
        conflicts_with = "collator",
        conflicts_with = "validator",
        value_delimiter = ','
    )]
    pub ethapi: Vec<EthApi>,

    /// Number of concurrent tracing tasks. Meant to be shared by both "debug" and "trace" modules.
    #[cfg(feature = "evm-tracing")]
    #[clap(long, default_value = "10")]
    pub ethapi_max_permits: u32,

    /// Maximum number of trace entries a single request of `trace_filter` is allowed to return.
    /// A request asking for more or an unbounded one going over this limit will both return an
    /// error.
    #[cfg(feature = "evm-tracing")]
    #[clap(long, default_value = "500")]
    pub ethapi_trace_max_count: u32,

    /// Duration (in seconds) after which the cache of `trace_filter` for a given block will be
    /// discarded.
    #[cfg(feature = "evm-tracing")]
    #[clap(long, default_value = "300")]
    pub ethapi_trace_cache_duration: u64,

    /// Size in bytes of the LRU cache for block data.
    #[cfg(feature = "evm-tracing")]
    #[clap(long, default_value = "300000000")]
    pub eth_log_block_cache: usize,

    /// Size in bytes of the LRU cache for transactions statuses data.
    #[cfg(feature = "evm-tracing")]
    #[clap(long, default_value = "300000000")]
    pub eth_statuses_cache: usize,

    /// Size in bytes of data a raw tracing request is allowed to use.
    /// Bound the size of memory, stack and storage data.
    #[cfg(feature = "evm-tracing")]
    #[clap(long, default_value = "20000000")]
    pub tracing_raw_max_memory_usage: usize,

    /// Maximum number of logs in a query.
    #[cfg(feature = "evm-tracing")]
    #[clap(long, default_value = "10000")]
    pub max_past_logs: u32,

    /// Enable Ethereum compatible JSON-RPC servers (disabled by default).
    #[clap(name = "enable-evm-rpc", long)]
    pub enable_evm_rpc: bool,

    /// Relaychain arguments
    #[clap(raw = true)]
    pub relaychain_args: Vec<String>,

    /// Proposer's maximum block size limit in bytes
    #[clap(long, default_value = sc_basic_authorship::DEFAULT_BLOCK_SIZE_LIMIT.to_string())]
    pub proposer_block_size_limit: usize,

    /// Proposer's soft deadline in percents of block size
    #[clap(long, default_value = "50")]
    pub proposer_soft_deadline_percent: u8,
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

/// EVM tracing CLI flags.
#[cfg(feature = "evm-tracing")]
#[derive(Debug, PartialEq, Clone)]
pub enum EthApi {
    /// Enable EVM debug RPC methods.
    Debug,
    /// Enable EVM trace RPC methods.
    Trace,
    /// Enable pending transactions RPC methods.
    TxPool,
}

#[cfg(feature = "evm-tracing")]
impl std::str::FromStr for EthApi {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "debug" => Self::Debug,
            "trace" => Self::Trace,
            "txpool" => Self::TxPool,
            _ => {
                return Err(format!(
                    "`{}` is not recognized as a supported Ethereum Api",
                    s
                ))
            }
        })
    }
}

/// EVM tracing CLI config.
#[cfg(feature = "evm-tracing")]
pub struct EvmTracingConfig {
    /// Enabled EVM tracing flags.
    pub ethapi: Vec<EthApi>,
    /// Number of concurrent tracing tasks.
    pub ethapi_max_permits: u32,
    /// Maximum number of trace entries a single request of `trace_filter` is allowed to return.
    /// A request asking for more or an unbounded one going over this limit will both return an
    /// error.
    pub ethapi_trace_max_count: u32,
    /// Duration (in seconds) after which the cache of `trace_filter` for a given block will be
    /// discarded.
    pub ethapi_trace_cache_duration: u64,
    /// Size in bytes of the LRU cache for block data.
    pub eth_log_block_cache: usize,
    /// Size in bytes of the LRU cache for transactions statuses data.
    pub eth_statuses_cache: usize,
    /// Maximum number of logs in a query.
    pub max_past_logs: u32,
    /// Size in bytes of data a raw tracing request is allowed to use.
    /// Bound the size of memory, stack and storage data.
    pub tracing_raw_max_memory_usage: usize,
}
