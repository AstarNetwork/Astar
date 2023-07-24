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
    /// Prints relay-chain SS58 account Id
    RelayChainAccount,
    /// Prints parachains sovereign SS58 account Id.
    SovereignAccount(SovereignAccountCmd),
    /// Prints AssetId for desired parachain asset.
    AssetId(AssetIdCmd),
    /// Prints derived remote SS58 account for the derived multilocation.
    RemoteAccount(RemoteAccountCmd),
}

/// Helper that prints AccountId of parachain.
#[derive(Debug, clap::Parser)]
pub struct SovereignAccountCmd {
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

/// Helper that prints the derived AccountId32 value for the multilocation.
#[derive(Debug, clap::Parser)]
pub struct RemoteAccountCmd {
    /// Parachain id in case sender is from a sibling parachain.
    #[clap(short, long, default_value = None)]
    pub parachain_id: Option<u32>,
    /// Public key (SS58 or H160) in hex format. Must be either 32 or 20 bytes long.
    #[clap(short, long)]
    pub account_key: AccountWrapper,
}

#[derive(Debug, Clone)]
pub struct AccountWrapper {
    account: [u8; 32],
    is_32: bool,
}

impl AccountWrapper {
    /// Get AccountId32 public key (SS58) or error if it is not 32 bytes long.
    pub fn get_account_id_32(&self) -> Result<[u8; 32], &str> {
        if self.is_32 {
            Ok(self.account)
        } else {
            Err("Account is not 32 bytes long")
        }
    }

    /// Get AccountKey20 public key (H160) or error if it is not 20 bytes long.
    pub fn get_account_key_20(&self) -> Result<[u8; 20], &str> {
        if !self.is_32 {
            let mut account = [0u8; 20];
            account.copy_from_slice(&self.account[0..20]);
            Ok(account)
        } else {
            Err("Account is not 20 bytes long")
        }
    }

    /// `true` if AccountId32, `false` if AccountKey20.
    pub fn is_32_bytes(&self) -> bool {
        self.is_32
    }
}

impl std::str::FromStr for AccountWrapper {
    type Err = String;

    fn from_str(account_pub_key: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = account_pub_key.strip_prefix("0x") {
            if let Some(pos) = rest.chars().position(|c| !c.is_ascii_hexdigit()) {
                Err(format!(
					"Expected account public key in hex format, found illegal hex character at position: {}",
					2 + pos,
				))
            } else {
                if rest.len() == 40 {
                    let mut account = [0u8; 32];
                    account[0..20].copy_from_slice(&hex::decode(rest).unwrap());
                    Ok(AccountWrapper {
                        account,
                        is_32: false,
                    })
                } else if rest.len() == 64 {
                    let mut account = [0u8; 32];
                    account.copy_from_slice(&hex::decode(rest).unwrap());
                    Ok(AccountWrapper {
                        account,
                        is_32: true,
                    })
                } else {
                    Err("Account key should be 20 or 32 bytes long".into())
                }
            }
        } else {
            Err("Account key should start with '0x'".into())
        }
    }
}
