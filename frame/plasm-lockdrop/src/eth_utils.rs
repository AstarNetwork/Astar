use tiny_keccak::{Keccak, Hasher};
use sp_core::{ecdsa, U256};
use sp_std::prelude::*;
use codec::Encode;

/// Do a keccak 256-bit hash and return result.
pub fn keccak_256(data: &[u8]) -> [u8; 32] {
	let mut keccak = Keccak::v256();
	keccak.update(data);
	let mut output = [0u8; 32];
	keccak.finalize(&mut output);
	output
}

pub fn to_address(public: ecdsa::Public) -> [u8; 20] {
    let pubkey = secp256k1::PublicKey::parse_slice(public.as_ref(), None)
        .expect("ecdsa contains correct public key");
    let mut output = [0u8; 20];
    output.copy_from_slice(&keccak_256(&pubkey.serialize()[..]));
    output
}

pub fn lock_method(duration: u64) -> Vec<u8> {
    let mut method = vec![0xdd, 0x46, 0x70, 0x64]; // lock(uint256) signature
    method.extend(&U256::from(duration).encode()[..]);
    method
}
