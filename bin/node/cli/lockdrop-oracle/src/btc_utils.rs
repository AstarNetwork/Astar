use codec::Encode;
use ripemd160::{Digest, Ripemd160};
use sp_core::{ecdsa, hashing::sha2_256};

pub const BTC_TESTNET: u8 = 0x6f;
//pub const BTC_MAINNET: u8 = 0x00;

/// Bitcoin RIPEMD160 hashing function.
pub fn ripemd160(data: &[u8]) -> [u8; 20] {
    let mut hasher = Ripemd160::new();
    hasher.input(data);
    let mut output = [0u8; 20];
    output.copy_from_slice(&hasher.result());
    output
}

/// Compile BTC sequence lock script for givent public key and duration in blocks.
pub fn lock_script(public: &ecdsa::Public, duration: u64) -> Vec<u8> {
    let blocks = duration / 600;
    let full_public = secp256k1::PublicKey::parse_slice(public.as_ref(), None)
        .expect("public key has correct length")
        .serialize();
    blocks.using_encoded(|enc_blocks| {
        let mut output = vec![];
        output.extend(vec![enc_blocks.len() as u8]); // Lock duration length
        output.extend(enc_blocks); // Lock duration in blocks
        output.extend(vec![0x27, 0x55]); // OP_CHECKSEQUENCEVERIFY OP_DROP
        output.extend(vec![full_public.len() as u8 - 1]); // Public key lenght
        output.extend(&full_public[1..]); // Public key
        output.extend(vec![0xAC]); // OP_CHECKSIG
        output
    })
}

/// Get hash of binary BTC script.
pub fn script_hash(script: &[u8]) -> [u8; 20] {
    ripemd160(&sha2_256(script)[..])
}

/// Compile BTC pay-to-script-hash script for given script hash.
pub fn p2sh(script_hash: &[u8; 20]) -> Vec<u8> {
    let mut output = vec![];
    output.extend(vec![0xa9, 0x14]); // OP_HASH160 20
    output.extend(script_hash); // <scriptHash>
    output.extend(vec![0x87]); // OP_EQUAL
    output
}

/// Get Bitcoin addres for given ECDSA public key and network tag.
/// Note: It works for `1`-prefixed addresses
pub fn to_address(public_key: &ecdsa::Public, network: u8) -> String {
    let mut key_hash = vec![network];
    key_hash.extend(&ripemd160(&sha2_256(public_key.as_ref())[..])[..]);
    let check_sum = sha2_256(&sha2_256(&key_hash)[..]);
    key_hash.extend(&check_sum[0..4]);
    bs58::encode(key_hash).into_string()
}
