//! Bitcoin locking helpers.

use bitcoin::blockdata::script::Script;
use bitcoin::network::constants::Network;
use bitcoin::util::address::Address;
use bitcoin::util::key::PublicKey;
use bitcoin_script::bitcoin_script;
use sp_core::ecdsa;

/// Encode block delay in BIP68 standard
fn bip68_encode(blocks: u32) -> u32 {
    0x0000ffff & blocks
}

/// Compile BTC sequence lock script for givent public key and duration in blocks.
fn lock_script(public: &ecdsa::Public, duration: u32) -> Result<Script, String> {
    if duration > 0 && duration < 65535 {
        let public_key = PublicKey::from_slice(public.as_ref()).unwrap();
        let blocks = bip68_encode(duration) as i64;
        let script = bitcoin_script! {
            <blocks>
            OP_CSV
            OP_DROP
            <public_key>
            OP_CHECKSIG
        };
        Ok(script.to_p2sh())
    } else {
        return Err("Lock duration sanity check failed".to_string());
    }
}

/// Create lock script address from given params.
pub fn lock_script_address(public: &ecdsa::Public, duration: u32) -> Result<String, String> {
    let script = lock_script(public, duration)?;
    let address = Address::from_script(&script, Network::Testnet).unwrap();
    Ok(address.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use sp_core::crypto::Public;

    #[test]
    fn test_lock_scipt() {
        let public = ecdsa::Public::from_slice(
            &hex!["038ea27103fb646a2cea9eca9080737e0b23640caaaef2853416c9b286b353313e"][..],
        );
        assert_eq!(
            lock_script(&public, 0),
            Err("Lock duration sanity check failed".to_string())
        );

        let duration = 10;
        let script = lock_script(&public, duration).unwrap();
        let address = Address::from_script(&script, Network::Testnet).unwrap();
        assert_eq!(address.to_string(), "2MuJcWGWe8XkPc6h7pt6vQDyaTwDZxKJZ8p");
    }
}
