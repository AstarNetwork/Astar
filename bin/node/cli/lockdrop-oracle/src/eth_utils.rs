//! Ethereum locking helpers.

use sp_core::{keccak_256, H160, U256};

/// Get Ethereum address for given ECDSA public key.
pub fn to_address(public: &[u8]) -> Option<H160> {
    if let Ok(pubkey) = secp256k1::PublicKey::parse_slice(public, None) {
        let address = H160::from_slice(&keccak_256(&pubkey.serialize()[1..])[12..32]);
        Some(address)
    } else {
        None
    }
}

/// Compile smart contract input for lock value on given duration.
pub fn lock_method(duration: u64) -> Vec<u8> {
    // Lock method signature
    let method = keccak_256("lock(uint256,address)".as_bytes());
    // Duration in days
    let duration_param = U256::from(duration / 86400);
    // Transaction input
    ethabi::encode(&[
        ethabi::Token::FixedBytes(method[0..4].to_vec()),
        ethabi::Token::Uint(duration_param),
    ])
}
