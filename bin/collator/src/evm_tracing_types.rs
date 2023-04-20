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

/// EVM tracing CLI flags.
#[derive(Debug, PartialEq, Clone)]
pub enum EthApi {
    /// Enable EVM debug RPC methods.
    Debug,
    /// Enable EVM trace RPC methods.
    Trace,
    /// Enable pending transactions RPC methods.
    TxPool,
}

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

#[allow(dead_code)]
#[derive(Clone)]
/// EVM tracing CLI config.
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

#[derive(Debug, Parser)]
pub struct EthApiOptions {
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
}
