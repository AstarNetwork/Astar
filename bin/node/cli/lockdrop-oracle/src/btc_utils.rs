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
pub fn lock_script(public: &ecdsa::Public, duration: u32) -> Script {
    let public_key = PublicKey::from_slice(public.as_ref()).unwrap();
    let blocks = bip68_encode(duration) as i64;
    let script = bitcoin_script! {
        <blocks>
        OP_CSV
        OP_DROP
        <public_key>
        OP_CHECKSIG
    };
    script.to_p2sh()
}

pub fn to_address(public: &ecdsa::Public) -> String {
    let public_key = PublicKey::from_slice(public.as_ref()).unwrap();
    Address::p2pkh(&public_key, Network::Testnet).to_string()
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
        let duration = 10;
        let script = lock_script(&public, duration);
        let address = Address::from_script(&script, Network::Testnet).unwrap();
        assert_eq!(address.to_string(), "2MuJcWGWe8XkPc6h7pt6vQDyaTwDZxKJZ8p");
    }

    #[test]
    fn test_to_address() {
        let public = ecdsa::Public::from_full(
            &hex!["0431e12c2db27f3b07fcc560cdbff90923bf9b5b03769103a44b38426f9469172f3eef59e4f01df729428161c33ec5b32763e2e5a0072551b7808ae9d89286b37b"][..]
        ).unwrap();
        assert_eq!(to_address(&public), "mzUQaN6vnYDYNNYJVpRz2ipxLcWsQg6b8z");
    }
}
