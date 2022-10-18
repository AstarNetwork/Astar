//! Astar XCM CLI handlers.

use crate::cli::*;

use clap::Parser;
use cumulus_primitives_core::ParaId;
use polkadot_parachain::primitives::Sibling;
use polkadot_primitives::v2::AccountId;
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::{AccountIdConversion, Get};
use xcm::latest::prelude::*;
use xcm_builder::{Account32Hash, SiblingParachainConvertsVia};
use xcm_executor::traits::Convert;

/// CLI error type.
pub type Error = String;

/// Parse command line arguments into service configuration.
pub fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    match &cli.subcommand {
        Some(Subcommand::ParachainAccount(cmd)) => {
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
        Some(Subcommand::Account32Hash(cmd)) => {
            let sender_multilocation = if let Some(parachain_id) = cmd.parachain_id {
                MultiLocation {
                    parents: 1,
                    interior: X2(
                        Parachain(parachain_id),
                        AccountId32 {
                            network: NetworkId::Any,
                            id: cmd.account_id_32,
                        },
                    ),
                }
            } else {
                MultiLocation {
                    parents: 1,
                    interior: X1(AccountId32 {
                        network: NetworkId::Any,
                        id: cmd.account_id_32,
                    }),
                }
            };

            // Not important for the functionality, totally redundant
            struct AnyNetwork;
            impl Get<NetworkId> for AnyNetwork {
                fn get() -> NetworkId {
                    NetworkId::Any
                }
            }

            let derived_acc =
                Account32Hash::<AnyNetwork, AccountId>::convert_ref(&sender_multilocation).unwrap();
            println!("{}", derived_acc);
        }
        None => {}
    }
    Ok(())
}
