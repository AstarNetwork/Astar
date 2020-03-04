use sp_core::{keccak_256, ecdsa, U256};
use codec::Encode;

pub fn to_address(public: ecdsa::Public) -> [u8; 20] {
    let pubkey = secp256k1::PublicKey::parse_slice(public.as_ref(), None)
        .expect("ecdsa contains correct public key");
    let mut output = [0u8; 20];
    output.copy_from_slice(&keccak_256(&pubkey.serialize()[..]));
    output
}

pub fn lock_method(duration: u64) -> Vec<u8> {
    let mut method = vec![0xdd, 0x46, 0x70, 0x64];  // lock(uint256) signature
    method.extend(&U256::from(duration).encode()[..]);
    method
}
