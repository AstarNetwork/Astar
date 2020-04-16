use ripemd160::{Ripemd160, Digest};
use sha2::Sha256;
use sp_core::ecdsa;
use sp_std::prelude::*;
use codec::Encode;

/// Do a sha2 256-bit hash and return result.
pub fn sha2_256(data: &[u8]) -> [u8; 32] {
	let mut hasher = Sha256::new();
	hasher.input(data);
	let mut output = [0u8; 32];
	output.copy_from_slice(&hasher.result());
	output
}

/// Bitcoin RIPEMD160 hashing function
pub fn ripemd160(data: &[u8]) -> [u8; 20] {
    let mut hasher = Ripemd160::new();
    hasher.input(data);
    let mut output = [0u8; 20];
    output.copy_from_slice(&hasher.result());
    output
}

/// Compile BTC sequence lock script for givent public key and duration
pub fn lock_script(public: ecdsa::Public, duration: u64) -> Vec<u8> {
    duration.using_encoded(|enc_duration| {
        let mut output = vec![];
        output.extend(vec![0x21]); // Public key lenght (should be 33 bytes)
        output.extend(public.as_ref()); // Public key
        output.extend(vec![0xad]); // OP_CHECKSIGVERIFY
        output.extend(vec![enc_duration.len() as u8]); // Lock duration length
        output.extend(enc_duration.as_ref()); // Lock duration in blocks
        output.extend(vec![0x27, 0x55, 0x01]); // OP_CHECKSEQUENCEVERIFY OP_DROP 1
        output
    })
}

/// Get hash of binary BTC script
pub fn script_hash(script: &[u8]) -> [u8; 20] {
    ripemd160(&sha2_256(script)[..])
}

/// Compile BTC pay-to-script-hash script for given script hash
pub fn p2sh(script_hash: &[u8; 20]) -> Vec<u8> {
    let mut output = vec![];
    output.extend(vec![0xa9, 0x14]); // OP_HASH160 20
    output.extend(script_hash); // <scriptHash>
    output.extend(vec![0x87]); // OP_EQUAL
    output
}
