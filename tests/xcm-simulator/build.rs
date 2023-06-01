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

/// build.rs
/// Compile and copy the contract artifacts to be used as fixture
/// in tests
use std::{
    fs, io,
    path::{Path, PathBuf},
};

use contract_build::{
    BuildArtifacts, BuildMode, Features, ManifestPath, Network, OptimizationPasses, OutputType,
    Target, UnstableFlags, Verbosity,
};

const DEFAULT_FIXTURES_DIR: &'static str = "./fixtures";
const DEFAULT_CONTRACTS_DIR: &'static str = "./contracts";

/// Execute the clousre with given directory as current dir
fn with_directory<T, F: FnOnce() -> T>(dir: &Path, f: F) -> T {
    let curr_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(dir).unwrap();
    let res = f();
    std::env::set_current_dir(curr_dir).unwrap();

    res
}

/// Build config for adjusting the ink! contract compilation
struct BuildConfig {
    /// Directory where artifacts will be copied to after compilation
    fixtures_dir: PathBuf,
    /// Directory where individual contract are present, each on it's own sub-directory
    /// All the sub-folders inside this directory will be treated as ink! contracts.
    contracts_dir: PathBuf,
    is_verbose: bool,
    /// Whether to build the metadata json along with WASM blob
    build_metadata: bool,
    /// Skip Wasm post build validation
    skip_wasm_validation: bool,
}

impl BuildConfig {
    fn from_env() -> Self {
        Self {
            fixtures_dir: PathBuf::from(
                std::env::var("CB_FIXTURES_DIR").unwrap_or(DEFAULT_FIXTURES_DIR.to_string()),
            ),
            contracts_dir: PathBuf::from(
                std::env::var("CB_CONTRACTS_DIR").unwrap_or(DEFAULT_CONTRACTS_DIR.to_string()),
            ),
            is_verbose: std::env::var("CB_BUILD_VERBOSE").is_ok(),
            build_metadata: std::env::var("CB_BUILD_METADATA").is_ok(),
            skip_wasm_validation: std::env::var("CB_SKIP_WASM_VALIDATION").is_ok(),
        }
    }

    /// Build the contracts and copy the artifacts to fixtures dir
    fn build_contracts(&self) -> io::Result<()> {
        for dir in self.contracts_dir.read_dir()? {
            let dir = dir?;
            let contract = dir.file_name().to_os_string().into_string().unwrap();
            println!("[build.rs] Building Contract - {contract}");
            let build = with_directory(&dir.path(), || {
                let manifest_path = ManifestPath::new("Cargo.toml").unwrap();
                let verbosity = if self.is_verbose {
                    Verbosity::Verbose
                } else {
                    Verbosity::Default
                };
                let build_artifact = if self.build_metadata {
                    BuildArtifacts::All
                } else {
                    BuildArtifacts::CodeOnly
                };
                let args = contract_build::ExecuteArgs {
                    manifest_path,
                    verbosity,
                    build_artifact,
                    skip_wasm_validation: self.skip_wasm_validation,
                    build_mode: BuildMode::Debug,
                    features: Features::default(),
                    network: Network::Online,
                    unstable_flags: UnstableFlags::default(),
                    optimization_passes: Some(OptimizationPasses::default()),
                    keep_debug_symbols: true,
                    lint: false,
                    output_type: OutputType::HumanReadable,
                    target: Target::Wasm,
                };
                contract_build::execute(args)
                    .expect(&format!("Failed to build contract at - {dir:?}"))
            });

            // copy wasm artifact
            fs::copy(
                build.dest_wasm.unwrap(),
                self.fixtures_dir.join(format!("{contract}.wasm")),
            )
            .unwrap();

            // copy metadata
            if let Some(res) = build.metadata_result {
                fs::copy(
                    res.dest_metadata,
                    self.fixtures_dir.join(format!("{contract}.json")),
                )
                .unwrap();
            }
        }

        Ok(())
    }
}

fn main() {
    let config = BuildConfig::from_env();
    // create fixtures dir if not exists
    fs::create_dir_all(&config.fixtures_dir).unwrap();

    // build all the contracts
    config.build_contracts().unwrap();

    println!(
        "cargo:rerun-if-changed={}",
        config.contracts_dir.to_str().unwrap()
    );
}
