// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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

use crate::rpc::FrontierBackendType;
use clap::Parser;

/// Defines the frontier backend configuration.
#[derive(Clone)]
pub enum FrontierBackendConfig {
    KeyValue,
    Sql {
        pool_size: u32,
        num_ops_timeout: u32,
        thread_count: u32,
        cache_size: u64,
    },
}

impl Default for FrontierBackendConfig {
    fn default() -> FrontierBackendConfig {
        FrontierBackendConfig::KeyValue
    }
}

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
/// Overall Frontier (EVM compatibility) configuration:
/// Controls enabled APIs, tracing, and backend storage.
pub struct FrontierConfig {
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
    /// Configuration for the frontier db backend.
    pub frontier_backend_config: FrontierBackendConfig,
}

#[derive(Debug, Parser)]
pub struct EthApiOptions {
    /// Enable EVM tracing module on a non-authority node.
    #[cfg_attr(
        not(feature = "manual-seal"),
        clap(
            long,
            conflicts_with = "collator",
            conflicts_with = "validator",
            value_delimiter = ','
        )
    )]
    #[cfg_attr(feature = "manual-seal", clap(long))]
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

    /// Sets the backend type (KeyValue or Sql)
    #[clap(long, value_enum, ignore_case = true, default_value_t = FrontierBackendType::default())]
    pub frontier_backend_type: FrontierBackendType,

    /// Sets the SQL backend's pool size.
    #[arg(long, default_value = "100")]
    pub frontier_sql_backend_pool_size: u32,

    /// Sets the SQL backend's query timeout in number of VM ops.
    #[clap(long, default_value = "10000000")]
    pub frontier_sql_backend_num_ops_timeout: u32,

    /// Sets the SQL backend's auxiliary thread limit.
    #[clap(long, default_value = "4")]
    pub frontier_sql_backend_thread_count: u32,

    /// Sets the SQL backend's query timeout in number of VM ops.
    /// Default value is 200MB.
    #[clap(long, default_value = "209715200")]
    pub frontier_sql_backend_cache_size: u64,
}

impl EthApiOptions {
    pub fn new_rpc_config(&self) -> FrontierConfig {
        FrontierConfig {
            ethapi: self.ethapi.clone(),
            ethapi_max_permits: self.ethapi_max_permits,
            ethapi_trace_max_count: self.ethapi_trace_max_count,
            ethapi_trace_cache_duration: self.ethapi_trace_cache_duration,
            eth_log_block_cache: self.eth_log_block_cache,
            eth_statuses_cache: self.eth_statuses_cache,
            max_past_logs: self.max_past_logs,
            tracing_raw_max_memory_usage: self.tracing_raw_max_memory_usage,
            frontier_backend_config: match self.frontier_backend_type {
                FrontierBackendType::KeyValue => FrontierBackendConfig::KeyValue,
                FrontierBackendType::Sql => FrontierBackendConfig::Sql {
                    pool_size: self.frontier_sql_backend_pool_size,
                    num_ops_timeout: self.frontier_sql_backend_num_ops_timeout,
                    thread_count: self.frontier_sql_backend_thread_count,
                    cache_size: self.frontier_sql_backend_cache_size,
                },
            },
        }
    }
}
