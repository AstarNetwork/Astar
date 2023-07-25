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

//! Astar XCM CLI handlers.

use crate::cli::*;

use clap::Parser;
use cumulus_primitives_core::ParaId;
use polkadot_parachain::primitives::Sibling;
use polkadot_primitives::AccountId;
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::AccountIdConversion;
use xcm::latest::prelude::*;
use xcm_builder::{ParentIsPreset, SiblingParachainConvertsVia};
use xcm_executor::traits::Convert;

use astar_primitives::xcm::{DescribeAllTerminal, DescribeFamily, HashedDescription};

/// CLI error type.
pub type Error = String;

/// Parse command line arguments into service configuration.
pub fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    match &cli.subcommand {
        Some(Subcommand::RelayChainAccount) => {
            let relay_account =
                ParentIsPreset::<AccountId>::convert_ref(&MultiLocation::parent()).unwrap();
            println!("{}", relay_account);
        }
        Some(Subcommand::SovereignAccount(cmd)) => {
            let parachain_account = if cmd.sibling {
                let location = MultiLocation {
                    parents: 1,
                    interior: X1(Parachain(cmd.parachain_id)),
                };
                SiblingParachainConvertsVia::<Sibling, AccountId>::convert_ref(&location).unwrap()
            } else {
                let para_id = ParaId::from(cmd.parachain_id);
                AccountIdConversion::<AccountId>::into_account_truncating(&para_id)
            };
            println!("{}", parachain_account);
        }
        Some(Subcommand::AssetId(cmd)) => {
            const ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8; 4];
            let mut data = [0u8; 20];
            data[0..4].copy_from_slice(ASSET_PRECOMPILE_ADDRESS_PREFIX);
            data[4..20].copy_from_slice(&cmd.asset_id.to_be_bytes());
            println!("pallet_assets: {}", cmd.asset_id);
            println!("EVM XC20: 0x{}", HexDisplay::from(&data));
        }
        Some(Subcommand::RemoteAccount(cmd)) => {
            let mut sender_multilocation = MultiLocation::parent();

            if let Some(parachain_id) = cmd.parachain_id {
                sender_multilocation
                    .append_with(X1(Parachain(parachain_id)))
                    .expect("infallible, short sequence");
            }

            match cmd.account_key {
                AccountWrapper::SS58(id) => {
                    sender_multilocation
                        .append_with(X1(AccountId32 {
                            id,
                            // network is not relevant for account derivation
                            network: None,
                        }))
                        .expect("infallible, short sequence");
                }
                AccountWrapper::H160(key) => {
                    sender_multilocation
                        .append_with(X1(AccountKey20 {
                            key,
                            // network is not relevant for account derivation
                            network: None,
                        }))
                        .expect("infallible, short sequence");
                }
            }

            let derived_acc =
                HashedDescription::<AccountId, DescribeFamily<DescribeAllTerminal>>::convert(
                    sender_multilocation,
                );
            if let Ok(derived_acc) = derived_acc {
                println!("{}", derived_acc);
            } else {
                println!("Failed to derive account Id.");
            }
        }
        None => {}
    }
    Ok(())
}
