//! Astar XCM CLI handlers.

use crate::cli::*;

use clap::Parser;
use cumulus_primitives_core::ParaId;
use polkadot_parachain::primitives::{AccountIdConversion, Sibling};
use polkadot_primitives::v2::AccountId;
use sp_core::hexdisplay::HexDisplay;
use xcm::latest::prelude::*;
use xcm_builder::SiblingParachainConvertsVia;
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
                AccountIdConversion::<AccountId>::into_account(&para_id)
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
        None => {}
    }
    Ok(())
}
