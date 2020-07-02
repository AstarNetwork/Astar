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
pub fn lock_method_check(input: &[u8], duration: u64) -> bool {
    let method = [0x66, 0xdf, 0xbf, 0xb4]; // lock(uint256,address) signature
    let mut encoded_duration: [u8; 32] = [0; 32]; // duration in days
    U256::from(duration / 86400).to_big_endian(&mut encoded_duration);
    input[0..4] == method && input[4..36] == encoded_duration
}
