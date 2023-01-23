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

use regex::Regex;

/// Astar XCM tools.
#[derive(Debug, clap::Parser)]
#[clap(subcommand_required = true)]
pub struct Cli {
    /// Possible subcommand with parameters.
    #[clap(subcommand)]
    pub subcommand: Option<Subcommand>,
}

/// Possible subcommands of the main binary.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Prints relay-chain AccountId
    RelayChainAccount,
    /// Prints parachain AccountId.
    ParachainAccount(ParachainAccountCmd),
    /// Prints AssetId for desired parachain asset.
    AssetId(AssetIdCmd),
    /// Prints Account32Hash for the derived multilocation.
    /// In case parachain-id is provided, multilocation is in format { parents: 1, X2(Parachain, AccountId32) }.
    /// In case parachain-id is omitted, multilocation is in format  { parents: 1, X1(AccountId32) }.
    Account32Hash(Account32HashCmd),
}

/// Helper that prints AccountId of parachain.
#[derive(Debug, clap::Parser)]
pub struct ParachainAccountCmd {
    /// Print address for sibling parachain [child by default].
    #[clap(short)]
    pub sibling: bool,

    /// Target ParaId.
    pub parachain_id: u32,
}

/// Helper that prints AssetId for sibling parachain asset.
#[derive(Debug, clap::Parser)]
pub struct AssetIdCmd {
    /// External AssetId [relay by default].
    #[clap(default_value = "340282366920938463463374607431768211455")]
    pub asset_id: u128,
}

/// Helper that prints AccountId32 hash value for the derived multilocation.
#[derive(Debug, clap::Parser)]
pub struct Account32HashCmd {
    /// Parachain id in case sender is from a sibling parachain.
    #[clap(short, long, default_value = None)]
    pub parachain_id: Option<u32>,
    /// AccountId32 (SS58 scheme, public key) of the sender account.
    #[clap(short, long, value_parser = account_id_32_parser)]
    pub account_id_32: [u8; 32],
    /// NetworkId of the AccountId32 - if not provided, will be set to `Any`
    #[clap(short, long)]
    pub network_id: Option<String>,
}

/// Used to parse AccountId32 as [u8; 32] from the received string.
fn account_id_32_parser(account_str: &str) -> Result<[u8; 32], String> {
    let re = Regex::new(r"^0x([0-9a-fA-F]{64})$").map_err(|e| e.to_string())?;
    if !re.is_match(account_str) {
        return Err(
            "Invalid AccountId32 received. Expected format is '0x1234...4321' (64 hex digits)."
                .into(),
        );
    }

    let hex_acc = re
        .captures(account_str)
        .expect("Regex match confirmed above.")
        .get(1)
        .expect("Group 1 confirmed in match above.")
        .as_str();
    let decoded_hex = hex::decode(hex_acc).expect("Regex ensures correctness; infallible.");

    TryInto::<[u8; 32]>::try_into(decoded_hex)
        .map_err(|_| "Failed to create [u8; 32] array from received account Id string.".into())
}
